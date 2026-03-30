package drive_coordinator

import (
	"bufio"
	"context"
	"crypto/md5"
	"encoding/json"
	"errors"
	"io"
	"log/slog"
	"maps"
	"os"
	"path"
	"slices"
	"strconv"
	"sync"

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
	ripJobUploads    *sync_extras.Watch[map[int64]*RipUploadTracker]
	ripDir           string
	// Used to communicate that a job has finished, and has begun uploading.
	// This *must* be consumed to avoid deadlocks. It's not easy for the drive controller to mark this directly
	// without introducing circular references, which is why this is a channel.
	FinishedQueue chan int64
	// Used to communicate that a job upload has finished, and should be imported.
	// This *must* be consumed to avoid deadlocks. It's not easy for the drive controller to mark this directly
	// without introducing circular references, which is why this is a channel.
	ImportQueue chan int64
}

func NewDriveCoordinatorService(ripDir string) *DriveCoordinatorService {
	return &DriveCoordinatorService{
		driveControllers: sync_extras.NewWatch(map[string]*DriveController{}),
		ripJobUploads:    sync_extras.NewWatch(map[int64]*RipUploadTracker{}),
		ripDir:           ripDir,
		FinishedQueue:    make(chan int64),
		ImportQueue:      make(chan int64),
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

func (coordinator *DriveCoordinatorService) FinalizeRipJob(
	request drive_coordinatorv1.DriveCoordinatorService_FinalizeRipJobServer,
) error {
	// Watch for header message
	headerRaw, err := request.Recv()
	if err != nil && !errors.Is(err, io.EOF) {
		slog.Error("Finalization read error", "error", err.Error())
		return err
	}
	if !headerRaw.HasHeader() {
		// First packet must always be a discovery packet so we know how to catalog it
		slog.Error("Finalization protocol error", "error", "first message was not discovery")
		return ErrProtocol
	}
	header := headerRaw.GetHeader()
	ripJob := header.GetRipJob()
	files := header.GetUploadFiles()

	// Create directory
	ripDir := path.Join(
		coordinator.ripDir,
		strconv.FormatInt(ripJob, 10),
	)
	if err := os.MkdirAll(ripDir, os.FileMode(0o777)); err != nil {
		if !errors.Is(err, os.ErrExist) {
			return err
		}
	}
	coordinator.FinishedQueue <- ripJob

	finishedFiles, err := getFinishedFiles(ripDir, files)
	if err != nil {
		// This isn't *really* a fatal error, but if this fails, something is almost definitely wrong with the ripDir.
		slog.Error("Error getting finished files", "ripJob", ripJob, "error", err.Error())
		return err
	}
	uploadProgress := make(map[string]UploadProgress, len(finishedFiles)+len(files))
	for _, file := range files {
		uploadProgress[file.GetFileName()] = UploadProgress{
			Received:  0,
			TotalSize: file.GetFileSize(),
		}
	}
	for _, file := range finishedFiles {
		uploadProgress[file.GetFileName()] = UploadProgress{
			Received:  file.GetFileSize(),
			TotalSize: file.GetFileSize(),
		}
	}
	uploadTracker := &RipUploadTracker{
		FileList:       slices.Concat(finishedFiles, files),
		uploadProgress: sync_extras.NewWatch(uploadProgress),
	}

	// Set up rip tracker
	coordinator.ripJobUploads.ConditionalSet(func(uploads *map[int64]*RipUploadTracker) bool {
		(*uploads)[ripJob] = uploadTracker
		return true
	})
	defer func() {
		coordinator.ripJobUploads.ConditionalSet(func(uploads *map[int64]*RipUploadTracker) bool {
			if tracker, ok := (*uploads)[ripJob]; ok {
				if tracker == uploadTracker {
					delete(*uploads, ripJob)
					return true
				}
			}
			return false
		})
	}()

	var corruptedFilesMtx sync.Mutex
	var corruptedFiles []string

	// Receive files
	if len(files) > 0 {
		var closers sync.WaitGroup

		uploadIndex := 0
		filePath := path.Join(ripDir, files[uploadIndex].GetFileName())
		fileRaw, err := os.Create(filePath)
		if err != nil {
			slog.Error(
				"Couldn't create upload file",
				"ripJob",
				ripJob,
				"filePath",
				filePath,
				"error",
				err.Error(),
			)
			return err
		}
		file := bufio.NewWriter(fileRaw)
		for uploadIndex < len(files) {
			msg, err := request.Recv()
			if err != nil {
				file.Flush()
				fileRaw.Close()
				return err
			}

			switch {
			case msg.HasHeader():
				file.Flush()
				fileRaw.Close()
				return ErrProtocol
			case msg.HasDataChunk():
				chunk := msg.GetDataChunk()
				if _, err := file.Write(chunk); err != nil {
					return err
				}
				uploadTracker.uploadProgress.ConditionalSet(
					func(fileMap *map[string]UploadProgress) bool {
						fileMapDeref := *fileMap
						fileName := files[uploadIndex].GetFileName()
						progress := fileMapDeref[fileName]
						progress.Received += uint64(len(chunk))
						fileMapDeref[fileName] = progress
						return true
					},
				)
			case msg.HasMd5Hash():
				uploadIndex += 1

				// Close the file
				file.Flush()

				// Start checking the hash
				expectedHash := msg.GetMd5Hash()
				hashFilePath := filePath
				hashFileRaw := fileRaw
				if _, err := hashFileRaw.Seek(0, io.SeekStart); err != nil {
					return err
				}
				closers.Go(func() {
					defer hashFileRaw.Close()
					hasher := md5.New()
					hashFile := bufio.NewReader(hashFileRaw)
					fileChunk := make([]byte, hasher.BlockSize())
					for {
						bytesRead, err := hashFile.Read(fileChunk)
						if err != nil && err != io.EOF {
							slog.Error("Unable to read uploaded file", "error", err.Error())
							corruptedFilesMtx.Lock()
							defer corruptedFilesMtx.Unlock()
							corruptedFiles = append(corruptedFiles, hashFilePath)
							return
						}
						if _, err := hasher.Write(fileChunk[:bytesRead]); err != nil {
							slog.Error(
								"Unable to calculate ms5 sum of uploaded file",
								"error",
								err.Error(),
							)
							return
						}
						if err == io.EOF {
							hash := hasher.Sum(nil)
							if !slices.Equal(hash, expectedHash) {
								corruptedFilesMtx.Lock()
								defer corruptedFilesMtx.Unlock()
								corruptedFiles = append(corruptedFiles, hashFilePath)
							}
							return
						}
					}
				})

				// Rotate to next file
				if uploadIndex < len(files) {
					filePath = path.Join(ripDir, files[uploadIndex].GetFileName())
					fileRaw, err = os.Create(filePath)
					if err != nil {
						return err
					}
					file = bufio.NewWriter(fileRaw)
				}
			}
		}

		closers.Wait()
	}

	if len(corruptedFiles) == 0 {
		coordinator.ImportQueue <- ripJob
		slog.Info("Successfully downloaded ripped media.", "ripJob", ripJob)
	} else {
		slog.Error(
			"Files corrupted during download.",
			"ripJob",
			ripJob,
			"corruptedFiles",
			corruptedFiles,
		)
	}

	request.SendAndClose(drive_coordinatorv1.FinalizeRipJobResponse_builder{
		CorruptedFiles: corruptedFiles,
	}.Build())

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

// Gets a list of IDs for rip jobs that are currently being uploaded
func (coordinator *DriveCoordinatorService) ListUploadingJobs() []int64 {
	ripJobs, unlock := coordinator.ripJobUploads.GetLocked()
	defer unlock()
	return slices.Collect(maps.Keys(ripJobs))
}

// Gets an upload watcher for a given rip job.
//
// Returns a pointer to an upload tracker, which will be nil if the job is not currently uploading.
func (coordinator *DriveCoordinatorService) GetUploadTracker(ripJob int64) *RipUploadTracker {
	ripJobs, unlock := coordinator.ripJobUploads.GetLocked()
	defer unlock()
	if watcher, ok := ripJobs[ripJob]; ok {
		return watcher
	}
	return nil
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

// Returns the ID of the drive
func (controller *DriveController) DriveId() string {
	return controller.discovery.GetDriveId()
}

// Returns the name of the drive
func (controller *DriveController) DriveName() string {
	return controller.discovery.GetDriveName()
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

type UploadProgress struct {
	Received  uint64
	TotalSize uint64
}

// Tracks the upload progress of a rip job. This only shows the progress of the latest call to `FinalizeRipJob`.
type RipUploadTracker struct {
	FileList       []*drive_coordinatorv1.FileDescription
	uploadProgress *sync_extras.Watch[map[string]UploadProgress]
}

func (tracker *RipUploadTracker) GetUploadProgress() map[string]UploadProgress {
	progress, unlock := tracker.uploadProgress.GetLocked()
	defer unlock()
	return maps.Clone(progress)
}

func (tracker *RipUploadTracker) WatchUploadProgress() sync_extras.WatchReceiverLocked[map[string]UploadProgress] {
	return tracker.uploadProgress.WatchLocked()
}
