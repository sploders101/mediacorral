package drive_coordinator

import (
	"context"
	"encoding/json"
	"errors"
	"io"
	"log/slog"

	drive_coordinatorv1 "github.com/sploders101/mediacorral/backend/gen/mediacorral/drive_coordinator/v1"
	"github.com/sploders101/mediacorral/backend/helpers/sync_extras"
	"google.golang.org/grpc"
	"google.golang.org/protobuf/encoding/protojson"
)

var (
	ErrProtocol = errors.New("protocol violation")
)

type DriveCoordinatorService struct {
	// Holds the list of active controllers. Update signals are shallow (ie. addition, removal, replacement).
	driveControllers *sync_extras.Watch[map[string]*DriveController]
	ripDir           string
}

func NewDriveCoordinatorService() *DriveCoordinatorService {
	return &DriveCoordinatorService{
		driveControllers: sync_extras.NewWatch(map[string]*DriveController{}),
	}
}

func (coordinator *DriveCoordinatorService) ConnectDrive(
	driveConn drive_coordinatorv1.DriveCoordinatorService_ConnectDriveServer,
) error {
	// This is needed to allow the rust code to enter the select loop.
	// This might be better solved in another way, but this is easy and works for now.
	if err := driveConn.SendHeader(nil); err != nil {
		return err
	}

	// Watch for discovery message
	discoveryRaw, err := driveConn.Recv()
	if err != nil && !errors.Is(err, io.EOF) {
		slog.Error("Drive read error", "error", err.Error())
		return err
	}
	if !discoveryRaw.HasDiscovery() {
		// First packet must always be a discovery packet so we know how to catalog it
		slog.Error("Drive protocol error", "error", "first message was not discovery")
		return ErrProtocol
	}
	discovery := discoveryRaw.GetDiscovery()
	driveId := discovery.GetDriveId()
	driveName := discovery.GetDriveName()
	slog.Info("Drive discovered", "id", driveId, "name", driveName)

	// Create controller interface
	ctx, cancel := context.WithCancel(driveConn.Context())
	defer cancel() // Command *senders* track this, too
	statusWatcher := sync_extras.NewWatchSilent(drive_coordinatorv1.DriveStatus_builder{}.Build())
	defer statusWatcher.Close()
	ripJobWatcher := sync_extras.NewWatchSilent(drive_coordinatorv1.RipStatus_builder{}.Build())
	defer ripJobWatcher.Close()
	writeChan := make(chan *drive_coordinatorv1.DriveConnectionResponse)
	driveController := &DriveController{
		discovery,
		cancel,
		ctx,
		writeChan,
		statusWatcher,
		ripJobWatcher,
	}

	// Add the controller handle to the coordinator
	coordinator.driveControllers.ConditionalSet(
		func(controllers *map[string]*DriveController) bool {
			if controller, ok := (*controllers)[driveId]; ok {
				controller.cancel()
			}
			(*controllers)[driveId] = driveController
			return true
		},
	)
	// Remove ourselves from the list when we're done
	defer coordinator.driveControllers.ConditionalSet(
		func(controllers *map[string]*DriveController) bool {
			if activeController, ok := (*controllers)[driveId]; ok &&
				activeController == driveController {
				delete(*controllers, driveId)
			}
			return true
		},
	)

	// Start read loop
	readErrChan := make(chan error, 1)
	go func() {
		defer close(readErrChan)
		for {
			msg, err := driveConn.Recv()
			if err != nil {
				readErrChan <- err
				return
			}

			switch {
			case msg.HasDiscovery():
				readErrChan <- ErrProtocol
				return
			case msg.HasDriveStatusUpdate():
				if status, err := protojson.Marshal(msg.GetDriveStatusUpdate()); err == nil {
					slog.Info(
						"Drive status update",
						"driveId",
						driveId,
						"status",
						json.RawMessage(status),
					)
				}
				statusWatcher.Set(msg.GetDriveStatusUpdate())
			case msg.HasRipStatusUpdate():
				if status, err := protojson.Marshal(msg.GetRipStatusUpdate()); err == nil {
					slog.Debug(
						"Rip status update",
						"driveId",
						driveId,
						"ripJob",
						msg.GetRipStatusUpdate().GetRipJob(),
						"status",
						json.RawMessage(status),
					)
				}
				ripJobWatcher.Set(msg.GetRipStatusUpdate())
			}
		}
	}()

	// Start write loop. writeChan is unbuffered, so ***ALL SENDERS MUST RESPECT THE CONTEXT***
	writeErrChan := make(chan error, 1)
	go func() {
		defer close(writeErrChan)
		for {
			select {
			case <-ctx.Done():
				return
			case msg := <-writeChan:
				driveConn.Send(msg)
			}
		}
	}()

	// Shut down immediately when the context ends
	select {
	case err, ok := <-readErrChan:
		if ok {
			return err
		} else {
			return nil
		}
	case err, ok := <-writeErrChan:
		if ok {
			return err
		} else {
			return nil
		}
	case <-ctx.Done():
		return nil
	}
}

func (coordinator *DriveCoordinatorService) UploadFile(
	fileStream drive_coordinatorv1.DriveCoordinatorService_UploadFileServer,
) error {
	return nil
}

func (coordinator *DriveCoordinatorService) RegisterGrpc(server *grpc.Server) {
	drive_coordinatorv1.RegisterDriveCoordinatorServiceServer(server, coordinator)
}

func (coordinator *DriveCoordinatorService) ListDrives() []*drive_coordinatorv1.DiscoveryInfo {
	drives, unlock := coordinator.driveControllers.GetLocked()
	defer unlock()
	var discoveryInfo []*drive_coordinatorv1.DiscoveryInfo
	for _, drive := range drives {
		discoveryInfo = append(discoveryInfo, drive.discovery)
	}
	return discoveryInfo
}

// func (coordinator *DriveCoordinatorService) ForeachDrive()

func (coordinator *DriveCoordinatorService) GetDriveById(driveId string) *DriveController {
	drives, unlock := coordinator.driveControllers.GetLocked()
	defer unlock()
	if drive, ok := drives[driveId]; ok {
		return drive
	} else {
		return nil
	}
}

// Gets a watcher to see when drives change
//
// Since this watcher tracks a pointer type, the value will be locked when it is received. Take care to unlock the
// value using the returned function as quickly as possible to allow other functions to track the value.
func (coordinator *DriveCoordinatorService) WatchDrives() sync_extras.WatchReceiverLocked[map[string]*DriveController] {
	return coordinator.driveControllers.WatchLocked()
}

// A handle to a drive, which can be used to control it or monitor status
type DriveController struct {
	discovery     *drive_coordinatorv1.DiscoveryInfo
	cancel        context.CancelFunc
	ctx           context.Context // Senders should consider this to prevent deadlocks
	writeChan     chan<- *drive_coordinatorv1.DriveConnectionResponse
	statusWatcher *sync_extras.Watch[*drive_coordinatorv1.DriveStatus]
	ripJobWatcher *sync_extras.Watch[*drive_coordinatorv1.RipStatus]
}

// Returns a watcher that tracks the status of the drive
func (controller *DriveController) DriveStatus() sync_extras.WatchReceiver[*drive_coordinatorv1.DriveStatus] {
	return controller.statusWatcher.Watch()
}

// Returns a watcher that tracks the status of the drive's active rip job
//
// The same watcher is used for multiple rip jobs, so keep this in mind when tracking updates.
// A drive can only have one rip job at a time.
func (controller *DriveController) RipJobStatus() sync_extras.WatchReceiver[*drive_coordinatorv1.RipStatus] {
	return controller.ripJobWatcher.Watch()
}

// Opens the drive tray
func (controller *DriveController) OpenTray() {
	trayCommand := drive_coordinatorv1.TrayCommand_TRAY_COMMAND_OPEN_TRAY
	message := drive_coordinatorv1.DriveConnectionResponse_builder{
		TrayCommand: &trayCommand,
	}.Build()
	select {
	case <-controller.ctx.Done():
	case controller.writeChan <- message:
	}
}

// Closes the drive tray
func (controller *DriveController) CloseTray() {
	trayCommand := drive_coordinatorv1.TrayCommand_TRAY_COMMAND_CLOSE_TRAY
	message := drive_coordinatorv1.DriveConnectionResponse_builder{
		TrayCommand: &trayCommand,
	}.Build()
	select {
	case <-controller.ctx.Done():
	case controller.writeChan <- message:
	}
}

// Closes the drive tray
func (controller *DriveController) RipMedia(jobId int64, autoeject bool) {
	message := drive_coordinatorv1.DriveConnectionResponse_builder{
		RipMedia: drive_coordinatorv1.RipMediaCommand_builder{
			JobId:     jobId,
			Autoeject: autoeject,
		}.Build(),
	}.Build()
	select {
	case <-controller.ctx.Done():
	case controller.writeChan <- message:
	}
}
