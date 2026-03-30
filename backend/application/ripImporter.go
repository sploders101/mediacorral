package application

import (
	"fmt"
	"io/fs"
	"log/slog"
	"os"
	"path"
	"strconv"
)

func (app *Application) jobFinisher() {
	for {
		slog.Info("Listening for jobs")
		ripJob := <-app.DriveCoordinator.FinishedQueue
		slog.Info("Got job")
		tx, err := app.Db.Begin()
		if err != nil {
			slog.Error("Failed to start transaction", "error", err.Error())
			continue
		}
		if err := tx.MarkRipJobFinished(ripJob, true); err != nil {
			tx.Rollback()
			slog.Error("Failed to mark job finished", "ripJob", ripJob, "error", err.Error())
			continue
		}
		if err := tx.Commit(); err != nil {
			slog.Error("Failed to commit transaction", "error", err.Error())
			continue
		}
	}
}

func (app *Application) jobImporter() {
	for {
		ripJob := <-app.DriveCoordinator.ImportQueue
		if err := app.ImportJob(ripJob); err != nil {
			slog.Error("Failed to import job", "error", err.Error())
		}
	}
}

// Imports a rip job from the `rips` directory
func (app *Application) ImportJob(jobId int64) error {
	dbTx, err := app.Db.Begin()
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
		".",
		func(filePath string, d fs.DirEntry, err error) error {
			if path.Ext(filePath) != ".mkv" {
				return nil
			}
			filePath = path.Join(ripDir, filePath)

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

	go func() {
		_ = app.ReprocessRipJob(jobId, true)
	}()

	return nil
}
