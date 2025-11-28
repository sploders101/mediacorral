package dbapi

import (
	"context"
	"database/sql"
	"fmt"

	"github.com/sploders101/mediacorral/backend/dbapi/migrations"
	gproto "google.golang.org/protobuf/proto"

	proto "github.com/sploders101/mediacorral/backend/gen/mediacorral/server/v1"
)

type Db struct {
	db *sql.DB
}

type DbTx struct {
	tx              *sql.Tx
	commitCallbacks []func()
}

func NewDb(dbPath string) (Db, error) {
	db, err := sql.Open("sqlite3", dbPath + "?_timeout=60000")
	if err != nil {
		return Db{}, fmt.Errorf("an error occurred while opening the database: %w", err)
	}
	if err := migrations.InitDb(db); err != nil {
		return Db{}, fmt.Errorf("failed to initialize database: %w", err)
	}

	return Db{db: db}, nil
}

func (db Db) Begin() (*DbTx, error) {
	dbTx, err := db.db.Begin()
	if err != nil {
		return nil, err
	}
	return &DbTx{tx: dbTx}, nil
}

func (db Db) BeginTx(ctx context.Context, opts *sql.TxOptions) (DbTx, error) {
	dbTx, err := db.db.BeginTx(ctx, opts)
	if err != nil {
		return DbTx{}, err
	}
	return DbTx{tx: dbTx}, nil
}

// Registers a callback to be run after commit. This is useful for things like the blob
// controller, which need to delete files, but don't want to leave broken links in the db.
func (db *DbTx) OnCommit(callback func()) {
	db.commitCallbacks = append(db.commitCallbacks, callback)
}

func (db *DbTx) Commit() error {
	err := db.tx.Commit()
	if err != nil {
		return err
	}
	for _, callback := range db.commitCallbacks {
		callback()
	}
	return nil
}

func (db *DbTx) Rollback() error {
	return db.tx.Rollback()
}

func (db *DbTx) InsertMovie(
	tmdbId sql.NullInt32,
	posterBlob sql.NullInt64,
	title string,
	releaseYear sql.NullString,
	description sql.NullString,
	runtime sql.Null[uint32],
) (MoviesItem, error) {
	result := db.tx.QueryRow(
		`
            INSERT INTO movies (
                tmdb_id,
                poster_blob,
                title,
                release_year,
                description,
                runtime
            ) VALUES (?, ?, ?, ?, ?, ?)
            RETURNING id
		`,
		tmdbId,
		posterBlob,
		title,
		releaseYear,
		description,
		runtime,
	)

	var id int64
	if err := result.Scan(&id); err != nil {
		return MoviesItem{}, err
	}

	return MoviesItem{
		Id:          id,
		TmdbId:      tmdbId,
		PosterBlob:  posterBlob,
		Title:       title,
		ReleaseYear: releaseYear,
		Description: description,
		Runtime:     runtime,
	}, nil
}

func (db *DbTx) UpdateMovie(movie MoviesItem) error {
	_, err := db.tx.Exec(
		`
			UPDATE movies
			SET
                tmdb_id = ?,
                poster_blob = ?,
                title = ?,
                release_year = ?,
                description = ?,
                runtime = ?
            WHERE
            	id = ?
		`,
		movie.TmdbId,
		movie.PosterBlob,
		movie.Title,
		movie.ReleaseYear,
		movie.Description,
		movie.Runtime,
		movie.Id,
	)
	if err != nil {
		return err
	}
	return nil
}

func (db *DbTx) UpsertTmdbMovie(
	tmdbId sql.NullInt32,
	posterBlob sql.NullInt64,
	title string,
	releaseYear sql.NullString,
	description sql.NullString,
	runtime sql.Null[uint32],
) (MoviesItem, error) {
	result := db.tx.QueryRow(
		`
			INSERT INTO movies (
                tmdb_id,
                poster_blob,
                title,
                release_year,
                description,
                runtime
            ) VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT (tmdb_id) DO UPDATE SET
                poster_blob = ?,
                title = ?,
                release_year = ?,
                description = ?,
                runtime = ?
            RETURNING id
		`,
		tmdbId,
		posterBlob,
		title,
		releaseYear,
		description,
		runtime,
		posterBlob,
		title,
		releaseYear,
		description,
		runtime,
	)
	var id int64
	if err := result.Scan(&id); err != nil {
		return MoviesItem{}, err
	}
	return MoviesItem{
		Id:          id,
		TmdbId:      tmdbId,
		PosterBlob:  posterBlob,
		Title:       title,
		ReleaseYear: releaseYear,
		Description: description,
		Runtime:     runtime,
	}, nil
}

func (db *DbTx) GetMovies() ([]MoviesItem, error) {
	result, err := db.tx.Query(`
		SELECT
			id,
            tmdb_id,
            poster_blob,
            title,
            release_year,
            description,
            runtime
        FROM movies
	`)
	if err != nil {
		return nil, err
	}
	var movieResults []MoviesItem
	for result.Next() {
		var movie MoviesItem
		if err := result.Scan(
			&movie.Id,
			&movie.TmdbId,
			&movie.PosterBlob,
			&movie.Title,
			&movie.ReleaseYear,
			&movie.Description,
			&movie.Runtime,
		); err != nil {
			return nil, err
		}
		movieResults = append(movieResults, movie)
	}
	return movieResults, nil
}

func (db *DbTx) GetMovieById(id int64) (MoviesItem, error) {
	result := db.tx.QueryRow(
		`
			SELECT
				id,
				tmdb_id,
				poster_blob,
				title,
				release_year,
				description,
				runtime
			FROM movies
			WHERE
				id = ?
		`,
		id,
	)
	var movie MoviesItem
	if err := result.Scan(
		&movie.Id,
		&movie.TmdbId,
		&movie.PosterBlob,
		&movie.Title,
		&movie.ReleaseYear,
		&movie.Description,
		&movie.Runtime,
	); err != nil {
		return MoviesItem{}, err
	}
	return movie, nil
}

func (db *DbTx) GetMovieByTmdbId(tmdbId int32) (MoviesItem, error) {
	result := db.tx.QueryRow(
		`
			SELECT
				id,
				tmdb_id,
				poster_blob,
				title,
				release_year,
				description,
				runtime
			FROM movies
			WHERE
				tmdb_id = ?
		`,
		tmdbId,
	)
	var movie MoviesItem
	if err := result.Scan(
		&movie.Id,
		&movie.TmdbId,
		&movie.PosterBlob,
		&movie.Title,
		&movie.ReleaseYear,
		&movie.Description,
		&movie.Runtime,
	); err != nil {
		return MoviesItem{}, err
	}
	return movie, nil
}

func (db *DbTx) InsertMoviesSpecialFeature(
	movieId sql.NullInt64,
	thumbnailBlob sql.NullInt64,
	title string,
	description sql.NullString,
	runtime sql.NullInt64,
) (MoviesSpecialFeaturesItem, error) {
	result := db.tx.QueryRow(
		`
			INSERT INTO movies_special_features (
				movie_id,
				thumbnail_blob,
				title,
				description
			) VALUES (?, ?, ?, ?)
			RETURNING id
		`,
		movieId,
		thumbnailBlob,
		title,
		description,
	)
	specialFeature := MoviesSpecialFeaturesItem{
		MovieId:       movieId,
		ThumbnailBlob: thumbnailBlob,
		Title:         title,
		Description:   description,
		Runtime:       runtime,
	}
	if err := result.Scan(&specialFeature.Id); err != nil {
		return MoviesSpecialFeaturesItem{}, err
	}
	return specialFeature, nil
}

func (db *DbTx) UpdateMoviesSpecialFeature(
	specialFeature MoviesSpecialFeaturesItem,
) error {
	if _, err := db.tx.Exec(
		`
			UPDATE movies_special_features
			SET
				movie_id = ?,
				thumbnail_blob = ?,
				title = ?,
				description = ?
			WHERE
				id = ?
		`,
		specialFeature.MovieId,
		specialFeature.ThumbnailBlob,
		specialFeature.Title,
		specialFeature.Description,
		specialFeature.Id,
	); err != nil {
		return err
	}
	return nil
}

func (db *DbTx) InsertTvShow(
	tmdbId sql.NullInt32,
	posterBlob sql.NullInt64,
	title string,
	originalReleaseYear sql.NullString,
	description sql.NullString,
) (TvShowsItem, error) {
	result := db.tx.QueryRow(
		`
			INSERT INTO tv_shows (
				tmdb_id,
				poster_blob,
				title,
				original_release_year,
				description
			) VALUES (?, ?, ?, ?, ?)
			RETURNING id
		`,
	)
	tvShow := TvShowsItem{
		TmdbId:              tmdbId,
		PosterBlob:          posterBlob,
		Title:               title,
		OriginalReleaseYear: originalReleaseYear,
		Description:         description,
	}
	if err := result.Scan(&tvShow.Id); err != nil {
		return TvShowsItem{}, err
	}
	return tvShow, nil
}

func (db *DbTx) UpdateTvShow(tvShow TvShowsItem) (TvShowsItem, error) {
	if _, err := db.tx.Exec(
		`
			UPDATE tv_shows
			SET
				tmdb_id = ?,
				poster_blob = ?,
				title = ?,
				original_release_year = ?,
				description = ?
			WHERE
				id = ?
			RETURNING id
		`,
		tvShow.TmdbId,
		tvShow.PosterBlob,
		tvShow.Title,
		tvShow.OriginalReleaseYear,
		tvShow.Description,
		tvShow.Id,
	); err != nil {
		return TvShowsItem{}, err
	}
	return tvShow, nil
}

func (db *DbTx) UpsertTmdbTvShow(
	tmdbId int32,
	posterBlob sql.NullInt64,
	title string,
	originalReleaseYear sql.NullString,
	description sql.NullString,
) (TvShowsItem, error) {
	result := db.tx.QueryRow(
		`
            INSERT INTO tv_shows (
                tmdb_id,
                poster_blob,
                title,
                original_release_year,
                description
            ) VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (tmdb_id) DO UPDATE SET
                poster_blob = ?,
                title = ?,
                original_release_year = ?,
                description = ?
            RETURNING id
		`,
		tmdbId,
		posterBlob,
		title,
		originalReleaseYear,
		description,
		posterBlob,
		title,
		originalReleaseYear,
		description,
	)
	var id int64
	if err := result.Scan(&id); err != nil {
		return TvShowsItem{}, err
	}
	return TvShowsItem{
		Id:                  id,
		TmdbId:              sql.NullInt32{Valid: true, Int32: tmdbId},
		PosterBlob:          posterBlob,
		Title:               title,
		OriginalReleaseYear: originalReleaseYear,
		Description:         description,
	}, nil
}

func (db *DbTx) GetTvShows() ([]TvShowsItem, error) {
	results, err := db.tx.Query(
		`
			SELECT
				id,
				tmdb_id,
				poster_blob,
				title,
				original_release_year,
				description
			FROM tv_shows
			LIMIT 1000
		`,
	)
	if err != nil {
		return nil, err
	}
	var tvShows []TvShowsItem
	for results.Next() {
		var tvShow TvShowsItem
		if err := results.Scan(
			&tvShow.Id,
			&tvShow.TmdbId,
			&tvShow.PosterBlob,
			&tvShow.Title,
			&tvShow.OriginalReleaseYear,
			&tvShow.Description,
		); err != nil {
			return nil, err
		}
		tvShows = append(tvShows, tvShow)
	}
	return tvShows, nil
}

func (db *DbTx) GetTvSeasons(seriesId int64) ([]TvSeasonsItem, error) {
	results, err := db.tx.Query(
		`
			SELECT
				id,
				tmdb_id,
				tv_show_id,
				season_number,
				poster_blob,
				title,
				description
			FROM tv_seasons
			WHERE tv_show_id = ?
			LIMIT 1000
		`,
		seriesId,
	)
	if err != nil {
		return nil, err
	}
	var tvSeasons []TvSeasonsItem
	for results.Next() {
		var tvSeason TvSeasonsItem
		if err := results.Scan(
			&tvSeason.Id,
			&tvSeason.TmdbId,
			&tvSeason.TvShowId,
			&tvSeason.SeasonNumber,
			&tvSeason.PosterBlob,
			&tvSeason.Title,
			&tvSeason.Description,
		); err != nil {
			return nil, err
		}
		tvSeasons = append(tvSeasons, tvSeason)
	}
	return tvSeasons, nil
}

func (db *DbTx) GetTvEpisodes(seasonId int64) ([]TvEpisodesItem, error) {
	results, err := db.tx.Query(
		`
			SELECT
				id,
				tmdb_id,
				tv_show_id,
				tv_season_id,
				episode_number,
				thumbnail_blob,
				title,
				description,
				runtime
			FROM tv_episodes
			WHERE tv_season_id = ?
			LIMIT 1000
		`,
		seasonId,
	)
	if err != nil {
		return nil, err
	}
	var tvEpisodes []TvEpisodesItem
	for results.Next() {
		var tvEpisode TvEpisodesItem
		if err := results.Scan(
			&tvEpisode.Id,
			&tvEpisode.TmdbId,
			&tvEpisode.TvShowId,
			&tvEpisode.TvSeasonId,
			&tvEpisode.EpisodeNumber,
			&tvEpisode.ThumbnailBlob,
			&tvEpisode.Title,
			&tvEpisode.Description,
			&tvEpisode.Runtime,
		); err != nil {
			return nil, err
		}
		tvEpisodes = append(tvEpisodes, tvEpisode)
	}
	return tvEpisodes, err
}

func (db *DbTx) GetTvShowById(seriesId int64) (TvShowsItem, error) {
	result := db.tx.QueryRow(
		`
			SELECT
				id,
				tmdb_id,
				poster_blob,
				title,
				original_release_year,
				description
			FROM tv_shows
			WHERE id = ?
		`,
		seriesId,
	)
	var tvShow TvShowsItem
	if err := result.Scan(
		&tvShow.Id,
		&tvShow.TmdbId,
		&tvShow.PosterBlob,
		&tvShow.Title,
		&tvShow.OriginalReleaseYear,
		&tvShow.Description,
	); err != nil {
		return TvShowsItem{}, err
	}
	return tvShow, nil
}

func (db *DbTx) GetTvSeasonById(seasonId int64) (TvSeasonsItem, error) {
	result := db.tx.QueryRow(
		`
			SELECT
				id,
				tmdb_id,
				tv_show_id,
				season_number,
				poster_blob,
				title,
				description
			FROM tv_seasons
			WHERE id = ?
		`,
		seasonId,
	)
	var tvSeason TvSeasonsItem
	if err := result.Scan(
		&tvSeason.Id,
		&tvSeason.TmdbId,
		&tvSeason.TvShowId,
		&tvSeason.SeasonNumber,
		&tvSeason.PosterBlob,
		&tvSeason.Title,
		&tvSeason.Description,
	); err != nil {
		return TvSeasonsItem{}, err
	}
	return tvSeason, nil
}

func (db *DbTx) GetTvEpisodeById(episodeId int64) (TvEpisodesItem, error) {
	result := db.tx.QueryRow(
		`
			SELECT
				id,
                tmdb_id,
                tv_show_id,
                tv_season_id,
                episode_number,
                thumbnail_blob,
                title,
                description,
                runtime
            FROM tv_episodes
            WHERE id = ?
		`,
		episodeId,
	)
	var tvEpisode TvEpisodesItem
	if err := result.Scan(
		&tvEpisode.Id,
		&tvEpisode.TmdbId,
		&tvEpisode.TvShowId,
		&tvEpisode.TvSeasonId,
		&tvEpisode.EpisodeNumber,
		&tvEpisode.ThumbnailBlob,
		&tvEpisode.Title,
		&tvEpisode.Description,
		&tvEpisode.Runtime,
	); err != nil {
		return TvEpisodesItem{}, err
	}
	return tvEpisode, nil
}

func (db *DbTx) GetTvEpisodeByTmdbId(tmdbId int32) (TvEpisodesItem, error) {
	result := db.tx.QueryRow(
		`
			SELECT
				id,
				tmdb_id,
				tv_show_id,
				tv_season_id,
                episode_number,
                thumbnail_blob,
                title,
                description,
                runtime
            FROM tv_episodes
            WHERE tmdb_id = ?
		`,
		tmdbId,
	)
	var tvEpisode TvEpisodesItem
	if err := result.Scan(
		&tvEpisode.Id,
		&tvEpisode.TmdbId,
		&tvEpisode.TvShowId,
		&tvEpisode.TvSeasonId,
		&tvEpisode.EpisodeNumber,
		&tvEpisode.ThumbnailBlob,
		&tvEpisode.Title,
		&tvEpisode.Description,
		&tvEpisode.Runtime,
	); err != nil {
		return TvEpisodesItem{}, err
	}
	return tvEpisode, nil
}

func (db *DbTx) InsertTvSeason(
	tmdbId sql.NullInt32,
	tvShowId int64,
	seasonNumber uint32,
	posterBlob sql.NullInt64,
	title string,
	description sql.NullString,
) (TvSeasonsItem, error) {
	result := db.tx.QueryRow(
		`
			INSERT INTO tv_seasons (
				tmdb_id,
                tv_show_id,
                season_number,
                poster_blob,
                title,
                description
			) VALUES (?, ?, ?, ?, ?, ?)
			RETURNING id
		`,
		tmdbId,
		tvShowId,
		seasonNumber,
		posterBlob,
		title,
		description,
	)
	tvSeason := TvSeasonsItem{
		TmdbId:       tmdbId,
		TvShowId:     tvShowId,
		SeasonNumber: seasonNumber,
		PosterBlob:   posterBlob,
		Title:        title,
		Description:  description,
	}
	if err := result.Scan(&tvSeason.Id); err != nil {
		return TvSeasonsItem{}, err
	}
	return tvSeason, nil
}

func (db *DbTx) UpdateTvSeason(tvSeason TvSeasonsItem) error {
	_, err := db.tx.Exec(
		`
			UPDATE tv_seasons
			SET
				tmdb_id = ?,
                tv_show_id = ?,
                season_number = ?,
                poster_blob = ?,
                title = ?,
                description = ?
            WHERE
            	id = ?
		`,
		tvSeason.TmdbId,
		tvSeason.TvShowId,
		tvSeason.SeasonNumber,
		tvSeason.PosterBlob,
		tvSeason.Title,
		tvSeason.Description,
		tvSeason.Id,
	)
	if err != nil {
		return err
	}
	return nil
}

func (db *DbTx) UpsertTmdbTvSeason(
	tmdbId int32,
	tvShowId int64,
	seasonNumber uint32,
	posterBlob sql.NullInt64,
	title string,
	description sql.NullString,
) (TvSeasonsItem, error) {
	result := db.tx.QueryRow(
		`
			INSERT INTO tv_seasons (
				tmdb_id,
				tv_show_id,
				season_number,
				poster_blob,
				title,
				description
			) VALUES (?, ?, ?, ?, ?, ?)
			ON CONFLICT (tmdb_id) DO UPDATE SET
				poster_blob = ?,
				title = ?,
				description = ?
			RETURNING id, tv_show_id, season_number
		`,
		tmdbId,
		tvShowId,
		seasonNumber,
		posterBlob,
		title,
		description,
		posterBlob,
		title,
		description,
	)
	tvSeason := TvSeasonsItem{
		TmdbId:      sql.NullInt32{Valid: true, Int32: tmdbId},
		PosterBlob:  posterBlob,
		Title:       title,
		Description: description,
	}
	if err := result.Scan(
		&tvSeason.Id,
		&tvSeason.TvShowId,
		&tvSeason.SeasonNumber,
	); err != nil {
		return TvSeasonsItem{}, err
	}
	return tvSeason, nil
}

func (db *DbTx) InsertTvEpisode(
	tmdbId int32,
	tvShowId int64,
	tvSeasonId int64,
	episodeNumber uint32,
	thumbnailBlob sql.NullInt64,
	title string,
	description sql.NullString,
	runtime sql.Null[uint32],
) (TvEpisodesItem, error) {
	result := db.tx.QueryRow(
		`
			INSERT INTO tv_episodes (
                tmdb_id,
                tv_show_id,
                tv_season_id,
                episode_number,
                thumbnail_blob,
                title,
                description,
                runtime
			) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
			RETURNING id
		`,
		tmdbId,
		tvShowId,
		tvSeasonId,
		episodeNumber,
		thumbnailBlob,
		title,
		description,
		runtime,
	)
	tvEpisode := TvEpisodesItem{
		TmdbId:        sql.NullInt32{Valid: true, Int32: tmdbId},
		TvShowId:      tvShowId,
		TvSeasonId:    tvSeasonId,
		EpisodeNumber: episodeNumber,
		ThumbnailBlob: thumbnailBlob,
		Title:         title,
		Description:   description,
		Runtime:       runtime,
	}
	if err := result.Scan(&tvEpisode.Id); err != nil {
		return TvEpisodesItem{}, err
	}
	return tvEpisode, nil
}

func (db *DbTx) UpdateTvEpisode(tvEpisode TvEpisodesItem) error {
	_, err := db.tx.Exec(
		`
			UPDATE tv_episodes
			SET
                tmdb_id = ?,
                tv_show_id = ?,
                tv_season_id = ?,
                episode_number = ?,
                thumbnail_blob = ?,
                title = ?,
                description = ?,
                runtime = ?
            WHERE
            	id = ?
		`,
		tvEpisode.TmdbId,
		tvEpisode.TvShowId,
		tvEpisode.TvSeasonId,
		tvEpisode.EpisodeNumber,
		tvEpisode.ThumbnailBlob,
		tvEpisode.Title,
		tvEpisode.Description,
		tvEpisode.Runtime,
		tvEpisode.Id,
	)
	if err != nil {
		return err
	}
	return nil
}

func (db *DbTx) UpsertTmdbTvEpisode(
	tmdbId int32,
	tvShowId int64,
	tvSeasonId int64,
	episodeNumber uint32,
	thumbnailBlob sql.NullInt64,
	title string,
	description sql.NullString,
	runtime sql.Null[uint32],
) (TvEpisodesItem, error) {
	result := db.tx.QueryRow(
		`
			INSERT INTO tv_episodes (
				tmdb_id,
				tv_show_id,
				tv_season_id,
				episode_number,
				thumbnail_blob,
                title,
                description,
                runtime
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (tmdb_id) DO UPDATE SET
                thumbnail_blob = ?,
                title = ?,
                description = ?,
                runtime = ?
            RETURNING id, tv_show_id, tv_season_id, episode_number
		`,
		tmdbId,
		tvShowId,
		tvSeasonId,
		episodeNumber,
		thumbnailBlob,
		title,
		description,
		runtime,
		thumbnailBlob,
		title,
		description,
		runtime,
	)
	tvEpisode := TvEpisodesItem{
		TmdbId:        sql.NullInt32{Valid: true, Int32: tmdbId},
		ThumbnailBlob: thumbnailBlob,
		Title:         title,
		Description:   description,
		Runtime:       runtime,
	}
	if err := result.Scan(
		&tvEpisode.Id,
		&tvEpisode.TvShowId,
		&tvEpisode.TvSeasonId,
		&tvEpisode.EpisodeNumber,
	); err != nil {
		return TvEpisodesItem{}, err
	}
	return tvEpisode, nil
}

func (db *DbTx) CreateRipJob(
	startTime int64,
	discTitle sql.NullString,
	suspectedContents sql.Null[[]byte],
) (RipJobsItem, error) {
	result := db.tx.QueryRow(
		`
			INSERT INTO rip_jobs (
                start_time,
                disc_title,
                suspected_contents,
                rip_finished,
                imported
            ) VALUES (?, ?, ?, false, false)
            RETURNING id
		`,
		startTime,
		discTitle,
		suspectedContents,
	)
	ripJob := RipJobsItem{
		StartTime:         startTime,
		DiscTitle:         discTitle,
		SuspectedContents: suspectedContents,
		RipFinished:       false,
		Imported:          false,
	}
	if err := result.Scan(&ripJob.Id); err != nil {
		return RipJobsItem{}, err
	}
	return ripJob, nil
}

func (db *DbTx) SetRipSuspicion(ripJob int64, suspicion sql.Null[[]byte]) error {
	_, err := db.tx.Exec(
		`
			UPDATE rip_jobs
			SET
				suspected_contents = ?
			WHERE
				id = ?
		`,
		suspicion,
		ripJob,
	)
	if err != nil {
		return err
	}
	return nil
}

func (db *DbTx) GetRipJob(ripJob int64) (RipJobsItem, error) {
	result := db.tx.QueryRow(
		`
			SELECT
				id,
				start_time,
				disc_title,
				suspected_contents,
				rip_finished,
				imported
			FROM rip_jobs
			WHERE
				id = ?
		`,
		ripJob,
	)
	var ripJobItem RipJobsItem
	if err := result.Scan(
		&ripJobItem.Id,
		&ripJobItem.StartTime,
		&ripJobItem.DiscTitle,
		&ripJobItem.SuspectedContents,
		&ripJobItem.RipFinished,
		&ripJobItem.Imported,
	); err != nil {
		return RipJobsItem{}, err
	}
	return ripJobItem, nil
}

func (db *DbTx) RenameRipJob(ripJob int64, newName string) error {
	_, err := db.tx.Exec(
		`
			UPDATE rip_jobs
			SET
				disc_title = ?
			WHERE
				id = ?
		`,
		newName,
		ripJob,
	)
	if err != nil {
		return err
	}
	return nil
}

func (db *DbTx) MarkRipJobFinished(ripJob int64, finished bool) error {
	_, err := db.tx.Exec(
		`
			UPDATE rip_jobs
			SET
				rip_finished = ?
			WHERE
				id = ?
		`,
		finished,
		ripJob,
	)
	if err != nil {
		return err
	}
	return nil
}

func (db *DbTx) MarkRipJobImported(ripJob int64, imported bool) error {
	_, err := db.tx.Exec(
		`
			UPDATE rip_jobs
			SET
				imported = ?
			WHERE
				id = ?
		`,
		imported,
		ripJob,
	)
	if err != nil {
		return err
	}
	return nil
}

func (db *DbTx) InsertVideoFile(
	videoType proto.VideoType,
	matchId sql.NullInt64,
	blobId string,
	resolutionWidth sql.Null[uint32],
	resolutionHeight sql.Null[uint32],
	length sql.Null[uint32],
	originalVideoHash sql.Null[[]byte],
	ripJob sql.NullInt64,
	extendedMetadata sql.Null[[]byte],
) (VideoFilesItem, error) {
	result := db.tx.QueryRow(
		`
			INSERT INTO video_files (
                video_type,
                match_id,
                blob_id,
                resolution_width,
                resolution_height,
                length,
                original_video_hash,
                rip_job,
                extended_metadata
			) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
			RETURNING id
		`,
		videoType,
		matchId,
		blobId,
		resolutionWidth,
		resolutionHeight,
		length,
		originalVideoHash,
		ripJob,
		extendedMetadata,
	)
	videoFile := VideoFilesItem{
		VideoType:         videoType,
		MatchId:           matchId,
		BlobId:            blobId,
		ResolutionWidth:   resolutionWidth,
		ResolutionHeight:  resolutionHeight,
		Length:            length,
		OriginalVideoHash: originalVideoHash,
		RipJob:            ripJob,
		ExtendedMetadata:  extendedMetadata,
	}
	if err := result.Scan(&videoFile.Id); err != nil {
		return VideoFilesItem{}, err
	}
	return videoFile, nil
}

func (db *DbTx) GetVideoFile(videoId int64) (VideoFilesItem, error) {
	result := db.tx.QueryRow(
		`
			SELECT
				id,
                video_type,
                match_id,
                blob_id,
                resolution_width,
                resolution_height,
                length,
                original_video_hash,
                rip_job,
                extended_metadata
            FROM video_files
            WHERE
                id = ?
		`,
		videoId,
	)
	var videoFile VideoFilesItem
	if err := result.Scan(
		&videoFile.Id,
		&videoFile.VideoType,
		&videoFile.MatchId,
		&videoFile.BlobId,
		&videoFile.ResolutionWidth,
		&videoFile.ResolutionHeight,
		&videoFile.Length,
		&videoFile.OriginalVideoHash,
		&videoFile.RipJob,
		&videoFile.ExtendedMetadata,
	); err != nil {
		return VideoFilesItem{}, err
	}
	return videoFile, nil
}

func (db *DbTx) AddVideoMetadata(
	videoId int64,
	resolutionWidth uint32,
	resolutionHeight uint32,
	length uint32,
	originalVideoHash []byte,
	extendedMetadata sql.Null[*proto.VideoExtendedMetadata],
) error {
	var extMetaSerialized sql.Null[[]byte]
	if extendedMetadata.Valid {
		extMetaSerialized.Valid = true
		buf, err := gproto.Marshal(extendedMetadata.V)
		if err != nil {
			return err
		}
		extMetaSerialized.V = buf
	}
	_, err := db.tx.Exec(
		`
			UPDATE video_files
			SET
				resolution_width = ?,
				resolution_height = ?,
				length = ?,
				original_video_hash = ?,
				extended_metadata = ?
			WHERE
				id = ?
		`,
		resolutionWidth,
		resolutionHeight,
		length,
		originalVideoHash,
		extMetaSerialized,
		videoId,
	)
	if err != nil {
		return err
	}
	return nil
}

func (db *DbTx) TagVideoFile(
	videoId int64,
	videoType proto.VideoType,
	matchId sql.NullInt64,
) error {
	_, err := db.tx.Exec(
		`
			UPDATE video_files
			SET
				video_type = ?,
				match_id = ?
			WHERE
				id = ?
		`,
		videoType,
		matchId,
		videoId,
	)
	if err != nil {
		return err
	}
	return nil
}

func (db *DbTx) InsertSubtitleFile(
	blobId string,
	videoId int64,
) (SubtitleFilesItem, error) {
	result := db.tx.QueryRow(
		`
			INSERT INTO subtitle_files (
				blob_id,
				video_file
			) VALUES (?, ?)
			RETURNING id
		`,
		blobId,
		videoId,
	)
	file := SubtitleFilesItem{
		BlobId:    blobId,
		VideoFile: videoId,
	}
	if err := result.Scan(&file.Id); err != nil {
		return SubtitleFilesItem{}, err
	}
	return file, nil
}

func (db *DbTx) GetSubtitlesForVideo(videoId int64) ([]SubtitleFilesItem, error) {
	result, err := db.tx.Query(
		`
			SELECT
				id,
				blob_id,
				video_file
			FROM subtitle_files
			WHERE
				video_file = ?
		`,
		videoId,
	)
	if err != nil {
		return nil, err
	}
	var files []SubtitleFilesItem
	for result.Next() {
		var file SubtitleFilesItem
		if err := result.Scan(&file.Id, &file.BlobId, &file.VideoFile); err != nil {
			return nil, err
		}
		files = append(files, file)
	}
	return files, nil
}

func (db *DbTx) InsertOstDownloadItem(
	videoType proto.VideoType,
	matchId int64,
	filename string,
	blobId string,
) (OstDownloadsItem, error) {
	result := db.tx.QueryRow(
		`
			INSERT INTO ost_downloads (
				video_type,
                match_id,
                filename,
                blob_id
			) VALUES (?, ?, ?, ?)
			RETURNING id
		`,
		videoType,
		matchId,
		filename,
		blobId,
	)
	ostDownload := OstDownloadsItem{
		VideoType: videoType,
		MatchId:   matchId,
		Filename:  filename,
		BlobId:    blobId,
	}
	if err := result.Scan(&ostDownload.Id); err != nil {
		return OstDownloadsItem{}, err
	}
	return ostDownload, nil
}

func (db *DbTx) GetOstDownloadItemsByTmdbId(tmdbId int32) (OstDownloadsItem, error) {
	result := db.tx.QueryRow(
		`
			SELECT
				ost_downloads.id,
				ost_downloads.video_type,
				ost_downloads.match_id,
				ost_downloads.filename,
				ost_downloads.blob_id,
			FROM ost_downloads
			LEFT JOIN tv_episodes ON
				ost_downloads.video_type = 3
				AND tv_episodes.id = ost_downloads.match_id
			LEFT JOIN movies ON
				ost_downloads.video_type = 1
				AND movies.id = ost_downloads.match_id
			WHERE
				tv_episodes.tmdb_id = ?
				OR movies.tmdb_id = ?
		`,
		tmdbId,
		tmdbId,
	)
	var ostDownload OstDownloadsItem
	if err := result.Scan(
		&ostDownload.Id,
		&ostDownload.VideoType,
		&ostDownload.MatchId,
		&ostDownload.Filename,
		&ostDownload.BlobId,
	); err != nil {
		return OstDownloadsItem{}, err
	}
	return ostDownload, nil
}

func (db *DbTx) GetOstDownloadItemsByMatch(
	videoType proto.VideoType,
	matchId int64,
) ([]OstDownloadsItem, error) {
	results, err := db.tx.Query(
		`
			SELECT
				id,
				video_type,
				match_id,
				filename,
				blob_id
			FROM ost_downloads
			WHERE
				video_type = ?
				AND match_id = ?
		`,
		videoType,
		matchId,
	)
	if err != nil {
		return nil, err
	}
	var downloads []OstDownloadsItem
	for results.Next() {
		var download OstDownloadsItem
		if err := results.Scan(
			&download.Id,
			&download.VideoType,
			&download.MatchId,
			&download.Filename,
			&download.BlobId,
		); err != nil {
			return nil, err
		}
		downloads = append(downloads, download)
	}
	return downloads, err
}

func (db *DbTx) ClearMatchInfoForJob(jobId int64) error {
	_, err := db.tx.Exec(
		`
			DELETE FROM match_info
			WHERE video_file_id IN (
				SELECT id
				FROM video_files
				WHERE
					rip_job = ?
			)
		`,
		jobId,
	)
	if err != nil {
		return err
	}
	return nil
}

func (db *DbTx) InsertMatchInfoItem(
	videoFileId int64,
	ostDownloadId int64,
	distance uint32,
	maxDistance uint32,
) (MatchInfoItem, error) {
	result := db.tx.QueryRow(
		`
			INSERT INTO match_info (
				video_file_id,
				ost_download_id,
				distance,
				max_distance
			) VALUES (?, ?, ?, ?)
			RETURNING id
		`,
		videoFileId,
		ostDownloadId,
		distance,
		maxDistance,
	)
	matchItem := MatchInfoItem{
		VideoFileId:   videoFileId,
		OstDownloadId: ostDownloadId,
		Distance:      distance,
		MaxDistance:   maxDistance,
	}
	if err := result.Scan(&matchItem.Id); err != nil {
		return MatchInfoItem{}, err
	}
	return matchItem, nil
}

func (db *DbTx) InsertImageFile(
	blobId string,
	mimeType string,
	name sql.NullString,
	ripJob sql.NullInt64,
) (ImageFilesItem, error) {
	result := db.tx.QueryRow(
		`
			INSERT INTO image_files (
				blob_id,
				mime_type,
				name,
				rip_job
			) VALUES (?, ?, ?, ?)
			RETURNING id
		`,
		blobId,
		mimeType,
		name,
		ripJob,
	)
	imageFile := ImageFilesItem{
		BlobId:   blobId,
		MimeType: mimeType,
		Name:     name,
		RipJob:   ripJob,
	}
	if err := result.Scan(&imageFile.Id); err != nil {
		return ImageFilesItem{}, err
	}
	return imageFile, nil
}

func (db *DbTx) DeleteBlob(blobId string) error {
	_, err := db.tx.Exec(
		`
			DELETE FROM video_files
			WHERE
				blob_id = ?;
			DELETE FROM subtitle_files
			WHERE
				blob_id = ?;
			DELETE FROM ost_downloads
			WHERE
				blob_id = ?;
			DELETE FROM image_files
			WHERE
				blob_id = ?;
		`,
		blobId,
		blobId,
		blobId,
		blobId,
	)
	if err != nil {
		return err
	}
	return nil
}

type RipVideoBlobs struct {
	Id           int64
	JobId        int64
	VideoBlob    string
	SubtitleBlob sql.NullString
}

func (blob RipVideoBlobs) IntoProto() *proto.RipVideoBlobs {
	protoBlob := proto.RipVideoBlobs_builder{}
	protoBlob.Id = blob.Id
	protoBlob.JobId = blob.JobId
	protoBlob.VideoBlob = blob.VideoBlob
	if blob.SubtitleBlob.Valid {
		protoBlob.SubtitleBlob = &blob.SubtitleBlob.String
	}
	return protoBlob.Build()
}

func (db *DbTx) GetRipVideoBlobs(ripJob int64) ([]RipVideoBlobs, error) {
	result, err := db.tx.Query(
		`
			SELECT
				video_files.id as id,
				rip_jobs.id as job_id,
				video_files.blob_id as video_blob,
				subtitle_files.blob_id as subtitle_blob
			FROM rip_jobs
			INNER JOIN video_files ON
				video_files.rip_job = rip_jobs.id
			LEFT JOIN subtitle_files ON
				subtitle_files.video_file = video_files.id
			WHERE
				rip_jobs.id = ?
			ORDER BY
				rip_jobs.start_time asc
		`,
		ripJob,
	)
	if err != nil {
		return nil, err
	}

	var blobs []RipVideoBlobs
	for result.Next() {
		var blob RipVideoBlobs
		if err := result.Scan(
			&blob.Id,
			&blob.JobId,
			&blob.VideoBlob,
			&blob.SubtitleBlob,
		); err != nil {
			return nil, err
		}
		blobs = append(blobs, blob)
	}
	return blobs, nil
}

type RipImageBlob struct {
	JobId     int64
	ImageBlob string
}

func (db *DbTx) GetRipImageBlobs(ripJob int64) ([]RipImageBlob, error) {
	result, err := db.tx.Query(
		`
			SELECT
				rip_jobs.id as job_id,
				image_files.blob_id as image_blob
			FROM rip_jobs
			INNER JOIN image_files ON
				rip_jobs.id = image_files.rip_job
			WHERE
				rip_jobs.id = ?
		`,
		ripJob,
	)
	if err != nil {
		return nil, err
	}
	var blobs []RipImageBlob
	for result.Next() {
		var blob RipImageBlob
		if err := result.Scan(&blob.JobId, &blob.ImageBlob); err != nil {
			return nil, err
		}
		blobs = append(blobs, blob)
	}
	return blobs, nil
}

func (db *DbTx) DeleteRipJob(ripJob int64) error {
	_, err := db.tx.Exec(
		`
			DELETE FROM rip_jobs
			WHERE
				id = ?
		`,
		ripJob,
	)
	if err != nil {
		return err
	}
	return nil
}

func (db *DbTx) GetUntaggedVideosFromJob(ripJob int64) ([]RipVideoBlobs, error) {
	result, err := db.tx.Query(
		`
			SELECT
				video_files.id as id,
				rip_jobs.id as job_id,
				video_files.blob_id as video_blob,
				subtitle_files.blob_id as subtitle_blob
			FROM rip_jobs
			INNER JOIN video_files ON
				rip_jobs.id = video_files.rip_job
			LEFT JOIN subtitle_files ON
				subtitle_files.video_file = video_files.id
			WHERE
				rip_jobs.id = ?
				AND video_files.match_id is null
		`,
		ripJob,
	)
	if err != nil {
		return nil, err
	}
	var blobs []RipVideoBlobs
	for result.Next() {
		var blob RipVideoBlobs
		if err := result.Scan(
			&blob.Id,
			&blob.JobId,
			&blob.VideoBlob,
			&blob.SubtitleBlob,
		); err != nil {
			return nil, err
		}
		blobs = append(blobs, blob)
	}
	return blobs, nil
}

func (db *DbTx) GetRipJobsWithUntaggedVideos(
	skip uint32,
	limit uint32,
) ([]RipJobsItem, error) {
	result, err := db.tx.Query(
		`
			SELECT
				rip_jobs.id,
				rip_jobs.start_time,
				rip_jobs.disc_title,
				rip_jobs.suspected_contents,
				rip_jobs.rip_finished,
				rip_jobs.imported
			FROM rip_jobs
			INNER JOIN video_files ON
				rip_jobs.id = video_files.rip_job
			WHERE
				video_files.match_id is null
			GROUP BY
				rip_jobs.id
			ORDER BY
				rip_jobs.start_time
			LIMIT ?
			OFFSET ?
		`,
		limit,
		skip,
	)
	if err != nil {
		return nil, err
	}
	var jobs []RipJobsItem
	for result.Next() {
		var job RipJobsItem
		if err := result.Scan(
			&job.Id,
			&job.StartTime,
			&job.DiscTitle,
			&job.SuspectedContents,
			&job.RipFinished,
			&job.Imported,
		); err != nil {
			return nil, err
		}
		jobs = append(jobs, job)
	}
	return jobs, nil
}

func (db *DbTx) GetVideosFromRip(ripJob int64) ([]VideoFilesItem, error) {
	result, err := db.tx.Query(
		`
			SELECT
				id,
				video_type,
				match_id,
				blob_id,
				resolution_width,
				resolution_height,
				length,
				original_video_hash,
				rip_job,
				extended_metadata
			FROM video_files
			WHERE
				rip_job = ?
		`,
		ripJob,
	)
	if err != nil {
		return nil, err
	}
	var videos []VideoFilesItem
	for result.Next() {
		var video VideoFilesItem
		if err := result.Scan(
			&video.Id,
			&video.VideoType,
			&video.MatchId,
			&video.BlobId,
			&video.ResolutionWidth,
			&video.ResolutionHeight,
			&video.Length,
			&video.OriginalVideoHash,
			&video.RipJob,
			&video.ExtendedMetadata,
		); err != nil {
			return nil, err
		}
		videos = append(videos, video)
	}
	return videos, nil
}

type DiscSubsWithVideo struct {
	VideoId      int64
	SubtitleId   int64
	SubtitleBlob string
}

func (db *DbTx) GetDiscSubsFromRip(ripJob int64) ([]DiscSubsWithVideo, error) {
	results, err := db.tx.Query(
		`
			SELECT
				video_files.id as video_id,
				subtitle_files.id as subtitle_id,
				subtitle_files.blob_id as subtitle_blob
			FROM video_files
			INNER JOIN subtitle_files ON
				video_files.id = subtitle_files.video_file
			WHERE
				video_files.rip_job = ?
		`,
		ripJob,
	)
	if err != nil {
		return nil, err
	}
	var subsList []DiscSubsWithVideo
	for results.Next() {
		var subs DiscSubsWithVideo
		if err := results.Scan(
			&subs.VideoId,
			&subs.SubtitleId,
			&subs.SubtitleBlob,
		); err != nil {
			return nil, err
		}
		subsList = append(subsList, subs)
	}
	return subsList, nil
}

func (db *DbTx) GetMatchesFromRip(ripJob int64) ([]MatchInfoItem, error) {
	result, err := db.tx.Query(
		`
			SELECT
				match_info.id,
				match_info.video_file_id,
				match_info.ost_download_id,
				match_info.distance,
				match_info.max_distance
			FROM video_files
			INNER JOIN match_info ON
				video_files.id = match_info.video_file_id
			WHERE
				video_files.rip_job = ?
		`,
		ripJob,
	)
	if err != nil {
		return nil, err
	}
	var matchItems []MatchInfoItem
	for result.Next() {
		var matchItem MatchInfoItem
		if err := result.Scan(
			&matchItem.Id,
			&matchItem.VideoFileId,
			&matchItem.OstDownloadId,
			&matchItem.Distance,
			&matchItem.MaxDistance,
		); err != nil {
			return nil, err
		}
		matchItems = append(matchItems, matchItem)
	}
	return matchItems, nil
}

func (db *DbTx) DeleteMatchesFromRip(ripJob int64) error {
	_, err := db.tx.Exec(
		`
			DELETE
			FROM video_files
			WHERE
				id IN (
					SELECT video_files.id
					FROM video_files
					INNER JOIN match_info ON
						video_files.id = match_info.video_file_id
					WHERE
						video_files.rip_job = ?
				)
		`,
		ripJob,
	)
	if err != nil {
		return err
	}
	return nil
}

func (db *DbTx) GetOstSubtitlesFromRip(ripJob int64) ([]OstDownloadsItem, error) {
	result, err := db.tx.Query(
		`
			SELECT
				ost_downloads.id,
				ost_downloads.video_type,
				ost_downloads.match_id,
				ost_downloads.filename,
				ost_downloads.blob_id
			FROM video_files
			INNER JOIN match_info ON
				video_files.id = match_info.video_file_id
			INNER JOIN ost_downloads ON
				ost_downloads.id = match_info.ost_download_id
			WHERE video_files.rip_job = ?
			GROUP BY ost_downloads.id
		`,
		ripJob,
	)
	if err != nil {
		return nil, err
	}
	var downloads []OstDownloadsItem
	for result.Next() {
		var download OstDownloadsItem
		if err := result.Scan(
			&download.Id,
			&download.VideoType,
			&download.MatchId,
			&download.Filename,
			&download.BlobId,
		); err != nil {
			return nil, err
		}
		downloads = append(downloads, download)
	}
	return downloads, nil
}

// This function is used to stream information for TV show exports and act on them.
// For each row, `cb` is called with the results. If `cb` or the sql driver returns
// an error, it is immediately returned by this function.
func (db *DbTx) ProcessTvExportsInfo(cb func(TvExportEntry) error) error {
	result, err := db.tx.Query(
		`
            SELECT
                tv_shows.title as tv_title,
                tv_shows.original_release_year as tv_release_year,
                tv_shows.tmdb_id as tv_tmdb,
                tv_seasons.season_number as season_number,
                tv_episodes.title as episode_title,
                tv_episodes.episode_number as episode_number,
                tv_episodes.tmdb_id as episode_tmdb,
                video_files.blob_id as episode_blob
            FROM video_files
            JOIN tv_episodes ON
                video_files.match_id = tv_episodes.id
            JOIN tv_seasons ON
                tv_episodes.tv_season_id = tv_seasons.id
            JOIN tv_shows ON
                tv_episodes.tv_show_id = tv_shows.id
            WHERE video_type = 3
            ORDER BY tv_episodes.id
		`,
	)
	if err != nil {
		return err
	}
	for result.Next() {
		var entry TvExportEntry
		if err := result.Scan(
			&entry.TvTitle,
			&entry.TvReleaseYear,
			&entry.TvTmdb,
			&entry.SeasonNumber,
			&entry.EpisodeTitle,
			&entry.EpisodeNumber,
			&entry.EpisodeTmdb,
			&entry.EpisodeBlob,
		); err != nil {
			return err
		}
		if err := cb(entry); err != nil {
			return err
		}
	}

	return nil
}

// This function is used to stream information for TV show exports and act on them.
// For each row, `cb` is called with the results. If `cb` or the sql driver returns
// an error, it is immediately returned by this function.
func (db *DbTx) ProcessMovieExportsInfo(cb func(MovieExportEntry) error) error {
	result, err := db.tx.Query(
		`
            SELECT
            	movies.title,
            	movies.release_year,
            	movies.tmdb_id,
            	video_files.blob_id
            FROM video_files
            JOIN movies ON
                video_files.match_id = movies.id
            WHERE
            	video_files.video_type = 1
		`,
	)
	if err != nil {
		return err
	}
	for result.Next() {
		var entry MovieExportEntry
		if err := result.Scan(
			&entry.MovieTitle,
			&entry.MovieReleaseYear,
			&entry.MovieTmdb,
			&entry.MovieBlob,
		); err != nil {
			return err
		}
		if err := cb(entry); err != nil {
			return err
		}
	}

	return nil
}

// This function is used to retrieve information for one TV show export and act on it.
func (db *DbTx) FetchOneTvExportInfo(videoId int64) (TvExportEntry, error) {
	result := db.tx.QueryRow(
		`
            SELECT
                tv_shows.title as tv_title,
                tv_shows.original_release_year as tv_release_year,
                tv_shows.tmdb_id as tv_tmdb,
                tv_seasons.season_number as season_number,
                tv_episodes.title as episode_title,
                tv_episodes.episode_number as episode_number,
                tv_episodes.tmdb_id as episode_tmdb,
                video_files.blob_id as episode_blob
            FROM video_files
            JOIN tv_episodes ON
                video_files.match_id = tv_episodes.id
            JOIN tv_seasons ON
                tv_episodes.tv_season_id = tv_seasons.id
            JOIN tv_shows ON
                tv_episodes.tv_show_id = tv_shows.id
            WHERE
                video_files.video_type = 3
                AND video_files.id = ?
            LIMIT 1
		`,
		videoId,
	)
	var entry TvExportEntry
	if err := result.Scan(
		&entry.TvTitle,
		&entry.TvReleaseYear,
		&entry.TvTmdb,
		&entry.SeasonNumber,
		&entry.EpisodeTitle,
		&entry.EpisodeNumber,
		&entry.EpisodeTmdb,
		&entry.EpisodeBlob,
	); err != nil {
		return TvExportEntry{}, err
	}

	return entry, nil
}

// This function is used to retrieve information for one TV show export and act on it.
func (db *DbTx) FetchOneMovieExportInfo(videoId int64) (MovieExportEntry, error) {
	result := db.tx.QueryRow(
		`
            SELECT
            	movies.title,
            	movies.release_year,
            	movies.tmdb_id,
            	video_files.blob_id
            FROM video_files
            JOIN movies ON
                video_files.match_id = movies.id
            WHERE
            	video_files.video_type = 1
                AND video_files.id = ?
            LIMIT 1
		`,
		videoId,
	)
	var entry MovieExportEntry
	if err := result.Scan(
		&entry.MovieTitle,
		&entry.MovieReleaseYear,
		&entry.MovieTmdb,
		&entry.MovieBlob,
	); err != nil {
		return MovieExportEntry{}, err
	}

	return entry, nil
}
