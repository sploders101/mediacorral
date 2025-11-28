package main

import (
	"embed"
	"io/fs"
	"log/slog"
	"net"
	"net/http"
	"os"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/reflection"

	"github.com/sploders101/mediacorral/backend/application"
	grpcservices "github.com/sploders101/mediacorral/backend/grpc_services"
	"github.com/sploders101/mediacorral/backend/helpers/config"
	twirpservices "github.com/sploders101/mediacorral/backend/twirp_services"
)

//go:embed all:frontend
var frontendFiles embed.FS

func main() {
	slog.SetDefault(slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{
		Level: slog.LevelDebug,
	})))

	config, err := config.LoadConfig()
	if err != nil {
		slog.Error("An error occurred while reading the config file.", "error", err.Error())
		os.Exit(1)
	}

	app, err := application.NewApplication(config)
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
	go func() {
		if err := http.ListenAndServe(config.WebServeAddress, router); err != nil {
			slog.Error("An error occurred while starting the web server.", "error", err.Error())
			os.Exit(1)
		}
	}()

	// Set up gRPC server & services
	grpcServer := grpc.NewServer(grpc.Creds(insecure.NewCredentials()))
	grpcservices.RegisterNotificationService(grpcServer, app)
	reflection.Register(grpcServer)
	grpcListener, err := net.Listen("tcp", config.GrpcServeAddress)
	if err != nil {
		slog.Error("An error occurred while binding the gRPC server.", "error", err.Error())
		os.Exit(1)
	}
	if err := grpcServer.Serve(grpcListener); err != nil {
		slog.Error("An error occurred while starting the gRPC server.", "error", err.Error())
		os.Exit(1)
	}
}

// TODO: Clear exports directories before rebuild
