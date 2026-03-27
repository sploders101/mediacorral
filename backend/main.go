package main

import (
	"context"
	"embed"
	"io/fs"
	"log/slog"
	"net/http"
	"os"
	"path"
	"strings"

	"golang.org/x/net/http2"
	"golang.org/x/net/http2/h2c"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/metadata"
	"google.golang.org/grpc/reflection"
	"google.golang.org/grpc/status"

	"github.com/sploders101/mediacorral/backend/application"
	"github.com/sploders101/mediacorral/backend/drive_coordinator"
	"github.com/sploders101/mediacorral/backend/helpers/config"
	twirpservices "github.com/sploders101/mediacorral/backend/twirp_services"
)

//go:embed all:frontend
var frontendFiles embed.FS

func main() {
	slog.SetDefault(slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{
		Level: slog.LevelDebug,
	})))

	// Load config
	config, err := config.LoadConfig()
	if err != nil {
		slog.Error("An error occurred while reading the config file.", "error", err.Error())
		os.Exit(1)
	}

	// Create drive coordinator
	//
	// This is created separately because it needs to be registered with the gRPC server
	driveCoordinator := drive_coordinator.NewDriveCoordinatorService(
		path.Join(config.DataDirectory, "rips"),
	)

	// Create the application instance
	app, err := application.NewApplication(config, driveCoordinator)
	if err != nil {
		slog.Error("Failed to initialize application service.", "error", err.Error())
		os.Exit(1)
	}

	// Set up HTTP server & services
	router := http.NewServeMux()
	subFs, err := fs.Sub(frontendFiles, "frontend")
	if err != nil {
		panic("Could not get frontend directory")
	}
	router.Handle("GET /", http.FileServerFS(subFs))
	twirpservices.RegisterApiService(router, app)

	// Set up gRPC server & services
	grpcServer := grpc.NewServer(
		grpc.UnaryInterceptor(pskUnaryInterceptor(*config.DriveControllerPsk)),
		grpc.StreamInterceptor(pskStreamInterceptor(*config.DriveControllerPsk)),
		grpc.Creds(insecure.NewCredentials()),
	)
	driveCoordinator.RegisterGrpc(grpcServer)
	reflection.Register(grpcServer)

	// Listen for routes & gRPC calls.
	// h2c makes this a little more complicated here.
	h2s := &http2.Server{}
	server := http.Server{
		Addr: config.ServeAddress,
		Handler: h2c.NewHandler(
			http.HandlerFunc(
				func(response http.ResponseWriter, request *http.Request) {
					if strings.Contains(request.Header.Get("content-type"), "application/grpc") {
						grpcServer.ServeHTTP(response, request)
						return
					}

					router.ServeHTTP(response, request)
				},
			),
			h2s,
		),
	}
	if err := http2.ConfigureServer(&server, h2s); err != nil {
		slog.Error("An error occurred while setting up h2s.", "error", err.Error())
		os.Exit(1)
	}
	err = server.ListenAndServe()
	if err != nil {
		slog.Error("An error occurred while listening on private address.", "error", err.Error())
	}
}

// Validates that a gRPC header contains a pre-shared key
func pskUnaryInterceptor(expectedKey string) grpc.UnaryServerInterceptor {
	return func(
		ctx context.Context,
		req any,
		info *grpc.UnaryServerInfo,
		handler grpc.UnaryHandler,
	) (resp any, err error) {
		md, ok := metadata.FromIncomingContext(ctx)
		if !ok {
			return nil, status.Error(codes.Unauthenticated, "missing metadata")
		}

		key := md.Get("x-api-key")
		if len(key) == 0 || key[0] != expectedKey {
			return nil, status.Error(codes.Unauthenticated, "invalid API key")
		}

		return handler(ctx, req)
	}
}

// Validates that a gRPC header contains a pre-shared key
func pskStreamInterceptor(expectedKey string) grpc.StreamServerInterceptor {
	return func(
		srv any,
		ss grpc.ServerStream,
		info *grpc.StreamServerInfo,
		handler grpc.StreamHandler,
	) error {
		md, ok := metadata.FromIncomingContext(ss.Context())
		if !ok {
			return status.Error(codes.Unauthenticated, "missing metadata")
		}

		key := md.Get("x-api-key")
		if len(key) == 0 || key[0] != expectedKey {
			return status.Error(codes.Unauthenticated, "invalid API key")
		}

		return handler(srv, ss)
	}
}

// TODO: Clear exports directories before rebuild
