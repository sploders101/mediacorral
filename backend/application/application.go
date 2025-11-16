package application

import (
	"errors"
	"fmt"
	"os"
	"path"
	"sync"

	"github.com/sploders101/mediacorral/backend/dbapi"
	drive_controllerv1 "github.com/sploders101/mediacorral/backend/gen/mediacorral/drive_controller/v1"
	"github.com/sploders101/mediacorral/backend/helpers/analysis"
	"github.com/sploders101/mediacorral/backend/helpers/blobs"
	"github.com/sploders101/mediacorral/backend/helpers/config"
	"github.com/sploders101/mediacorral/backend/helpers/exports"
	"github.com/sploders101/mediacorral/backend/helpers/opensubtitles"
	"github.com/sploders101/mediacorral/backend/helpers/tmdb"
	"google.golang.org/grpc"
)

// Application settings which can be changed at runtime
type applicationSettings struct {
	mutex sync.RWMutex

	// Enables automatic ripping on disc insertion
	autoripEnabled bool

	// Drive controllers are responsible for performing the actual ripping process.
	driveControllers map[string]drive_controllerv1.DriveControllerServiceClient
}

// This is the application service layer, which separates the all-encompassing
// application logic from the API layer.
type Application struct {
	db             dbapi.Db
	settings       applicationSettings
	ripDir         string
	blobStorage    *blobs.BlobStorageController
	tmdbImporter   *tmdb.TmdbImporter
	ostImporter    *opensubtitles.OstImporter
	exportsManager *exports.ExportsManager
}

func NewApplication(configData config.ConfigFile) (*Application, error) {
	ripDir := path.Join(configData.DataDirectory, "rips")
	blobDir := path.Join(configData.DataDirectory, "blobs")
	exportsDir := path.Join(configData.DataDirectory, "exports")
	sqlitePath := path.Join(configData.DataDirectory, "database.sqlite")

	// Set up DB
	db, err := dbapi.NewDb(sqlitePath)
	if err != nil {
		return nil, err
	}

	// Create data directories
	for _, dir := range []string{ripDir, blobDir, exportsDir} {
		if err := os.Mkdir(dir, 0755); err != nil && !errors.Is(err, os.ErrExist) {
			return nil, err
		}
	}

	// Set up helpers
	analysisController, err := analysis.NewController(configData.AnalysisCli)
	if err != nil {
		return nil, fmt.Errorf("failed to set up analysis controller: %w", err)
	}
	blobStorage, err := blobs.NewController(blobDir, analysisController)
	if err != nil {
		return nil, fmt.Errorf("failed to set up blob storage: %w", err)
	}
	tmdbImporter, err := tmdb.NewImporter(db, configData.TmdbApiKey)
	if err != nil {
		return nil, fmt.Errorf("failed to set up tmdb importer: %w", err)
	}
	ostImporter, err := opensubtitles.NewOstImporter(
		configData.OstLogin.ApiKey,
		configData.OstLogin.Username,
		configData.OstLogin.Password,
	)
	if err != nil {
		return nil, fmt.Errorf("failed to set up ost importer: %w", err)
	}
	exportsManager, err := exports.NewExportsManager(db, exportsDir, configData.ExportsDirs)
	if err != nil {
		return nil, fmt.Errorf("failed to set up exports manager: %w", err)
	}

	driveControllers := make(
		map[string]drive_controllerv1.DriveControllerServiceClient,
		len(configData.DriveControllers),
	)
	for controllerName, controllerUrl := range configData.DriveControllers {
		conn, err := grpc.NewClient(controllerUrl)
		if err != nil {
			return nil, fmt.Errorf(
				"failed create drive controller client for \"%s\": %w",
				controllerName,
				err,
			)
		}
		driveControllers[controllerName] = drive_controllerv1.NewDriveControllerServiceClient(conn)
	}

	return &Application{
		db:             db,
		settings:       applicationSettings{
			autoripEnabled: configData.EnableAutorip,
			driveControllers: driveControllers,
		},
		ripDir:         ripDir,
		blobStorage:    blobStorage,
		tmdbImporter:   tmdbImporter,
		ostImporter:    ostImporter,
		exportsManager: exportsManager,
	}, nil
}
