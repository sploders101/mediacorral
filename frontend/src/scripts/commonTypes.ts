import type {
	Movie,
	TvEpisode,
	TvSeason,
	TvShow,
} from "@/generated/mediacorral/server/v1/api";

export enum SearchType {
	Unspecified = 0,
	Movie = 1,
	TvSeries = 2,
}

export interface MetaCache {
	movies: Map<bigint, Movie>;
	tvShows: Map<bigint, TvShow>;
	tvSeasons: Map<bigint, TvSeason>;
	tvEpisodes: Map<bigint, TvEpisode>;
}
