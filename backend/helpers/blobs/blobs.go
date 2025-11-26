package blobs

import (
	"database/sql"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"os"
	"path"
	"path/filepath"
	"syscall"

	"github.com/google/uuid"
	"github.com/sploders101/mediacorral/backend/dbapi"
	"github.com/sploders101/mediacorral/backend/helpers/analysis"

	proto "github.com/sploders101/mediacorral/backend/gen/mediacorral/server/v1"
)

// Wraps the database and uses it to keep track of files on the filesystem
type BlobStorageController struct {
	blobDir            string
	analysisController *analysis.AnalysisController
}

func NewController(
	blobDir string,
	analysisController *analysis.AnalysisController,
) (*BlobStorageController, error) {
	if data, err := os.Stat(blobDir); err == nil {
		fileType := data.Mode().Type()
		if fileType != os.ModeDir {
			return nil, fmt.Errorf(
				"blobDir must be a directory: %w",
				os.ErrInvalid,
			)
		}
	} else {
		return nil, fmt.Errorf("error calling stat on blobDir: %w", err)
	}
	return &BlobStorageController{
		blobDir,
		analysisController,
	}, nil
}

func (controller *BlobStorageController) AddVideoFile(
	db *dbapi.DbTx,
	filePath string,
	ripJob *int64,
) error {
	videoUuid := uuid.New().String()
	newPath := controller.GetFilePath(videoUuid)

	// Move file
	if err := os.Rename(filePath, newPath); err != nil {
		if !errors.Is(err, syscall.EXDEV) {
			return err
		}
		if err := copyFile(filePath, newPath); err != nil {
			return err
		}
		if err := os.Remove(filePath); err != nil {
			return err
		}
	}

	// Insert DB record
	var ripJobSql sql.NullInt64
	if ripJob != nil {
		ripJobSql.Valid = true
		ripJobSql.Int64 = *ripJob
	}
	record, err := db.InsertVideoFile(
		proto.VideoType_VIDEO_TYPE_UNSPECIFIED,
		sql.NullInt64{},
		videoUuid,
		sql.Null[uint32]{},
		sql.Null[uint32]{},
		sql.Null[uint32]{},
		sql.Null[[]byte]{},
		ripJobSql,
		sql.Null[[]byte]{},
	)
	if err != nil {
		return err
	}

	// Collect metadata & subtitles

	metadata, err := controller.analysisController.AnalyzeMkv(newPath)
	if err != nil {
		return fmt.Errorf("error running analyzer: %w", err)
	}

	// Insert Metadata

	videoHash, err := hex.DecodeString(metadata.VideoHash)
	if err != nil {
		return fmt.Errorf("error decoding video hash from analyzer: %w", err)
	}
	var extendedMetadata sql.Null[*proto.VideoExtendedMetadata]
	if metadata.ExtendedMetadata != nil {
		extendedMetadata.Valid = true
		extendedMetadata.V = metadata.ExtendedMetadata.IntoProto()
	}
	if err := db.AddVideoMetadata(
		record.Id,
		metadata.ResolutionWidth,
		metadata.ResolutionHeight,
		metadata.Duration,
		videoHash,
		extendedMetadata,
	); err != nil {
		return err
	}

	// Insert Subtitles

	if metadata.Subtitles != nil {
		err := controller.AddSubtitlesFile(db, record.Id, *metadata.Subtitles)
		if err != nil {
			return err
		}
	}

	return nil
}

func (controller *BlobStorageController) AddSubtitlesFile(
	db *dbapi.DbTx,
	videoFile int64,
	subtitles string,
) error {
	subsUuid := uuid.New().String()
	subsPath := controller.GetFilePath(subsUuid)
	file, err := os.Create(subsPath)
	if err != nil {
		return fmt.Errorf("failed to create subtitle file: %w", err)
	}
	if _, err := file.Write([]byte(subtitles)); err != nil {
		return err
	}

	if _, err := db.InsertSubtitleFile(subsUuid, videoFile); err != nil {
		return fmt.Errorf("error inserting subtitle db entry: %w", err)
	}

	return nil
}

func (controller *BlobStorageController) DeleteRipJob(db *dbapi.DbTx, ripJob int64) error {
	videos, err := db.GetVideosFromRip(ripJob)
	if err != nil {
		return err
	}
	for _, video := range videos {
		if err := controller.DeleteBlob(db, video.BlobId); err != nil {
			return err
		}
	}

	subtitles, err := db.GetDiscSubsFromRip(ripJob)
	if err != nil {
		return err
	}
	for _, video := range subtitles {
		if err := controller.DeleteBlob(db, video.SubtitleBlob); err != nil {
			return err
		}
	}

	if err := db.DeleteMatchesFromRip(ripJob); err != nil {
		return err
	}
	if err := db.DeleteRipJob(ripJob); err != nil {
		return err
	}

	return nil
}

func (controller *BlobStorageController) AddOstSubtitles(
	db *dbapi.DbTx,
	videoType proto.VideoType,
	matchId int64,
	filename string,
	data string,
) (dbapi.OstDownloadsItem, error) {
	stUuid := uuid.New().String()
	filePath := controller.GetFilePath(stUuid)
	file, err := os.Create(filePath)
	if err != nil {
		return dbapi.OstDownloadsItem{}, err
	}

	if _, err := file.Write([]byte(data)); err != nil {
		return dbapi.OstDownloadsItem{}, err
	}

	entry, err := db.InsertOstDownloadItem(
		videoType,
		matchId,
		filename,
		stUuid,
	)
	if err != nil {
		return dbapi.OstDownloadsItem{}, err
	}

	return entry, nil
}

func (controller *BlobStorageController) AddImage(
	db *dbapi.DbTx,
	name string,
	mimeType string,
	file io.ReadCloser,
) (dbapi.ImageFilesItem, error) {
	imageUuid := uuid.New().String()
	imagePath := controller.GetFilePath(imageUuid)
	blobFile, err := os.Create(imagePath)
	if err != nil {
		return dbapi.ImageFilesItem{}, err
	}

	if _, err := io.Copy(blobFile, file); err != nil {
		return dbapi.ImageFilesItem{}, err
	}

	dbItem, err := db.InsertImageFile(
		imageUuid,
		mimeType,
		sql.NullString{Valid: true, String: name},
		sql.NullInt64{},
	)
	if err != nil {
		return dbapi.ImageFilesItem{}, err
	}

	return dbItem, nil
}

func (controller *BlobStorageController) DeleteBlob(db *dbapi.DbTx, blobId string) error {
	// Delete from the db first so we don't have bad data in the db.
	if err := db.DeleteBlob(blobId); err != nil {
		return err
	}
	blobPath := controller.GetFilePath(blobId)
	db.OnCommit(func() {
		if err := os.Remove(blobPath); err != nil {
			slog.Error("Failed to remove blob \"%s\": %s", blobId, err.Error())
		}
	})
	return nil
}

func (controller *BlobStorageController) GetFilePath(id string) string {
	return path.Join(controller.blobDir, id)
}

func (controller *BlobStorageController) HardLink(blobId string, destination string) error {
	sourcePath := controller.GetFilePath(blobId)
	err := os.Link(sourcePath, destination)
	if err != nil {
		return err
	}
	return nil
}

func (controller *BlobStorageController) SymbolicLink(blobId string, destination string) error {
	if !path.IsAbs(destination) {
		return errors.New("destination must be absolute")
	}
	source := controller.GetFilePath(blobId)
	destDir := path.Dir(destination)

	linkTarget, err := filepath.Rel(destDir, source)
	if err != nil {
		// This shouldn't really ever happen. I validate the preconditions already.
		return err
	}

	err = os.Symlink(linkTarget, destination)
	if err != nil {
		return err
	}
	return nil
}

// Essentially does the same thing as the `cp` command.
func copyFile(filePath string, newPath string) error {
	file, err := os.Open(filePath)
	if err != nil {
		return err
	}
	defer func() {
		_ = file.Close()
	}()
	newFile, err := os.Create(newPath)
	if err != nil {
		return err
	}
	defer func() {
		_ = newFile.Close()
	}()
	_, err = io.Copy(newFile, file)
	return err
}
