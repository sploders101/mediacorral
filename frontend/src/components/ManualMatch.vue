<script lang="ts" setup>
import {
	GetJobCatalogueInfoResponse,
	Movie,
	TvEpisode,
	VideoType,
} from "@/generated/mediacorral/server/v1/api";
import MatchSelector, { type SubmitData } from "./MatchSelector.vue";
import { SearchType } from "@/scripts/commonTypes";
import { injectKeys } from "@/scripts/config";
import type { ProcessedVideoItem } from "@/pages/catalogue/[id].vue";
import { formatRuntime } from "@/scripts/utils";

const rpc = inject(injectKeys.rpc)!;
const metaCache = inject(injectKeys.metaCache);
if (metaCache === undefined) throw new Error("metaCache not provided");
const metaCacheByTmdbId = computed(() => {
	const movies = new Map<number, Movie>();
	const tvEpisodes = new Map<number, TvEpisode>();
	for (const [_, movie] of metaCache.movies) {
		if (movie.tmdbId === undefined) continue;
		movies.set(movie.tmdbId, movie);
	}
	for (const [_, tvEpisode] of metaCache.tvEpisodes) {
		if (tvEpisode.tmdbId === undefined) continue;
		tvEpisodes.set(tvEpisode.tmdbId, tvEpisode);
	}
	return { movies, tvEpisodes };
});

const props = defineProps<{
	catInfo: GetJobCatalogueInfoResponse;
	videoFile: ProcessedVideoItem;
}>();
const emits = defineEmits<{
	cancel: [];
	submitted: [];
}>();

async function matchItem(details: SubmitData) {
	matchManually.value = false;
	switch (details.type) {
		case SearchType.Movie:
			await rpc.tagFile({
				file: props.videoFile.id,
				videoType: VideoType.MOVIE,
				matchId: details.movie.id,
			});
			emits("submitted");
			break;
		case SearchType.TvSeries:
			await rpc.tagFile({
				file: props.videoFile.id,
				videoType: VideoType.TV_EPISODE,
				matchId: details.episodes[0].id,
			});
			emits("submitted");
			break;
	}
}

const videoSubtitles = ref<string | undefined>();
watch(
	() => props.videoFile,
	async () => {
		videoSubtitles.value = undefined;
		let subtitles = props.catInfo.subtitleMaps.find(
			(map) => map.id === props.videoFile.id
		);
		if (subtitles === undefined || subtitles.subtitleBlob === undefined) {
			videoSubtitles.value = "[No subtitles found]";
			return;
		}
		const { response } = await rpc.getSubtitles({
			blobId: subtitles.subtitleBlob,
		});
		videoSubtitles.value = response.subtitles;
	},
	{ immediate: true }
);

const episodeOptions = computed(() => {
	const options: Array<{ title: string; value: bigint }> = [];
	let contents = props.catInfo.suspectedContents?.suspectedContents;
	switch (contents?.oneofKind) {
		case "movie":
			let title = "???";
			const movie = metaCacheByTmdbId.value.movies.get(contents.movie.tmdbId);
			if (movie !== undefined) {
				title = movie.title || "???";
				if (movie.releaseYear !== undefined) title += ` (${movie.releaseYear})`;
				options.push({ title, value: movie.id });
			}
			break;
		case "tvEpisodes":
			for (const tvEpisode of contents.tvEpisodes.episodeTmdbIds) {
				const episode = metaCacheByTmdbId.value.tvEpisodes.get(tvEpisode);
				if (episode === undefined) break;
				const season = metaCache.tvSeasons.get(episode.tvSeasonId);
				if (season === undefined) break;
				const title = `S${season.seasonNumber}E${episode.episodeNumber} - ${episode.title}`;
				options.push({ title, value: episode.id });
			}
			break;
	}
	return options;
});

const matchSelection = ref<bigint | undefined>();
const matchSubtitles = ref<string | undefined>();
watch(
	() => matchSelection.value,
	async () => {
		matchSubtitles.value = undefined;
		if (matchSelection.value === undefined) {
			matchSubtitles.value = "[No selection]";
		}
		let subs = props.catInfo.ostSubtitleFiles.find(
			(subtitle) => matchSelection.value === subtitle.matchId
		);
		if (subs === undefined) return;
		const { response } = await rpc.getSubtitles({ blobId: subs.blobId });
		matchSubtitles.value = response.subtitles;
	},
	{ immediate: true }
);
watch(
	[() => props.videoFile.id, props.videoFile.likelyOstMatch],
	() => {
		if (props.videoFile.likelyOstMatch === undefined) return;
		const likelyOstId = props.videoFile.likelyOstMatch.ostDownloadId;
		const likelyOst = props.catInfo.ostSubtitleFiles.find(
			(subtitle) => subtitle.id === likelyOstId
		);
		if (likelyOst === undefined) return;
		matchSelection.value = likelyOst.matchId;
	},
	{ immediate: true }
);

const matchInfo = computed(() => {
	const points: string[] = [];
	if (matchSelection.value === undefined) return "";

	points.push(`File runtime:      ${props.videoFile.runtime}`);
	const suspectedContents = props.catInfo.suspectedContents?.suspectedContents;
	switch (suspectedContents?.oneofKind) {
		case "movie":
			const movie = metaCache.movies.get(matchSelection.value);
			if (movie?.runtime !== undefined) {
				points.push(`Movie runtime:     ${formatRuntime(movie.runtime * 60)}`);
			}
			break;
		case "tvEpisodes":
			const tvEpisode = metaCache.tvEpisodes.get(matchSelection.value);
			if (tvEpisode?.runtime !== undefined) {
				points.push(
					`Episode runtime:   ${formatRuntime(tvEpisode.runtime * 60)}`
				);
			}
			break;
	}

	const ostDownload = props.catInfo.ostSubtitleFiles.find(
		(file) => file.matchId === matchSelection.value
	);
	const matchInfo = props.catInfo.matches.find(
		(match) =>
			match.videoFileId === props.videoFile.id &&
			match.ostDownloadId === ostDownload?.id
	);
	if (matchInfo !== undefined) {
		points.push(
			`Subtitle Distance: ${matchInfo.distance}/${matchInfo.maxDistance} (${100 - Math.round((matchInfo.distance / matchInfo.maxDistance) * 1000) / 10}% match)`
		);
	} else {
		points.push("Match info not found.");
	}

	return points.join("\n");
});

async function selectMatch() {
	if (matchSelection.value === undefined) return;
	switch (props.catInfo.suspectedContents?.suspectedContents.oneofKind) {
		case "movie":
			await rpc.tagFile({
				file: props.videoFile.id,
				matchId: matchSelection.value,
				videoType: VideoType.MOVIE,
			});
			emits("submitted");
			break;
		case "tvEpisodes":
			await rpc.tagFile({
				file: props.videoFile.id,
				matchId: matchSelection.value,
				videoType: VideoType.TV_EPISODE,
			});
			emits("submitted");
			break;
	}
}

const matchManually = ref(false);
</script>

<template>
	<v-card>
		<v-card-title> Find Match </v-card-title>
		<v-card-text class="of-auto">
			<v-row>
				<v-col cols="6">
					<v-select
						label="Suspected Items"
						variant="outlined"
						hide-details
						:items="episodeOptions"
						v-model="matchSelection"
					>
						<template v-slot:no-data />
						<template v-slot:append-item>
							<v-list-item @click="matchManually = true"> Other </v-list-item>
						</template>
					</v-select>
				</v-col>
				<v-col cols="6">
					<pre>{{ matchInfo }}</pre>
				</v-col>
			</v-row>
			<v-row>
				<v-col cols="6">
					<div class="text-h6 ma-1">Original Subtitles</div>
				</v-col>
				<v-col cols="6">
					<div class="text-h6 ma-1">OST Subtitles</div>
				</v-col>
			</v-row>
			<v-row>
				<v-col cols="6">
					<v-sheet
						v-if="videoSubtitles !== undefined"
						color="#101010"
						class="pre-wrap pa-2"
						elevation="3"
						rounded="lg"
						>{{ videoSubtitles }}</v-sheet
					>
					<v-skeleton-loader v-else type="paragraph" />
				</v-col>
				<v-col cols="6">
					<v-sheet
						v-if="matchSubtitles !== undefined"
						color="#101010"
						class="pre-wrap pa-2"
						elevation="3"
						rounded="lg"
						>{{ matchSubtitles }}</v-sheet
					>
					<v-skeleton-loader v-else type="paragraph" />
				</v-col>
			</v-row>
		</v-card-text>
		<v-divider />
		<v-card-actions>
			<v-btn color="red" @click="emits('cancel')">Cancel</v-btn>
			<v-spacer />
			<v-btn color="green" @click="selectMatch">Confirm</v-btn>
		</v-card-actions>
	</v-card>
	<v-dialog v-model="matchManually">
		<MatchSelector @cancel="matchManually = false" @submit="matchItem" />
	</v-dialog>
</template>
