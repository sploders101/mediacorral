-- Movie Metadata --

CREATE TABLE `movies`(
	`id` INTEGER NOT NULL PRIMARY KEY,
	`tmdb_id` TEXT,
	`poster_blob` INTEGER,
	`title` TEXT NOT NULL,
	`description` TEXT
);

CREATE TABLE `movies_special_features`(
	`id` INTEGER NOT NULL PRIMARY KEY,
	`movie_id` INTEGER NOT NULL,
	`thumbnail_blob` INTEGER,
	`title` TEXT NOT NULL,
	`description` TEXT
);

-- TV Show Metadata --

CREATE TABLE `tv_shows`(
	`id` INTEGER NOT NULL PRIMARY KEY,
	`tmdb_id` TEXT,
	`poster_blob` INTEGER,
	`title` TEXT NOT NULL,
	`description` TEXT
);

CREATE TABLE `tv_seasons`(
	`id` INTEGER NOT NULL PRIMARY KEY,
	`tv_show_id` INTEGER NOT NULL,
	`season_number` INTEGER NOT NULL,
	`poster_blob` INTEGER,
	`title` TEXT NOT NULL,
	`description` TEXT
);

CREATE TABLE `tv_episodes`(
	`id` INTEGER NOT NULL PRIMARY KEY,
	`tv_show_id` INTEGER NOT NULL,
	`tv_season_id` INTEGER NOT NULL,
	`episode_number` INTEGER NOT NULL,
	`thumbnail_blob` INTEGER,
	`title` TEXT,
	`description` TEXT
);

-- File tags (Movie, Special features, TV Episode) --

CREATE TABLE `movie_files`(
	`blob_id` INTEGER NOT NULL PRIMARY KEY,
	`movie_id` INTEGER NOT NULL
);

CREATE TABLE `movie_special_features_files`(
	`blob_id` INTEGER NOT NULL PRIMARY KEY,
	`movie_id` INTEGER NOT NULL
);

CREATE TABLE `tv_files`(
	`blob_id` INTEGER NOT NULL PRIMARY KEY,
	`tv_show_id` INTEGER NOT NULL,
	`tv_season_id` INTEGER NOT NULL,
	`tv_episode_id` INTEGER NOT NULL
);

-- Untagged Media --

CREATE TABLE `untagged_media`(
	`blob_id` INTEGER NOT NULL PRIMARY KEY,
	`subtitle_id` INTEGER
)

-- Video File Stream Info --

CREATE TABLE `video_metadata`(
	`blob_id` INTEGER NOT NULL PRIMARY KEY,
	`resolution` TEXT NOT NULL,
	`resolution_width` INTEGER NOT NULL,
	`resolution_height` INTEGER NOT NULL,
	`video_format` TEXT NOT NULL,
	`length` INTEGER NOT NULL,
	`audio_hash` BINARY NOT NULL
);

-- Subtitle file info --

CREATE TABLE `subtitle_metadata`(
	`blob_id` INTEGER NOT NULL PRIMARY KEY,
	`video_blob_id` INTEGER NOT NULL,
	`language` TEXT
);

-- File References --

CREATE TABLE `blobs`(
	`id` INTEGER NOT NULL PRIMARY KEY,
	`creation_time` INTEGER NOT NULL,
	`mime_type` TEXT,
	`hash` BINARY NOT NULL,
	`filename` TEXT NOT NULL
);
