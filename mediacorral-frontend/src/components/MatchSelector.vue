<script lang="ts">
export type SubmitData =
	| { type: SearchType.TvSeries; episodes: TvEpisode[] }
	| { type: SearchType.Movie; movie: Movie };
</script>

<script lang="ts" setup>
import {
	Movie,
	TvEpisode,
	TvSeason,
	TvShow,
} from "@/generated/mediacorral/server/v1/api";
import { SearchType } from "@/scripts/commonTypes";
import { injectKeys } from "@/scripts/config";
import MetadataImport from "./MetadataImport.vue";
import { reportErrorsFactory } from "@/scripts/uiUtils";

const rpc = inject(injectKeys.rpc)!;
const reportErrors = reportErrorsFactory();

const props = defineProps<{
	multipleEpisodes?: boolean;
}>();
const emit = defineEmits<{
	cancel: [];
	input: [SubmitData];
	submit: [SubmitData];
}>();

const importer = ref<InstanceType<typeof MetadataImport>>();
const importDialog = ref(false);
const preImportQuery = ref("");
const importQuery = ref("");
async function useImport(event: {
	type: "tv" | "movie";
	tmdb_id: number;
	internal_id: bigint;
}) {
	switch (event.type) {
		case "movie":
			if (mediaType.value === SearchType.Movie) {
				let { response } = await reportErrors(
					rpc.listMovies({}),
					"Failed to list movies"
				);
				moviesList.value = response.movies;
			}
			mediaType.value = SearchType.Movie;
			movieSelection.value = moviesList.value?.find(
				(movie) => movie.id === event.internal_id
			);
			break;
		case "tv":
			if (mediaType.value === SearchType.TvSeries) {
				let { response } = await reportErrors(
					rpc.listTvShows({}),
					"Failed to list TV shows"
				);
				tvShowsList.value = response.tvShows;
			}
			tvShowSelection.value = tvShowsList.value?.find(
				(movie) => movie.id === event.internal_id
			);
			break;
	}
	importDialog.value = false;
}

const mediaType = ref(SearchType.Unspecified);

const moviesList = ref<Movie[] | undefined>();
watch(
	() => mediaType.value,
	async () => {
		moviesList.value = undefined;
		if (mediaType.value !== SearchType.Movie) return;

		let { response } = await reportErrors(
			rpc.listMovies({}),
			"Failed to list movies"
		);
		moviesList.value = response.movies;
	}
);
const moviesListSelections = computed(() =>
	moviesList.value?.map((movie) => {
		let title = movie.title;
		if (movie.releaseYear !== undefined) {
			title += ` (${movie.releaseYear})`;
		}
		return { title, value: movie };
	})
);
const movieSelection = ref<Movie | undefined>();
watch(
	() => mediaType.value,
	() => {
		if (mediaType.value !== SearchType.Movie) movieSelection.value = undefined;
	}
);

const tvShowsList = ref<TvShow[] | undefined>();
watch(
	() => mediaType.value,
	async () => {
		tvShowsList.value = undefined;
		if (mediaType.value !== SearchType.TvSeries) return;

		let { response } = await reportErrors(
			rpc.listTvShows({}),
			"Failed to list TV shows"
		);
		tvShowsList.value = response.tvShows;
	}
);
const tvShowsListSelections = computed(() =>
	tvShowsList.value?.map((tvShow) => {
		let title = tvShow.title;
		if (tvShow.originalReleaseYear !== undefined) {
			title += ` (${tvShow.originalReleaseYear})`;
		}
		return { title, value: tvShow };
	})
);
const tvShowSelection = ref<TvShow | undefined>();
watch(
	() => mediaType.value,
	() => {
		if (mediaType.value !== SearchType.TvSeries)
			tvShowSelection.value = undefined;
	}
);

const tvSeasonsList = ref<TvSeason[] | undefined>();
watch(
	() => tvShowSelection.value,
	async () => {
		tvSeasonsList.value = undefined;
		if (tvShowSelection.value === undefined) return;
		let { response } = await reportErrors(
			rpc.listTvSeasons({
				seriesId: tvShowSelection.value.id,
			}),
			"Failed to list TV seasons"
		);
		tvSeasonsList.value = response.tvSeasons;
	}
);
const tvSeasonsListSelections = computed(() =>
	tvSeasonsList.value?.map((tvSeason) => ({
		title: tvSeason.title,
		value: tvSeason,
	}))
);
const tvSeasonSelection = ref<TvSeason[] | undefined>();
watch([() => tvShowSelection.value], () => {
	if (tvShowSelection.value === undefined) tvSeasonSelection.value = undefined;
});

const tvSeasonsById = computed(() => {
	if (tvSeasonsList.value === undefined) return undefined;
	const seasons = new Map<bigint, TvSeason>();
	for (const tvSeason of tvSeasonsList.value) {
		seasons.set(tvSeason.id, tvSeason);
	}
	return seasons;
});

const tvEpisodesLoading = ref(false);
const tvEpisodesList = ref<TvEpisode[] | undefined>();
watch(
	() => tvSeasonSelection.value,
	async () => {
		tvEpisodesLoading.value = true;
		if (tvSeasonSelection.value === undefined) return;

		const foundSeasons = new Set<bigint>();
		const newEpisodesList: TvEpisode[] =
			tvEpisodesList.value?.filter((episode) => {
				foundSeasons.add(episode.tvSeasonId);
				return tvSeasonSelection.value?.some(
					(season) => season.id === episode.tvSeasonId
				);
			}) || [];

		tvSeasonSelection.value.sort((a, b) => a.seasonNumber - b.seasonNumber);
		for (const season of tvSeasonSelection.value) {
			if (foundSeasons.has(season.id)) continue;
			let { response } = await reportErrors(
				rpc.listTvEpisodes({
					tvSeasonId: season.id,
				}),
				"Failed to list TV episodes"
			);
			newEpisodesList.push(...response.tvEpisodes);
		}

		newEpisodesList.sort(
			(a, b) =>
				(tvSeasonsById.value?.get(a.tvSeasonId)?.seasonNumber || 0) -
				(tvSeasonsById.value?.get(b.tvSeasonId)?.seasonNumber || 0)
		);
		tvEpisodesList.value = newEpisodesList;
		tvEpisodesLoading.value = false;
	}
);
const tvEpisodesListSelections = computed(() =>
	tvEpisodesList.value?.map((tvEpisode) => {
		const seasonNumber = tvSeasonsById.value?.get(
			tvEpisode.tvSeasonId
		)?.seasonNumber;
		return {
			title: `S${seasonNumber === undefined ? "?" : seasonNumber}E${tvEpisode.episodeNumber} - ${tvEpisode.title}`,
			value: tvEpisode,
		};
	})
);
const tvEpisodeSelection = ref<TvEpisode[] | undefined>();

const isValid = computed(() => {
	switch (mediaType.value) {
		case SearchType.Unspecified:
			return false;
		case SearchType.Movie:
			return movieSelection.value !== undefined;
		case SearchType.TvSeries:
			return (
				tvEpisodeSelection.value !== undefined &&
				tvEpisodeSelection.value.length > 0
			);
	}
});

function cancel() {
	mediaType.value = SearchType.Unspecified;
	movieSelection.value = undefined;
	tvShowSelection.value = undefined;
	tvSeasonSelection.value = undefined;
	tvEpisodeSelection.value = undefined;
	emit("cancel");
}
watch(
	[() => movieSelection.value, () => tvEpisodeSelection.value],
	() => submit("input"),
	{ deep: true }
);
function submit(event: "submit" | "input") {
	if (!isValid.value) return;
	switch (mediaType.value) {
		case SearchType.Movie:
			emit(event as any, {
				type: SearchType.Movie,
				movie: movieSelection.value!,
			});
			break;
		case SearchType.TvSeries:
			emit(event as any, {
				type: SearchType.TvSeries,
				episodes: tvEpisodeSelection.value!,
			});
			break;
	}
}
</script>

<template>
	<v-card>
		<v-card-title> Select Match </v-card-title>

		<v-card-text>
			<v-row>
				<v-col cols="6">
					<v-select
						label="Media Type"
						:items="[
							{ title: 'Movies', value: SearchType.Movie },
							{ title: 'TV Series', value: SearchType.TvSeries },
						]"
						:modelValue="mediaType || null"
						@update:modelValue="mediaType = $event"
					/>
				</v-col>
				<v-col cols="6">
					<div class="flex row">
						<v-autocomplete
							v-if="mediaType === SearchType.Movie"
							label="Movie"
							:loading="moviesList === undefined"
							:items="moviesListSelections"
							v-model:search="preImportQuery"
							v-model="movieSelection"
						>
							<template v-slot:no-data />
							<template v-slot:append-item>
								<v-list-item
									@click="
										importDialog = true;
										importQuery = preImportQuery;
										if (importQuery !== '') {
											nextTick(importer?.submit);
										}
									"
								>
									+ Import from TMDB
								</v-list-item>
							</template>
						</v-autocomplete>
						<v-autocomplete
							v-if="mediaType === SearchType.TvSeries"
							label="TV Show"
							:loading="tvShowsList === undefined"
							:items="tvShowsListSelections"
							v-model:search="preImportQuery"
							v-model="tvShowSelection"
						>
							<template v-slot:no-data />
							<template v-slot:append-item>
								<v-list-item
									@click="
										importDialog = true;
										importQuery = preImportQuery;
										if (importQuery !== '') {
											nextTick(importer?.submit);
										}
									"
								>
									+ Import from TMDB
								</v-list-item>
							</template>
						</v-autocomplete>
					</div>
				</v-col>
			</v-row>
			<v-row v-if="mediaType === SearchType.TvSeries">
				<v-col cols="6">
					<v-autocomplete
						v-if="tvShowSelection !== undefined && props.multipleEpisodes"
						label="Seasons"
						multiple
						chips
						:loading="tvSeasonsList === undefined"
						:items="tvSeasonsListSelections"
						v-model="tvSeasonSelection"
					/>
					<v-autocomplete
						v-if="tvShowSelection !== undefined && !props.multipleEpisodes"
						label="Season"
						:loading="tvSeasonsList === undefined"
						:items="tvSeasonsListSelections"
						:modelValue="
							tvSeasonSelection === undefined ? undefined : tvSeasonSelection[0]
						"
						@update:modelValue="tvSeasonSelection = [$event]"
					/>
				</v-col>
				<v-col cols="6">
					<v-autocomplete
						v-if="
							tvSeasonSelection !== undefined &&
							tvSeasonSelection.length > 0 &&
							props.multipleEpisodes
						"
						label="Episodes"
						multiple
						chips
						:loading="tvEpisodesLoading"
						:items="tvEpisodesListSelections"
						v-model="tvEpisodeSelection"
					/>
					<v-autocomplete
						v-if="
							tvSeasonSelection !== undefined &&
							tvSeasonSelection.length > 0 &&
							!props.multipleEpisodes
						"
						label="Episodes"
						:loading="tvEpisodesLoading"
						:items="tvEpisodesListSelections"
						:modelValue="
							tvEpisodeSelection === undefined
								? undefined
								: tvEpisodeSelection[0]
						"
						@update:modelValue="tvEpisodeSelection = [$event]"
					/>
				</v-col>
			</v-row>
		</v-card-text>

		<v-card-actions>
			<v-btn color="red" @click="cancel()">Cancel</v-btn>
			<v-spacer />
			<v-btn color="green" @click="submit('submit')" :disabled="!isValid">
				Submit
			</v-btn>
		</v-card-actions>
	</v-card>
	<v-dialog v-model="importDialog" eager>
		<MetadataImport
			ref="importer"
			close-btn
			@close="importDialog = false"
			@dataImported="useImport($event)"
			:searchType="mediaType"
			:query="importQuery"
		/>
	</v-dialog>
</template>
