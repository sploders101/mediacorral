package main

import (
	"log/slog"
	"net"
	"net/http"
	"os"

	"google.golang.org/grpc"
	"google.golang.org/grpc/reflection"

	"github.com/sploders101/mediacorral/backend/helpers/config"
)

func main() {
	config, err := config.LoadConfig()
	if err != nil {
		slog.Error("An error occurred while reading the config file.", "error", err.Error())
		os.Exit(1)
	}

	// Set up HTTP server & services
	router := http.NewServeMux()
	if err := http.ListenAndServe(config.WebServeAddress, router); err != nil {
		slog.Error("An error occurred while starting the web server.", "error", err.Error())
		os.Exit(1)
	}

	// Set up gRPC server & services
	grpcServer := grpc.NewServer()
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
