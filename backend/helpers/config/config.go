package config

import (
	"encoding/json"
	"fmt"
	"os"
	"path"
	"slices"
)

type ConfigFile struct {
	AnalysisCli      *string               `json:"analysis_cli"`
	BasePath         *string               `json:"base_path"`
	DataDirectory    string                `json:"data_directory"`
	TmdbApiKey       string                `json:"tmdb_api_key"`
	OstLogin         OstLoginConfig        `json:"ost_login"`
	WebServeAddress  string                `json:"web_serve_address"`
	GrpcServeAddress string                `json:"grpc_serve_address"`
	ExportsDirs      map[string]ExportsDir `json:"exports_dirs"`
	EnableAutorip    bool                  `json:"enable_autorip"`
	DriveControllers map[string]string     `json:"drive_controllers"`
}

type OstLoginConfig struct {
	ApiKey   string `json:"api_key"`
	Username string `json:"username"`
	Password string `json:"password"`
}

type ExportsDir struct {
	MediaType ExportMediaType `json:"media_type"`
	LinkType  ExportLinkType  `json:"link_type"`
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

	if config.AnalysisCli == nil {
		defaultAnalysisCli := "mediacorral-analysis-cli"
		config.AnalysisCli = &defaultAnalysisCli
	}

	for _, details := range config.ExportsDirs {
		if !slices.Contains([]ExportMediaType{
			EXPORT_MEDIA_TYPE_TV,
			EXPORT_MEDIA_TYPE_MOVIES,
		}, details.MediaType) {
			return ConfigFile{}, fmt.Errorf("invalid media_type %s", details.MediaType)
		}
		if !slices.Contains([]ExportLinkType{
			EXPORT_LINK_TYPE_SYMBOLIC,
			EXPORT_LINK_TYPE_HARD,
		}, details.LinkType) {
			return ConfigFile{}, fmt.Errorf("invalid link_type %s", details.LinkType)
		}
	}

	return config, nil
}
