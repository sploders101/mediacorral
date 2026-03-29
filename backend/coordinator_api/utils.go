package coordinator_api

import (
	drive_coordinatorv1 "github.com/sploders101/mediacorral/backend/gen/mediacorral/drive_coordinator/v1"
	server_pb "github.com/sploders101/mediacorral/backend/gen/mediacorral/server/v1"
)

func convertDriveStatus(
	driveId string,
	driveStatus *drive_coordinatorv1.DriveStatus,
	driveRipStatus *drive_coordinatorv1.RipStatus,
) *server_pb.DriveStatus {
	var driveStatusTag server_pb.DriveStatusTag
	switch driveStatus.GetStatus() {
	case drive_coordinatorv1.DriveStatusTag_DRIVE_STATUS_TAG_EMPTY:
		driveStatusTag = server_pb.DriveStatusTag_DRIVE_STATUS_TAG_EMPTY
	case drive_coordinatorv1.DriveStatusTag_DRIVE_STATUS_TAG_TRAY_OPEN:
		driveStatusTag = server_pb.DriveStatusTag_DRIVE_STATUS_TAG_TRAY_OPEN
	case drive_coordinatorv1.DriveStatusTag_DRIVE_STATUS_TAG_NOT_READY:
		driveStatusTag = server_pb.DriveStatusTag_DRIVE_STATUS_TAG_NOT_READY
	case drive_coordinatorv1.DriveStatusTag_DRIVE_STATUS_TAG_DISC_LOADED:
		driveStatusTag = server_pb.DriveStatusTag_DRIVE_STATUS_TAG_DISC_LOADED
	}
	var discName *string
	if driveStatus.HasDiscName() {
		discNameTmp := driveStatus.GetDiscName()
		discName = &discNameTmp
	}

	var ripStatus *server_pb.RipJobStatus
	if driveStatus.HasActiveRipJob() &&
		driveRipStatus != nil &&
		driveRipStatus.GetRipJob() == driveStatus.GetActiveRipJob() {
		ripStatus = server_pb.RipJobStatus_builder{
			JobId:        driveRipStatus.GetRipJob(),
			CprogTitle:   driveRipStatus.GetCprogTitle(),
			TprogTitle:   driveRipStatus.GetTprogTitle(),
			CprogValue:   driveRipStatus.GetCprogValue(),
			TprogValue:   driveRipStatus.GetTprogValue(),
			MaxProgValue: driveRipStatus.GetMaxProgValue(),
			Logs:         driveRipStatus.GetLogs(),
		}.Build()
	}

	return server_pb.DriveStatus_builder{
		DriveId:  driveId,
		Status:   driveStatusTag,
		DiscName: discName,
		RipJob:   ripStatus,
	}.Build()
}
