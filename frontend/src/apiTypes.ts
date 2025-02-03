export interface JobInfo {
	id: number;
	start_time: number;
	disc_title: String | null;
	suspected_contents: SuspectedContents | null;
	video_files: VideoFilesItem[];
	matches: MatchInfoItem[];
	subtitle_maps: RipVideoBlobs[];
	ost_subtitle_files: OstDownloadsItem[];
}

export interface OstDownloadsItem {
	id: number;
	video_type: VideoType;
	match_id: number;
	filename: string;
	blob_id: string;
}

export interface RipVideoBlobs {
	id: number;
	job_id: number;
	video_blob: string;
	subtitle_blob: string | null;
}

export interface MatchInfoItem {
	id: number;
	video_file_id: number;
	ost_download_id: number;
	distance: number;
	max_distance: number;
}

export interface VideoFilesItem {
	id: number;
	video_type: VideoType;
	match_id: number | null;
	blob_id: string;
	resolution_width: number;
	resolution_height: number;
	length: number;
	original_video_hash: string;
	rip_job: number | null;
}

export type VideoType = "Untagged" | "Movie" | "SpecialFeature" | "TvEpisode";

export interface MovieMetadata {
	id: number;
	tmdb_id: number | null;
	poster_blob: number | null;
	title: string;
	release_year: string | null;
	description: string | null;
}

export interface TvShowMetadata {
	id: number;
	tmdb_id: number | null;
	poster_blob: number | null;
	title: string;
	original_release_year: string | null;
	description: string | null;
}

export interface TvSeasonMetadata {
	id: number;
	tmdb_id: number | null;
	tv_show_id: number;
	season_number: number;
	poster_blob: number | null;
	title: String;
	description: String | null;
}

export interface TvEpisodeMetadata {
	id: number;
	tmdb_id: number | null;
	tv_show_id: number;
	tv_season_id: number;
	episode_number: number;
	thumbnail_blob: number | null;
	title: string | null;
	description: string | null;
}

export interface RipJobsItem {
	id: number;
	start_time: number;
	disc_title: string | null;
	suspected_contents: string | null;
}

export interface DriveState {
	active_command: ActiveDriveCommand;
	status: DriveStatus;
	disc_name: string | null;
}
export type DriveStatus =
	| "Unknown"
	| "Empty"
	| "TrayOpen"
	| "NotReady"
	| "Loaded";
export type ActiveDriveCommand =
	| { type: "None" }
	| { type: "Error"; message: string }
	| {
			type: "Ripping";
			job_id: number;
			cprog_title: string;
			cprog_value: number;
			tprog_title: string;
			tprog_value: number;
			max_prog_value: number;
			logs: string[];
	  };

export type SuspectedContents =
	| {
			type: "Movie";
			tmdb_id: number;
	  }
	| {
			type: "TvEpisodes";
			episode_tmdb_ids: number[];
	  };

export interface RipInstruction {
	device: string;
	disc_name: string | null;
	suspected_contents: SuspectedContents | null;
	autoeject: boolean;
}

export interface TagFile {
	file: number;
	video_type: VideoType;
	match_id: number;
}
