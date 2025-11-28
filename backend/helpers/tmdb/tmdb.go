package tmdb

import (
	"database/sql"
	"errors"
	"fmt"
	"log/slog"
	"net/http"
	"path"
	"strconv"
	"strings"

	tmdb "github.com/cyruzin/golang-tmdb"

	"github.com/sploders101/mediacorral/backend/dbapi"
	"github.com/sploders101/mediacorral/backend/helpers/blobs"
)

type TmdbImporter struct {
	db     dbapi.Db
	client *tmdb.Client
}

func NewImporter(db dbapi.Db, bearerToken string) (*TmdbImporter, error) {
	tmdbClient, err := tmdb.InitV4(bearerToken)
	if err != nil {
		return nil, err
	}
	return &TmdbImporter{
		db:     db,
		client: tmdbClient,
	}, nil
}

func (importer *TmdbImporter) QueryAny(
	query string,
	language string,
	page uint32,
) (tmdb.SearchMulti, error) {
	params := make(map[string]string)
	if language != "" {
		params["language"] = language
	}
	params["page"] = strconv.FormatUint(uint64(page), 10)

	results, err := importer.client.GetSearchMulti(query, params)
	if err != nil {
		return tmdb.SearchMulti{}, err
	}
	return *results, nil
}

func (importer *TmdbImporter) QueryMovies(
	query string,
	language string,
	primaryReleaseYear string,
	region string,
	year string,
	page uint32,
) (tmdb.SearchMovies, error) {
	params := make(map[string]string)
	if language != "" {
		params["language"] = language
	}
	if primaryReleaseYear != "" {
		params["primary_release_year"] = primaryReleaseYear
	}
	if region != "" {
		params["region"] = region
	}
	if year != "" {
		params["year"] = year
	}
	params["page"] = strconv.FormatUint(uint64(page), 10)

	results, err := importer.client.GetSearchMovies(query, params)
	if err != nil {
		return tmdb.SearchMovies{}, err
	}
	return *results, nil
}

func (importer *TmdbImporter) QueryTv(
	query string,
	firstAirDateYear string,
	language string,
	year string,
	page uint32,
) (tmdb.SearchTVShows, error) {
	params := make(map[string]string)
	if firstAirDateYear != "" {
		params["first_air_date_year"] = firstAirDateYear
	}
	if language != "" {
		params["language"] = language
	}
	if year != "" {
		params["year"] = year
	}
	params["page"] = strconv.FormatUint(uint64(page), 10)

	results, err := importer.client.GetSearchTVShow(query, params)
	if err != nil {
		return tmdb.SearchTVShows{}, err
	}
	return *results, nil
}

func (importer *TmdbImporter) getPoster(
	db *dbapi.DbTx,
	posterPath string,
	blobStorage blobs.BlobStorageController,
) (dbapi.ImageFilesItem, error) {
	imageUrl := tmdb.GetImageURL(posterPath, tmdb.Original)
	response, err := http.Get(imageUrl)
	if err != nil {
		return dbapi.ImageFilesItem{}, err
	}
	mimeType := response.Header.Get("content-type")
	if mimeType == "" {
		return dbapi.ImageFilesItem{}, errors.New("poster missing \"content-type\" header")
	}
	name := path.Base(posterPath)
	imageItem, err := blobStorage.AddImage(db, name, mimeType, response.Body)
	if err != nil {
		return dbapi.ImageFilesItem{}, err
	}
	return imageItem, nil
}

func (importer *TmdbImporter) ImportMovie(
	movieId int,
	blobStorage *blobs.BlobStorageController,
) (dbapi.MoviesItem, error) {
	results, err := importer.client.GetMovieDetails(movieId, nil)
	if err != nil {
		return dbapi.MoviesItem{}, nil
	}

	if results.Title == "" {
		return dbapi.MoviesItem{}, errors.New("cannot import movie with no title")
	}

	dbTx, err := importer.db.Begin()
	if err != nil {
		return dbapi.MoviesItem{}, err
	}
	defer func() {
		_ = dbTx.Rollback()
	}()

	var posterBlob sql.NullInt64
	if blobStorage != nil && results.PosterPath != "" {
		posterItem, err := importer.getPoster(dbTx, results.PosterPath, *blobStorage)
		if err != nil {
			slog.Error(
				"An error occurred while fetching poster",
				"movieName",
				results.Title,
				"movieId",
				results.ID,
				"error",
				err.Error(),
			)
		} else {
			posterBlob.Valid = true
			posterBlob.Int64 = posterItem.Id
		}
	}

	var releaseYear sql.NullString
	if results.ReleaseDate != "" {
		year, _, ok := strings.Cut(results.ReleaseDate, "-")
		if ok {
			releaseYear.Valid = true
			releaseYear.String = year
		}
	}

	var description sql.NullString
	if results.Overview != "" {
		description.Valid = true
		description.String = results.Overview
	}

	var runtime sql.Null[uint32]
	if results.Runtime != 0 {
		runtime.Valid = true
		runtime.V = uint32(results.Runtime)
	}

	moviesItem, err := dbTx.UpsertTmdbMovie(
		sql.NullInt32{Valid: true, Int32: int32(results.ID)},
		posterBlob,
		results.Title,
		releaseYear,
		description,
		runtime,
	)
	if err != nil {
		return dbapi.MoviesItem{}, nil
	}
	if err := dbTx.Commit(); err != nil {
		return dbapi.MoviesItem{}, err
	}

	return moviesItem, nil
}

func (importer *TmdbImporter) ImportTv(
	tvId int,
	blobStorage *blobs.BlobStorageController,
) (dbapi.TvShowsItem, error) {
	results, err := importer.client.GetTVDetails(tvId, nil)
	if err != nil {
		return dbapi.TvShowsItem{}, fmt.Errorf("error fetching tv show: %w", err)
	}

	dbTx, err := importer.db.Begin()
	if err != nil {
		return dbapi.TvShowsItem{}, err
	}
	defer func() {
		_ = dbTx.Rollback()
	}()

	if results.Name == "" {
		return dbapi.TvShowsItem{}, errors.New("cannot import show without name")
	}

	var originalReleaseYear sql.NullString
	if results.FirstAirDate != "" {
		year, _, ok := strings.Cut(results.FirstAirDate, "-")
		if ok {
			originalReleaseYear.Valid = true
			originalReleaseYear.String = year
		}
	}

	var overview sql.NullString
	if results.Overview != "" {
		overview.Valid = true
		overview.String = results.Overview
	}

	var posterBlob sql.NullInt64
	if blobStorage != nil && results.PosterPath != "" {
		posterItem, err := importer.getPoster(dbTx, results.PosterPath, *blobStorage)
		if err != nil {
			slog.Error(
				"An error occurred while fetching poster",
				"tvName",
				results.Name,
				"tvId",
				results.ID,
				"error",
				err.Error(),
			)
		} else {
			posterBlob.Valid = true
			posterBlob.Int64 = posterItem.Id
		}
	}

	seriesEntry, err := dbTx.UpsertTmdbTvShow(
		int32(results.ID),
		posterBlob,
		results.Name,
		originalReleaseYear,
		overview,
	)
	if err != nil {
		return dbapi.TvShowsItem{}, err
	}

	// Loop over each season
	for _, season := range results.Seasons {
		seasonDetails, err := importer.client.GetTVSeasonDetails(
			int(results.ID),
			season.SeasonNumber,
			nil,
		)
		if err != nil {
			return dbapi.TvShowsItem{}, fmt.Errorf("error fetching tv season: %w", err)
		}

		var posterBlob sql.NullInt64
		if blobStorage != nil && results.PosterPath != "" {
			posterItem, err := importer.getPoster(dbTx, seasonDetails.PosterPath, *blobStorage)
			if err != nil {
				slog.Error(
					"An error occurred while fetching poster",
					"tvName",
					results.Name,
					"tvId",
					results.ID,
					"tvSeason",
					season.SeasonNumber,
					"error",
					err.Error(),
				)
			} else {
				posterBlob.Valid = true
				posterBlob.Int64 = posterItem.Id
			}
		}

		var overview sql.NullString
		if seasonDetails.Overview != "" {
			overview.Valid = true
			overview.String = results.Overview
		}

		seasonitem, err := dbTx.UpsertTmdbTvSeason(
			int32(seasonDetails.ID),
			seriesEntry.Id,
			uint32(season.SeasonNumber),
			posterBlob,
			seasonDetails.Name,
			overview,
		)
		if err != nil {
			return dbapi.TvShowsItem{}, err
		}

		for _, episodeDetails := range seasonDetails.Episodes {
			var thumbnailBlob sql.NullInt64
			if blobStorage != nil && results.PosterPath != "" {
				thumbnailItem, err := importer.getPoster(
					dbTx,
					episodeDetails.StillPath,
					*blobStorage,
				)
				if err != nil {
					slog.Error(
						"An error occurred while fetching poster",
						"tvName",
						results.Name,
						"tvId",
						results.ID,
						"tvSeason",
						season.SeasonNumber,
						"episode",
						episodeDetails.EpisodeNumber,
						"error",
						err.Error(),
					)
				} else {
					thumbnailBlob.Valid = true
					thumbnailBlob.Int64 = thumbnailItem.Id
				}
			}

			var overview sql.NullString
			if episodeDetails.Overview != "" {
				overview.Valid = true
				overview.String = episodeDetails.Overview
			}

			var runtime sql.Null[uint32]
			if episodeDetails.Runtime != 0 {
				runtime.Valid = true
				runtime.V = uint32(episodeDetails.Runtime)
			}

			_, err := dbTx.UpsertTmdbTvEpisode(
				int32(episodeDetails.ID),
				seriesEntry.Id,
				seasonitem.Id,
				uint32(episodeDetails.EpisodeNumber),
				thumbnailBlob,
				episodeDetails.Name,
				overview,
				runtime,
			)
			if err != nil {
				return dbapi.TvShowsItem{}, err
			}
		}
	}
	if err := dbTx.Commit(); err != nil {
		return dbapi.TvShowsItem{}, err
	}

	return seriesEntry, nil
}
