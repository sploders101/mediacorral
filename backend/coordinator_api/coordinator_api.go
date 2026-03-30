package coordinator_api

import (
	"context"
	"database/sql"
	"errors"
	"fmt"
	"net/http"
	"os"
	"sync"

	"connectrpc.com/connect"
	"github.com/sploders101/mediacorral/backend/application"
	"github.com/sploders101/mediacorral/backend/dbapi"
	analysis_pb "github.com/sploders101/mediacorral/backend/gen/mediacorral/analysis/v1"
	drive_coordinatorv1 "github.com/sploders101/mediacorral/backend/gen/mediacorral/drive_coordinator/v1"
	server_pb "github.com/sploders101/mediacorral/backend/gen/mediacorral/server/v1"
	server_connect "github.com/sploders101/mediacorral/backend/gen/mediacorral/server/v1/serverv1connect"
	"github.com/sploders101/mediacorral/backend/helpers/sync_extras"

	gcodes "google.golang.org/grpc/codes"
	gstatus "google.golang.org/grpc/status"
	"google.golang.org/protobuf/proto"
)

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
		return nil, connect.NewError(
			connect.CodeNotFound,
			errors.New("The requested blob does not exist"),
		)
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
	if err := server.app.ExportsManager.RebuildDir(
		request.GetExportsDir(),
		server.app.BlobStorage,
	); err != nil {
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
	driveDiscovery := server.app.DriveCoordinator.ListDrives()
	var drives []*server_pb.DiscDrive
	for _, drive := range driveDiscovery {
		drives = append(drives, server_pb.DiscDrive_builder{
			Id:   drive.GetDriveId(),
			Name: drive.GetDriveName(),
		}.Build())
	}

	return server_pb.ListDrivesResponse_builder{
		Drives: drives,
	}.Build(), nil
}

// Streams the list the currently-registered drives
func (server ApiServer) StreamDrivesList(
	ctx context.Context,
	request *server_pb.StreamDrivesListRequest,
	resp *connect.ServerStream[server_pb.StreamDrivesListResponse],
) error {
	driveDiscovery := server.app.DriveCoordinator.WatchDrives()
	driveDiscovery.SetContext(ctx)
	for {
		drivesMap, unlock, err := driveDiscovery.Changed()
		if err != nil {
			return nil
		}

		var drives []*server_pb.DiscDrive
		for _, drive := range drivesMap {
			drives = append(drives, server_pb.DiscDrive_builder{
				Id:   drive.DriveId(),
				Name: drive.DriveName(),
			}.Build())
		}
		unlock()
		resp.Send(server_pb.StreamDrivesListResponse_builder{
			Drives: drives,
		}.Build())
	}
}

// Starts a rip job
func (server ApiServer) StartRipJob(
	ctx context.Context,
	request *server_pb.StartRipJobRequest,
) (*server_pb.StartRipJobResponse, error) {
	driveId := request.GetDriveId()
	ripJobItem, err := server.app.RipMedia(
		driveId,
		request.GetSuspectedContents(),
		request.GetAutoeject(),
	)
	if err != nil {
		return nil, convertError(err)
	}
	return server_pb.StartRipJobResponse_builder{JobId: ripJobItem.Id}.Build(), nil
}

// Ejects a disc
func (server ApiServer) Eject(
	ctx context.Context,
	request *server_pb.EjectRequest,
) (*server_pb.EjectResponse, error) {
	driveId := request.GetDriveId()
	controller := server.app.DriveCoordinator.GetDriveById(driveId)
	if controller == nil {
		return nil, connect.NewError(
			connect.CodeNotFound,
			errors.New("The requested drive controller was not found."),
		)
	}
	controller.OpenTray()
	return server_pb.EjectResponse_builder{}.Build(), nil
}

// Retracts a disc
func (server ApiServer) Retract(
	ctx context.Context,
	request *server_pb.RetractRequest,
) (*server_pb.RetractResponse, error) {
	driveId := request.GetDriveId()
	controller := server.app.DriveCoordinator.GetDriveById(driveId)
	if controller == nil {
		return nil, connect.NewError(
			connect.CodeNotFound,
			errors.New("The requested drive controller was not found."),
		)
	}
	controller.CloseTray()
	return server_pb.RetractResponse_builder{}.Build(), nil
}

// Gets the current status of the drive
func (server ApiServer) GetDriveStatus(
	ctx context.Context,
	request *server_pb.GetDriveStatusRequest,
) (*server_pb.GetDriveStatusResponse, error) {
	controller := server.app.DriveCoordinator.GetDriveById(request.GetDriveId())
	if controller == nil {
		return nil, connect.NewError(
			connect.CodeNotFound,
			errors.New("The requested drive controller was not found."),
		)
	}

	driveStatusWatcher := controller.DriveStatus()
	driveStatus := driveStatusWatcher.Get()

	ripStatusWatcher := controller.RipJobStatus()
	driveRipStatus := ripStatusWatcher.Get()

	return server_pb.GetDriveStatusResponse_builder{
		DriveStatus: convertDriveStatus(request.GetDriveId(), driveStatus, driveRipStatus),
	}.Build(), nil
}

// Streams the drive status
func (server ApiServer) StreamDriveStatus(
	ctx context.Context,
	req *server_pb.StreamDriveStatusRequest,
	resp *connect.ServerStream[server_pb.StreamDriveStatusResponse],
) error {
	controller := server.app.DriveCoordinator.GetDriveById(req.GetDriveId())
	if controller == nil {
		return connect.NewError(
			connect.CodeNotFound,
			errors.New("The requested drive controller was not found."),
		)
	}

	driveStatusWatcher := controller.DriveStatus()
	driveStatusWatcher.SetContext(ctx)
	ripStatusWatcher := controller.RipJobStatus()
	ripStatusWatcher.SetContext(ctx)

	var wg sync.WaitGroup
	var varLock sync.Mutex
	var driveStatus *drive_coordinatorv1.DriveStatus
	var ripStatus *drive_coordinatorv1.RipStatus

	wg.Add(2)
	sendUpdate := func() {
		varLock.Lock()
		defer varLock.Unlock()

		resp.Send(server_pb.StreamDriveStatusResponse_builder{
			DriveStatus: convertDriveStatus(req.GetDriveId(), driveStatus, ripStatus),
		}.Build())
	}
	go func() {
		defer wg.Done()
		for {
			var err error
			driveStatus, err = driveStatusWatcher.Changed()
			if err != nil {
				return
			}
			sendUpdate()
		}
	}()
	go func() {
		defer wg.Done()
		for {
			var err error
			ripStatus, err = ripStatusWatcher.Changed()
			if err != nil {
				return
			}
			sendUpdate()
		}
	}()
	wg.Wait()
	return nil
}

// Lists the movies in the database
func (server ApiServer) ListMovies(
	ctx context.Context,
	request *server_pb.ListMoviesRequest,
) (*server_pb.ListMoviesResponse, error) {
	dbTx, err := server.app.Db.BeginTx(ctx, &sql.TxOptions{ReadOnly: true})
	if err != nil {
		return nil, convertError(err)
	}
	defer func() { _ = dbTx.Rollback() }()
	movies, err := dbTx.GetMovies()
	if err != nil {
		return nil, convertError(err)
	}
	var protoMovies []*server_pb.Movie
	for _, movie := range movies {
		protoMovies = append(protoMovies, movieDbToProto(movie))
	}
	return server_pb.ListMoviesResponse_builder{
		Movies: protoMovies,
	}.Build(), nil
}

// Gets a movie by id
func (server ApiServer) GetMovie(
	ctx context.Context,
	request *server_pb.GetMovieRequest,
) (*server_pb.GetMovieResponse, error) {
	dbTx, err := server.app.Db.BeginTx(ctx, &sql.TxOptions{ReadOnly: true})
	if err != nil {
		return nil, convertError(err)
	}
	defer func() { _ = dbTx.Rollback() }()

	movie, err := dbTx.GetMovieById(request.GetMovieId())
	if err != nil {
		return nil, convertError(err)
	}

	return server_pb.GetMovieResponse_builder{
		Movie: movieDbToProto(movie),
	}.Build(), nil
}

// Gets a movie from the database by its TMDB ID
func (server ApiServer) GetMovieByTmdbId(
	ctx context.Context,
	request *server_pb.GetMovieByTmdbIdRequest,
) (*server_pb.GetMovieByTmdbIdResponse, error) {
	dbTx, err := server.app.Db.BeginTx(ctx, &sql.TxOptions{ReadOnly: true})
	if err != nil {
		return nil, convertError(err)
	}
	defer func() { _ = dbTx.Rollback() }()

	movie, err := dbTx.GetMovieByTmdbId(request.GetTmdbId())
	if err != nil {
		return nil, convertError(err)
	}

	return server_pb.GetMovieByTmdbIdResponse_builder{
		Movie: movieDbToProto(movie),
	}.Build(), nil
}

// Lists the TV shows in the database
func (server ApiServer) ListTvShows(
	ctx context.Context,
	request *server_pb.ListTvShowsRequest,
) (*server_pb.ListTvShowsResponse, error) {
	dbTx, err := server.app.Db.BeginTx(ctx, &sql.TxOptions{ReadOnly: true})
	if err != nil {
		return nil, convertError(err)
	}
	defer func() { _ = dbTx.Rollback() }()

	tvShows, err := dbTx.GetTvShows()
	if err != nil {
		return nil, convertError(err)
	}

	var protoTvShows []*server_pb.TvShow
	for _, dbTvShow := range tvShows {
		protoTvShows = append(protoTvShows, tvShowDbToProto(dbTvShow))
	}

	return server_pb.ListTvShowsResponse_builder{
		TvShows: protoTvShows,
	}.Build(), nil
}

// Lists the seasons for a given TV show
func (server ApiServer) ListTvSeasons(
	ctx context.Context,
	request *server_pb.ListTvSeasonsRequest,
) (*server_pb.ListTvSeasonsResponse, error) {
	dbTx, err := server.app.Db.BeginTx(ctx, &sql.TxOptions{ReadOnly: true})
	if err != nil {
		return nil, convertError(err)
	}
	defer func() { _ = dbTx.Rollback() }()

	seasons, err := dbTx.GetTvSeasons(request.GetSeriesId())
	if err != nil {
		return nil, convertError(err)
	}

	var protoSeasons []*server_pb.TvSeason
	for _, season := range seasons {
		protoSeasons = append(protoSeasons, tvSeasonDbToProto(season))
	}

	return server_pb.ListTvSeasonsResponse_builder{
		SeriesId:  request.GetSeriesId(),
		TvSeasons: protoSeasons,
	}.Build(), nil
}

// Lists the episodes for a given season
func (server ApiServer) ListTvEpisodes(
	ctx context.Context,
	request *server_pb.ListTvEpisodesRequest,
) (*server_pb.ListTvEpisodesResponse, error) {
	dbTx, err := server.app.Db.BeginTx(ctx, &sql.TxOptions{ReadOnly: true})
	if err != nil {
		return nil, convertError(err)
	}
	defer func() { _ = dbTx.Rollback() }()

	episodes, err := dbTx.GetTvEpisodes(request.GetTvSeasonId())
	if err != nil {
		return nil, convertError(err)
	}

	var protoEpisodes []*server_pb.TvEpisode
	for _, episode := range episodes {
		protoEpisodes = append(protoEpisodes, tvEpisodeDbToProto(episode))
	}

	return server_pb.ListTvEpisodesResponse_builder{
		TvSeasonId: request.GetTvSeasonId(),
		TvEpisodes: protoEpisodes,
	}.Build(), nil
}

// Gets a TV show by id
func (server ApiServer) GetTvShow(
	ctx context.Context,
	request *server_pb.GetTvShowRequest,
) (*server_pb.GetTvShowResponse, error) {
	dbTx, err := server.app.Db.BeginTx(ctx, &sql.TxOptions{ReadOnly: true})
	if err != nil {
		return nil, convertError(err)
	}
	defer func() { _ = dbTx.Rollback() }()

	tvShow, err := dbTx.GetTvShowById(request.GetShowId())
	if err != nil {
		return nil, convertError(err)
	}

	return server_pb.GetTvShowResponse_builder{
		TvShow: tvShowDbToProto(tvShow),
	}.Build(), nil
}

// Gets a TV series by id
func (server ApiServer) GetTvSeason(
	ctx context.Context,
	request *server_pb.GetTvSeasonRequest,
) (*server_pb.GetTvSeasonResponse, error) {
	dbTx, err := server.app.Db.BeginTx(ctx, &sql.TxOptions{ReadOnly: true})
	if err != nil {
		return nil, convertError(err)
	}
	defer func() { _ = dbTx.Rollback() }()

	tvSeason, err := dbTx.GetTvSeasonById(request.GetSeasonId())
	if err != nil {
		return nil, convertError(err)
	}

	return server_pb.GetTvSeasonResponse_builder{
		TvSeason: tvSeasonDbToProto(tvSeason),
	}.Build(), nil
}

// Gets a particular TV episode
func (server ApiServer) GetTvEpisode(
	ctx context.Context,
	request *server_pb.GetTvEpisodeRequest,
) (*server_pb.GetTvEpisodeResponse, error) {
	dbTx, err := server.app.Db.BeginTx(ctx, &sql.TxOptions{ReadOnly: true})
	if err != nil {
		return nil, convertError(err)
	}
	defer func() { _ = dbTx.Rollback() }()

	tvEpisode, err := dbTx.GetTvEpisodeById(request.GetEpisodeId())
	if err != nil {
		return nil, convertError(err)
	}

	return server_pb.GetTvEpisodeResponse_builder{
		Episode: tvEpisodeDbToProto(tvEpisode),
	}.Build(), nil
}

// Gets a particular TV episode by TMDB id
func (server ApiServer) GetTvEpisodeByTmdbId(
	ctx context.Context,
	request *server_pb.GetTvEpisodeByTmdbIdRequest,
) (*server_pb.GetTvEpisodeByTmdbIdResponse, error) {
	dbTx, err := server.app.Db.BeginTx(ctx, &sql.TxOptions{ReadOnly: true})
	if err != nil {
		return nil, convertError(err)
	}
	defer func() { _ = dbTx.Rollback() }()

	episode, err := dbTx.GetTvEpisodeByTmdbId(request.GetTmdbId())
	if err != nil {
		return nil, convertError(err)
	}

	return server_pb.GetTvEpisodeByTmdbIdResponse_builder{
		Episode: tvEpisodeDbToProto(episode),
	}.Build(), nil
}

// Tags a video file with metadata
func (server ApiServer) TagFile(
	ctx context.Context,
	request *server_pb.TagFileRequest,
) (*server_pb.TagFileResponse, error) {
	dbTx, err := server.app.Db.BeginTx(ctx, &sql.TxOptions{ReadOnly: true})
	if err != nil {
		return nil, convertError(err)
	}
	defer func() { _ = dbTx.Rollback() }()

	var matchId sql.NullInt64
	if request.HasMatchId() {
		matchId.Valid = true
		matchId.Int64 = request.GetMatchId()
	}
	if err := dbTx.TagVideoFile(
		request.GetFile(),
		request.GetVideoType(),
		matchId,
	); err != nil {
		return nil, convertError(err)
	}
	if err := dbTx.Commit(); err != nil {
		return nil, convertError(err)
	}

	if err := server.app.ExportsManager.SpliceContent(
		request.GetVideoType(),
		request.GetFile(),
		server.app.BlobStorage,
	); err != nil {
		return nil, convertError(err)
	}

	return server_pb.TagFileResponse_builder{}.Build(), nil
}

// Gets a particular job
func (server ApiServer) GetJobInfo(
	ctx context.Context,
	request *server_pb.GetJobInfoRequest,
) (*server_pb.GetJobInfoResponse, error) {
	dbTx, err := server.app.Db.BeginTx(ctx, &sql.TxOptions{ReadOnly: true})
	if err != nil {
		return nil, convertError(err)
	}
	defer func() { _ = dbTx.Rollback() }()

	ripJob, err := dbTx.GetRipJob(request.GetJobId())
	if err != nil {
		return nil, convertError(err)
	}

	protoRipJob, err := ripJobDbToProto(ripJob)
	if err != nil {
		return nil, convertError(err)
	}

	return server_pb.GetJobInfoResponse_builder{
		Details: protoRipJob,
	}.Build(), nil
}

// Renames a job
func (server ApiServer) RenameJob(
	ctx context.Context,
	request *server_pb.RenameJobRequest,
) (*server_pb.RenameJobResponse, error) {
	dbTx, err := server.app.Db.BeginTx(ctx, &sql.TxOptions{ReadOnly: true})
	if err != nil {
		return nil, convertError(err)
	}
	defer func() { _ = dbTx.Rollback() }()

	if err := dbTx.RenameRipJob(request.GetJobId(), request.GetNewName()); err != nil {
		return nil, convertError(err)
	}

	if err := dbTx.Commit(); err != nil {
		return nil, convertError(err)
	}

	return server_pb.RenameJobResponse_builder{}.Build(), nil
}

// Deletes a job
func (server ApiServer) DeleteJob(
	ctx context.Context,
	request *server_pb.DeleteJobRequest,
) (*server_pb.DeleteJobResponse, error) {
	dbTx, err := server.app.Db.BeginTx(ctx, &sql.TxOptions{ReadOnly: true})
	if err != nil {
		return nil, convertError(err)
	}
	defer func() { _ = dbTx.Rollback() }()

	if err := dbTx.DeleteRipJob(request.GetJobId()); err != nil {
		return nil, convertError(err)
	}

	if err := dbTx.Commit(); err != nil {
		return nil, convertError(err)
	}

	return server_pb.DeleteJobResponse_builder{}.Build(), nil
}

// Adds a suspicion to a job
func (server ApiServer) SuspectJob(
	ctx context.Context,
	request *server_pb.SuspectJobRequest,
) (*server_pb.SuspectJobResponse, error) {
	dbTx, err := server.app.Db.BeginTx(ctx, &sql.TxOptions{})
	if err != nil {
		return nil, convertError(err)
	}
	defer func() { _ = dbTx.Rollback() }()

	var suspicionBytes sql.Null[[]byte]
	if request.HasSuspicion() {
		suspicionBytes.Valid = true
		bytes, err := proto.Marshal(request.GetSuspicion())
		if err != nil {
			return nil, convertError(err)
		}
		suspicionBytes.V = bytes
	}
	if err := dbTx.SetRipSuspicion(request.GetJobId(), suspicionBytes); err != nil {
		return nil, convertError(err)
	}

	if err := dbTx.Commit(); err != nil {
		return nil, convertError(err)
	}

	if err := server.app.AnalyzeJob(request.GetJobId()); err != nil {
		return nil, convertError(err)
	}

	return server_pb.SuspectJobResponse_builder{}.Build(), nil
}

func (server ApiServer) ReanalyzeJob(
	ctx context.Context,
	request *server_pb.ReanalyzeJobRequest,
) (*server_pb.ReanalyzeJobResponse, error) {
	if err := server.app.AnalyzeJob(request.GetJobId()); err != nil {
		return nil, convertError(err)
	}
	return server_pb.ReanalyzeJobResponse_builder{}.Build(), nil
}

// Gets a list of jobs containing untagged files
func (server ApiServer) GetUntaggedJobs(
	ctx context.Context,
	request *server_pb.GetUntaggedJobsRequest,
) (*server_pb.GetUntaggedJobsResponse, error) {
	dbTx, err := server.app.Db.BeginTx(ctx, &sql.TxOptions{ReadOnly: true})
	if err != nil {
		return nil, convertError(err)
	}
	defer func() { _ = dbTx.Rollback() }()

	ripJobs, err := dbTx.GetRipJobsWithUntaggedVideos(request.GetSkip(), request.GetLimit())
	if err != nil {
		return nil, convertError(err)
	}

	var protoRipJobs []*server_pb.RipJob
	for _, job := range ripJobs {
		protoRipJob, err := ripJobDbToProto(job)
		if err != nil {
			return nil, convertError(err)
		}
		protoRipJobs = append(protoRipJobs, protoRipJob)
	}

	// Get in-progress uploads
	uploadingJobs := server.app.DriveCoordinator.ListUploadingJobs()
	for _, jobId := range uploadingJobs {
		job, err := dbTx.GetRipJob(jobId)
		if err != nil {
			return nil, convertError(err)
		}
		protoRipJob, err := ripJobDbToProto(job)
		if err != nil {
			return nil, convertError(err)
		}
		protoRipJobs = append(protoRipJobs, protoRipJob)
	}

	return server_pb.GetUntaggedJobsResponse_builder{
		RipJobs: protoRipJobs,
	}.Build(), nil
}

// Gets all info needed to catalog a job
func (server ApiServer) GetJobCatalogueInfo(
	ctx context.Context,
	request *server_pb.GetJobCatalogueInfoRequest,
) (*server_pb.GetJobCatalogueInfoResponse, error) {
	dbTx, err := server.app.Db.BeginTx(ctx, &sql.TxOptions{ReadOnly: true})
	if err != nil {
		return nil, convertError(fmt.Errorf("error starting transaction: %w", err))
	}
	defer func() { _ = dbTx.Rollback() }()

	jobInfo, err := dbTx.GetRipJob(request.GetJobId())
	if err != nil {
		return nil, convertError(fmt.Errorf("error getting rip job: %w", err))
	}

	videoFiles, err := dbTx.GetVideosFromRip(request.GetJobId())
	if err != nil {
		return nil, convertError(fmt.Errorf("error getting videos from rip: %w", err))
	}

	matches, err := dbTx.GetMatchesFromRip(request.GetJobId())
	if err != nil {
		return nil, convertError(fmt.Errorf("error getting matches from rip: %w", err))
	}

	subtitleMaps, err := dbTx.GetRipVideoBlobs(request.GetJobId())
	if err != nil {
		return nil, convertError(fmt.Errorf("error getting rip video blobs: %w", err))
	}

	ostSubtitleFiles, err := dbTx.GetOstSubtitlesFromRip(request.GetJobId())
	if err != nil {
		return nil, convertError(fmt.Errorf("error getting ost subtitles from rip: %w", err))
	}

	var discTitle *string
	if jobInfo.DiscTitle.Valid {
		discTitle = &jobInfo.DiscTitle.String
	}

	var suspectedContents *server_pb.SuspectedContents
	if jobInfo.SuspectedContents.Valid {
		suspectedContents = &server_pb.SuspectedContents{}
		if err := proto.Unmarshal(jobInfo.SuspectedContents.V, suspectedContents); err != nil {
			return nil, convertError(err)
		}
	}

	var protoVideoFiles []*server_pb.VideoFile
	for _, videoFile := range videoFiles {
		protoVideoFile, err := videoFileDbToProto(videoFile)
		if err != nil {
			return nil, convertError(err)
		}
		protoVideoFiles = append(protoVideoFiles, protoVideoFile)
	}

	var protoMatches []*server_pb.MatchInfoItem
	for _, matchItem := range matches {
		protoMatches = append(protoMatches, matchInfoItemDbToProto(matchItem))
	}

	var protoSubtitleMaps []*server_pb.RipVideoBlobs
	for _, subtitleMap := range subtitleMaps {
		protoSubtitleMaps = append(protoSubtitleMaps, ripVideoBlobsDbToProto(subtitleMap))
	}

	var protoOstSubtitleFiles []*server_pb.OstDownloadsItem
	for _, ostSubtitleFile := range ostSubtitleFiles {
		protoOstSubtitleFiles = append(
			protoOstSubtitleFiles,
			ostSubtitleFileDbToProto(ostSubtitleFile),
		)
	}

	return server_pb.GetJobCatalogueInfoResponse_builder{
		Id:                jobInfo.Id,
		StartTime:         jobInfo.StartTime,
		DiscTitle:         discTitle,
		SuspectedContents: suspectedContents,
		VideoFiles:        protoVideoFiles,
		Matches:           protoMatches,
		SubtitleMaps:      protoSubtitleMaps,
		OstSubtitleFiles:  protoOstSubtitleFiles,
	}.Build(), nil
}

// Re-processes all video files in a rip job
func (server ApiServer) ReprocessJob(
	ctx context.Context,
	request *server_pb.ReprocessJobRequest,
) (*server_pb.ReprocessJobResponse, error) {
	err := server.app.ReprocessRipJob(request.GetJobId(), true)
	if err != nil {
		return nil, convertError(err)
	}
	return server_pb.ReprocessJobResponse_builder{}.Build(), nil
}

// Prunes a rip job, removing all untagged content
func (server ApiServer) PruneRipJob(
	ctx context.Context,
	request *server_pb.PruneRipJobRequest,
) (*server_pb.PruneRipJobResponse, error) {
	err := server.app.PruneRipJob(request.GetJobId())
	if err != nil {
		return nil, convertError(err)
	}
	return server_pb.PruneRipJobResponse_builder{}.Build(), nil
}

// Streams upload status for rip jobs
func (server ApiServer) StreamRipJobUploadStatus(
	ctx context.Context,
	req *server_pb.StreamRipJobUploadStatusRequest,
	resp *connect.ServerStream[server_pb.StreamRipJobUploadStatusResponse],
) error {
	tracker := server.app.DriveCoordinator.GetUploadTracker(req.GetJobId())
	if tracker == nil {
		return connect.NewError(
			connect.CodeNotFound,
			errors.New("The requested job is not currently being uploaded"),
		)
	}
	watcher := tracker.WatchUploadProgress()
	watcher.SetContext(ctx)
	for {
		progress, unlock, err := watcher.Changed()
		if err != nil {
			unlock()
			return convertError(err)
		}
		update := make(map[string]*server_pb.RipJobFileUploadStatus, len(progress))
		for fileName, fileProgress := range progress {
			update[fileName] = server_pb.RipJobFileUploadStatus_builder{
				Received:  fileProgress.Received,
				TotalSize: fileProgress.TotalSize,
			}.Build()
		}
		unlock()

		resp.Send(server_pb.StreamRipJobUploadStatusResponse_builder{
			Status: update,
		}.Build())
	}
}

func RegisterApiService(server *http.ServeMux, app *application.Application) {
	prefix, apiHandler := server_connect.NewCoordinatorApiServiceHandler(ApiServer{app: app})
	server.Handle("POST "+prefix, apiHandler)
}

func convertError(err error) error {
	if errors.Is(err, application.ErrNotFound) {
		return connect.NewError(connect.CodeNotFound, err)
	}
	if status, ok := gstatus.FromError(err); ok {
		switch status.Code() {
		case gcodes.NotFound:
			return connect.NewError(connect.CodeNotFound, errors.New(status.Message()))
		case gcodes.Unknown:
			return connect.NewError(connect.CodeUnknown, errors.New(status.Message()))
		}
	}
	if errors.Is(err, sync_extras.ErrImmutable) {
		return nil
	}
	return connect.NewError(connect.CodeUnknown, err)
}

func movieDbToProto(movie dbapi.MoviesItem) *server_pb.Movie {
	var tmdbId *int32
	if movie.TmdbId.Valid {
		tmdbId = &movie.TmdbId.Int32
	}
	var posterBlob *int64
	if movie.PosterBlob.Valid {
		posterBlob = &movie.PosterBlob.Int64
	}
	var releaseYear *string
	if movie.ReleaseYear.Valid {
		releaseYear = &movie.ReleaseYear.String
	}
	var description *string
	if movie.Description.Valid {
		description = &movie.Description.String
	}
	var runtime *uint32
	if movie.Runtime.Valid {
		runtime = &movie.Runtime.V
	}

	return server_pb.Movie_builder{
		Id:          movie.Id,
		TmdbId:      tmdbId,
		PosterBlob:  posterBlob,
		Title:       movie.Title,
		ReleaseYear: releaseYear,
		Description: description,
		Runtime:     runtime,
	}.Build()
}

func tvShowDbToProto(tvShow dbapi.TvShowsItem) *server_pb.TvShow {
	var tmdbId *int32
	if tvShow.TmdbId.Valid {
		tmdbId = &tvShow.TmdbId.Int32
	}
	var posterBlob *int64
	if tvShow.PosterBlob.Valid {
		posterBlob = &tvShow.PosterBlob.Int64
	}
	var originalReleaseYear *string
	if tvShow.OriginalReleaseYear.Valid {
		originalReleaseYear = &tvShow.OriginalReleaseYear.String
	}
	var description *string
	if tvShow.Description.Valid {
		description = &tvShow.Description.String
	}

	return server_pb.TvShow_builder{
		Id:                  tvShow.Id,
		TmdbId:              tmdbId,
		PosterBlob:          posterBlob,
		Title:               tvShow.Title,
		OriginalReleaseYear: originalReleaseYear,
		Description:         description,
	}.Build()
}

func tvSeasonDbToProto(tvSeason dbapi.TvSeasonsItem) *server_pb.TvSeason {
	var tmdbId *int32
	if tvSeason.TmdbId.Valid {
		tmdbId = &tvSeason.TmdbId.Int32
	}
	var posterBlob *int64
	if tvSeason.PosterBlob.Valid {
		posterBlob = &tvSeason.PosterBlob.Int64
	}
	var description *string
	if tvSeason.Description.Valid {
		description = &tvSeason.Description.String
	}

	return server_pb.TvSeason_builder{
		Id:           tvSeason.Id,
		TmdbId:       tmdbId,
		TvShowId:     tvSeason.TvShowId,
		SeasonNumber: tvSeason.SeasonNumber,
		PosterBlob:   posterBlob,
		Title:        tvSeason.Title,
		Description:  description,
	}.Build()
}

func tvEpisodeDbToProto(tvEpisode dbapi.TvEpisodesItem) *server_pb.TvEpisode {
	var tmdbId *int32
	if tvEpisode.TmdbId.Valid {
		tmdbId = &tvEpisode.TmdbId.Int32
	}
	var thumbnailBlob *int64
	if tvEpisode.ThumbnailBlob.Valid {
		thumbnailBlob = &tvEpisode.ThumbnailBlob.Int64
	}
	var description *string
	if tvEpisode.Description.Valid {
		description = &tvEpisode.Description.String
	}
	var runtime *uint32
	if tvEpisode.Runtime.Valid {
		runtime = &tvEpisode.Runtime.V
	}

	return server_pb.TvEpisode_builder{
		Id:            tvEpisode.Id,
		TmdbId:        tmdbId,
		TvShowId:      tvEpisode.TvShowId,
		TvSeasonId:    tvEpisode.TvSeasonId,
		EpisodeNumber: tvEpisode.EpisodeNumber,
		ThumbnailBlob: thumbnailBlob,
		Title:         tvEpisode.Title,
		Description:   description,
		Runtime:       runtime,
	}.Build()
}

func ripJobDbToProto(ripJob dbapi.RipJobsItem) (*server_pb.RipJob, error) {
	var discTitle *string
	if ripJob.DiscTitle.Valid {
		discTitle = &ripJob.DiscTitle.String
	}
	var suspectedContents *server_pb.SuspectedContents
	if ripJob.SuspectedContents.Valid {
		suspectedContents = &server_pb.SuspectedContents{}
		if err := proto.Unmarshal(ripJob.SuspectedContents.V, suspectedContents); err != nil {
			return nil, convertError(err)
		}
	}

	return server_pb.RipJob_builder{
		Id:                ripJob.Id,
		StartTime:         ripJob.StartTime,
		DiscTitle:         discTitle,
		SuspectedContents: suspectedContents,
		RipFinished:       ripJob.RipFinished,
		Imported:          ripJob.Imported,
	}.Build(), nil
}

func videoFileDbToProto(videoFile dbapi.VideoFilesItem) (*server_pb.VideoFile, error) {
	var matchId *int64
	if videoFile.MatchId.Valid {
		matchId = &videoFile.MatchId.Int64
	}
	var resolutionWidth *uint32
	if videoFile.ResolutionWidth.Valid {
		resolutionWidth = &videoFile.ResolutionWidth.V
	}
	var resolutionHeight *uint32
	if videoFile.ResolutionHeight.Valid {
		resolutionHeight = &videoFile.ResolutionHeight.V
	}
	var length *uint32
	if videoFile.Length.Valid {
		length = &videoFile.Length.V
	}
	var originalVideoHash []byte
	if videoFile.OriginalVideoHash.Valid {
		originalVideoHash = videoFile.OriginalVideoHash.V
	}
	var ripJob *int64
	if videoFile.RipJob.Valid {
		ripJob = &videoFile.RipJob.Int64
	}
	var extendedMetadata *analysis_pb.MediaDetails
	if videoFile.ExtendedMetadata.Valid {
		metadata := &analysis_pb.MediaDetails{}
		if err := proto.Unmarshal(videoFile.ExtendedMetadata.V, metadata); err != nil {
			return nil, convertError(err)
		}
		extendedMetadata = metadata
	}

	return server_pb.VideoFile_builder{
		Id:                videoFile.Id,
		VideoType:         videoFile.VideoType,
		MatchId:           matchId,
		BlobId:            videoFile.BlobId,
		ResolutionWidth:   resolutionWidth,
		ResolutionHeight:  resolutionHeight,
		Length:            length,
		OriginalVideoHash: originalVideoHash,
		RipJob:            ripJob,
		ExtendedMetadata:  extendedMetadata,
	}.Build(), nil
}

func matchInfoItemDbToProto(matchItem dbapi.MatchInfoItem) *server_pb.MatchInfoItem {
	return server_pb.MatchInfoItem_builder{
		Id:            matchItem.Id,
		VideoFileId:   matchItem.VideoFileId,
		OstDownloadId: matchItem.OstDownloadId,
		Distance:      matchItem.Distance,
		MaxDistance:   matchItem.MaxDistance,
	}.Build()
}

func ripVideoBlobsDbToProto(ripVideoBlob dbapi.RipVideoBlobs) *server_pb.RipVideoBlobs {
	var subtitleBlob *string
	if ripVideoBlob.SubtitleBlob.Valid {
		subtitleBlob = &ripVideoBlob.SubtitleBlob.String
	}
	var subtitleTrack *uint32
	if ripVideoBlob.SubtitleTrack.Valid {
		subtitleTrack = &ripVideoBlob.SubtitleTrack.V
	}
	return server_pb.RipVideoBlobs_builder{
		Id:            ripVideoBlob.Id,
		JobId:         ripVideoBlob.JobId,
		VideoBlob:     ripVideoBlob.VideoBlob,
		SubtitleBlob:  subtitleBlob,
		SubtitleTrack: subtitleTrack,
	}.Build()
}

func ostSubtitleFileDbToProto(subtitleFile dbapi.OstDownloadsItem) *server_pb.OstDownloadsItem {
	return server_pb.OstDownloadsItem_builder{
		Id:        subtitleFile.Id,
		VideoType: subtitleFile.VideoType,
		MatchId:   subtitleFile.MatchId,
		Filename:  subtitleFile.Filename,
		BlobId:    subtitleFile.BlobId,
	}.Build()
}
