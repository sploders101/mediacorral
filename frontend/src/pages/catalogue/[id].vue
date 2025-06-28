<script lang="ts" setup>
import type {
	GetJobCatalogueInfoResponse,
	Movie,
	RipJob,
	TvEpisode,
	TvSeason,
	TvShow,
	VideoFile,
} from "@/generated/mediacorral/server/v1/api";
import { injectKeys } from "@/scripts/config";
import { formatRuntime } from "@/scripts/utils";
import type { utf8read } from "@protobuf-ts/runtime";

interface MetaCache {
	movies: Record<string, Movie>;
	tvShows: Record<string, TvShow>;
	tvSeasons: Record<string, TvSeason>;
	tvEpisodes: Record<string, TvEpisode>;
}

const loading = ref(false);
const route = useRoute("/catalogue/[id]");
const rpc = inject(injectKeys.rpc)!;
const data = ref("");
const jobInfo = ref<RipJob | undefined>();
const catInfo = ref<GetJobCatalogueInfoResponse>();
const cache = reactive<MetaCache>({
	movies: {},
	tvShows: {},
	tvSeasons: {},
	tvEpisodes: {},
});

watch(() => route.params.id, refreshData, { immediate: true });

async function refreshData() {
	loading.value = true;
	const jobInfoResponse = await rpc.getJobInfo({ jobId: route.params.id });
	const catInfoResponse = await rpc.getJobCatalogueInfo({
		jobId: route.params.id,
	});
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
				if (cache.movies[movie.movie.id] === undefined) {
					cache.movies[movie.movie.id] = movie.movie;
				}
				break;
			case "tvEpisodes":
				const tvShows = new Set<string>();
				const tvSeasons = new Set<string>();
				for (const tmdbId of suspectedContents.tvEpisodes.episodeTmdbIds) {
					const { response: tvEpisode } = await rpc.getTvEpisodeByTmdbId({
						tmdbId: tmdbId,
					});
					if (tvEpisode.episode === undefined)
						throw new Error("TV episode missing");
					cache.tvEpisodes[tvEpisode.episode.id] = tvEpisode.episode;
					tvShows.add(tvEpisode.episode.tvShowId);
					tvSeasons.add(tvEpisode.episode.tvSeasonId);
				}
				for (const tvId of tvShows) {
					if (cache.tvShows[tvId] !== undefined) continue;
					const { response: tvShow } = await rpc.getTvShow({ showId: tvId });
					if (tvShow.tvShow === undefined) throw new Error("TV show missing");
					cache.tvShows[tvShow.tvShow.id] = tvShow.tvShow;
				}
				for (const seasonId of tvSeasons) {
					if (cache.tvSeasons[seasonId] !== undefined) continue;
					const { response: tvSeason } = await rpc.getTvSeason({ seasonId });
					if (tvSeason.tvSeason === undefined)
						throw new Error("TV season missing");
					cache.tvSeasons[tvSeason.tvSeason.id] = tvSeason.tvSeason;
				}
		}
	}
	loading.value = false;
}

const tableItems = computed(() => {
	if (jobInfo.value === undefined || catInfo.value === undefined) return [];

	return catInfo.value.videoFiles.map((videoFile) => ({
		id: videoFile.id,
		runtime:
			videoFile.length === undefined ? "?" : formatRuntime(videoFile.length),
		resolution: formatResolution(videoFile),
	}));
});

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
		case "720x480":
			return "720p";
		case "1920x1080":
			return "1080p";
		case "3840x2160":
			return "2160p";
		default:
			return formatted;
	}
}
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
					]"
					hide-default-footer
				/>
			</v-card-text>
		</v-card>
	</div>
	<pre>{{ data }}</pre>
</template>

<style lang="scss"></style>
