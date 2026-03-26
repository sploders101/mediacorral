package application

import (
	"context"
	"log/slog"

	"github.com/sploders101/mediacorral/backend/drive_coordinator"
	drive_coordinatorv1 "github.com/sploders101/mediacorral/backend/gen/mediacorral/drive_coordinator/v1"
)

type autoripInterface struct {
	// This can be used to cancel the autoripper for old sessions.
	cancel context.CancelFunc

	// This is only used for checking pointer equality.
	// The ID can stay the same across multiple sessions, and we need to make sure we've got the right one.
	driveController *drive_coordinator.DriveController
}

func (app *Application) autoripper() {
	driveListWatcher := app.DriveCoordinator.WatchDrives()
	driveWatchers := make(map[string]autoripInterface)
	for {
		drives, unlock, err := driveListWatcher.Changed()
		if err != nil {
			return
		}
		for driveId, driveController := range drives {
			if autoripper, ok := driveWatchers[driveId]; ok {
				if autoripper.driveController != driveController {
					// Autoripper exists, but is no longer valid
					autoripper.cancel()
				} else {
					// Autoripper exists and is valid
					continue
				}
			}
			// Autoripper should be created
			ctx, cancel := context.WithCancel(context.TODO())
			driveWatchers[driveId] = autoripInterface{cancel, driveController}
			go app.autoripDrive(ctx, driveController)
		}
		for driveId, intfc := range driveWatchers {
			if _, ok := drives[driveId]; !ok {
				intfc.cancel()
				delete(drives, driveId)
			}
		}
		unlock()
	}
}

func (app *Application) autoripDrive(ctx context.Context, driveController *drive_coordinator.DriveController) {
	statusWatcher := driveController.DriveStatus()
	statusWatcher.SetContext(ctx)
	for {
		status, err := statusWatcher.Changed()
		if err != nil {
			return
		}
		app.settings.mutex.Lock()
		autoripEnabled := app.settings.autoripEnabled
		app.settings.mutex.Unlock()
		// TODO: optimize this by cancelling & recreating watchers
		if !autoripEnabled {
			continue
		}
		switch status.GetStatus() {
		case drive_coordinatorv1.DriveStatusTag_DRIVE_STATUS_TAG_DISC_LOADED:
			discName := status.GetDiscName()
			if discName == "" {
				continue
			}
			if status.HasActiveRipJob() {
				continue
			}
			// We've met ripping criteria. Start!
			_, err := app.RipMedia(driveController.DriveId(), nil, true)
			if err != nil {
				slog.Error("An error occurred while submitting an autorip job", "error", err.Error())
			}
		}
	}
}
