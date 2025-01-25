use schemas::*;

pub mod schemas;

type Db = sqlx::SqlitePool;

pub async fn insert_movie(db: &Db, movie: &MoviesItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        "
            INSERT INTO movies (
                id,
                tmdb_id,
                poster_blob,
                title,
                description
            ) VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                tmdb_id = ?,
                poster_blob = ?,
                title = ?,
                description = ?
        ",
        movie.id,
        movie.tmdb_id,
        movie.poster_blob,
        movie.title,
        movie.description,
        movie.tmdb_id,
        movie.poster_blob,
        movie.title,
        movie.description,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn insert_tmdb_movie(db: &Db, movie: &MoviesItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        "
            INSERT INTO movies (
                tmdb_id,
                poster_blob,
                title,
                description
            ) VALUES (?, ?, ?, ?)
            ON CONFLICT (tmdb_id) DO UPDATE SET
                poster_blob = ?,
                title = ?,
                description = ?
        ",
        movie.tmdb_id,
        movie.poster_blob,
        movie.title,
        movie.description,
        movie.poster_blob,
        movie.title,
        movie.description,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn insert_movies_special_feature(
    db: &Db,
    movie_special_feature: &MoviesSpecialFeaturesItem,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
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
        ",
        movie_special_feature.id,
        movie_special_feature.movie_id,
        movie_special_feature.thumbnail_blob,
        movie_special_feature.title,
        movie_special_feature.description,
        movie_special_feature.movie_id,
        movie_special_feature.thumbnail_blob,
        movie_special_feature.title,
        movie_special_feature.description,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn insert_tv_show(db: &Db, tv_show: &TvShowsItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        "
            INSERT INTO tv_shows (
                id,
                tmdb_id,
                poster_blob,
                title,
                description
            ) VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                tmdb_id = ?,
                poster_blob = ?,
                title = ?,
                description = ?
        ",
        tv_show.id,
        tv_show.tmdb_id,
        tv_show.poster_blob,
        tv_show.title,
        tv_show.description,
        tv_show.tmdb_id,
        tv_show.poster_blob,
        tv_show.title,
        tv_show.description,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn insert_tmdb_tv_show(db: &Db, tv_show: &TvShowsItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        "
            INSERT INTO tv_shows (
                tmdb_id,
                poster_blob,
                title,
                description
            ) VALUES (?, ?, ?, ?)
            ON CONFLICT (tmdb_id) DO UPDATE SET
                poster_blob = ?,
                title = ?,
                description = ?
        ",
        tv_show.tmdb_id,
        tv_show.poster_blob,
        tv_show.title,
        tv_show.description,
        tv_show.poster_blob,
        tv_show.title,
        tv_show.description,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn insert_tv_season(db: &Db, tv_season: &TvSeasonsItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        "
            INSERT INTO tv_seasons (
                id,
                tv_show_id,
                season_number,
                poster_blob,
                title,
                description
            ) VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                tv_show_id = ?,
                season_number = ?,
                poster_blob = ?,
                title = ?,
                description = ?
        ",
        tv_season.id,
        tv_season.tv_show_id,
        tv_season.season_number,
        tv_season.poster_blob,
        tv_season.title,
        tv_season.description,
        tv_season.tv_show_id,
        tv_season.season_number,
        tv_season.poster_blob,
        tv_season.title,
        tv_season.description,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn upsert_tv_season(db: &Db, tv_season: &TvSeasonsItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        "
            INSERT INTO tv_seasons (
                tv_show_id,
                season_number,
                poster_blob,
                title,
                description
            ) VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (tv_show_id, season_number) DO UPDATE SET
                poster_blob = ?,
                title = ?,
                description = ?
        ",
        tv_season.tv_show_id,
        tv_season.season_number,
        tv_season.poster_blob,
        tv_season.title,
        tv_season.description,
        tv_season.poster_blob,
        tv_season.title,
        tv_season.description,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn insert_tv_episode(db: &Db, tv_episode: &TvEpisodesItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        "
            INSERT INTO tv_episodes (
                id,
                tv_show_id,
                tv_season_id,
                episode_number,
                thumbnail_blob,
                title,
                description
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                tv_show_id = ?,
                tv_season_id = ?,
                episode_number = ?,
                thumbnail_blob = ?,
                title = ?,
                description = ?
        ",
        tv_episode.id,
        tv_episode.tv_show_id,
        tv_episode.tv_season_id,
        tv_episode.episode_number,
        tv_episode.thumbnail_blob,
        tv_episode.title,
        tv_episode.description,
        tv_episode.tv_show_id,
        tv_episode.tv_season_id,
        tv_episode.episode_number,
        tv_episode.thumbnail_blob,
        tv_episode.title,
        tv_episode.description,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn upsert_tv_episode(db: &Db, tv_episode: &TvEpisodesItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        "
            INSERT INTO tv_episodes (
                tv_season_id,
                episode_number,
                thumbnail_blob,
                title,
                description
            ) VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (tv_season_id, episode_number) DO UPDATE SET
                thumbnail_blob = ?,
                title = ?,
                description = ?
        ",
        tv_episode.tv_season_id,
        tv_episode.episode_number,
        tv_episode.thumbnail_blob,
        tv_episode.title,
        tv_episode.description,
        tv_episode.thumbnail_blob,
        tv_episode.title,
        tv_episode.description,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn insert_rip_jobs(db: &Db, rip_job: &RipJobsItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        "
            INSERT INTO rip_jobs (
                id,
                start_time,
                disc_title,
                suspected_contents
            ) VALUES (?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                start_time = ?,
                disc_title = ?,
                suspected_contents = ?
        ",
        rip_job.id,
        rip_job.start_time,
        rip_job.disc_title,
        rip_job.suspected_contents,
        rip_job.start_time,
        rip_job.disc_title,
        rip_job.suspected_contents,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn insert_video_file(db: &Db, video_file: &VideoFilesItem) -> Result<i64, sqlx::Error> {
    let video_type = video_file.video_type.to_db();
    let mkv_hash = video_file.original_video_hash.as_slice();
    let result = sqlx::query!(
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
        ",
        video_file.id,
        video_type,
        video_file.match_id,
        video_file.blob_id,
        video_file.resolution_width,
        video_file.resolution_height,
        video_file.length,
        mkv_hash,
        video_file.rip_job,
        video_type,
        video_file.match_id,
        video_file.blob_id,
        video_file.resolution_width,
        video_file.resolution_height,
        video_file.length,
        mkv_hash,
        video_file.rip_job,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn tag_video_file(
    db: &Db,
    id: i64,
    video_type: VideoType,
    match_id: i64,
) -> Result<(), sqlx::Error> {
    let video_type = video_type.to_db();
    sqlx::query!(
        "
            update video_files
            set
                video_type = ?,
                match_id = ?
            where
                id = ?;
        ",
        video_type,
        match_id,
        id,
    )
    .execute(db)
    .await?;

    return Ok(());
}

pub async fn insert_subtitle_file(
    db: &Db,
    subtitle_file: &SubtitleFilesItem,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        "
            INSERT INTO subtitle_files (
                id,
                blob_id,
                video_file
            ) VALUES (?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                blob_id = ?,
                video_file = ?
        ",
        subtitle_file.id,
        subtitle_file.blob_id,
        subtitle_file.video_file,
        subtitle_file.blob_id,
        subtitle_file.video_file,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn insert_ost_download_item(
    db: &Db,
    ost_download_item: &OstDownloadsItem,
) -> Result<i64, sqlx::Error> {
    let video_type = ost_download_item.video_type.to_db();
    let result = sqlx::query!(
        "
            INSERT INTO ost_downloads (
                id,
                video_type,
                match_id,
                filename,
                ost_url,
                blob_id
            ) VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                video_type = ?,
                match_id = ?,
                filename = ?,
                ost_url = ?,
                blob_id = ?
        ",
        ost_download_item.id,
        video_type,
        ost_download_item.match_id,
        ost_download_item.filename,
        ost_download_item.ost_url,
        ost_download_item.blob_id,
        video_type,
        ost_download_item.match_id,
        ost_download_item.filename,
        ost_download_item.ost_url,
        ost_download_item.blob_id,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn insert_match_info_item(
    db: &Db,
    match_info_item: &MatchInfoItem,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
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
        ",
        match_info_item.id,
        match_info_item.video_file_id,
        match_info_item.ost_download_id,
        match_info_item.distance,
        match_info_item.max_distance,
        match_info_item.video_file_id,
        match_info_item.ost_download_id,
        match_info_item.distance,
        match_info_item.max_distance,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn insert_image_file(db: &Db, image_file: &ImageFilesItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
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
        ",
        image_file.id,
        image_file.blob_id,
        image_file.mime_type,
        image_file.name,
        image_file.rip_job,
        image_file.blob_id,
        image_file.mime_type,
        image_file.name,
        image_file.rip_job,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}
