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
	"google.golang.org/grpc/status"

	"github.com/sploders101/mediacorral/backend/application"
	"github.com/sploders101/mediacorral/backend/coordinator_api"
	"github.com/sploders101/mediacorral/backend/drive_coordinator"
	"github.com/sploders101/mediacorral/backend/handlers"
	"github.com/sploders101/mediacorral/backend/helpers/config"
)

//go:embed all:frontend
var frontendFiles embed.FS

func main() {
	slog.SetDefault(slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{
		Level: slog.LevelInfo,
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
	coordinator_api.RegisterApiService(router, app)

	var httpHandler http.Handler = router
	if config.OIDC != nil {
		oidcHandler, err := handlers.NewOIDCHandler(config.OIDC, app.Db)
		if err != nil {
			slog.Error("Failed to initialize OIDC handler.", "error", err.Error())
			os.Exit(1)
		}
		oidcHandler.Register(router)
		httpHandler = http.HandlerFunc(func(resp http.ResponseWriter, req *http.Request) {
			// User may currently be in login flow
			bypassAuth := strings.HasPrefix(req.URL.Path, "/auth/")
			// Drive controllers have their own auth
			bypassAuth = bypassAuth || strings.HasPrefix(req.URL.Path, "/mediacorral.drive_coordinator.v1.DriveCoordinatorService/")
			if bypassAuth {
				// Authentication paths don't need to be authorized already.
				// ServeHTTP protects against bad paths by sending a redirect to the canonical
				// form instead, so this is safe.
				router.ServeHTTP(resp, req)
				return
			}
			sessionCookie, err := req.Cookie("session")
			if err != nil {
				http.Redirect(resp, req, "/auth/login", http.StatusFound)
				return
			}
			dbTx, err := app.Db.Begin()
			if err != nil {
				slog.Error("Error beginning transaction", "error", err.Error())
				http.Error(resp, "Internal server error", http.StatusInternalServerError)
			}
			defer func() { _ = dbTx.Rollback() }()
			valid, err := dbTx.ProbeSession(sessionCookie.Value)
			if err != nil {
				slog.Error("Unable to probe session", "error", err.Error())
				http.Error(resp, "Internal server error", http.StatusInternalServerError)
				return
			}
			if !valid {
				http.Redirect(resp, req, "/auth/login", http.StatusFound)
				return
			}
			dbTx.Commit()
			router.ServeHTTP(resp, req)
		})
	}

	// Set up gRPC server & services
	grpcServer := grpc.NewServer(
		grpc.UnaryInterceptor(pskUnaryInterceptor(*config.DriveControllerPsk)),
		grpc.StreamInterceptor(pskStreamInterceptor(*config.DriveControllerPsk)),
		grpc.Creds(insecure.NewCredentials()),
	)
	driveCoordinator.RegisterGrpc(grpcServer)

	// Register gRPC routes
	router.Handle("POST /mediacorral.drive_coordinator.v1.DriveCoordinatorService/", grpcServer)

	// Listen for routes & gRPC calls.
	// h2c makes this a little more complicated here.
	h2s := &http2.Server{}
	server := http.Server{
		Addr:    config.ServeAddress,
		Handler: h2c.NewHandler(httpHandler, h2s),
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
