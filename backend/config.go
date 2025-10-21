package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path"
	"slices"
)

type ConfigFile struct {
	BasePath         *string               `yaml:"base_path"`
	DataDirectory    string                `yaml:"data_directory"`
	TmdbApiKey       string                `yaml:"tmdb_api_key"`
	OstLogin         OstLoginConfig        `yaml:"ost_login"`
	WebServeAddress  string                `yaml:"web_serve_address"`
	GrpcServeAddress string                `yaml:"grpc_serve_address"`
	ExportsDirs      map[string]ExportsDir `yaml:"exports_dirs"`
	EnableAutorip    bool                  `yaml:"enable_autorip"`
	DriveControllers map[string]string     `yaml:"drive_controllers"`
}

type OstLoginConfig struct {
	ApiKey   string `yaml:"api_key"`
	Username string `yaml:"username"`
	Password string `yaml:"password"`
}

type ExportsDir struct {
	MediaType ExportMediaType `yaml:"media_type"`
	LinkType  ExportLinkType  `yaml:"link_type"`
}

type ExportMediaType string

const (
	EXPORT_MEDIA_TYPE_TV     ExportMediaType = "TvShows"
	EXPORT_MEDIA_TYPE_MOVIES ExportMediaType = "Movies"
)

type ExportLinkType string

const (
	EXPORT_LINK_TYPE_SYMBOLIC ExportLinkType = "Symbolic"
	EXPORT_LINK_TYPE_HARD     ExportLinkType = "Hard"
)

func LoadConfig() (ConfigFile, error) {
	filePath := "./config/config.json"
	if env := os.Getenv("CONFIG_PATH"); env != "" {
		filePath = env
	}

	contents, err := os.ReadFile(filePath)
	if err != nil {
		return ConfigFile{}, err
	}

	var config ConfigFile
	if err := json.Unmarshal(contents, &config); err != nil {
		return ConfigFile{}, err
	}

	// Add BasePath if it is unspecified
	if config.BasePath == nil {
		basePath := path.Dir(filePath)
		config.BasePath = &basePath
	}

	if !path.IsAbs(config.DataDirectory) {
		config.DataDirectory = path.Join(*config.BasePath, config.DataDirectory)
	}

	for _, details := range config.ExportsDirs {
		if slices.Contains([]ExportMediaType{
			EXPORT_MEDIA_TYPE_TV,
			EXPORT_MEDIA_TYPE_MOVIES,
		}, details.MediaType) {
			return ConfigFile{}, fmt.Errorf("invalid media_type %s", details.MediaType)
		}
		if slices.Contains([]ExportLinkType{
			EXPORT_LINK_TYPE_SYMBOLIC,
			EXPORT_LINK_TYPE_HARD,
		}, details.LinkType) {
			return ConfigFile{}, fmt.Errorf("invalid link_type %s", details.LinkType)
		}
	}

	return config, nil
}
