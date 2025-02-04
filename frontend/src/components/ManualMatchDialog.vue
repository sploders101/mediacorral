<script lang="ts" setup>
import { type TagFile } from "@/apiTypes";
import { BASE_URL } from "@/scripts/config";
import { useAppStore } from "@/stores/app";

const props = defineProps<{
	videoId: number;
	subtitleId: string | null;
}>();
const open = defineModel<boolean>();
const appStore = useAppStore();

onMounted(() => {
	appStore.getMovieList();
	appStore.getTvShows();
});

const tagType = ref<"Movie" | "TV" | null>(null);
const selectedMovie = ref<{ label: string; value: number } | null>(null);
const selectedTv = ref<{ label: string; value: number } | null>(null);
const selectedSeason = ref<{ label: string; value: number } | null>(null);
const selectedEpisode = ref<{ label: string; value: number } | null>(null);

function clearData() {
	tagType.value = null;
	selectedMovie.value = null;
	selectedTv.value = null;
	selectedSeason.value = null;
	selectedEpisode.value = null;
}
watch(open, () => {
	if (!open.value) {
		clearData();
	}
});

const subtitles = ref<string | null>(null);
watch(
	() => props.subtitleId,
	async () => {
		if (props.subtitleId === null) {
			subtitles.value = null;
			return;
		}
		let response = await fetch(
			`${BASE_URL}/blobs/subtitles/${props.subtitleId}`
		);
		if (response.status !== 200) throw new Error("Couldn't fetch subtitles"); // TODO: Notify user
		subtitles.value = await response.text();
	},
	{ immediate: true }
);

const moviesList = computed(() =>
	Object.values(appStore.movies).map((item) => ({
		label: item.title,
		value: item.tmdb_id,
	}))
);
const tvList = computed(() =>
	Object.values(appStore.tvShows).map((item) => ({
		label: item.title,
		value: item.id,
	}))
);
const tvSeasons = computed(() => {
	if (selectedTv.value === null) return null;
	if (!(selectedTv.value.value in appStore.tvSeasons)) return null;
	const seasons = appStore.tvSeasons[selectedTv.value.value];
	return Object.values(seasons)
		.sort((a, b) => a.season_number - b.season_number)
		.map((item) => ({
			label: item.title,
			value: item.id,
		}));
});
const tvEpisodes = computed(() => {
	if (selectedSeason.value === null) return null;

	const episodes = [];

	// This shouldn't really happen. Just double-checking
	if (!(selectedSeason.value.value in appStore.tvEpisodes)) return null;

	const episodeData = appStore.tvEpisodes[selectedSeason.value.value];
	const season_data = appStore.tvSeasonsFlat[selectedSeason.value.value];
	const season_number = season_data.season_number;
	episodes.push(
		...Object.values(episodeData).map((item) => ({
			label: `S${season_number}E${item.episode_number} - ${item.title}`,
			value: item.id,
		}))
	);

	return episodes;
});

// Fetch TV seasons if we don't have them
const loadingSeasons = ref(false);
watch(selectedTv, async () => {
	if (selectedTv.value === null) return;
	if (selectedTv.value.value in appStore.tvSeasons) return;
	try {
		loadingSeasons.value = true;
		await appStore.getTvSeasons(selectedTv.value.value);
	} finally {
		loadingSeasons.value = false;
	}
});

const loadingEpisodes = ref(false);
const triggeredSeasons = new Set<number>();
watch(selectedSeason, async () => {
	if (selectedSeason.value === null) return;
	try {
		loadingEpisodes.value = true;
		if (
			!(selectedSeason.value.value in appStore.tvEpisodes) &&
			!triggeredSeasons.has(selectedSeason.value.value)
		) {
			triggeredSeasons.add(selectedSeason.value.value);
			await appStore.getTvEpisodes(selectedSeason.value.value);
		}
	} finally {
		triggeredSeasons.clear();
		loadingEpisodes.value = false;
	}
});

function padZeros(num: number) {
	let str = num.toString();
	while (str.length < 2) {
		str = "0" + str;
	}
	return str;
}

const selectedDescription = computed(() => {
	if (selectedEpisode.value === null) return null;
	const episodeId = selectedEpisode.value.value;
	if (!(episodeId in appStore.tvEpisodesFlat)) return null;
	const episode = appStore.tvEpisodesFlat[episodeId];
	const season = appStore.tvSeasonsFlat[episode.tv_season_id];
	const seasonNumber = padZeros(season.season_number);
	const episodeNumber = padZeros(episode.episode_number);

	return {
		title: `S${seasonNumber}E${episodeNumber} - ${episode.title}`,
		description: episode.description,
	};
});

const assert = <T>(i: T) => i;

const taggingEpisode = ref(false);
async function tagEpisode() {
	if (selectedEpisode.value === null) return null;
	taggingEpisode.value = true;
	try {
		let response = await fetch(`${BASE_URL}/tagging/tag_file`, {
			method: "POST",
			headers: {
				"content-type": "application/json",
			},
			body: JSON.stringify(assert<TagFile>({
				file: props.videoId,
				video_type: "TvEpisode",
				match_id: selectedEpisode.value.value,
			})),
		});
		if (response.status !== 200) {
			throw new Error("Unable to tag file"); // TODO: Report to user
		}
	} finally {
		taggingEpisode.value = false;
	}
	location.reload();
}
</script>

<template>
	<v-dialog v-model="open" max-width="800">
		<v-card>
			<v-card-title> Manual Tagging </v-card-title>
			<v-card-text>
				<v-container>
					<v-row justify="space-evenly">
						<v-col cols="12" sm="4">
							<v-select
								label="Movie or TV?"
								:items="['Movie', 'TV']"
								variant="outlined"
								v-model="tagType"
							/>
						</v-col>
						<v-col cols="12" sm="4" v-if="tagType === 'Movie'">
							<v-combobox
								label="Movie"
								:items="moviesList"
								item-title="label"
								v-model="selectedMovie"
							/>
						</v-col>
						<v-col cols="12" sm="4" v-else-if="tagType === 'TV'">
							<v-combobox
								label="TV Show"
								:items="tvList"
								:loading="loadingSeasons"
								item-title="label"
								v-model="selectedTv"
							/>
						</v-col>
					</v-row>
					<v-row justify="space-evenly">
						<v-col
							cols="12"
							sm="4"
							v-if="tagType === 'TV' && tvSeasons !== null"
						>
							<v-combobox
								label="Season"
								:items="tvSeasons"
								item-title="label"
								:loading="loadingEpisodes"
								v-model="selectedSeason"
							/>
						</v-col>
						<v-col
							cols="12"
							sm="4"
							v-if="tagType === 'TV' && tvEpisodes !== null"
						>
							<v-combobox
								label="Episode"
								:items="tvEpisodes"
								item-title="label"
								v-model="selectedEpisode"
							/>
						</v-col>
					</v-row>
				</v-container>
				<v-divider />
				<v-sheet v-if="selectedDescription" class="ma-3">
					<h3>{{ selectedDescription.title }}</h3>
					<p>{{ selectedDescription.description }}</p>
				</v-sheet>
				<v-expansion-panels v-if="subtitles !== null">
					<v-expansion-panel title="Subtitles">
						<v-expansion-panel-text>
						<pre class="subtitle-viewer ma-3">{{ subtitles }}</pre>
						</v-expansion-panel-text>
					</v-expansion-panel>
				</v-expansion-panels>
			</v-card-text>
			<v-card-actions>
				<v-btn @click="tagEpisode()" :loading="taggingEpisode"> Confirm </v-btn>
				<v-spacer />
				<v-btn color="red" @click="open = false"> Cancel </v-btn>
			</v-card-actions>
		</v-card>
	</v-dialog>
</template>

<style lang="scss" scoped>
.subtitle-viewer {
	max-height: 60vh;
	overflow-y: scroll;
}
</style>
