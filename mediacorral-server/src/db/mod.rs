use mediacorral_proto::mediacorral::server::v1 as proto;
use prost::Message;
use schemas::*;
use serde::{Deserialize, Serialize};
use sqlx::{Row, prelude::FromRow};

pub mod schemas;

type Db = sqlx::SqlitePool;

pub async fn insert_movie(db: &Db, movie: &MoviesItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "
            INSERT INTO movies (
                id,
                tmdb_id,
                poster_blob,
                title,
                release_year,
                description,
                runtime
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                tmdb_id = ?,
                poster_blob = ?,
                title = ?,
                release_year = ?,
                description = ?,
                runtime = ?
            RETURNING id
        ",
    )
    .bind(movie.id)
    .bind(movie.tmdb_id)
    .bind(movie.poster_blob)
    .bind(&movie.title)
    .bind(&movie.release_year)
    .bind(&movie.description)
    .bind(movie.runtime)
    .bind(movie.tmdb_id)
    .bind(movie.poster_blob)
    .bind(&movie.title)
    .bind(&movie.release_year)
    .bind(&movie.description)
    .bind(movie.runtime)
    .fetch_one(db)
    .await?;

    return Ok(result.get(0));
}

pub async fn insert_tmdb_movie(db: &Db, movie: &MoviesItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "
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
        ",
    )
    .bind(movie.tmdb_id)
    .bind(movie.poster_blob)
    .bind(&movie.title)
    .bind(&movie.release_year)
    .bind(&movie.description)
    .bind(movie.runtime)
    .bind(movie.poster_blob)
    .bind(&movie.title)
    .bind(&movie.release_year)
    .bind(&movie.description)
    .bind(movie.runtime)
    .fetch_one(db)
    .await?;

    return Ok(result.get(0));
}

pub async fn get_movies(db: &Db) -> Result<Vec<MoviesItem>, sqlx::Error> {
    let result = sqlx::query_as(
        "
            SELECT
                id,
                tmdb_id,
                poster_blob,
                title,
                release_year,
                description,
                runtime
            FROM movies
        ",
    )
    .fetch_all(db)
    .await?;
    return Ok(result);
}

pub async fn get_movie_by_id(db: &Db, movie_id: i64) -> Result<MoviesItem, sqlx::Error> {
    let result = sqlx::query_as(
        "
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
            LIMIT 1
        ",
    )
    .bind(movie_id)
    .fetch_one(db)
    .await?;
    return Ok(result);
}

pub async fn get_movie_by_tmdb_id(db: &Db, tmdb_id: i32) -> Result<MoviesItem, sqlx::Error> {
    let result = sqlx::query_as(
        "
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
            LIMIT 1
        ",
    )
    .bind(tmdb_id)
    .fetch_one(db)
    .await?;
    return Ok(result);
}

pub async fn insert_movies_special_feature(
    db: &Db,
    movie_special_feature: &MoviesSpecialFeaturesItem,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "
            INSERT INTO movies_special_features (
                id,
                movie_id,
                thumbnail_blob,
                title,
                description
            ) VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                movie_id = ?,
                thumbnail_blob = ?,
                title = ?,
                description = ?
            RETURNING id
        ",
    )
    .bind(movie_special_feature.id)
    .bind(movie_special_feature.movie_id)
    .bind(movie_special_feature.thumbnail_blob)
    .bind(&movie_special_feature.title)
    .bind(&movie_special_feature.description)
    .bind(movie_special_feature.movie_id)
    .bind(movie_special_feature.thumbnail_blob)
    .bind(&movie_special_feature.title)
    .bind(&movie_special_feature.description)
    .fetch_one(db)
    .await?;

    return Ok(result.get(0));
}

pub async fn insert_tv_show(db: &Db, tv_show: &TvShowsItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "
            INSERT INTO tv_shows (
                id,
                tmdb_id,
                poster_blob,
                title,
                original_release_year,
                description
            ) VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                tmdb_id = ?,
                poster_blob = ?,
                title = ?,
                original_release_year = ?,
                description = ?
            RETURNING id
        ",
    )
    .bind(tv_show.id)
    .bind(tv_show.tmdb_id)
    .bind(tv_show.poster_blob)
    .bind(&tv_show.title)
    .bind(&tv_show.original_release_year)
    .bind(&tv_show.description)
    .bind(tv_show.tmdb_id)
    .bind(tv_show.poster_blob)
    .bind(&tv_show.title)
    .bind(&tv_show.original_release_year)
    .bind(&tv_show.description)
    .fetch_one(db)
    .await?;

    return Ok(result.get(0));
}

pub async fn insert_tmdb_tv_show(db: &Db, tv_show: &TvShowsItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "
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
        ",
    )
    .bind(tv_show.tmdb_id)
    .bind(tv_show.poster_blob)
    .bind(&tv_show.title)
    .bind(&tv_show.original_release_year)
    .bind(&tv_show.description)
    .bind(tv_show.poster_blob)
    .bind(&tv_show.title)
    .bind(&tv_show.original_release_year)
    .bind(&tv_show.description)
    .fetch_one(db)
    .await?;

    return Ok(result.get(0));
}

// TODO: Add paging
pub async fn get_tv_shows(db: &Db) -> Result<Vec<TvShowsItem>, sqlx::Error> {
    return Ok(sqlx::query_as(
        "
            SELECT
                id,
                tmdb_id,
                poster_blob,
                title,
                original_release_year,
                description
            FROM tv_shows
            LIMIT 1000
        ",
    )
    .fetch_all(db)
    .await?);
}

pub async fn get_tv_seasons(db: &Db, series_id: i64) -> Result<Vec<TvSeasonsItem>, sqlx::Error> {
    return Ok(sqlx::query_as(
        "
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
        ",
    )
    .bind(series_id)
    .fetch_all(db)
    .await?);
}

pub async fn get_tv_episodes(db: &Db, season_id: i64) -> Result<Vec<TvEpisodesItem>, sqlx::Error> {
    return Ok(sqlx::query_as(
        "
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
        ",
    )
    .bind(season_id)
    .fetch_all(db)
    .await?);
}

pub async fn get_tv_show_by_id(db: &Db, show_id: i64) -> Result<TvShowsItem, sqlx::Error> {
    return Ok(sqlx::query_as(
        "
            SELECT
                id,
                tmdb_id,
                poster_blob,
                title,
                original_release_year,
                description
            FROM tv_shows
            WHERE id = ?
            LIMIT 1
        ",
    )
    .bind(show_id)
    .fetch_one(db)
    .await?);
}

pub async fn get_tv_season_by_id(db: &Db, season_id: i64) -> Result<TvSeasonsItem, sqlx::Error> {
    return Ok(sqlx::query_as(
        "
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
            LIMIT 1
        ",
    )
    .bind(season_id)
    .fetch_one(db)
    .await?);
}

pub async fn get_tv_episode_by_id(db: &Db, episode_id: i64) -> Result<TvEpisodesItem, sqlx::Error> {
    return Ok(sqlx::query_as(
        "
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
            LIMIT 1
        ",
    )
    .bind(episode_id)
    .fetch_one(db)
    .await?);
}

pub async fn get_tv_episode_by_tmdb_id(
    db: &Db,
    episode_id: i32,
) -> Result<TvEpisodesItem, sqlx::Error> {
    return Ok(sqlx::query_as(
        "
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
            LIMIT 1
        ",
    )
    .bind(episode_id)
    .fetch_one(db)
    .await?);
}

pub async fn insert_tv_season(db: &Db, tv_season: &TvSeasonsItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "
            INSERT INTO tv_seasons (
                id,
                tmdb_id,
                tv_show_id,
                season_number,
                poster_blob,
                title,
                description
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                tmdb_id = ?,
                tv_show_id = ?,
                season_number = ?,
                poster_blob = ?,
                title = ?,
                description = ?
            RETURNING id
        ",
    )
    .bind(tv_season.id)
    .bind(tv_season.tmdb_id)
    .bind(tv_season.tv_show_id)
    .bind(tv_season.season_number)
    .bind(tv_season.poster_blob)
    .bind(&tv_season.title)
    .bind(&tv_season.description)
    .bind(tv_season.tmdb_id)
    .bind(tv_season.tv_show_id)
    .bind(tv_season.season_number)
    .bind(tv_season.poster_blob)
    .bind(&tv_season.title)
    .bind(&tv_season.description)
    .fetch_one(db)
    .await?;

    return Ok(result.get(0));
}

pub async fn upsert_tv_season(db: &Db, tv_season: &TvSeasonsItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "
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
            RETURNING id
        ",
    )
    .bind(tv_season.tmdb_id)
    .bind(tv_season.tv_show_id)
    .bind(tv_season.season_number)
    .bind(tv_season.poster_blob)
    .bind(&tv_season.title)
    .bind(&tv_season.description)
    .bind(tv_season.poster_blob)
    .bind(&tv_season.title)
    .bind(&tv_season.description)
    .fetch_one(db)
    .await?;

    return Ok(result.get(0));
}

pub async fn insert_tv_episode(db: &Db, tv_episode: &TvEpisodesItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "
            INSERT INTO tv_episodes (
                id,
                tmdb_id,
                tv_show_id,
                tv_season_id,
                episode_number,
                thumbnail_blob,
                title,
                description,
                runtime
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                tmdb_id = ?,
                tv_show_id = ?,
                tv_season_id = ?,
                episode_number = ?,
                thumbnail_blob = ?,
                title = ?,
                description = ?
                runtime = ?
            RETURNING id
        ",
    )
    .bind(tv_episode.id)
    .bind(tv_episode.tmdb_id)
    .bind(tv_episode.tv_show_id)
    .bind(tv_episode.tv_season_id)
    .bind(tv_episode.episode_number)
    .bind(tv_episode.thumbnail_blob)
    .bind(&tv_episode.title)
    .bind(&tv_episode.description)
    .bind(tv_episode.runtime)
    .bind(tv_episode.tmdb_id)
    .bind(tv_episode.tv_show_id)
    .bind(tv_episode.tv_season_id)
    .bind(tv_episode.episode_number)
    .bind(tv_episode.thumbnail_blob)
    .bind(&tv_episode.title)
    .bind(&tv_episode.description)
    .bind(tv_episode.runtime)
    .fetch_one(db)
    .await?;

    return Ok(result.get(0));
}

pub async fn upsert_tv_episode(db: &Db, tv_episode: &TvEpisodesItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "
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
            RETURNING id
        ",
    )
    .bind(tv_episode.tmdb_id)
    .bind(tv_episode.tv_show_id)
    .bind(tv_episode.tv_season_id)
    .bind(tv_episode.episode_number)
    .bind(tv_episode.thumbnail_blob)
    .bind(&tv_episode.title)
    .bind(&tv_episode.description)
    .bind(tv_episode.runtime)
    .bind(tv_episode.thumbnail_blob)
    .bind(&tv_episode.title)
    .bind(&tv_episode.description)
    .bind(tv_episode.runtime)
    .fetch_one(db)
    .await?;

    return Ok(result.get(0));
}

pub async fn insert_rip_jobs(db: &Db, rip_job: &RipJobsItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "
            INSERT INTO rip_jobs (
                id,
                start_time,
                disc_title,
                suspected_contents,
                rip_finished,
                imported
            ) VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                start_time = ?,
                disc_title = ?,
                suspected_contents = ?,
                rip_finished = ?,
                imported = ?
            RETURNING id
        ",
    )
    .bind(rip_job.id)
    .bind(rip_job.start_time)
    .bind(&rip_job.disc_title)
    .bind(&rip_job.suspected_contents)
    .bind(rip_job.rip_finished)
    .bind(rip_job.imported)
    .bind(rip_job.start_time)
    .bind(&rip_job.disc_title)
    .bind(&rip_job.suspected_contents)
    .bind(rip_job.rip_finished)
    .bind(rip_job.imported)
    .fetch_one(db)
    .await?;

    return Ok(result.get(0));
}

pub async fn add_suspicion(
    db: &Db,
    rip_job: i64,
    suspicion: Option<&proto::SuspectedContents>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "
            UPDATE rip_jobs
            SET
                suspected_contents = ?
            WHERE
                id = ?
        ",
    )
    .bind(suspicion.map(|suspicion| suspicion.encode_to_vec()))
    .bind(rip_job)
    .execute(db)
    .await?;
    return Ok(());
}

pub async fn get_rip_job(db: &Db, rip_job: i64) -> Result<RipJobsItem, sqlx::Error> {
    let result = sqlx::query_as(
        "
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
        ",
    )
    .bind(rip_job)
    .fetch_one(db)
    .await?;
    return Ok(result);
}

pub async fn rename_rip_job(db: &Db, rip_job: i64, new_name: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "
            UPDATE rip_jobs
            SET
                disc_title = ?
            WHERE
                id = ?
        ",
    )
    .bind(new_name)
    .bind(rip_job)
    .execute(db)
    .await?;
    return Ok(());
}

pub async fn mark_rip_job_finished(
    db: &Db,
    rip_job: i64,
    finished: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "
            UPDATE rip_jobs
            SET
                rip_finished = ?
            WHERE
                id = ?
        ",
    )
    .bind(finished)
    .bind(rip_job)
    .execute(db)
    .await?;
    return Ok(());
}

pub async fn mark_rip_job_imported(
    db: &Db,
    rip_job: i64,
    imported: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "
            UPDATE rip_jobs
            SET
                imported = ?
            WHERE
                id = ?
        ",
    )
    .bind(imported)
    .bind(rip_job)
    .execute(db)
    .await?;
    return Ok(());
}

pub async fn insert_video_file(db: &Db, video_file: &VideoFilesItem) -> Result<i64, sqlx::Error> {
    let mkv_hash = video_file
        .original_video_hash
        .as_ref()
        .map(|i| i.as_slice());
    let result = sqlx::query(
        "
            INSERT INTO video_files (
                id,
                video_type,
                match_id,
                blob_id,
                resolution_width,
                resolution_height,
                length,
                original_video_hash,
                rip_job
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                video_type = ?,
                match_id = ?,
                blob_id = ?,
                resolution_width = ?,
                resolution_height = ?,
                length = ?,
                original_video_hash = ?,
                rip_job = ?
            RETURNING id
        ",
    )
    .bind(video_file.id)
    .bind(video_file.video_type)
    .bind(video_file.match_id)
    .bind(&video_file.blob_id)
    .bind(video_file.resolution_width)
    .bind(video_file.resolution_height)
    .bind(video_file.length)
    .bind(mkv_hash)
    .bind(video_file.rip_job)
    .bind(video_file.video_type)
    .bind(video_file.match_id)
    .bind(&video_file.blob_id)
    .bind(video_file.resolution_width)
    .bind(video_file.resolution_height)
    .bind(video_file.length)
    .bind(mkv_hash)
    .bind(video_file.rip_job)
    .fetch_one(db)
    .await?;

    return Ok(result.get(0));
}

pub async fn get_video_file(db: &Db, id: i64) -> sqlx::Result<VideoFilesItem> {
    return sqlx::query_as(
        "
            SELECT
                id,
                video_type,
                match_id,
                blob_id,
                resolution_width,
                resolution_height,
                length,
                original_video_hash,
                rip_job
            FROM video_files
            WHERE
                id = ?
        ",
    )
    .bind(id)
    .fetch_one(db)
    .await;
}

pub async fn add_video_metadata(
    db: &Db,
    id: i64,
    resolution_width: u32,
    resolution_height: u32,
    length: u32,
    original_video_hash: &[u8],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "
            UPDATE video_files
            SET
                resolution_width = ?,
                resolution_height = ?,
                length = ?,
                original_video_hash = ?
            WHERE
                id = ?
        ",
    )
    .bind(resolution_width)
    .bind(resolution_height)
    .bind(length)
    .bind(original_video_hash)
    .bind(id)
    .execute(db)
    .await?;

    return Ok(());
}

pub async fn tag_video_file(
    db: &Db,
    id: i64,
    video_type: VideoType,
    match_id: Option<i64>,
) -> Result<(), sqlx::Error> {
    let result = sqlx::query(
        "
            UPDATE video_files
            SET
                video_type = ?,
                match_id = ?
            WHERE
                id = ?;
        ",
    )
    .bind(video_type)
    .bind(match_id)
    .bind(id)
    .execute(db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }

    return Ok(());
}

pub async fn insert_subtitle_file(
    db: &Db,
    subtitle_file: &SubtitleFilesItem,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "
            INSERT INTO subtitle_files (
                id,
                blob_id,
                video_file
            ) VALUES (?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                blob_id = ?,
                video_file = ?
            RETURNING id
        ",
    )
    .bind(subtitle_file.id)
    .bind(&subtitle_file.blob_id)
    .bind(subtitle_file.video_file)
    .bind(&subtitle_file.blob_id)
    .bind(subtitle_file.video_file)
    .fetch_one(db)
    .await?;

    return Ok(result.get(0));
}

pub async fn get_subtitles_for_video(
    db: &Db,
    video_file: i64,
) -> Result<Vec<SubtitleFilesItem>, sqlx::Error> {
    let result = sqlx::query_as(
        "
            SELECT
                id,
                blob_id,
                video_file
            FROM subtitle_files
            WHERE
                video_file = ?
        ",
    )
    .bind(video_file)
    .fetch_all(db)
    .await?;

    return Ok(result);
}

pub async fn insert_ost_download_item(
    db: &Db,
    ost_download_item: &OstDownloadsItem,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "
            INSERT INTO ost_downloads (
                id,
                video_type,
                match_id,
                filename,
                blob_id
            ) VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                video_type = ?,
                match_id = ?,
                filename = ?,
                blob_id = ?
            RETURNING id
        ",
    )
    .bind(ost_download_item.id)
    .bind(ost_download_item.video_type)
    .bind(ost_download_item.match_id)
    .bind(&ost_download_item.filename)
    .bind(&ost_download_item.blob_id)
    .bind(ost_download_item.video_type)
    .bind(ost_download_item.match_id)
    .bind(&ost_download_item.filename)
    .bind(&ost_download_item.blob_id)
    .fetch_one(db)
    .await?;

    return Ok(result.get(0));
}

pub async fn get_ost_download_items_by_tmdb_id(
    db: &Db,
    tmdb_id: i32,
) -> Result<OstDownloadsItem, sqlx::Error> {
    let results = sqlx::query_as(
        "
            SELECT
                ost_downloads.id,
                ost_downloads.video_type,
                ost_downloads.match_id,
                ost_downloads.filename,
                ost_downloads.blob_id
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
        ",
    )
    .bind(tmdb_id)
    .bind(tmdb_id)
    .fetch_one(db)
    .await?;
    return Ok(results);
}

pub async fn get_ost_download_items_by_match(
    db: &Db,
    video_type: VideoType,
    match_id: i64,
) -> Result<Vec<OstDownloadsItem>, sqlx::Error> {
    let results = sqlx::query_as(
        "
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
        ",
    )
    .bind(video_type)
    .bind(match_id)
    .fetch_all(db)
    .await?;
    return Ok(results);
}

pub async fn clear_match_info_for_job(db: &Db, job_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        "
            DELETE FROM match_info
            WHERE video_file_id IN (
                SELECT id
                FROM video_files
                WHERE
                    rip_job = ?
            )
        ",
    )
    .bind(job_id)
    .execute(db)
    .await?;
    return Ok(());
}

pub async fn insert_match_info_item(
    db: &Db,
    match_info_item: &MatchInfoItem,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "
            INSERT INTO match_info (
                id,
                video_file_id,
                ost_download_id,
                distance,
                max_distance
            ) VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                video_file_id = ?,
                ost_download_id = ?,
                distance = ?,
                max_distance = ?
            RETURNING id
        ",
    )
    .bind(match_info_item.id)
    .bind(match_info_item.video_file_id)
    .bind(match_info_item.ost_download_id)
    .bind(match_info_item.distance)
    .bind(match_info_item.max_distance)
    .bind(match_info_item.video_file_id)
    .bind(match_info_item.ost_download_id)
    .bind(match_info_item.distance)
    .bind(match_info_item.max_distance)
    .fetch_one(db)
    .await?;

    return Ok(result.get(0));
}

pub async fn insert_image_file(db: &Db, image_file: &ImageFilesItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "
            INSERT INTO image_files (
                id,
                blob_id,
                mime_type,
                name,
                rip_job
            ) VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                blob_id = ?,
                mime_type = ?,
                name = ?,
                rip_job = ?
            RETURNING id
        ",
    )
    .bind(image_file.id)
    .bind(&image_file.blob_id)
    .bind(&image_file.mime_type)
    .bind(&image_file.name)
    .bind(image_file.rip_job)
    .bind(&image_file.blob_id)
    .bind(&image_file.mime_type)
    .bind(&image_file.name)
    .bind(image_file.rip_job)
    .fetch_one(db)
    .await?;

    return Ok(result.get(0));
}

pub async fn delete_blob(db: &Db, blob_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "
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
        ",
    )
    .bind(blob_id)
    .bind(blob_id)
    .bind(blob_id)
    .bind(blob_id)
    .execute(db)
    .await?;
    return Ok(());
}

#[derive(Serialize, Deserialize, Debug, Clone, FromRow)]
pub struct RipVideoBlobs {
    pub id: i64,
    pub job_id: i64,
    pub video_blob: String,
    pub subtitle_blob: Option<String>,
}
impl Into<proto::RipVideoBlobs> for RipVideoBlobs {
    fn into(self) -> proto::RipVideoBlobs {
        return proto::RipVideoBlobs {
            id: self.id,
            job_id: self.job_id,
            video_blob: self.video_blob,
            subtitle_blob: self.subtitle_blob,
        };
    }
}

/// Fetches all of the blobs associated with videos from a rip job
pub async fn get_rip_video_blobs(db: &Db, rip_job: i64) -> Result<Vec<RipVideoBlobs>, sqlx::Error> {
    return Ok(sqlx::query_as(
        "
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
        ",
    )
    .bind(rip_job)
    .fetch_all(db)
    .await?);
}

#[derive(Serialize, Deserialize, Debug, Clone, FromRow)]
pub struct RipImageBlob {
    pub job_id: i64,
    pub image_blob: String,
}

pub async fn get_rip_image_blobs(db: &Db, rip_job: i64) -> Result<Vec<RipImageBlob>, sqlx::Error> {
    return Ok(sqlx::query_as(
        "
            SELECT
                rip_jobs.id AS job_id,
                image_files.blob_id AS image_blob
            FROM rip_jobs
            INNER JOIN image_files ON
                rip_jobs.id = image_files.rip_job
            WHERE
                rip_jobs.id = ?
        ",
    )
    .bind(rip_job)
    .fetch_all(db)
    .await?);
}

pub async fn delete_rip_job(db: &Db, rip_job: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        "
            DELETE FROM rip_jobs
            WHERE
                id = ?
        ",
    )
    .bind(rip_job)
    .execute(db)
    .await?;

    return Ok(());
}

pub async fn get_untagged_videos_from_job(
    db: &Db,
    rip_job: i64,
) -> Result<Vec<RipVideoBlobs>, sqlx::Error> {
    return Ok(sqlx::query_as(
        "
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
        ",
    )
    .bind(rip_job)
    .fetch_all(db)
    .await?);
}

pub async fn get_rip_jobs_with_untagged_videos(
    db: &Db,
    skip: u32,
    limit: u32,
) -> Result<Vec<RipJobsItem>, sqlx::Error> {
    return Ok(sqlx::query_as(
        "
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
        ",
    )
    .bind(limit)
    .bind(skip)
    .fetch_all(db)
    .await?);
}

pub async fn get_videos_from_rip(
    db: &Db,
    rip_job: i64,
) -> Result<Vec<VideoFilesItem>, sqlx::Error> {
    let results: Vec<VideoFilesItem> = sqlx::query_as(
        "
            SELECT
                id,
                video_type,
                match_id,
                blob_id,
                resolution_width,
                resolution_height,
                length,
                original_video_hash,
                rip_job
            FROM video_files
            WHERE
                rip_job = ?
        ",
    )
    .bind(rip_job)
    .fetch_all(db)
    .await?;
    return Ok(results);
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq, FromRow)]
pub struct DiscSubsWithVideo {
    pub video_id: i64,
    pub subtitle_id: i64,
    pub subtitle_blob: String,
}
pub async fn get_disc_subs_from_rip(
    db: &Db,
    rip_job: i64,
) -> Result<Vec<DiscSubsWithVideo>, sqlx::Error> {
    let results: Vec<DiscSubsWithVideo> = sqlx::query_as(
        "
            SELECT
                video_files.id as video_id,
                subtitle_files.id as subtitle_id,
                subtitle_files.blob_id as subtitle_blob
            FROM video_files
            INNER JOIN subtitle_files ON
                video_files.id = subtitle_files.video_file
            WHERE
                video_files.rip_job = ?
        ",
    )
    .bind(rip_job)
    .fetch_all(db)
    .await?;
    return Ok(results);
}

pub async fn get_matches_from_rip(
    db: &Db,
    rip_job: i64,
) -> Result<Vec<MatchInfoItem>, sqlx::Error> {
    let results: Vec<MatchInfoItem> = sqlx::query_as(
        "
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
        ",
    )
    .bind(rip_job)
    .fetch_all(db)
    .await?;
    return Ok(results);
}

pub async fn delete_matches_from_rip(db: &Db, rip_job: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        "
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
        ",
    )
    .bind(rip_job)
    .execute(db)
    .await?;
    return Ok(());
}

pub async fn get_ost_subtitles_from_rip(
    db: &Db,
    rip_job: i64,
) -> Result<Vec<OstDownloadsItem>, sqlx::Error> {
    let results: Vec<OstDownloadsItem> = sqlx::query_as(
        "
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
        ",
    )
    .bind(rip_job)
    .fetch_all(db)
    .await?;
    return Ok(results);
}
