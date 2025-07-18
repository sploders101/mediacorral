-- Contains a cache of TMDB data for a movie
CREATE TABLE `movies`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`tmdb_id` INTEGER UNIQUE,
	`poster_blob` INTEGER,
	`title` TEXT NOT NULL,
	`release_year` TEXT,
	`description` TEXT,
	`runtime` INTEGER
);

-- Contains information about a special feature from a movie
CREATE TABLE `movies_special_features`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`movie_id` INTEGER NOT NULL,
	`thumbnail_blob` INTEGER,
	`title` TEXT NOT NULL,
	`description` TEXT,
	`runtime` INTEGER
);

-- Contains a cache of TMDB data for a TV show
CREATE TABLE `tv_shows`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`tmdb_id` INTEGER UNIQUE,
	`poster_blob` INTEGER,
	`title` TEXT NOT NULL,
	`original_release_year` TEXT,
	`description` TEXT
);

-- Contains a cache of TMDB data for a TV season (part of a TV show)
CREATE TABLE `tv_seasons`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`tmdb_id` INTEGER UNIQUE,
	`tv_show_id` INTEGER NOT NULL,
	`season_number` INTEGER NOT NULL,
	`poster_blob` INTEGER,
	`title` TEXT NOT NULL,
	`description` TEXT
);
CREATE UNIQUE INDEX tv_season_unique ON tv_seasons (tv_show_id, season_number);

-- Contains a cache of TMDB data for a TV episode (part of a TV season)
CREATE TABLE `tv_episodes`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`tmdb_id` INTEGER UNIQUE,
	`tv_show_id` INTEGER NOT NULL,
	`tv_season_id` INTEGER NOT NULL,
	`episode_number` INTEGER NOT NULL,
	`thumbnail_blob` INTEGER,
	`title` TEXT NOT NULL,
	`description` TEXT,
	`runtime` INTEGER
);
CREATE UNIQUE INDEX tv_episode_unique ON tv_episodes (tv_season_id, episode_number);

-- Contains information about each rip job. Useful for grouping video files by the discs they came from.
CREATE TABLE `rip_jobs`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`start_time` INTEGER NOT NULL,
	`disc_title` TEXT,
	`suspected_contents` BINARY,
	`rip_finished` BOOLEAN,
	`imported` BOOLEAN
);

-- Contains references to the files containing video content
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
	`resolution_width` INTEGER,
	`resolution_height` INTEGER,
	`length` INTEGER,
	`original_video_hash` BINARY,
	`rip_job` INTEGER
);

-- Contains subtitle files extracted from the mkv used for comparison.
-- Subtitle entries may be added here with a NULL blob_id to mark that no subtitles were
-- found in the video file. This is used for job tracking.
CREATE TABLE `subtitle_files`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`blob_id` TEXT,
	`video_file` INTEGER NOT NULL
);

-- Contains subtitles downloaded from Opensubtitles before they've been processed.
CREATE TABLE `ost_downloads`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`video_type` INTEGER NOT NULL,
	`match_id` INTEGER NOT NULL,
	`filename` STRING NOT NULL,
	`blob_id` TEXT NOT NULL
);

-- Contains information about the comparisons made between video files
CREATE TABLE `match_info`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`video_file_id` INTEGER NOT NULL,
	`ost_download_id` INTEGER NOT NULL,
	`distance` INTEGER NOT NULL,
	`max_distance` INTEGER NOT NULL
);

CREATE TABLE `image_files`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`blob_id` TEXT NOT NULL,
	`mime_type` TEXT NOT NULL,
	`name` TEXT,
	`rip_job` INTEGER
);
