package exports

import (
	"errors"
	"fmt"
	"log/slog"
	"os"
	"path"
	"strings"
	"sync"

	"github.com/sploders101/mediacorral/backend/dbapi"
	proto "github.com/sploders101/mediacorral/backend/gen/mediacorral/server/v1"
	"github.com/sploders101/mediacorral/backend/helpers/blobs"
	"github.com/sploders101/mediacorral/backend/helpers/config"
)

var (
	ErrNotDir      = errors.New("the specified resource was not a directory")
	ErrDirNotFound = errors.New("the specified export directory was not found")
)

type ExportsManager struct {
	exportsBaseDir string
	db             dbapi.Db

	usageMutex        sync.RWMutex
	configuredExports map[string]config.ExportsDir
}

func NewExportsManager(
	db dbapi.Db,
	exportsPath string,
	configuredExports map[string]config.ExportsDir,
) (*ExportsManager, error) {
	stats, err := os.Stat(exportsPath)
	if err != nil {
		return nil, err
	}

	if !stats.IsDir() {
		return nil, ErrNotDir
	}

	return &ExportsManager{
		exportsBaseDir:    exportsPath,
		db:                db,
		configuredExports: configuredExports,
	}, nil
}

func (exporter *ExportsManager) RebuildDir(
	exportName string,
	blobController *blobs.BlobStorageController,
) error {
	exporter.usageMutex.RLock()
	defer exporter.usageMutex.RUnlock()

	exportDir := path.Join(exporter.exportsBaseDir, exportName)
	exportConfig, ok := exporter.configuredExports[exportName]
	if !ok {
		return ErrDirNotFound
	}

	// Remove existing contents
	existingEntries, err := os.ReadDir(exportDir)
	switch {
	case errors.Is(err, os.ErrNotExist):
		if err := os.Mkdir(exportDir, 0755); err != nil {
			return fmt.Errorf("failed to create export directory: %w", err)
		}
	case err != nil:
		return err
	}
	for _, entry := range existingEntries {
		filePath := path.Join(exportDir, entry.Name())
		if err := os.RemoveAll(filePath); err != nil {
			return err
		}
	}

	// Populate new contents
	switch exportConfig.MediaType {
	case config.EXPORT_MEDIA_TYPE_MOVIES:
		dbTx, err := exporter.db.Begin()
		if err != nil {
			return err
		}
		defer func() { _ = dbTx.Rollback() }()

		if err := dbTx.ProcessMovieExportsInfo(addMovie(blobController, exportDir, exportConfig)); err != nil {
			return err
		}
	case config.EXPORT_MEDIA_TYPE_TV:
		dbTx, err := exporter.db.Begin()
		if err != nil {
			return err
		}
		defer func() { _ = dbTx.Rollback() }()

		if err := dbTx.ProcessTvExportsInfo(addTvEpisode(blobController, exportDir, exportConfig)); err != nil {
			return err
		}
	}

	return nil
}

func (exporter *ExportsManager) SpliceContent(
	videoType proto.VideoType,
	videoId int64,
	blobController *blobs.BlobStorageController,
) error {
	dbTx, err := exporter.db.Begin()
	if err != nil {
		return err
	}
	defer func() { _ = dbTx.Rollback() }()
	exporter.usageMutex.RLock()
	defer exporter.usageMutex.RUnlock()

	// Add media to filesystem
	switch videoType {
	case proto.VideoType_VIDEO_TYPE_MOVIE:
		result, err := dbTx.FetchOneMovieExportInfo(videoId)
		if err != nil {
			return err
		}
		for exportName, exportConfig := range exporter.configuredExports {
			if exportConfig.MediaType != config.EXPORT_MEDIA_TYPE_MOVIES {
				continue
			}
			exportDir := path.Join(exporter.exportsBaseDir, exportName)
			if err := addMovie(blobController, exportDir, exportConfig)(result); err != nil {
				return err
			}
		}
	case proto.VideoType_VIDEO_TYPE_TV_EPISODE:
		result, err := dbTx.FetchOneTvExportInfo(videoId)
		if err != nil {
			return err
		}
		for exportName, exportConfig := range exporter.configuredExports {
			if exportConfig.MediaType != config.EXPORT_MEDIA_TYPE_TV {
				continue
			}
			exportDir := path.Join(exporter.exportsBaseDir, exportName)
			if err := addTvEpisode(blobController, exportDir, exportConfig)(result); err != nil {
				return err
			}
		}
	}

	return nil
}

func addTvEpisode(
	blobController *blobs.BlobStorageController,
	exportsDir string,
	exportConfig config.ExportsDir,
) func(dbapi.TvExportEntry) error {
	return func(entry dbapi.TvExportEntry) error {
		showFolder := path.Join(
			exportsDir,
			fmt.Sprintf(
				"%s (%s) {tmdb-%d}",
				pathEscape(entry.TvTitle),
				pathEscape(entry.TvReleaseYear),
				entry.TvTmdb,
			),
		)
		seasonFolder := path.Join(showFolder, fmt.Sprintf("Season %02d", entry.SeasonNumber))
		episodeFilename := fmt.Sprintf(
			"%s (%s) - S%02dE%02d - %s - {tmdb-%d}.mkv",
			pathEscape(entry.TvTitle),
			pathEscape(entry.TvReleaseYear),
			entry.SeasonNumber,
			entry.EpisodeNumber,
			pathEscape(entry.EpisodeTitle),
			entry.EpisodeTmdb,
		)
		episodePath := path.Join(seasonFolder, episodeFilename)

		var err error
		for {
			switch exportConfig.LinkType {
			case config.EXPORT_LINK_TYPE_SYMBOLIC:
				err = blobController.SymbolicLink(entry.EpisodeBlob, episodePath)
			case config.EXPORT_LINK_TYPE_HARD:
				err = blobController.HardLink(entry.EpisodeBlob, episodePath)
			}
			if errors.Is(err, os.ErrNotExist) {
				if err := os.MkdirAll(seasonFolder, 0755); err != nil {
					slog.Error("Error making directory", "error", err.Error())
					if errors.Is(err, os.ErrExist) {
						return err
					} else {
						return fmt.Errorf("couldn't create season folder: %w", err)
					}
				}
				continue
			} else if errors.Is(err, blobs.ErrBlobMissing) {
				slog.Error(
					"Blob missing from filesystem",
					"blobId", entry.EpisodeBlob,
					"tvShow", entry.TvTitle,
					"seasonNumber", entry.SeasonNumber,
					"episodeNumber", entry.EpisodeNumber,
					"episodeTitle", entry.EpisodeTitle,
					"episodeTmdb", entry.EpisodeTmdb,
				)
				// This is an okay-ish error. We already logged. Discard it for now.
				err = nil
				break
			}
			break
		}
		if err != nil {
			return err
		}

		return nil
	}
}

func addMovie(
	blobController *blobs.BlobStorageController,
	exportsDir string,
	exportConfig config.ExportsDir,
) func(dbapi.MovieExportEntry) error {
	return func(entry dbapi.MovieExportEntry) error {
		movieFolder := path.Join(
			exportsDir,
			fmt.Sprintf(
				"%s (%s) {tmdb-%d}",
				pathEscape(entry.MovieTitle),
				pathEscape(entry.MovieReleaseYear),
				entry.MovieTmdb,
			),
		)
		movieFilename := fmt.Sprintf(
			"%s (%s) - {tmdb-%d}.mkv",
			pathEscape(entry.MovieTitle),
			pathEscape(entry.MovieReleaseYear),
			entry.MovieTmdb,
		)
		moviePath := path.Join(movieFolder, movieFilename)

		var err error
		for {
			switch exportConfig.LinkType {
			case config.EXPORT_LINK_TYPE_SYMBOLIC:
				err = blobController.SymbolicLink(entry.MovieBlob, moviePath)
			case config.EXPORT_LINK_TYPE_HARD:
				err = blobController.HardLink(entry.MovieBlob, moviePath)
			}
			if errors.Is(err, os.ErrNotExist) {
				if err := os.MkdirAll(movieFolder, 0755); err != nil {
					slog.Error("Error making directory", "error", err.Error())
					if errors.Is(err, os.ErrExist) {
						return err
					} else {
						return fmt.Errorf("couldn't create season folder: %w", err)
					}
				}
				continue
			} else if errors.Is(err, blobs.ErrBlobMissing) {
				slog.Error(
					"Blob missing from filesystem",
					"blobId", entry.MovieBlob,
					"movie", entry.MovieTitle,
					"movieTmdb", entry.MovieTmdb,
				)
				// This is an okay-ish error. We already logged. Discard it for now.
				err = nil
				break
			}
			break
		}
		if err != nil {
			return err
		}

		return nil
	}
}

// Ensures that the given input is safe to use as a path
func pathEscape(input string) string {
	return strings.ReplaceAll(input, "/", "_")
}
