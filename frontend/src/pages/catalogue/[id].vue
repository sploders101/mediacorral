<script lang="ts">
export interface ProcessedVideoItem {
	id: bigint;
	runtime: string;
	resolution: string;
	matches: MatchInfoItem[];
	likelyOstMatchCount: number;
	likelyOstMatch: MatchInfoItem | undefined;
}
</script>

<script lang="ts" setup>
import ManualMatch from "@/components/ManualMatch.vue";
import type { SubmitData as MatchSubmitData } from "@/components/MatchSelector.vue";
import {
	VideoType,
	type GetJobCatalogueInfoResponse,
	type MatchInfoItem,
	type Movie,
	type OstDownloadsItem,
	type RipJob,
	type TvEpisode,
	type TvSeason,
	type TvShow,
	type VideoFile,
} from "@/generated/mediacorral/server/v1/api";
import { SearchType, type MetaCache } from "@/scripts/commonTypes";
import { injectKeys } from "@/scripts/config";
import { formatRuntime } from "@/scripts/utils";
import type { RefSymbol } from "@vue/reactivity";

const matchThreshold = ref(75);

const loading = ref(false);
const route = useRoute("/catalogue/[id]");
const rpc = inject(injectKeys.rpc)!;
const jobInfo = ref<RipJob | undefined>();
const catInfo = ref<GetJobCatalogueInfoResponse>();
const cache = reactive<MetaCache>({
	movies: new Map(),
	tvShows: new Map(),
	tvSeasons: new Map(),
	tvEpisodes: new Map(),
});
provide(injectKeys.metaCache, cache);
const ostFilesCache = computed(() => {
	const ostFiles = new Map<bigint, OstDownloadsItem>();
	if (catInfo.value === undefined) return ostFiles;
	for (const file of catInfo.value.ostSubtitleFiles) {
		ostFiles.set(file.id, file);
	}
	return ostFiles;
});

watch(() => route.params.id, refreshData, { immediate: true });

async function refreshData() {
	loading.value = true;
	let jobId = BigInt(route.params.id);
	const jobInfoResponse = await rpc.getJobInfo({ jobId });
	const catInfoResponse = await rpc.getJobCatalogueInfo({ jobId });
	jobInfo.value = jobInfoResponse.response.details;
	catInfo.value = catInfoResponse.response;
	console.log(jobInfoResponse.response.details, catInfoResponse.response);

	// Fetch relevant media
	if (jobInfoResponse.response.details?.suspectedContents !== undefined) {
		const suspectedContents =
			jobInfoResponse.response.details.suspectedContents.suspectedContents;
		switch (suspectedContents.oneofKind) {
			case "movie":
				const { response: movie } = await rpc.getMovieByTmdbId({
					tmdbId: suspectedContents.movie.tmdbId,
				});
				if (movie.movie === undefined) throw new Error("Movie missing");
				if (cache.movies.get(movie.movie.id) === undefined) {
					cache.movies.set(movie.movie.id, movie.movie);
				}
				break;
			case "tvEpisodes":
				const tvShows = new Set<bigint>();
				const tvSeasons = new Set<bigint>();
				for (const tmdbId of suspectedContents.tvEpisodes.episodeTmdbIds) {
					const { response: tvEpisode } = await rpc.getTvEpisodeByTmdbId({
						tmdbId: tmdbId,
					});
					if (tvEpisode.episode === undefined)
						throw new Error("TV episode missing");
					cache.tvEpisodes.set(tvEpisode.episode.id, tvEpisode.episode);
					tvShows.add(tvEpisode.episode.tvShowId);
					tvSeasons.add(tvEpisode.episode.tvSeasonId);
				}
				for (const tvId of tvShows) {
					if (cache.tvShows.has(tvId)) continue;
					const { response: tvShow } = await rpc.getTvShow({ showId: tvId });
					if (tvShow.tvShow === undefined) throw new Error("TV show missing");
					cache.tvShows.set(tvShow.tvShow.id, tvShow.tvShow);
				}
				for (const seasonId of tvSeasons) {
					if (cache.tvSeasons.has(seasonId)) continue;
					const { response: tvSeason } = await rpc.getTvSeason({ seasonId });
					if (tvSeason.tvSeason === undefined)
						throw new Error("TV season missing");
					cache.tvSeasons.set(tvSeason.tvSeason.id, tvSeason.tvSeason);
				}
		}
	}
	loading.value = false;
}

const tableItems = computed<ProcessedVideoItem[]>(() => {
	if (jobInfo.value === undefined || catInfo.value === undefined) return [];

	return catInfo.value.videoFiles.map((videoFile) => {
		const runtime =
			videoFile.length === undefined ? "?" : formatRuntime(videoFile.length);

		const matches = catInfo.value!.matches.filter(
			(match) => match.videoFileId === videoFile.id
		);
		matches.sort(
			(a, b) => a.distance / a.maxDistance - b.distance / b.maxDistance
		);

		const likelyMatches = matches.filter(
			(match) =>
				match.distance / match.maxDistance < 1 - matchThreshold.value / 100
		);

		let currentMatch = "";
		switch (videoFile.videoType) {
			case VideoType.MOVIE:
				const movie = cache.movies.get(videoFile.matchId!);
				if (movie === undefined) break;
				currentMatch = movie.title;
				if (movie.releaseYear !== undefined)
					currentMatch += ` (${movie.releaseYear})`;
				break;
			case VideoType.TV_EPISODE:
				const tvEpisode = cache.tvEpisodes.get(videoFile.matchId!);
				if (tvEpisode === undefined) break;
				const tvSeason = cache.tvSeasons.get(tvEpisode.tvSeasonId);
				if (tvSeason === undefined) break;
				currentMatch = `S${tvSeason.seasonNumber}E${tvEpisode.episodeNumber} - ${tvEpisode.title}`;
				break;
		}

		return {
			id: videoFile.id,
			runtime,
			resolution: formatResolution(videoFile),
			matches,
			currentMatch,
			likelyOstMatchCount: likelyMatches.length,
			likelyOstMatch: likelyMatches.length === 1 ? likelyMatches[0] : undefined,
		};
	});
});

function formatMatch(matchCount: number, match: MatchInfoItem | undefined) {
	if (matchCount > 1) return "Multiple matches";
	if (match === undefined) return "";
	const similarity =
		Math.round((1 - match.distance / match.maxDistance) * 1000) / 10;
	const ostFile = ostFilesCache.value.get(match.ostDownloadId);
	if (ostFile === undefined) return "???";
	switch (ostFile.videoType) {
		case VideoType.MOVIE:
			const movie = cache.movies.get(ostFile.matchId);
			if (movie === undefined) return "???";
			return movie.title;
		case VideoType.TV_EPISODE:
			const tvEpisode = cache.tvEpisodes.get(ostFile.matchId);
			if (tvEpisode === undefined) return "???";
			const season = cache.tvSeasons.get(tvEpisode.tvSeasonId);
			if (season === undefined) return "???";
			return `(${similarity}%) [S${season.seasonNumber}E${tvEpisode.episodeNumber}] ${tvEpisode.title}`;
			break;
	}
	return "???";
}

async function renameJob() {
	if (jobInfo.value === undefined) return;
	const newName = prompt("New name:", jobInfo.value.discTitle);
	if (newName === null) return;
	loading.value = true;
	await rpc.renameJob({
		jobId: jobInfo.value.id,
		newName,
	});
	const { response } = await rpc.getJobInfo({ jobId: jobInfo.value.id });
	jobInfo.value = response.details;
	loading.value = false;
}

function formatResolution(file: VideoFile) {
	const formatted = `${file.resolutionWidth}x${file.resolutionHeight}`;
	switch (formatted) {
		case "853x480":
			return "480p";
		case "1280x720":
			return "720p";
		case "1920x1080":
			return "1080p";
		case "3840x2160":
			return "2160p";
		default:
			return formatted;
	}
}

const suspectingContents = ref(false);
async function suspectContents(data: MatchSubmitData) {
	if (jobInfo.value === undefined) return;
	suspectingContents.value = false;
	loading.value = true;
	switch (data.type) {
		case SearchType.Movie:
			if (data.movie.tmdbId === undefined)
				throw new Error("Cannot suspect movie that was created manually");
			await rpc.suspectJob({
				jobId: jobInfo.value.id,
				suspicion: {
					suspectedContents: {
						oneofKind: "movie",
						movie: {
							tmdbId: data.movie.tmdbId,
						},
					},
				},
			});
			break;
		case SearchType.TvSeries:
			if (data.episodes.some((episode) => episode.tmdbId === undefined))
				throw new Error("Cannot suspect tv show that was created manually");
			await rpc.suspectJob({
				jobId: jobInfo.value.id,
				suspicion: {
					suspectedContents: {
						oneofKind: "tvEpisodes",
						tvEpisodes: {
							episodeTmdbIds: data.episodes.map((episode) => episode.tmdbId!),
						},
					},
				},
			});
	}
	await refreshData();
	loading.value = false;
}

const manualMatchItem = ref<ProcessedVideoItem | undefined>();
</script>

<template>
	<div class="padding-small">
		<v-card>
			<v-card-title>
				<div class="flex row">
					{{ jobInfo?.discTitle }}
					<v-btn
						density="compact"
						flat
						icon="mdi-rename"
						@click="renameJob()"
					/>
					<v-spacer />
					<v-btn
						density="compact"
						flat
						icon="mdi-reload"
						@click="refreshData()"
					/>
				</div>
			</v-card-title>
			<v-card-text>
				<v-data-table
					:loading="loading"
					:items="tableItems"
					:headers="[
						{ title: 'ID', value: 'id', sortable: false },
						{ title: 'Runtime', value: 'runtime', sortable: false },
						{ title: 'Resolution', value: 'resolution', sortable: false },
						{
							title: 'Likely Match',
							key: 'likelyMatch',
							value: (item) =>
								formatMatch(item.likelyOstMatchCount, item.likelyOstMatch),
							sortable: false,
						},
						{
							title: 'Current Match',
							key: 'currentMatch',
							sortable: false,
						},
						{ title: 'Actions', key: 'actions', sortable: false },
					]"
					hide-default-footer
				>
					<template v-slot:item.actions="{ item }">
						<v-btn @click="manualMatchItem = item" flat>Match</v-btn>
					</template>
				</v-data-table>
			</v-card-text>
			<v-card-actions>
				<v-btn :disabled="loading" @click="suspectingContents = true">
					Add suspicion
				</v-btn>
				<v-spacer />
				<v-number-input
					:reverse="false"
					density="compact"
					controlVariant="split"
					label="Match Threshold"
					inset
					variant="solo-filled"
					:min="0"
					:max="100"
					max-width="13rem"
					hide-details
					v-model="matchThreshold"
				/>
			</v-card-actions>
		</v-card>
	</div>
	<v-dialog v-model="suspectingContents">
		<MatchSelector
			multiple-episodes
			@cancel="suspectingContents = false"
			@submit="suspectContents"
		/>
	</v-dialog>
	<v-dialog
		:modelValue="!!manualMatchItem"
		@update:modelValue="if ($event === false) manualMatchItem = undefined;"
	>
		<ManualMatch
			v-if="manualMatchItem && catInfo"
			@cancel="manualMatchItem = undefined"
			@submitted="
				manualMatchItem = undefined;
				refreshData();
			"
			:catInfo="catInfo"
			:videoFile="manualMatchItem"
		/>
		<v-skeleton-loader v-else type="card" />
	</v-dialog>
</template>

<style lang="scss"></style>
