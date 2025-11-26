package twirpservices

import (
	"context"
	"errors"
	"net/http"
	"os"

	"github.com/sploders101/mediacorral/backend/application"
	drive_controller_v1 "github.com/sploders101/mediacorral/backend/gen/mediacorral/drive_controller/v1"
	server_pb "github.com/sploders101/mediacorral/backend/gen/mediacorral/server/v1"

	"github.com/twitchtv/twirp"
	gcodes "google.golang.org/grpc/codes"
	gstatus "google.golang.org/grpc/status"
)

func convertError(err error) error {
	if errors.Is(err, application.ErrNotFound) {
		return twirp.NotFound.Error(err.Error())
	}
	if status, ok := gstatus.FromError(err); ok {
		switch status.Code() {
		case gcodes.NotFound:
			return twirp.NotFound.Error(status.Message())
		case gcodes.Unknown:
			return twirp.Unknown.Error(status.Message())
		}
	}
	return twirp.Unknown.Error(err.Error())
}

type ApiServer struct {
	app *application.Application
}

// Gets textual subtitles
func (server ApiServer) GetSubtitles(
	ctx context.Context,
	request *server_pb.GetSubtitlesRequest,
) (*server_pb.GetSubtitlesResponse, error) {
	// TODO: Verify blob is actually subtitles
	filePath := server.app.BlobStorage.GetFilePath(request.GetBlobId())
	subtitles, err := os.ReadFile(filePath)
	switch {
	case errors.Is(err, os.ErrNotExist):
		return nil, twirp.NotFound.Error("The requested blob does not exist")
	case err != nil:
		return nil, convertError(err)
	}

	return server_pb.GetSubtitlesResponse_builder{
		Subtitles: string(subtitles),
	}.Build(), nil
}

// Searches TheMovieDatabase for a given query
func (server ApiServer) SearchTmdbMulti(
	ctx context.Context,
	request *server_pb.SearchTmdbMultiRequest,
) (*server_pb.SearchTmdbMultiResponse, error) {
	var page uint32 = 1
	if request.HasPage() {
		page = request.GetPage()
	}
	resp, err := server.app.TmdbImporter.QueryAny(
		request.GetQuery(),
		request.GetLanguage(),
		page,
	)
	if err != nil {
		return nil, convertError(err)
	}

	var results []*server_pb.TmdbAnyTitle
	for _, result := range resp.Results {
		var title *string
		if result.Title != "" {
			title = &result.Title
		}
		var backdropPath *string
		if result.BackdropPath != "" {
			backdropPath = &result.BackdropPath
		}
		var posterPath *string
		if result.PosterPath != "" {
			posterPath = &result.PosterPath
		}
		var overview *string
		if result.Overview != "" {
			overview = &result.Overview
		}

		item := server_pb.TmdbAnyTitle_builder{
			Id:           int32(result.ID),
			Type:         result.MediaType,
			Title:        title,
			BackdropPath: backdropPath,
			PosterPath:   posterPath,
			Overview:     overview,
		}.Build()
		results = append(results, item)
	}

	return server_pb.SearchTmdbMultiResponse_builder{
		Page:         uint32(resp.Page),
		TotalPages:   uint32(resp.TotalPages),
		TotalResults: uint32(resp.TotalResults),
		Results:      results,
	}.Build(), nil
}

// Searches TheMovieDatabase for a TV show
func (server ApiServer) SearchTmdbTv(
	ctx context.Context,
	request *server_pb.SearchTmdbTvRequest,
) (*server_pb.SearchTmdbTvResponse, error) {
	resp, err := server.app.TmdbImporter.QueryTv(
		request.GetQuery(),
		request.GetFirstAirDateYear(),
		request.GetLanguage(),
		request.GetYear(),
		request.GetPage(),
	)
	if err != nil {
		return nil, convertError(err)
	}

	var results []*server_pb.TmdbTvResult
	for _, result := range resp.Results {
		var title *string
		if result.Name != "" {
			title = &result.Name
		}
		var originalLanguage *string
		if result.OriginalLanguage != "" {
			originalLanguage = &result.OriginalLanguage
		}
		var overview *string
		if result.Overview != "" {
			overview = &result.Overview
		}
		var posterPath *string
		if result.PosterPath != "" {
			posterPath = &result.PosterPath
		}
		var firstAirDate *string
		if result.FirstAirDate != "" {
			firstAirDate = &result.FirstAirDate
		}

		item := server_pb.TmdbTvResult_builder{
			Id:               int32(result.ID),
			Title:            title,
			OriginCountry:    result.OriginCountry,
			OriginalLanguage: originalLanguage,
			Overview:         overview,
			PosterPath:       posterPath,
			FirstAirDate:     firstAirDate,
		}.Build()
		results = append(results, item)
	}

	return server_pb.SearchTmdbTvResponse_builder{
		Page:         uint32(resp.Page),
		TotalPages:   uint32(resp.TotalPages),
		TotalResults: uint32(resp.TotalResults),
		Results:      results,
	}.Build(), nil
}

// Searches TheMovieDatabase for a Movie
func (server ApiServer) SearchTmdbMovie(
	ctx context.Context,
	request *server_pb.SearchTmdbMovieRequest,
) (*server_pb.SearchTmdbMovieResponse, error) {
	resp, err := server.app.TmdbImporter.QueryMovies(
		request.GetQuery(),
		request.GetLanguage(),
		request.GetPrimaryReleaseYear(),
		request.GetRegion(),
		request.GetYear(),
		request.GetPage(),
	)
	if err != nil {
		return nil, convertError(err)
	}

	var results []*server_pb.TmdbMovieResult
	for _, result := range resp.Results {
		var title *string
		if result.Title != "" {
			title = &result.Title
		}
		var releaseDate *string
		if result.ReleaseDate != "" {
			releaseDate = &result.ReleaseDate
		}
		var originalLanguage *string
		if result.OriginalLanguage != "" {
			originalLanguage = &result.OriginalLanguage
		}
		var posterPath *string
		if result.PosterPath != "" {
			posterPath = &result.PosterPath
		}
		var overview *string
		if result.Overview != "" {
			overview = &result.Overview
		}

		item := server_pb.TmdbMovieResult_builder{
			Id:               int32(result.ID),
			Title:            title,
			ReleaseDate:      releaseDate,
			OriginalLanguage: originalLanguage,
			PosterPath:       posterPath,
			Overview:         overview,
		}.Build()
		results = append(results, item)
	}

	return server_pb.SearchTmdbMovieResponse_builder{
		Page:         uint32(resp.Page),
		TotalPages:   uint32(resp.TotalPages),
		TotalResults: uint32(resp.TotalResults),
		Results:      results,
	}.Build(), nil
}

// Imports a TV show from TheMovieDatabase
func (server ApiServer) ImportTmdbTv(
	ctx context.Context,
	request *server_pb.ImportTmdbTvRequest,
) (*server_pb.ImportTmdbTvResponse, error) {
	dbItem, err := server.app.TmdbImporter.ImportTv(
		int(request.GetTmdbId()),
		server.app.BlobStorage,
	)
	if err != nil {
		return nil, convertError(err)
	}

	return server_pb.ImportTmdbTvResponse_builder{
		TvId: dbItem.Id,
	}.Build(), nil
}

// Imports a Movie from TheMovieDatabase
func (server ApiServer) ImportTmdbMovie(
	ctx context.Context,
	request *server_pb.ImportTmdbMovieRequest,
) (*server_pb.ImportTmdbMovieResponse, error) {
	dbItem, err := server.app.TmdbImporter.ImportMovie(
		int(request.GetTmdbId()),
		server.app.BlobStorage,
	)
	if err != nil {
		return nil, convertError(err)
	}

	return server_pb.ImportTmdbMovieResponse_builder{MovieId: dbItem.Id}.Build(), nil
}

// Rebuild exports directory
func (server ApiServer) RebuildExportsDir(
	ctx context.Context,
	request *server_pb.RebuildExportsDirRequest,
) (*server_pb.RebuildExportsDirResponse, error) {
	if err := server.app.ExportsManager.RebuildDir(request.GetExportsDir(), server.app.BlobStorage); err != nil {
		return nil, convertError(err)
	}

	return server_pb.RebuildExportsDirResponse_builder{}.Build(), nil
}

// Gets/sets the status of the auto-ripper
func (server ApiServer) AutoripStatus(
	ctx context.Context,
	request *server_pb.AutoripStatusRequest,
) (*server_pb.AutoripStatusResponse, error) {
	autoripStatus := request.GetStatus()

	switch request.GetStatus() {
	case server_pb.AutoripStatus_AUTORIP_STATUS_DISABLED:
		server.app.SetAutorip(false)
	case server_pb.AutoripStatus_AUTORIP_STATUS_ENABLED:
		server.app.SetAutorip(true)
	default:
		if server.app.GetAutorip() {
			autoripStatus = server_pb.AutoripStatus_AUTORIP_STATUS_ENABLED
		} else {
			autoripStatus = server_pb.AutoripStatus_AUTORIP_STATUS_DISABLED
		}
	}

	return server_pb.AutoripStatusResponse_builder{
		Status: autoripStatus,
	}.Build(), nil
}

// Lists the currently-registered drives
func (server ApiServer) ListDrives(
	ctx context.Context,
	request *server_pb.ListDrivesRequest,
) (*server_pb.ListDrivesResponse, error) {
	var drives []*server_pb.DiscDrive

	if err := server.app.ForeachDriveController(
		func(controller string, client drive_controller_v1.DriveControllerServiceClient) (bool, error) {
			results, err := client.GetDriveCount(
				ctx,
				drive_controller_v1.GetDriveCountRequest_builder{}.Build(),
			)
			if err != nil {
				return false, err
			}
			driveCount := results.GetDriveCount()
			for driveNum := range driveCount {
				driveMeta, err := client.GetDriveMeta(
					ctx,
					drive_controller_v1.GetDriveMetaRequest_builder{
						DriveId: driveNum,
					}.Build(),
				)
				if err != nil {
					return false, err
				}

				drives = append(drives, server_pb.DiscDrive_builder{
					Controller: controller,
					DriveId:    driveNum,
					Name:       driveMeta.GetName(),
				}.Build())
			}
			return true, nil
		},
	); err != nil {
		return nil, convertError(err)
	}

	return server_pb.ListDrivesResponse_builder{
		Drives: drives,
	}.Build(), nil
}

// Starts a rip job
func (server ApiServer) StartRipJob(
	ctx context.Context,
	request *server_pb.StartRipJobRequest,
) (*server_pb.StartRipJobResponse, error) {
	drive := request.GetDrive()
	ripJobItem, err := server.app.RipMedia(
		drive.GetController(),
		drive.GetDriveId(),
		request.GetSuspectedContents(),
		request.GetAutoeject(),
	)
	if err != nil {
		return nil, convertError(err)
	}
	return server_pb.StartRipJobResponse_builder{JobId: ripJobItem.Id}.Build(), nil
}

// Gets the current status of a rip job
func (server ApiServer) GetRipJobStatus(
	ctx context.Context,
	request *server_pb.GetRipJobStatusRequest,
) (*server_pb.GetRipJobStatusResponse, error) {
	var status *drive_controller_v1.RipStatus
	if err := server.app.ForeachDriveController(
		func(driveController string, client drive_controller_v1.DriveControllerServiceClient) (bool, error) {
			resp, err := client.GetJobStatus(
				ctx,
				drive_controller_v1.GetJobStatusRequest_builder{
					JobId: request.GetJobId(),
				}.Build(),
			)
			if err != nil {
				if gstatus.Code(err) == gcodes.NotFound {
					return true, nil
				}
				return false, err
			}
			status = resp
			return false, nil
		},
	); err != nil {
		return nil, convertError(err)
	}

	if status == nil {
		return nil, twirp.NotFound.Error("The requested job was not found on any drive controller.")
	}
	return server_pb.GetRipJobStatusResponse_builder{
		Status: status,
	}.Build(), nil
}

// Ejects a disc
func (server ApiServer) Eject(
	ctx context.Context,
	request *server_pb.EjectRequest,
) (*server_pb.EjectResponse, error) {
	drive := request.GetDrive()
	controller, ok := server.app.GetDriveController(drive.GetController())
	if !ok {
		return nil, twirp.NotFound.Error("The requested drive controller was not found.")
	}
	_, err := controller.Eject(
		ctx,
		drive_controller_v1.EjectRequest_builder{
			DriveId: drive.GetDriveId(),
		}.Build(),
	)
	if err != nil {
		return nil, convertError(err)
	}
	return server_pb.EjectResponse_builder{}.Build(), nil
}

// Retracts a disc
func (server ApiServer) Retract(
	ctx context.Context,
	request *server_pb.RetractRequest,
) (*server_pb.RetractResponse, error) {
	drive := request.GetDrive()
	controller, ok := server.app.GetDriveController(drive.GetController())
	if !ok {
		return nil, twirp.NotFound.Error("The requested drive controller was not found.")
	}
	_, err := controller.Retract(
		ctx,
		drive_controller_v1.RetractRequest_builder{
			DriveId: drive.GetDriveId(),
		}.Build(),
	)
	if err != nil {
		return nil, convertError(err)
	}
	return server_pb.RetractResponse_builder{}.Build(), nil
}

// Gets the current state of the drive
func (server ApiServer) GetDriveState(
	ctx context.Context,
	request *server_pb.GetDriveStateRequest,
) (*drive_controller_v1.DriveState, error) {
	controller, ok := server.app.GetDriveController(request.GetControllerId())
	if !ok {
		return nil, twirp.NotFound.Error("The requested drive controller was not found.")
	}
	resp, err := controller.GetDriveState(
		ctx,
		drive_controller_v1.GetDriveStateRequest_builder{
			DriveId: request.GetDriveId(),
		}.Build(),
	)
	if err != nil {
		return nil, convertError(err)
	}
	return resp, nil
}

// Lists the movies in the database
func (server ApiServer) ListMovies(
	ctx context.Context,
	request *server_pb.ListMoviesRequest,
) (*server_pb.ListMoviesResponse, error) {
	return nil, twirp.Unimplemented.Error("TODO")
}

// Gets a movie by id
func (server ApiServer) GetMovie(
	ctx context.Context,
	request *server_pb.GetMovieRequest,
) (*server_pb.GetMovieResponse, error) {
	return nil, twirp.Unimplemented.Error("TODO")
}

// Gets a movie from the database by its TMDB ID
func (server ApiServer) GetMovieByTmdbId(
	ctx context.Context,
	request *server_pb.GetMovieByTmdbIdRequest,
) (*server_pb.GetMovieByTmdbIdResponse, error) {
	return nil, twirp.Unimplemented.Error("TODO")
}

// Lists the TV shows in the database
func (server ApiServer) ListTvShows(
	ctx context.Context,
	request *server_pb.ListTvShowsRequest,
) (*server_pb.ListTvShowsResponse, error) {
	return nil, twirp.Unimplemented.Error("TODO")
}

// Lists the seasons for a given TV show
func (server ApiServer) ListTvSeasons(
	ctx context.Context,
	request *server_pb.ListTvSeasonsRequest,
) (*server_pb.ListTvSeasonsResponse, error) {
	return nil, twirp.Unimplemented.Error("TODO")
}

// Lists the episodes for a given season
func (server ApiServer) ListTvEpisodes(
	ctx context.Context,
	request *server_pb.ListTvEpisodesRequest,
) (*server_pb.ListTvEpisodesResponse, error) {
	return nil, twirp.Unimplemented.Error("TODO")
}

// Gets a TV show by id
func (server ApiServer) GetTvShow(
	ctx context.Context,
	request *server_pb.GetTvShowRequest,
) (*server_pb.GetTvShowResponse, error) {
	return nil, twirp.Unimplemented.Error("TODO")
}

// Gets a TV series by id
func (server ApiServer) GetTvSeason(
	ctx context.Context,
	request *server_pb.GetTvSeasonRequest,
) (*server_pb.GetTvSeasonResponse, error) {
	return nil, twirp.Unimplemented.Error("TODO")
}

// Gets a particular TV episode
func (server ApiServer) GetTvEpisode(
	ctx context.Context,
	request *server_pb.GetTvEpisodeRequest,
) (*server_pb.GetTvEpisodeResponse, error) {
	return nil, twirp.Unimplemented.Error("TODO")
}

// Gets a particular TV episode by TMDB id
func (server ApiServer) GetTvEpisodeByTmdbId(
	ctx context.Context,
	request *server_pb.GetTvEpisodeByTmdbIdRequest,
) (*server_pb.GetTvEpisodeByTmdbIdResponse, error) {
	return nil, twirp.Unimplemented.Error("TODO")
}

// Tags a video file with metadata
func (server ApiServer) TagFile(
	ctx context.Context,
	request *server_pb.TagFileRequest,
) (*server_pb.TagFileResponse, error) {
	return nil, twirp.Unimplemented.Error("TODO")
}

// Gets a particular job
func (server ApiServer) GetJobInfo(
	ctx context.Context,
	request *server_pb.GetJobInfoRequest,
) (*server_pb.GetJobInfoResponse, error) {
	return nil, twirp.Unimplemented.Error("TODO")
}

// Renames a job
func (server ApiServer) RenameJob(
	ctx context.Context,
	request *server_pb.RenameJobRequest,
) (*server_pb.RenameJobResponse, error) {
	return nil, twirp.Unimplemented.Error("TODO")
}

// Deletes a job
func (server ApiServer) DeleteJob(
	ctx context.Context,
	request *server_pb.DeleteJobRequest,
) (*server_pb.DeleteJobResponse, error) {
	return nil, twirp.Unimplemented.Error("TODO")
}

// Adds a suspicion to a job
func (server ApiServer) SuspectJob(
	ctx context.Context,
	request *server_pb.SuspectJobRequest,
) (*server_pb.SuspectJobResponse, error) {
	return nil, twirp.Unimplemented.Error("TODO")
}

// Gets a list of jobs containing untagged files
func (server ApiServer) GetUntaggedJobs(
	ctx context.Context,
	request *server_pb.GetUntaggedJobsRequest,
) (*server_pb.GetUntaggedJobsResponse, error) {
	return nil, twirp.Unimplemented.Error("TODO")
}

// Gets all info needed to catalog a job
func (server ApiServer) GetJobCatalogueInfo(
	ctx context.Context,
	request *server_pb.GetJobCatalogueInfoRequest,
) (*server_pb.GetJobCatalogueInfoResponse, error) {
	return nil, twirp.Unimplemented.Error("TODO")
}

// Re-processes all video files in a rip job
func (server ApiServer) ReprocessJob(
	ctx context.Context,
	request *server_pb.ReprocessJobRequest,
) (*server_pb.ReprocessJobResponse, error) {
	return nil, twirp.Unimplemented.Error("TODO")
}

// Prunes a rip job, removing all untagged content
func (server ApiServer) PruneRipJob(
	ctx context.Context,
	request *server_pb.PruneRipJobRequest,
) (*server_pb.PruneRipJobResponse, error) {
	return nil, twirp.Unimplemented.Error("TODO")
}

func RegisterApiService(server *http.ServeMux, app *application.Application) {
	apiHandler := server_pb.NewCoordinatorApiServiceServer(ApiServer{app: app})
	server.Handle("POST "+apiHandler.PathPrefix(), apiHandler)
}
