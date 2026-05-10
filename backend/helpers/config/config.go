package config

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path"
	"slices"
)

var (
	ErrInvalidConfig = errors.New("Invalid config")
)

type ConfigFile struct {
	AnalysisUrl            string                `json:"analysis_url"`
	BasePath               *string               `json:"base_path"`
	DataDirectory          string                `json:"data_directory"`
	DatabasePath           *string               `json:"database_path"`
	TmdbApiKey             *string               `json:"tmdb_api_key"`
	TmdbApiKeyFile         *string               `json:"tmdb_api_key_file"`
	OstLogin               OstLoginConfig        `json:"ost_login"`
	OIDC                   *OIDCConfig           `json:"oidc"`
	ServeAddress           string                `json:"serve_address"`
	DriveControllerPsk     *string               `json:"drive_controller_psk"`
	DriveControllerPskFile *string               `json:"drive_controller_psk_file"`
	ExportsDirs            map[string]ExportsDir `json:"exports_dirs"`
	EnableAutorip          bool                  `json:"enable_autorip"`
}

type OIDCConfig struct {
	Issuer           string   `json:"issuer"`
	ClientID         *string  `json:"client_id"`
	ClientIDFile     *string  `json:"client_id_file"`
	ClientSecret     *string  `json:"client_secret"`
	ClientSecretFile *string  `json:"client_secret_file"`
	RedirectURL      string   `json:"redirect_url"`
	Scopes           []string `json:"scopes"`
}

type OstLoginConfig struct {
	ApiKey       *string `json:"api_key"`
	ApiKeyFile   *string `json:"api_key_file"`
	Username     *string `json:"username"`
	UsernameFile *string `json:"username_file"`
	Password     *string `json:"password"`
	PasswordFile *string `json:"password_file"`
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

	if config.DatabasePath == nil {
		dbPath := path.Join(*config.BasePath, "database.sqlite")
		config.DatabasePath = &dbPath
	} else if !path.IsAbs(*config.DatabasePath) {
		dbPath := path.Join(*config.BasePath, *config.DatabasePath)
		config.DatabasePath = &dbPath
	}

	if config.AnalysisUrl == "" {
		return ConfigFile{}, errors.New("missing analysis service URL in config")
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

	// Resolve drive controller preshared key
	if config.DriveControllerPsk == nil {
		if config.DriveControllerPskFile == nil {
			return ConfigFile{}, fmt.Errorf(
				"%w: missing drive_controller_psk[_file]",
				ErrInvalidConfig,
			)
		}
		driveControllerPsk, err := os.ReadFile(*config.DriveControllerPskFile)
		if err != nil {
			return ConfigFile{}, fmt.Errorf("error reading drive_controller_psk_file: %w", err)
		}
		apiKey := string(driveControllerPsk)
		config.DriveControllerPsk = &apiKey
	}

	// Resolve TMDB API key
	if config.TmdbApiKey == nil {
		if config.TmdbApiKeyFile == nil {
			return ConfigFile{}, fmt.Errorf("%w: missing tmdb_api_key[_file]", ErrInvalidConfig)
		}
		apiKeyRaw, err := os.ReadFile(*config.TmdbApiKeyFile)
		if err != nil {
			return ConfigFile{}, fmt.Errorf("error reading tmdb_api_key_file: %w", err)
		}
		apiKey := string(apiKeyRaw)
		config.TmdbApiKey = &apiKey
	}

	// Resolve OST API key
	if config.OstLogin.ApiKey == nil {
		if config.OstLogin.ApiKeyFile == nil {
			return ConfigFile{}, fmt.Errorf(
				"%w: missing ost_login.api_key[_file]",
				ErrInvalidConfig,
			)
		}
		apiKeyRaw, err := os.ReadFile(*config.OstLogin.ApiKeyFile)
		if err != nil {
			return ConfigFile{}, fmt.Errorf("error reading ost_login.api_key_file: %w", err)
		}
		apiKey := string(apiKeyRaw)
		config.OstLogin.ApiKey = &apiKey
	}

	// Resolve OST Username
	if config.OstLogin.Username == nil {
		if config.OstLogin.UsernameFile == nil {
			return ConfigFile{}, fmt.Errorf(
				"%w: missing ost_login.username[_file]",
				ErrInvalidConfig,
			)
		}
		usernameRaw, err := os.ReadFile(*config.OstLogin.UsernameFile)
		if err != nil {
			return ConfigFile{}, fmt.Errorf("error reading ost_login.username_file: %w", err)
		}
		username := string(usernameRaw)
		config.OstLogin.Username = &username
	}

	// Resolve OST Password
	if config.OstLogin.Password == nil {
		if config.OstLogin.PasswordFile == nil {
			return ConfigFile{}, fmt.Errorf(
				"%w: missing ost_login.password[_file]",
				ErrInvalidConfig,
			)
		}
		passwordRaw, err := os.ReadFile(*config.OstLogin.PasswordFile)
		if err != nil {
			return ConfigFile{}, fmt.Errorf("error reading ost_login.password_file: %w", err)
		}
		password := string(passwordRaw)
		config.OstLogin.Password = &password
	}

	// Resolve OIDC credentials
	if config.OIDC != nil {
		if config.OIDC.ClientID == nil {
			if config.OIDC.ClientIDFile == nil {
				return ConfigFile{}, fmt.Errorf(
					"%w: missing oidc.client_id[_file]",
					ErrInvalidConfig,
				)
			}
			clientIDRaw, err := os.ReadFile(*config.OIDC.ClientIDFile)
			if err != nil {
				return ConfigFile{}, fmt.Errorf("error reading oidc.client_id_file: %w", err)
			}
			clientID := string(clientIDRaw)
			config.OIDC.ClientID = &clientID
		}
		if config.OIDC.ClientSecret == nil {
			if config.OIDC.ClientSecretFile == nil {
				return ConfigFile{}, fmt.Errorf(
					"%w: missing oidc.client_secret[_file]",
					ErrInvalidConfig,
				)
			}
			clientSecretRaw, err := os.ReadFile(*config.OIDC.ClientSecretFile)
			if err != nil {
				return ConfigFile{}, fmt.Errorf("error reading oidc.client_secret_file: %w", err)
			}
			clientSecret := string(clientSecretRaw)
			config.OIDC.ClientSecret = &clientSecret
		}
	}

	return config, nil
}
