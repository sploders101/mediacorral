// Utilities
import type {
	MovieMetadata,
	TvEpisodeMetadata,
	TvSeasonMetadata,
	TvShowMetadata,
} from "@/apiTypes";
import { BASE_URL } from "@/scripts/config";
import { defineStore } from "pinia";

export const useAppStore = defineStore("app", () => {
	const driveList = ref<string[]>([]);

	async function getDriveList() {
		const response = await fetch(`${BASE_URL}/ripping/list_drives`);
		if (response.status !== 200)
			throw new Error("Bad response from drive list");
		const data = await response.json();
		driveList.value = data;
	}

	const movies = ref<Record<number, MovieMetadata>>({});
	async function getMovieList() {
		const response = await fetch(`${BASE_URL}/tagging/metadata/movies/list`);
		if (response.status !== 200)
			throw new Error("Bad response from movies list");
		const data: MovieMetadata[] = await response.json();
		const moviesIndex: Record<number, MovieMetadata> = {};
		for (const movie of data) {
			moviesIndex[movie.id] = movie;
		}
		movies.value = moviesIndex;
	}

	const tvShows = ref<Record<number, TvShowMetadata>>({});
	async function getTvShows() {
		const response = await fetch(`${BASE_URL}/tagging/metadata/tv/list`);
		if (response.status !== 200) throw new Error("Bad response from tv list");
		const data = await response.json();
		tvShows.value = data;
	}

	/** tv_show_id -> tv_series_id -> tv_season_metadata */
	const tvSeasons = ref<Record<number, Record<number, TvSeasonMetadata>>>({});
	const tvSeasonsFlat = computed(() =>
		Object.values(tvSeasons.value).reduce(
			(prev, curr) => Object.assign(prev, curr),
			{}
		)
	);
	async function getTvSeasons(seriesId: number) {
		const response = await fetch(
			`${BASE_URL}/tagging/metadata/tv/${seriesId}/seasons`
		);
		if (response.status !== 200)
			throw new Error("Bad response from tv series list");
		const data: TvSeasonMetadata[] = await response.json();
		const seasonData: Record<number, TvSeasonMetadata> = {};
		for (const season of data) {
			seasonData[season.id] = season;
		}
		tvSeasons.value[seriesId] = seasonData;
	}

	/** season_id -> episode_id -> episode */
	const tvEpisodes = ref<Record<number, Record<number, TvEpisodeMetadata>>>({});
	const tvEpisodesFlat = computed(() =>
		Object.values(tvEpisodes.value).reduce(
			(prev, curr) => Object.assign(prev, curr),
			{}
		)
	);
	async function getTvEpisodes(seasonId: number) {
		const response = await fetch(
			`${BASE_URL}/tagging/metadata/tv/seasons/${seasonId}/episodes`
		);
		if (response.status !== 200)
			throw new Error("Bad response from tv episodes list");
		const data: TvEpisodeMetadata[] = await response.json();
		const episodeData: Record<number, TvEpisodeMetadata> = {};
		for (const episode of data) {
			episodeData[episode.id] = episode;
		}
		tvEpisodes.value[seasonId] = episodeData;
	}

	return {
		driveList,
		getDriveList,
		movies,
		getMovieList,
		tvShows,
		getTvShows,
		tvSeasons,
		tvSeasonsFlat,
		getTvSeasons,
		tvEpisodes,
		tvEpisodesFlat,
		getTvEpisodes,
	};
});
