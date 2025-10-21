package grpcservices

import (
	"context"

	server_pb "github.com/sploders101/mediacorral/mediacorral-server/gen/mediacorral/server/v1"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

type CoordinatorNotificationService struct{}

func (server CoordinatorNotificationService) DiscInserted(
	ctx context.Context,
	request *server_pb.DiscInsertedRequest,
) (*server_pb.DiscInsertedResponse, error) {
	return nil, status.Errorf(codes.Unimplemented, "method DiscInserted not implemented")
}

func (server CoordinatorNotificationService) RipFinished(
	ctx context.Context,
	request *server_pb.RipFinishedRequest,
) (*server_pb.RipFinishedResponse, error) {
	return nil, status.Errorf(codes.Unimplemented, "method RipFinished not implemented")
}

func RegisterNotificationService(server *grpc.Server, coordinator CoordinatorNotificationService) {
	server_pb.RegisterCoordinatorNotificationServiceServer(server, coordinator)
}
