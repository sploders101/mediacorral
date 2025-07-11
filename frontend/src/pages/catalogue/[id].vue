<script lang="ts">
export interface ProcessedVideoItem {
	id: bigint;
	runtime: string;
	resolution: string;
	matches: MatchInfoItem[];
	suggestedMatch: TagFileRequest | undefined;
	likelyOstMatchCount: number;
	likelyOstMatch: MatchInfoItem | undefined;
	existingMatchType: VideoType;
	existingMatch: bigint | undefined;
	features: {
		subtitles: boolean;
	};
}
</script>

<script lang="ts" setup>
import ManualMatch from "@/components/ManualMatch.vue";
import type { SubmitData as MatchSubmitData } from "@/components/MatchSelector.vue";
import {
	TagFileRequest,
	VideoType,
	type GetJobCatalogueInfoResponse,
	type MatchInfoItem,
	type OstDownloadsItem,
	type RipJob,
	type VideoFile,
} from "@/generated/mediacorral/server/v1/api";
import router from "@/router";
import { SearchType, type MetaCache } from "@/scripts/commonTypes";
import { injectKeys } from "@/scripts/config";
import { formatRuntime } from "@/scripts/utils";

const prompter = inject(injectKeys.promptService)!;

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

	const movies = new Set<bigint>();
	const tvShows = new Set<bigint>();
	const tvSeasons = new Set<bigint>();

	const firstFetchers: Array<Promise<void>> = [];

	// Fetch relevant media for suspectedContents
	if (jobInfoResponse.response.details?.suspectedContents !== undefined) {
		const suspectedContents =
			jobInfoResponse.response.details.suspectedContents.suspectedContents;
		switch (suspectedContents.oneofKind) {
			case "movie":
				firstFetchers.push(
					rpc
						.getMovieByTmdbId({
							tmdbId: suspectedContents.movie.tmdbId,
						})
						.then(({ response }) => {
							if (response.movie === undefined)
								throw new Error("Movie missing");
							if (cache.movies.get(response.movie.id) === undefined) {
								cache.movies.set(response.movie.id, response.movie);
							}
						})
				);
				break;
			case "tvEpisodes":
				firstFetchers.push(
					...suspectedContents.tvEpisodes.episodeTmdbIds.map(async (tmdbId) => {
						const { response } = await rpc.getTvEpisodeByTmdbId({
							tmdbId: tmdbId,
						});
						if (response.episode === undefined)
							throw new Error("TV episode missing");
						cache.tvEpisodes.set(response.episode.id, response.episode);
						tvShows.add(response.episode.tvShowId);
						tvSeasons.add(response.episode.tvSeasonId);
					})
				);
		}
	}

	firstFetchers.push(
		...catInfo.value.videoFiles.map(async (file) => {
			switch (file.videoType) {
				case VideoType.MOVIE:
					if (file.matchId !== undefined) movies.add(file.matchId);
					break;
				case VideoType.TV_EPISODE:
					if (file.matchId !== undefined) {
						const { response } = await rpc.getTvEpisode({
							episodeId: file.matchId,
						});
						if (response.episode === undefined)
							throw new Error("TV episode missing");
						cache.tvEpisodes.set(response.episode.id, response.episode);
						tvShows.add(response.episode.tvShowId);
						tvSeasons.add(response.episode.tvSeasonId);
					}
					break;
			}
		})
	);

	// Wait for all the data we've started collecting so far.
	// Some of it will feed the next set of queries.
	await Promise.all(firstFetchers);

	// Fetch pending content
	const finalFetchers = [];
	for (const movieId of movies) {
		if (cache.movies.has(movieId)) continue;
		finalFetchers.push(
			rpc.getMovie({ movieId }).then(({ response }) => {
				if (response.movie === undefined) throw new Error("Movie missing");
				cache.movies.set(movieId, response.movie);
			})
		);
	}
	for (const showId of tvShows) {
		if (cache.tvShows.has(showId)) continue;
		finalFetchers.push(
			rpc.getTvShow({ showId }).then(({ response }) => {
				if (response.tvShow === undefined) throw new Error("TV show missing");
				cache.tvShows.set(response.tvShow.id, response.tvShow);
			})
		);
	}
	for (const seasonId of tvSeasons) {
		if (cache.tvSeasons.has(seasonId)) continue;
		finalFetchers.push(
			rpc.getTvSeason({ seasonId }).then(({ response }) => {
				if (response.tvSeason === undefined)
					throw new Error("TV season missing");
				cache.tvSeasons.set(response.tvSeason.id, response.tvSeason!);
			})
		);
	}

	// Wait for second-stage content to return
	await Promise.all(finalFetchers);

	loading.value = false;
}

const tableItems = computed<ProcessedVideoItem[]>(() => {
	if (jobInfo.value === undefined || catInfo.value === undefined) return [];

	return catInfo.value.videoFiles.map<ProcessedVideoItem>((videoFile) => {
		const runtime =
			videoFile.length === undefined ? "?" : formatRuntime(videoFile.length);

		const matches = catInfo.value!.matches.filter(
			(match) => match.videoFileId === videoFile.id
		);
		matches.sort(
			(a, b) => a.distance / a.maxDistance - b.distance / b.maxDistance
		);

		const likelyOstMatches = matches.filter(
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

		let suggestedMatch: TagFileRequest | undefined = undefined;
		if (likelyOstMatches.length === 1) {
			const ostMatch = catInfo.value?.ostSubtitleFiles.find(
				(subtitle) => subtitle.id === likelyOstMatches[0].ostDownloadId
			);
			if (ostMatch !== undefined) {
				suggestedMatch = {
					file: videoFile.id,
					videoType: ostMatch.videoType,
					matchId: ostMatch.matchId,
				};
			}
		}

		return {
			id: videoFile.id,
			runtime,
			resolution: formatResolution(videoFile),
			matches,
			currentMatch,
			suggestedMatch,
			likelyOstMatchCount: likelyOstMatches.length,
			likelyOstMatch:
				likelyOstMatches.length === 1 ? likelyOstMatches[0] : undefined,
			existingMatchType: videoFile.videoType,
			existingMatch: videoFile.matchId,
			features: {
				subtitles:
					catInfo.value?.subtitleMaps.find(
						(subtitle) => subtitle.id === videoFile.id
					)?.subtitleBlob !== undefined,
			},
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
				<div class="flex row gap-1rem">
					{{ jobInfo?.discTitle }}
					<v-btn
						density="compact"
						flat
						icon="mdi-rename"
						@click="renameJob()"
					/>
					<v-spacer />
					<v-tooltip text="Delete rip job">
						<template v-slot:activator="{ props }">
							<v-btn
								v-bind="props"
								density="compact"
								flat
								icon="mdi-delete"
								@click="
									prompter
										.prompt(
											'Delete rip job?',
											`Enter the following to delete the rip job:\n'Please delete job ${route.params.id}'`
										)
										.then((result) => {
											if (result === `Please delete job ${route.params.id}`) {
												rpc.deleteJob({
													jobId: BigInt(route.params.id),
												});
												router.push('/catalogue');
											}
										})
								"
							/>
						</template>
					</v-tooltip>
					<v-tooltip text="Delete unmatched videos">
						<template v-slot:activator="{ props }">
							<v-btn
								v-bind="props"
								density="compact"
								flat
								icon="mdi-delete-empty"
								@click="
									prompter
										.confirm(
											'Are you sure you want to delete all untagged media?',
											'Prune rip job?'
										)
										.then((result) => {
											if (result) {
												loading = true;
												rpc
													.pruneRipJob({ jobId: BigInt(route.params.id) })
													.then(() => {
														$router.push('/catalogue');
														loading = false;
													});
											}
										})
								"
							/>
						</template>
					</v-tooltip>
					<v-tooltip text="Reload Content">
						<template v-slot:activator="{ props }">
							<v-btn
								v-bind="props"
								density="compact"
								flat
								icon="mdi-reload"
								@click="refreshData()"
							/>
						</template>
					</v-tooltip>
				</div>
			</v-card-title>
			<v-card-text>
				<v-data-table
					:loading="loading"
					:items="tableItems"
					items-per-page="-1"
					:headers="[
						{ title: 'ID', value: 'id', sortable: false },
						{ title: 'Runtime', value: 'runtime', sortable: false },
						{ title: 'Resolution', value: 'resolution', sortable: false },
						{ title: 'Features', key: 'features', sortable: false },
						{
							title: 'Subtitle Match',
							key: 'likelyOstMatch',
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
					<template v-slot:item.features="{ item }">
						<template v-if="item.features.subtitles">
							<v-tooltip text="Subtitles">
								<template v-slot:activator="{ props }">
									<v-icon v-bind="props">mdi-subtitles</v-icon>
								</template>
							</v-tooltip>
						</template>
					</template>
					<template v-slot:item.actions="{ item }">
						<v-btn
							v-if="
								item.existingMatchType !== VideoType.UNSPECIFIED &&
								item.existingMatch !== undefined
							"
							flat
							@click="
								rpc
									.tagFile({
										file: item.id,
										videoType: VideoType.UNSPECIFIED,
										matchId: undefined,
									})
									.then(() => {
										let file = catInfo?.videoFiles.find(
											(video) => video.id === item.id
										);
										if (file === undefined) return;
										file.videoType = VideoType.UNSPECIFIED;
										file.matchId = undefined;
									})
							"
						>
							Unmatch
						</v-btn>
						<template v-else>
							<v-btn flat @click="manualMatchItem = item"> Match </v-btn>
							<v-tooltip
								v-if="item.suggestedMatch !== undefined"
								text="Approve Suggested Match"
							>
								<template v-slot:activator="{ props }">
									<v-btn
										flat
										icon="mdi-check"
										v-bind="props"
										@click="
											rpc.tagFile(item.suggestedMatch!).then(() => {
												let file = catInfo?.videoFiles.find(
													(video) => video.id === item.id
												);
												if (file === undefined) return;
												file.videoType = item.suggestedMatch!.videoType;
												file.matchId = item.suggestedMatch!.matchId;
											})
										"
									/>
								</template>
							</v-tooltip>
						</template>
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
