-- Movie Metadata --

CREATE TABLE `movies`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`tmdb_id` TEXT,
	`poster_blob` INTEGER,
	`title` TEXT NOT NULL,
	`description` TEXT
);

CREATE TABLE `movies_special_features`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`movie_id` INTEGER NOT NULL,
	`thumbnail_blob` INTEGER,
	`title` TEXT NOT NULL,
	`description` TEXT
);

-- TV Show Metadata --

CREATE TABLE `tv_shows`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`tmdb_id` TEXT,
	`poster_blob` INTEGER,
	`title` TEXT NOT NULL,
	`description` TEXT
);

CREATE TABLE `tv_seasons`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`tv_show_id` INTEGER NOT NULL,
	`season_number` INTEGER NOT NULL,
	`poster_blob` INTEGER,
	`title` TEXT NOT NULL,
	`description` TEXT
);

CREATE TABLE `tv_episodes`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`tv_show_id` INTEGER NOT NULL,
	`tv_season_id` INTEGER NOT NULL,
	`episode_number` INTEGER NOT NULL,
	`thumbnail_blob` INTEGER,
	`title` TEXT,
	`description` TEXT
);

-- Rip Job Tracking --
CREATE TABLE `rip_jobs`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`start_time` INTEGER NOT NULL,
	`disc_title` TEXT,
	`suspected_contents` STRING
);

-- File Tracking --

CREATE TABLE `video_files`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	-- Video type:
	-- 0 => Untagged
	-- 1 => Movie
	-- 2 => Special Feature
	-- 3 => TV Episode
	`video_type` INTEGER NOT NULL DEFAULT 0,
	-- Match ID: Identifies the specific movie, special feature, etc this video contains.
	`match_id` INTEGER,
	`blob_id` TEXT NOT NULL,
	`resolution_width` INTEGER NOT NULL,
	`resolution_height` INTEGER NOT NULL,
	`length` INTEGER NOT NULL,
	`original_mkv_hash` BINARY NOT NULL,
	`audio_hash` BINARY NOT NULL,
	`rip_job` INTEGER
);

CREATE TABLE `subtitle_files`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`blob_id` TEXT NOT NULL,
	`video_file` INTEGER NOT NULL
);

CREATE TABLE `image_files`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`blob_id` TEXT NOT NULL,
	`mime_type` TEXT NOT NULL,
	`name` TEXT,
	`rip_job` INTEGER
);
