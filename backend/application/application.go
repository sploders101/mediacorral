package application

import (
	"context"
	"database/sql"
	"encoding/hex"
	"errors"
	"fmt"
	"io/fs"
	"log/slog"
	"os"
	"path"
	"runtime"
	"strconv"
	"sync"
	"time"

	"github.com/agnivade/levenshtein"
	"github.com/sploders101/mediacorral/backend/dbapi"
	drive_control "github.com/sploders101/mediacorral/backend/gen/mediacorral/drive_controller/v1"
	server_proto "github.com/sploders101/mediacorral/backend/gen/mediacorral/server/v1"
	"github.com/sploders101/mediacorral/backend/helpers/analysis"
	"github.com/sploders101/mediacorral/backend/helpers/blobs"
	"github.com/sploders101/mediacorral/backend/helpers/config"
	"github.com/sploders101/mediacorral/backend/helpers/exports"
	"github.com/sploders101/mediacorral/backend/helpers/opensubtitles"
	"github.com/sploders101/mediacorral/backend/helpers/tmdb"
	"google.golang.org/grpc"
	"google.golang.org/protobuf/proto"
)

var (
	// The resource is not ready, but likely will be shortly
	ErrNotReady = errors.New("the requested resource is not yet available")

	// The resource is not ready, and likely won't be for a while
	ErrBusy = errors.New("the requested resource is busy with another request")

	ErrProto    = errors.New("protocol mismatch")
	ErrNoDisc   = errors.New("no disc")
	ErrTrayOpen = errors.New("drive tray open")
	ErrNotFound = errors.New("the requested resource was not found")
)

// Application settings which can be changed at runtime
type applicationSettings struct {
	mutex sync.RWMutex

	// Enables automatic ripping on disc insertion
	autoripEnabled bool

	// Drive controllers are responsible for performing the actual ripping process.
	driveControllers map[string]drive_control.DriveControllerServiceClient
}

// This is the application service layer, which separates the all-encompassing
// application logic from the API layer.
type Application struct {
	db                 dbapi.Db
	settings           applicationSettings
	ripDir             string
	AnalysisController *analysis.AnalysisController
	BlobStorage        *blobs.BlobStorageController
	TmdbImporter       *tmdb.TmdbImporter
	OstImporter        *opensubtitles.OstImporter
	ExportsManager     *exports.ExportsManager
}

func NewApplication(configData config.ConfigFile) (*Application, error) {
	ripDir := path.Join(configData.DataDirectory, "rips")
	blobDir := path.Join(configData.DataDirectory, "blobs")
	exportsDir := path.Join(configData.DataDirectory, "exports")
	sqlitePath := path.Join(configData.DataDirectory, "database.sqlite")

	// Set up DB
	db, err := dbapi.NewDb(sqlitePath)
	if err != nil {
		return nil, err
	}

	// Create data directories
	for _, dir := range []string{ripDir, blobDir, exportsDir} {
		if err := os.Mkdir(dir, 0755); err != nil && !errors.Is(err, os.ErrExist) {
			return nil, err
		}
	}

	// Set up helpers
	analysisController, err := analysis.NewController(configData.AnalysisCli)
	if err != nil {
		return nil, fmt.Errorf("failed to set up analysis controller: %w", err)
	}
	blobStorage, err := blobs.NewController(blobDir, analysisController)
	if err != nil {
		return nil, fmt.Errorf("failed to set up blob storage: %w", err)
	}
	tmdbImporter, err := tmdb.NewImporter(db, configData.TmdbApiKey)
	if err != nil {
		return nil, fmt.Errorf("failed to set up tmdb importer: %w", err)
	}
	ostImporter, err := opensubtitles.NewOstImporter(
		configData.OstLogin.ApiKey,
		configData.OstLogin.Username,
		configData.OstLogin.Password,
	)
	if err != nil {
		return nil, fmt.Errorf("failed to set up ost importer: %w", err)
	}
	exportsManager, err := exports.NewExportsManager(db, exportsDir, configData.ExportsDirs)
	if err != nil {
		return nil, fmt.Errorf("failed to set up exports manager: %w", err)
	}

	driveControllers := make(
		map[string]drive_control.DriveControllerServiceClient,
		len(configData.DriveControllers),
	)
	for controllerName, controllerUrl := range configData.DriveControllers {
		conn, err := grpc.NewClient(controllerUrl)
		if err != nil {
			return nil, fmt.Errorf(
				"failed create drive controller client for \"%s\": %w",
				controllerName,
				err,
			)
		}
		driveControllers[controllerName] = drive_control.NewDriveControllerServiceClient(conn)
	}

	return &Application{
		db: db,
		settings: applicationSettings{
			autoripEnabled:   configData.EnableAutorip,
			driveControllers: driveControllers,
		},
		ripDir:             ripDir,
		AnalysisController: analysisController,
		BlobStorage:        blobStorage,
		TmdbImporter:       tmdbImporter,
		OstImporter:        ostImporter,
		ExportsManager:     exportsManager,
	}, nil
}

func (app *Application) GetDriveController(
	controller string,
) (drive_control.DriveControllerServiceClient, bool) {
	app.settings.mutex.RLock()
	defer app.settings.mutex.RUnlock()
	client, ok := app.settings.driveControllers[controller]
	return client, ok
}

func (app *Application) ImportTmdbTv(tmdbId int) (dbapi.TvShowsItem, error) {
	return app.TmdbImporter.ImportTv(tmdbId, app.BlobStorage)
}

func (app *Application) ImportTmdbMovie(tmdbId int) (dbapi.MoviesItem, error) {
	return app.TmdbImporter.ImportMovie(tmdbId, app.BlobStorage)
}

func (app *Application) RebuildExportsDir(exportsDir string) error {
	return app.ExportsManager.RebuildDir(exportsDir, *app.BlobStorage)
}

func (app *Application) GetAutorip() bool {
	app.settings.mutex.RLock()
	defer app.settings.mutex.RUnlock()
	return app.settings.autoripEnabled
}

func (app *Application) SetAutorip(value bool) {
	app.settings.mutex.Lock()
	defer app.settings.mutex.Unlock()
	app.settings.autoripEnabled = value
}

func (app *Application) RipMedia(
	driveController string,
	driveId uint32,
	suspectedContents *server_proto.SuspectedContents,
	autoeject bool,
) (dbapi.RipJobsItem, error) {
	app.settings.mutex.RLock()
	defer app.settings.mutex.RUnlock()

	controller, ok := app.settings.driveControllers[driveController]
	if !ok {
		return dbapi.RipJobsItem{}, ErrNotFound
	}
	driveState, err := controller.GetDriveState(
		context.TODO(),
		drive_control.GetDriveStateRequest_builder{DriveId: driveId}.Build(),
	)
	if err != nil {
		return dbapi.RipJobsItem{}, fmt.Errorf(
			"an error occurred while fetching drive state: %w",
			err,
		)
	}

	// Check the status of the drive before ripping.
	// There is a TOCTOU here, but risk is low and this is just to give more
	// informed errors.
	switch driveState.GetStatus() {
	case drive_control.DriveStatusTag_DRIVE_STATUS_TAG_EMPTY:
		return dbapi.RipJobsItem{}, ErrNoDisc
	case drive_control.DriveStatusTag_DRIVE_STATUS_TAG_TRAY_OPEN:
		return dbapi.RipJobsItem{}, ErrTrayOpen
	case drive_control.DriveStatusTag_DRIVE_STATUS_TAG_NOT_READY:
		return dbapi.RipJobsItem{}, ErrNotReady
	case drive_control.DriveStatusTag_DRIVE_STATUS_TAG_DISC_LOADED:
		// Success case
	default:
		return dbapi.RipJobsItem{}, ErrProto
	}
	if driveState.HasActiveRipJob() {
		return dbapi.RipJobsItem{}, ErrBusy
	}

	dbTx, err := app.db.Begin()
	if err != nil {
		return dbapi.RipJobsItem{}, fmt.Errorf("failed to start db transaction: %w", err)
	}
	defer func() { _ = dbTx.Rollback() }()

	now := time.Now().Unix()
	discName := sql.NullString{
		Valid:  driveState.HasDiscName(),
		String: driveState.GetDiscName(),
	}
	var suspectedContentsDb sql.Null[[]byte]
	if suspectedContents != nil {
		suspectedContentsDb.Valid = true
		suspectedContentsDb.V, err = proto.Marshal(suspectedContents)
		if err != nil {
			return dbapi.RipJobsItem{}, fmt.Errorf("error marshalling suspectedContents: %w", err)
		}
	}
	ripJob, err := dbTx.CreateRipJob(now, discName, suspectedContentsDb)
	if err != nil {
		return dbapi.RipJobsItem{}, fmt.Errorf("failed to create rip job in db: %w", err)
	}

	_, err = controller.RipMedia(context.TODO(), drive_control.RipMediaRequest_builder{
		JobId:     ripJob.Id,
		DriveId:   driveId,
		Autoeject: autoeject,
	}.Build())
	if err != nil {
		return dbapi.RipJobsItem{}, fmt.Errorf("failed to start rip job: %w", err)
	}

	if err := dbTx.Commit(); err != nil {
		return dbapi.RipJobsItem{}, fmt.Errorf("failed to commit db transaction: %w", err)
	}

	return ripJob, nil
}

// Imports a rip job from the `rips` directory
func (app *Application) ImportJob(jobId int64) error {
	dbTx, err := app.db.Begin()
	if err != nil {
		return fmt.Errorf("failed to start db transaction: %w", err)
	}
	defer func() { _ = dbTx.Rollback() }()

	// 1. Mark rip job as finished
	if err := dbTx.MarkRipJobFinished(jobId, true); err != nil {
		return fmt.Errorf("failed to mark rip job as finished: %w", err)
	}

	// 2. Import video files
	ripDir := path.Join(app.ripDir, strconv.FormatInt(jobId, 10))
	ripDirFS, err := os.OpenRoot(ripDir)
	if err != nil {
		return fmt.Errorf("failed to open rip directory: %w", err)
	}
	deleteJob := true
	if err := fs.WalkDir(
		ripDirFS.FS(),
		"/",
		func(filePath string, d fs.DirEntry, err error) error {
			if path.Ext(filePath) != ".mkv" {
				return nil
			}

			if err := app.BlobStorage.AddVideoFile(dbTx, filePath, &jobId); err != nil {
				slog.Error(
					"An error occurred while importing job.",
					"job", jobId,
					"file", filePath,
					"error", err.Error(),
				)
				deleteJob = false
			}

			return nil
		},
	); err != nil {
		return err
	}

	if deleteJob {
		if err := os.RemoveAll(ripDir); err != nil {
			slog.Error(
				"Failed to remove rip directory.",
				"job", jobId,
				"directory", ripDir,
				"error", err.Error(),
			)
		}
	}

	if err := dbTx.MarkRipJobImported(jobId, true); err != nil {
		return fmt.Errorf("failed to mark rip job as imported: %w", err)
	}

	if err := dbTx.Commit(); err != nil {
		return fmt.Errorf("failed to commit changes to db: %w", err)
	}

	return nil
}

func (app *Application) AutoimportMovie(tmdbId int32) (dbapi.MoviesItem, error) {
	dbTx, err := app.db.Begin()
	if err != nil {
		return dbapi.MoviesItem{}, fmt.Errorf("failed to create db transaction: %w", err)
	}
	defer func() { _ = dbTx.Rollback() }()

	moviesItem, err := dbTx.GetMovieByTmdbId(tmdbId)
	switch {
	case err == nil:
		return moviesItem, nil
	case errors.Is(err, sql.ErrNoRows):
	default:
		return dbapi.MoviesItem{}, err
	}

	// No movie found, so let's import it
	moviesItem, err = app.TmdbImporter.ImportMovie(int(tmdbId), app.BlobStorage)
	if err != nil {
		return dbapi.MoviesItem{}, err
	}
	if err := dbTx.Commit(); err != nil {
		return dbapi.MoviesItem{}, err
	}

	return moviesItem, nil
}

func (app *Application) AnalyzeJob(jobId int64) error {
	dbTx, err := app.db.Begin()
	if err != nil {
		return fmt.Errorf("failed to start db transaction: %w", err)
	}
	defer func() { _ = dbTx.Rollback() }()

	ripJobItem, err := dbTx.GetRipJob(jobId)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return ErrNotFound
		}
		return fmt.Errorf("failed to get rip job: %w", err)
	}

	if !ripJobItem.SuspectedContents.Valid {
		// Nothing to analyze
		return nil
	}

	suspectedContents := &server_proto.SuspectedContents{}
	if err := proto.Unmarshal(ripJobItem.SuspectedContents.V, suspectedContents); err != nil {
		return fmt.Errorf("invalid format for suspectedContents: %w", err)
	}

	switch {
	case suspectedContents.HasMovie():
		suspectedMovie := suspectedContents.GetMovie()
		if _, err := app.AutoimportMovie(suspectedMovie.GetTmdbId()); err != nil {
			return fmt.Errorf("failed to import movie: %w", err)
		}
		if err := dbTx.Commit(); err != nil {
			return fmt.Errorf("failed to commit db transaction: %w", err)
		}
		return nil
	case suspectedContents.HasTvEpisodes():
		if err := compareTvOstSubs(dbTx, app.OstImporter, app.BlobStorage, jobId, suspectedContents.GetTvEpisodes()); err != nil {
			return fmt.Errorf("failed to analyze tv show: %w", err)
		}
		if err := dbTx.Commit(); err != nil {
			return fmt.Errorf("failed to commit db transaction: %w", err)
		}
		return nil
	}

	// No contents were found
	return nil
}

// Re-runs analysis on video files in rip job. Useful for debugging or backfilling metadata
func (app *Application) ReprocessRipJob(jobId int64, updateHash bool) error {
	dbRoTx, err := app.db.Begin()
	if err != nil {
		return fmt.Errorf("failed to start db transaction: %w", err)
	}
	defer func() { _ = dbRoTx.Rollback() }()

	videoFiles, err := dbRoTx.GetVideosFromRip(jobId)
	if err != nil {
		return fmt.Errorf("failed to list video files in rip: %w", err)
	}
	if err := dbRoTx.Rollback(); err != nil {
		slog.Error("failed to roll back dbRoTx in ReprocessRipJob", "error", err.Error())
	}

	var extractWg sync.WaitGroup
	for _, videoFile := range videoFiles {
		// Start new analysis job (to be joined later)
		videoFilePath := app.BlobStorage.GetFilePath(videoFile.BlobId)
		extractWg.Add(1)
		go func() {
			dbTx, err := app.db.Begin()
			if err != nil {
				slog.Error("Failed to start db transaction", "error", err.Error())
				return
			}
			defer func() { _ = dbTx.Rollback() }()

			// Delete existing subtitles for video
			subtitleFiles, err := dbTx.GetSubtitlesForVideo(videoFile.Id)
			if err != nil {
				slog.Error(
					"Failed to get subtitles for video",
					"error", err.Error(),
					"videoFileId", videoFile.Id,
				)
				return
			}
			for _, subtitleFile := range subtitleFiles {
				if err := app.BlobStorage.DeleteBlob(dbTx, subtitleFile.BlobId); err != nil {
					slog.Error(
						"Failed to delete subtitles",
						"error", err.Error(),
						"videoFileId", videoFile.Id,
						"subtitleFileId", subtitleFile.Id,
						"subtitleFileBlobId", subtitleFile.BlobId,
					)
					return
				}
			}

			defer extractWg.Done()
			result, err := app.AnalysisController.AnalyzeMkv(videoFilePath)
			if err != nil {
				slog.Error(
					"Failed to run analysis on video file",
					"error", err.Error(),
					"videoFileId", videoFile.Id,
					"videoFileBlobId", videoFile.BlobId,
				)
				return
			}

			var videoHash []byte
			if updateHash {
				videoHash, err = hex.DecodeString(result.VideoHash)
				if err != nil {
					slog.Error(
						"Error decoding video hash from analyzer",
						"error", err.Error(),
						"videoFileId", videoFile.Id,
						"videoFileBlobId", videoFile.BlobId,
					)
					return
				}
			} else if videoFile.OriginalVideoHash.Valid {
				videoHash = videoFile.OriginalVideoHash.V
			}
			var extendedMetadata sql.Null[*server_proto.VideoExtendedMetadata]
			if result.ExtendedMetadata != nil {
				extendedMetadata.Valid = true
				extendedMetadata.V = result.ExtendedMetadata.IntoProto()
			}

			if err := dbTx.AddVideoMetadata(
				videoFile.Id,
				result.ResolutionWidth,
				result.ResolutionHeight,
				result.Duration,
				videoHash,
				extendedMetadata,
			); err != nil {
				slog.Error(
					"Failed to add video metadata",
					"error", err.Error(),
					"videoFileId", videoFile.Id,
					"videoBlobId", videoFile.BlobId,
					"videoHash", videoHash,
				)
				return
			}

			if result.Subtitles != nil {
				err := app.BlobStorage.AddSubtitlesFile(dbTx, videoFile.Id, *result.Subtitles)
				if err != nil {
					slog.Error(
						"Failed to add subtitles file",
						"error", err.Error(),
						"videoFileId", videoFile.Id,
						"videoFileBlobId", videoFile.BlobId,
					)
					return
				}
			}

			if err := dbTx.Commit(); err != nil {
				slog.Error(
					"Failed to commit video metadata",
					"error", err.Error(),
					"videoFileId", videoFile.Id,
					"videoFileBlobId", videoFile.BlobId,
				)
			}
		}()
	}
	extractWg.Wait()

	return nil
}

// Prunes a rip job, deleting any files that aren't tagged, and their references
func (app *Application) PruneRipJob(jobId int64) error {
	dbTx, err := app.db.Begin()
	if err != nil {
		return fmt.Errorf("failed to start db tx: %w", err)
	}
	defer func() { _ = dbTx.Rollback() }()

	untaggedBlobs, err := dbTx.GetUntaggedVideosFromJob(jobId)
	if err != nil {
		return fmt.Errorf("failed to get untagged videos from job: %w", err)
	}

	for _, blob := range untaggedBlobs {
		if err := app.BlobStorage.DeleteBlob(dbTx, blob.VideoBlob); err != nil {
			return fmt.Errorf("failed to delete video blob %s: %w", blob.VideoBlob, err)
		}
		if blob.SubtitleBlob.Valid {
			if err := app.BlobStorage.DeleteBlob(dbTx, blob.SubtitleBlob.String); err != nil {
				return fmt.Errorf(
					"failed to delete subtitle blob %s: %w",
					blob.SubtitleBlob.String,
					err,
				)
			}
		}
	}

	return nil
}

// Compares subtitles from the disc with those found on opensubtitles and inserts
// statistics into the database to inform the tagging process.
func compareTvOstSubs(
	dbTx *dbapi.DbTx,
	ostImporter *opensubtitles.OstImporter,
	blobController *blobs.BlobStorageController,
	ripJob int64,
	suspectedTv *server_proto.SuspectedContents_TvEpisodes,
) error {
	type SubsInstruction struct {
		ostDownloadId int64
		ostSubs       string
		videoFileId   int64
		discSubs      string
	}

	feederFunc := func(workQueue chan<- SubsInstruction) error {
		for _, episodeId := range suspectedTv.GetEpisodeTmdbIds() {
			episode, err := dbTx.GetTvEpisodeByTmdbId(episodeId)
			if err != nil {
				// TODO: Change episode_tmdb_ids to episode_ids.
				//       As other analysis mechanisms are added,
				//       this will help decouple from TMDB.
				return fmt.Errorf("failed to get tmdb episode: %w", err)
			}

			ostSubs, err := ostImporter.GetSubtitles(
				dbTx,
				blobController,
				server_proto.VideoType_VIDEO_TYPE_TV_EPISODE,
				episode.Id,
				episodeId,
			)
			if err != nil {
				// This fails a lot. Just skip it if it's not working.
				slog.Error(
					"Failed to get OST subtitles.",
					"ripJob",
					ripJob,
					"tvEpisodeId",
					episode.Id,
					"tmdbId",
					episode.TmdbId.Int32,
					"error",
					err.Error(),
				)
				continue
			}

			discSubs, err := dbTx.GetDiscSubsFromRip(ripJob)
			if err != nil {
				return fmt.Errorf("failed to get disc subtitles: %w", err)
			}
			for _, videoFile := range discSubs {
				subtitlePath := blobController.GetFilePath(videoFile.SubtitleBlob)
				discSubString, err := os.ReadFile(subtitlePath)
				if err != nil {
					slog.Error(
						"Failed to read subtitles from blob storage.",
						"ripJob",
						ripJob,
						"blobId",
						videoFile.SubtitleBlob,
						"filePath",
						subtitlePath,
						"videoFileId",
						videoFile.VideoId,
						"error",
						err.Error(),
					)
					continue
				}
				workQueue <- SubsInstruction{
					ostDownloadId: ostSubs.SubtitlesItem.Id,
					ostSubs:       ostSubs.Subtitles,
					videoFileId:   videoFile.VideoId,
					discSubs:      string(discSubString),
				}
			}
		}
		return nil
	}

	workerFunc := func(job SubsInstruction) dbapi.MatchInfoItem {
		return dbapi.MatchInfoItem{
			VideoFileId:   job.videoFileId,
			OstDownloadId: job.ostDownloadId,
			Distance:      uint32(levenshtein.ComputeDistance(job.ostSubs, job.discSubs)),
			MaxDistance:   uint32(max(len(job.ostSubs), len(job.discSubs))),
		}
	}

	results, err := backpressuredWorkQueue(feederFunc, workerFunc)
	if err != nil {
		return err
	}

	if err := dbTx.ClearMatchInfoForJob(ripJob); err != nil {
		return fmt.Errorf("failed to clear existing match info: %w", err)
	}
	for _, result := range results {
		if _, err := dbTx.InsertMatchInfoItem(
			result.VideoFileId,
			result.OstDownloadId,
			result.Distance,
			result.MaxDistance,
		); err != nil {
			return fmt.Errorf("failed to insert match info item: %w", err)
		}
	}

	return nil
}

// This function is for running analytics on large data sets where the input is large, and the
// output is small. The feeder pushes data into a channel to be processed, and the goroutine
// pool will receive and process the data, producing a result. The results will be collected
// into a slice and returned.
//
// This strategy prevents the feeder from accumulating too much data before it can be processed.
func backpressuredWorkQueue[T, R any](
	feeder func(jobChannel chan<- T) error,
	worker func(T) R,
) ([]R, error) {
	workerThreads := runtime.GOMAXPROCS(0)
	feederChan := make(chan T, workerThreads)
	resultChan := make(chan R, workerThreads)

	var resultErr error

	go func() {
		if err := feeder(feederChan); err != nil {
			resultErr = err
		}
		close(feederChan)
	}()
	var workerWg sync.WaitGroup
	workerWg.Add(workerThreads)
	go func() {
		workerWg.Wait()
		close(resultChan)
	}()
	for range workerThreads {
		go func() {
			defer workerWg.Done()
			for job := range feederChan {
				resultChan <- worker(job)
			}
		}()
	}

	var results []R
	for result := range resultChan {
		results = append(results, result)
	}

	return results, resultErr
}
