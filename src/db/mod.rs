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

pub async fn insert_movie_file(db: &Db, movie_file: &MovieFilesItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        "
            INSERT INTO movie_files (
                blob_id,
                movie_id
            ) VALUES (?, ?)
            ON CONFLICT (blob_id) DO UPDATE SET
                movie_id = ?
        ",
        movie_file.blob_id,
        movie_file.movie_id,
        movie_file.movie_id,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn insert_movie_special_features_file(
    db: &Db,
    movie_special_features_file: &MovieSpecialFeaturesFilesItem,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        "
            INSERT INTO movie_special_features_files (
                blob_id,
                movie_id
            ) VALUES (?, ?)
            ON CONFLICT (blob_id) DO UPDATE SET
                movie_id = ?
        ",
        movie_special_features_file.blob_id,
        movie_special_features_file.movie_id,
        movie_special_features_file.movie_id,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn insert_tv_file(db: &Db, tv_file: &TvFilesItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        "
            INSERT INTO tv_files (
                blob_id,
                tv_show_id,
                tv_season_id,
                tv_episode_id
            ) VALUES (?, ?, ?, ?)
            ON CONFLICT (blob_id) DO UPDATE SET
                tv_show_id = ?,
                tv_season_id = ?,
                tv_episode_id = ?
        ",
        tv_file.blob_id,
        tv_file.tv_show_id,
        tv_file.tv_season_id,
        tv_file.tv_episode_id,
        tv_file.tv_show_id,
        tv_file.tv_season_id,
        tv_file.tv_episode_id,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn insert_untagged_media(
    db: &Db,
    untagged_media: &UntaggedMediaItem,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        "
            INSERT INTO untagged_media (
                blob_id,
                subtitle_id
            )
            VALUES (?, ?)
            ON CONFLICT (blob_id) DO UPDATE SET
                subtitle_id = ?
        ",
        untagged_media.blob_id,
        untagged_media.subtitle_id,
        untagged_media.subtitle_id,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn insert_video_meta(
    db: &Db,
    video_meta: &VideoMetadataItem,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        "
            INSERT INTO video_metadata (
                blob_id,
                resolution,
                resolution_width,
                resolution_height,
                video_format,
                length,
                audio_hash
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (blob_id) DO UPDATE SET
                resolution = ?,
                resolution_width = ?,
                resolution_height = ?,
                video_format = ?,
                length = ?,
                audio_hash = ?
        ",
        video_meta.blob_id,
        video_meta.resolution,
        video_meta.resolution_width,
        video_meta.resolution_height,
        video_meta.video_format,
        video_meta.length,
        video_meta.audio_hash,
        video_meta.resolution,
        video_meta.resolution_width,
        video_meta.resolution_height,
        video_meta.video_format,
        video_meta.length,
        video_meta.audio_hash,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn insert_subtitle_metadata(
    db: &Db,
    subtitle_meta: &SubtitleMetadataItem,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        "
            INSERT INTO subtitle_metadata (
                blob_id,
                video_blob_id,
                language
            ) VALUES (?, ?, ?)
            ON CONFLICT (blob_id) DO UPDATE SET
                video_blob_id = ?,
                language = ?
        ",
        subtitle_meta.blob_id,
        subtitle_meta.video_blob_id,
        subtitle_meta.language,
        subtitle_meta.video_blob_id,
        subtitle_meta.language,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}

pub async fn insert_blob(db: &Db, blob: &BlobItem) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        "
            INSERT INTO blobs (
                id,
                creation_time,
                mime_type,
                hash,
                filename
            )
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                creation_time = ?,
                mime_type = ?,
                hash = ?,
                filename = ?
        ",
        blob.id,
        blob.creation_time,
        blob.mime_type,
        blob.hash,
        blob.filename,
        blob.creation_time,
        blob.mime_type,
        blob.hash,
        blob.filename,
    )
    .execute(db)
    .await?;

    return Ok(result.last_insert_rowid());
}
