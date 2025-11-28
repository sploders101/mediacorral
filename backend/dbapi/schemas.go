package dbapi

import (
	"database/sql"

	proto "github.com/sploders101/mediacorral/backend/gen/mediacorral/server/v1"
	gproto "google.golang.org/protobuf/proto"
)

// Movie Metadata

type MoviesItem struct {
	Id          int64
	TmdbId      sql.NullInt32
	PosterBlob  sql.NullInt64
	Title       string
	ReleaseYear sql.NullString
	Description sql.NullString
	Runtime     sql.Null[uint32]
}

func (movie MoviesItem) IntoProto() *proto.Movie {
	builder := proto.Movie_builder{}
	builder.Id = movie.Id
	if movie.TmdbId.Valid {
		builder.TmdbId = &movie.TmdbId.Int32
	}
	if movie.PosterBlob.Valid {
		builder.PosterBlob = &movie.PosterBlob.Int64
	}
	builder.Title = movie.Title
	if movie.ReleaseYear.Valid {
		builder.ReleaseYear = &movie.ReleaseYear.String
	}
	if movie.Description.Valid {
		builder.Description = &movie.Description.String
	}
	if movie.Runtime.Valid {
		builder.Runtime = &movie.Runtime.V
	}
	return builder.Build()
}

type MoviesSpecialFeaturesItem struct {
	Id            int64
	MovieId       sql.NullInt64
	ThumbnailBlob sql.NullInt64
	Title         string
	Description   sql.NullString
	Runtime       sql.NullInt64
}

// // TV Show Metadata

type TvShowsItem struct {
	Id                  int64
	TmdbId              sql.NullInt32
	PosterBlob          sql.NullInt64
	Title               string
	OriginalReleaseYear sql.NullString
	Description         sql.NullString
}

func (tvShow TvShowsItem) IntoProto() *proto.TvShow {
	builder := proto.TvShow_builder{}
	builder.Id = tvShow.Id
	if tvShow.TmdbId.Valid {
		builder.TmdbId = &tvShow.TmdbId.Int32
	}
	if tvShow.PosterBlob.Valid {
		builder.PosterBlob = &tvShow.PosterBlob.Int64
	}
	builder.Title = tvShow.Title
	if tvShow.OriginalReleaseYear.Valid {
		builder.OriginalReleaseYear = &tvShow.OriginalReleaseYear.String
	}
	if tvShow.Description.Valid {
		builder.Description = &tvShow.Description.String
	}
	return builder.Build()
}

type TvSeasonsItem struct {
	Id           int64
	TmdbId       sql.NullInt32
	TvShowId     int64
	SeasonNumber uint32
	PosterBlob   sql.NullInt64
	Title        string
	Description  sql.NullString
}

func (tvSeason TvSeasonsItem) IntoProto() *proto.TvSeason {
	builder := proto.TvSeason_builder{}
	builder.Id = tvSeason.Id
	if tvSeason.TmdbId.Valid {
		builder.TmdbId = &tvSeason.TmdbId.Int32
	}
	builder.TvShowId = tvSeason.TvShowId
	builder.SeasonNumber = tvSeason.SeasonNumber
	if tvSeason.PosterBlob.Valid {
		builder.PosterBlob = &tvSeason.PosterBlob.Int64
	}
	builder.Title = tvSeason.Title
	if tvSeason.Description.Valid {
		builder.Description = &tvSeason.Description.String
	}
	return builder.Build()
}

type TvEpisodesItem struct {
	Id            int64
	TmdbId        sql.NullInt32
	TvShowId      int64
	TvSeasonId    int64
	EpisodeNumber uint32
	ThumbnailBlob sql.NullInt64
	Title         string
	Description   sql.NullString
	Runtime       sql.Null[uint32]
}

func (tvEpisode TvEpisodesItem) IntoProto() *proto.TvEpisode {
	builder := proto.TvEpisode_builder{}
	builder.Id = tvEpisode.Id
	if tvEpisode.TmdbId.Valid {
		builder.TmdbId = &tvEpisode.TmdbId.Int32
	}
	builder.TvShowId = tvEpisode.TvShowId
	builder.TvSeasonId = tvEpisode.TvSeasonId
	builder.EpisodeNumber = tvEpisode.EpisodeNumber
	if tvEpisode.ThumbnailBlob.Valid {
		builder.ThumbnailBlob = &tvEpisode.ThumbnailBlob.Int64
	}
	builder.Title = tvEpisode.Title
	if tvEpisode.Description.Valid {
		builder.Description = &tvEpisode.Description.String
	}
	if tvEpisode.Runtime.Valid {
		builder.Runtime = &tvEpisode.Runtime.V
	}
	return builder.Build()
}

type RipJobsItem struct {
	Id                int64
	StartTime         int64
	DiscTitle         sql.NullString
	SuspectedContents sql.Null[[]byte]
	RipFinished       bool
	Imported          bool
}

func (ripJob RipJobsItem) IntoProto() (*proto.RipJob, error) {
	builder := proto.RipJob_builder{}
	builder.Id = ripJob.Id
	builder.StartTime = ripJob.StartTime
	if ripJob.DiscTitle.Valid {
		builder.DiscTitle = &ripJob.DiscTitle.String
	}
	if ripJob.SuspectedContents.Valid {
		suspectedContents := &proto.SuspectedContents{}
		if err := gproto.Unmarshal(ripJob.SuspectedContents.V, suspectedContents); err != nil {
			return nil, err
		}
		builder.SuspectedContents = suspectedContents
	}
	builder.RipFinished = ripJob.RipFinished
	builder.Imported = ripJob.Imported
	return builder.Build(), nil
}

type VideoFilesItem struct {
	Id                int64
	VideoType         proto.VideoType
	MatchId           sql.NullInt64
	BlobId            string
	ResolutionWidth   sql.Null[uint32]
	ResolutionHeight  sql.Null[uint32]
	Length            sql.Null[uint32]
	OriginalVideoHash sql.Null[[]byte]
	RipJob            sql.NullInt64
	ExtendedMetadata  sql.Null[[]byte]
}

func (videoFile VideoFilesItem) IntoProto() (*proto.VideoFile, error) {
	builder := proto.VideoFile_builder{}
	builder.Id = videoFile.Id
	builder.VideoType = videoFile.VideoType
	if videoFile.MatchId.Valid {
		builder.MatchId = &videoFile.MatchId.Int64
	}
	builder.BlobId = videoFile.BlobId
	if videoFile.ResolutionWidth.Valid {
		builder.ResolutionWidth = &videoFile.ResolutionWidth.V
	}
	if videoFile.ResolutionHeight.Valid {
		builder.ResolutionHeight = &videoFile.ResolutionHeight.V
	}
	if videoFile.Length.Valid {
		builder.Length = &videoFile.Length.V
	}
	builder.OriginalVideoHash = videoFile.OriginalVideoHash.V
	if videoFile.RipJob.Valid {
		builder.RipJob = &videoFile.RipJob.Int64
	}
	if videoFile.ExtendedMetadata.Valid {
		extendedMetadata := &proto.VideoExtendedMetadata{}
		if err := gproto.Unmarshal(videoFile.ExtendedMetadata.V, extendedMetadata); err != nil {
			return nil, err
		}
		builder.ExtendedMetadata = extendedMetadata
	}
	return builder.Build(), nil
}

type SubtitleFilesItem struct {
	Id        int64
	BlobId    string
	VideoFile int64
}

type OstDownloadsItem struct {
	Id        int64
	VideoType proto.VideoType
	MatchId   int64
	Filename  string
	BlobId    string
}

func (ostDownload OstDownloadsItem) IntoProto() *proto.OstDownloadsItem {
	builder := proto.OstDownloadsItem_builder{}
	builder.Id = ostDownload.Id
	builder.VideoType = ostDownload.VideoType
	builder.MatchId = ostDownload.MatchId
	builder.Filename = ostDownload.Filename
	builder.BlobId = ostDownload.BlobId
	return builder.Build()
}

type MatchInfoItem struct {
	Id            int64
	VideoFileId   int64
	OstDownloadId int64
	Distance      uint32
	MaxDistance   uint32
}

func (matchInfo MatchInfoItem) IntoProto() *proto.MatchInfoItem {
	builder := proto.MatchInfoItem_builder{}
	builder.Id = matchInfo.Id
	builder.VideoFileId = matchInfo.VideoFileId
	builder.OstDownloadId = matchInfo.OstDownloadId
	builder.Distance = matchInfo.Distance
	builder.MaxDistance = matchInfo.MaxDistance
	return builder.Build()
}

type ImageFilesItem struct {
	Id       int64
	BlobId   string
	MimeType string
	Name     sql.NullString
	RipJob   sql.NullInt64
}

type TvExportEntry struct {
	TvTitle       string
	TvReleaseYear string
	TvTmdb        int32
	SeasonNumber  uint16
	EpisodeTitle  string
	EpisodeNumber uint16
	EpisodeTmdb   int32
	EpisodeBlob   string
}

type MovieExportEntry struct {
	MovieTitle       string
	MovieReleaseYear string
	MovieTmdb        int32
	MovieBlob        string
}
