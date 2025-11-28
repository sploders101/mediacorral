package opensubtitles

import (
	"bytes"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"os"
	"regexp"
	"slices"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/agnivade/levenshtein"
	"github.com/sploders101/mediacorral/backend/dbapi"
	"github.com/sploders101/mediacorral/backend/helpers/blobs"

	proto "github.com/sploders101/mediacorral/backend/gen/mediacorral/server/v1"
)

const (
	LOGIN_URL                 string        = "https://api.opensubtitles.com/api/v1/login"
	SEARCH_URL                string        = "https://api.opensubtitles.com/api/v1/subtitles"
	DOWNLOAD_URL              string        = "https://api.opensubtitles.com/api/v1/download"
	DEFAULT_TIMEOUT           time.Duration = time.Minute
	SUBTITLE_COMPARISON_LIMIT int           = 3
)

var (
	STRIP_SUBTITLE_REGEX *regexp.Regexp = regexp.MustCompile(
		`(?m)(?:<\s*[^>]*>|<\s*/\s*a>)|(?:^.*-->.*$|^[0-9]+$|[^a-zA-Z0-9 ?\.,!\n]|^\s*-*\s*|\r)`,
	)
	STRIP_WHITESPACE_REGEX *regexp.Regexp = regexp.MustCompile(`(?m)[\n ]{1,}`)
	ErrNoSubtitles                        = errors.New("no subtitles were found")
	ErrUnreliableSubtitles                = errors.New(
		"subtitles were found, but were inconsistent",
	)
)

type OstImporter struct {
	client http.Client

	apiKey   string
	username string
	password string

	authMutex    sync.Mutex
	authToken    string
	authIssuedAt time.Time
}

func NewOstImporter(apiKey, username, password string) (*OstImporter, error) {
	return &OstImporter{
		client: http.Client{
			Timeout: DEFAULT_TIMEOUT,
		},
		apiKey:   apiKey,
		username: username,
		password: password,
	}, nil
}

// Logs into OST and renews the authToken.
//
// This function should only be called while a mutex lock is held.
func (importer *OstImporter) login() error {
	loginCredsBuf, err := json.Marshal(struct {
		Username string `json:"username"`
		Password string `json:"password"`
	}{
		Username: importer.username,
		Password: importer.password,
	})
	if err != nil {
		return err
	}

	res, err := http.Post(LOGIN_URL, "application/json", bytes.NewBuffer(loginCredsBuf))
	if err != nil {
		return err
	}
	defer func() {
		_ = res.Body.Close()
	}()

	var response struct {
		Token string `json:"token"`
	}
	if err := json.NewDecoder(res.Body).Decode(&response); err != nil {
		return err
	}

	importer.authToken = response.Token
	importer.authIssuedAt = time.Now()

	return nil
}

// Adds authentication details to a request and runs it, handling token
// expiration automatically.
//
// `authMutex` is handled internally, unlike `login`. Calling this function
// with a locked `authMutex` will deadlock.
func (importer *OstImporter) runAuthenticated(
	req *http.Request,
	returnNon200 bool,
) (*http.Response, error) {
	importer.authMutex.Lock()
	if importer.authToken == "" {
		if err := importer.login(); err != nil {
			importer.authMutex.Unlock()
			return nil, err
		}
	}
	lastRefresh := importer.authIssuedAt
	authToken := importer.authToken
	importer.authMutex.Unlock()

	for {
		// Make request
		req.Header.Set("user-agent", "Mediacorral v1.0.0")
		req.Header.Set("api-key", importer.apiKey)
		req.Header.Set("authorization", "Bearer "+authToken)
		res, err := importer.client.Do(req)
		if err != nil {
			return nil, err
		}

		// Validate request
		switch res.StatusCode {
		case 200:
			return res, nil
		case 401:
			// We are unauthorized. Our token must have expired.
			importer.authMutex.Lock()
			if lastRefresh.Compare(importer.authIssuedAt) >= 0 {
				// Token was not renewed asynchronously. Current token is invalid.
				// Refresh it.
				err := importer.login()
				importer.authMutex.Unlock()
				if err != nil {
					return nil, err
				}
			} else {
				// Token was already renewed. Update our local cache & unlock.
				lastRefresh = importer.authIssuedAt
				authToken = importer.authToken
				importer.authMutex.Unlock()
			}
		default:
			if returnNon200 {
				return res, nil
			} else {
				return nil, fmt.Errorf("received status code %d from %s", res.StatusCode, req.URL.String())
			}
		}
	}
}

// Makes a request (with authentication) and automatically decodes the response
func (importer *OstImporter) makeRequest(req *http.Request, respValue any) error {
	resp, err := importer.runAuthenticated(req, false)
	if err != nil {
		return err
	}
	defer func() {
		_ = resp.Body.Close()
	}()
	if !slices.Contains(
		[]string{"application/json", "application/json; charset=utf-8"},
		resp.Header.Get("content-type"),
	) {
		bodyBytes, _ := io.ReadAll(io.LimitReader(resp.Body, 4096))
		slog.Debug(
			"Received non-json response from OST API",
			"url", req.URL.String(),
			"contentType", resp.Header.Get("content-type"),
			"response", base64.RawStdEncoding.EncodeToString(bodyBytes),
		)
		return fmt.Errorf("received non-json response from %s", req.URL.String())
	}
	if err := json.NewDecoder(resp.Body).Decode(respValue); err != nil {
		return err
	}
	return nil
}

func (importer *OstImporter) FindSubtitles(tmdbId int32) ([]SubtitleSummary, error) {
	req, err := http.NewRequest("GET", SEARCH_URL, nil)
	if err != nil {
		return nil, err
	}

	query := req.URL.Query()
	query.Set("tmdb_id", strconv.FormatInt(int64(tmdbId), 10))
	req.URL.RawQuery = query.Encode()

	var results ostSearchResults
	if err := importer.makeRequest(req, &results); err != nil {
		return nil, err
	}

	var files []SubtitleSummary
	for _, subtitle := range results.Data {
		// Filters
		if subtitle.Attributes.Language == nil {
			continue
		}
		if *subtitle.Attributes.Language != "en" {
			continue
		}

		lang := *subtitle.Attributes.Language

		for _, file := range subtitle.Attributes.Files {
			files = append(files, SubtitleSummary{
				Name: fmt.Sprintf(
					"lang: %s, name: %s, uploader: %s (%s)",
					lang,
					file.FileName,
					subtitle.Attributes.Uploader.Name,
					subtitle.Attributes.Uploader.Rank,
				),
				DownloadCount:    subtitle.Attributes.DownloadCount,
				NewDownloadCount: subtitle.Attributes.NewDownloadCount,
				FileId:           file.FileId,
				Uploader:         subtitle.Attributes.Uploader,
			})
		}
	}

	slices.SortFunc(files, func(a, b SubtitleSummary) int {
		switch {
		case a.Uploader.Rank == "Admin Warning":
			return 1
		case b.Uploader.Rank == "Admin Warning":
			return -1
		case a.NewDownloadCount < b.NewDownloadCount:
			return 1
		case a.NewDownloadCount > b.NewDownloadCount:
			return -1
		case a.DownloadCount < b.DownloadCount:
			return 1
		case a.DownloadCount > b.DownloadCount:
			return -1
		}
		aNumRank := numericRank(a.Uploader.Rank)
		bNumRank := numericRank(b.Uploader.Rank)
		return aNumRank - bNumRank
	})

	return files, nil
}

// Downloads a set of subtitles, returning them as a string
func (importer *OstImporter) DownloadSubtitles(fileId uint32) (string, error) {
	bodyBuf, err := json.Marshal(struct {
		FileId uint32 `json:"file_id"`
	}{
		FileId: fileId,
	})
	if err != nil {
		return "", err
	}
	req, err := http.NewRequest("POST", DOWNLOAD_URL, bytes.NewReader(bodyBuf))
	if err != nil {
		return "", err
	}

	var results struct {
		Link string `json:"link"`
	}
	if err := importer.makeRequest(req, &results); err != nil {
		return "", err
	}

	if results.Link == "" {
		return "", errors.New("OST returned an empty link")
	}

	req, err = http.NewRequest("GET", results.Link, nil)
	if err != nil {
		return "", err
	}
	res, err := importer.runAuthenticated(req, false)
	if err != nil {
		return "", err
	}

	subs, err := io.ReadAll(res.Body)
	if err != nil {
		return "", err
	}
	return string(subs), nil
}

// Finds the best subtitles by grabbing up to 3 and comparing them. One of the subtitles
// that match more closely with the others will be picked. This attempts to weed out
// outliers.
func (importer *OstImporter) FindBestSubtitles(tmdbId int32) (FindBestSubtitlesResult, error) {
	// Find subtitles
	subtitleResults, err := importer.FindSubtitles(tmdbId)
	if err != nil {
		return FindBestSubtitlesResult{}, err
	}
	if len(subtitleResults) == 0 {
		return FindBestSubtitlesResult{}, ErrNoSubtitles
	}

	// Download subtitles
	subtitles := make([]FindBestSubtitlesResult, SUBTITLE_COMPARISON_LIMIT)
	for i, subtitle := range subtitleResults {
		if i >= SUBTITLE_COMPARISON_LIMIT {
			break
		}
		subs, err := importer.DownloadSubtitles(subtitle.FileId)
		if err != nil {
			return FindBestSubtitlesResult{}, err
		}
		subtitles = append(subtitles, FindBestSubtitlesResult{
			Filename:          subtitle.Name,
			Subtitles:         subs,
			minifiedSubtitles: StripSubtitles(subs),
		})
	}

	// Compare subtitles
	//
	// This performs cross-analysis by computing the distance of every permutation and
	// collecting all the distances where each result was involved. The distances of
	// each result are then averaged. The average is used to score the subtitles in
	// aggregate. An outlier will have a higher score when matched with the
	// [theoretically] more frequent "correct" subtitles, of which there will be more,
	// resulting in a higher overall average score. Finally, the subtitles are sorted
	// by their score (TODO: come up with a min function to make this more efficient),
	// and the subtitles with the lowest score are chosen.

	type comparison struct {
		aIndex   int
		bIndex   int
		distance int
	}
	var resultChan = make(chan comparison)
	for aIndex, resultA := range subtitles {
		for bIndex, resultB := range subtitles[aIndex+1:] {
			go func() {
				distance := levenshtein.ComputeDistance(
					resultA.minifiedSubtitles,
					resultB.minifiedSubtitles,
				)
				resultChan <- comparison{
					aIndex:   aIndex,
					bIndex:   bIndex,
					distance: distance,
				}
			}()
		}
	}

	distancesByIndex := make(map[int][]int)
	for result := range resultChan {
		// Add the distance to both indexes. The differences will surface when we average them.
		distancesByIndex[result.aIndex] = append(distancesByIndex[result.aIndex], result.distance)
		distancesByIndex[result.bIndex] = append(distancesByIndex[result.bIndex], result.distance)
	}

	type averagedComparison struct {
		index            int
		averagedDistance int
	}
	var averagedResults []averagedComparison
	for resultIndex, distances := range distancesByIndex {
		averagedResults = append(averagedResults, averagedComparison{
			index:            resultIndex,
			averagedDistance: average(distances),
		})
	}

	topAverage := slices.MinFunc(averagedResults, func(a, b averagedComparison) int {
		return a.averagedDistance - b.averagedDistance
	})
	topResult := subtitles[topAverage.averagedDistance]

	// Check that the results are at least somewhat similar
	maxDistance := len(slices.MaxFunc(subtitles, func(a, b FindBestSubtitlesResult) int {
		return len(a.minifiedSubtitles) - len(b.minifiedSubtitles)
	}).minifiedSubtitles)
	if topAverage.averagedDistance > maxDistance/2 {
		return FindBestSubtitlesResult{}, ErrUnreliableSubtitles
	}

	return FindBestSubtitlesResult{
		Filename:  topResult.Filename,
		Subtitles: topResult.Subtitles,
	}, nil
}

func (importer *OstImporter) GetSubtitles(
	dbTx *dbapi.DbTx,
	blobController *blobs.BlobStorageController,
	videoType proto.VideoType,
	videoId int64,
	tmdbId int32,
) (GetSubtitlesResult, error) {
	results, err := dbTx.GetOstDownloadItemsByMatch(videoType, videoId)
	if err != nil {
		return GetSubtitlesResult{}, err
	}

	if len(results) > 0 {
		existingSubs := results[0]
		filePath := blobController.GetFilePath(existingSubs.BlobId)
		subsBuf, err := os.ReadFile(filePath)
		if err != nil {
			return GetSubtitlesResult{}, err
		}
		return GetSubtitlesResult{
			SubtitlesItem: existingSubs,
			Subtitles:     string(subsBuf),
		}, nil
	}

	subsResult, err := importer.FindBestSubtitles(tmdbId)
	if err != nil {
		return GetSubtitlesResult{}, err
	}
	newSubsItem, err := blobController.AddOstSubtitles(
		dbTx,
		videoType,
		videoId,
		subsResult.Filename,
		subsResult.Subtitles,
	)
	if err != nil {
		return GetSubtitlesResult{}, err
	}
	return GetSubtitlesResult{
		SubtitlesItem: newSubsItem,
		Subtitles:     subsResult.Subtitles,
	}, nil
}

// Strips subtitles into a long sequence of words with HTML formatting removed
// (using a naive regex approach), removing extra information not relevant for
// comparison
func StripSubtitles(subsText string) string {
	subsText = STRIP_SUBTITLE_REGEX.ReplaceAllLiteralString(subsText, "")
	return STRIP_WHITESPACE_REGEX.ReplaceAllLiteralString(subsText, " ")
}

// Assigns a numerical value to an uploader rank. Used for sorting files by what
// should hopefully be the reliability of the uploader.
func numericRank(rank string) int {
	switch strings.ToLower(rank) {
	case "administrator":
		return 0
	case "application developers":
		return 10
	case "gold member":
		return 20
	case "bronze member":
		return 30
	case "anonymous":
		return 100
	case "admin warning":
		return 110
	default:
		return 90
	}
}

// This is a little ridiculous. Why exactly is this not built in?
func average(numbers []int) int {
	sum := 0
	for _, num := range numbers {
		sum += num
	}
	return sum / len(numbers)
}
