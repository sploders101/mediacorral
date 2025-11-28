package grpcservices

import (
	"context"
	"log/slog"
	"strings"

	"github.com/sploders101/mediacorral/backend/application"
	drive_controllerv1 "github.com/sploders101/mediacorral/backend/gen/mediacorral/drive_controller/v1"
	server_pb "github.com/sploders101/mediacorral/backend/gen/mediacorral/server/v1"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

type CoordinatorNotificationService struct {
	app *application.Application
}

func (server CoordinatorNotificationService) DiscInserted(
	ctx context.Context,
	request *server_pb.DiscInsertedRequest,
) (*server_pb.DiscInsertedResponse, error) {
	slog.Debug(
		"Disc inserted",
		"controller", request.GetControllerId(),
		"driveId", request.GetDriveId(),
		"discName", request.GetName(),
	)
	if server.app.GetAutorip() {
		if _, err := server.app.RipMedia(request.GetControllerId(), request.GetDriveId(), nil, true); err != nil {
			slog.Error(
				"An error occurred while dispatching rip job",
				"controllerId", request.GetControllerId(),
				"driveId", request.GetDriveId(),
				"error", err.Error(),
			)
			return nil, status.Errorf(
				codes.Unknown,
				"an unknown error occurred while ripping: %s",
				err.Error(),
			)
		}
	}
	slog.Debug(
		"Finished processing disc insertion",
		"controller", request.GetControllerId(),
		"driveId", request.GetDriveId(),
		"discName", request.GetName(),
	)
	return server_pb.DiscInsertedResponse_builder{}.Build(), nil
}

func (server CoordinatorNotificationService) RipFinished(
	ctx context.Context,
	request *server_pb.RipFinishedRequest,
) (*server_pb.RipFinishedResponse, error) {
	slog.Debug(
		"Rip finished",
		"controller", request.GetControllerId(),
		"ripJob", request.GetJobId(),
	)
	bgCtx := context.Background()
	controller, ok := server.app.GetDriveController(request.GetControllerId())
	if !ok {
		return server_pb.RipFinishedResponse_builder{}.Build(), nil
	}

	jobInfo, err := controller.GetJobStatus(
		bgCtx,
		drive_controllerv1.GetJobStatusRequest_builder{JobId: request.GetJobId()}.Build(),
	)
	if err != nil {
		slog.Error(
			"Controller GetJobStatus failed",
			"controller", request.GetControllerId(),
			"ripJob", request.GetJobId(),
		)
		return nil, status.Errorf(
			codes.Unknown,
			"controller failed to respond with job status: %s",
			err.Error(),
		)
	}

	switch jobInfo.GetStatus() {
	case drive_controllerv1.JobStatus_JOB_STATUS_RUNNING:
		slog.Warn("Job was reported finished but is still running!", "jobId", request.GetJobId())
	case drive_controllerv1.JobStatus_JOB_STATUS_ERROR:
		slog.Error(
			"An error occurred while ripping job",
			"jobId",
			request.GetJobId(),
			"logs",
			strings.Join(jobInfo.GetLogs(), "\n"),
		)
	case drive_controllerv1.JobStatus_JOB_STATUS_COMPLETED:
		go func() {
			if err := server.app.ImportJob(request.GetJobId()); err != nil {
				slog.Error(
					"An error occurred while importing job",
					"jobId", request.GetJobId(),
					"error", err.Error(),
				)
			}
		}()
	default:
		slog.Warn("Unrecognized job status", "jobStatus", jobInfo.GetStatus())
	}

	if _, err := controller.ReapJob(
		bgCtx,
		drive_controllerv1.ReapJobRequest_builder{JobId: request.GetJobId()}.Build(),
	); err != nil {
		slog.Error(
			"Failed to reap job from controller",
			"jobId", request.GetJobId(),
			"error", err.Error(),
		)
	}

	slog.Debug(
		"Finished processing rip job",
		"controller", request.GetControllerId(),
		"ripJob", request.GetJobId(),
	)
	return server_pb.RipFinishedResponse_builder{}.Build(), nil
}

func RegisterNotificationService(server *grpc.Server, app *application.Application) {
	server_pb.RegisterCoordinatorNotificationServiceServer(server, CoordinatorNotificationService{
		app: app,
	})
}
