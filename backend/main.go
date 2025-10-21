package main

import (
	"log/slog"
	"net"
	"net/http"
	"os"

	"google.golang.org/grpc"
	"google.golang.org/grpc/reflection"
)

func main() {
	config, err := LoadConfig()
	if err != nil {
		slog.Error("An error occurred while reading the config file.", "error", err.Error())
		os.Exit(1)
	}

	// Set up HTTP server & services
	router := http.NewServeMux()
	http.ListenAndServe(config.WebServeAddress, router)

	// Set up gRPC server & services
	grpcServer := grpc.NewServer()
	reflection.Register(grpcServer)
	grpcListener, err := net.Listen("tcp", config.GrpcServeAddress)
	if err != nil {
		slog.Error("An error occurred while starting the gRPC server.", "error", err.Error())
		os.Exit(1)
	}
	grpcServer.Serve(grpcListener)
}
