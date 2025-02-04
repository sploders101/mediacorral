<script lang="ts" setup>
import { BASE_URL } from "@/scripts/config";
import { useAppStore } from "@/stores/app";

const props = defineProps<{
	jobId: number;
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
const selectedSeasons = ref<Array<{ label: string; value: number }>>([]);
const selectedEpisodes = ref<Array<{ label: string; value: number }>>([]);

const suspecting = ref(false);
async function suspectJob() {
	if (tagType.value === "Movie") {
		// TODO: Finish movies
		throw new Error("Movies aren't finished yet");
	} else if (tagType.value === "TV") {
		suspecting.value = true;
		const tmdbIds = selectedEpisodes.value.map((item) => item.value);
		try {
			let response = await fetch(
				`${BASE_URL}/tagging/jobs/${props.jobId}/suspicion`,
				{
					method: "POST",
					body: JSON.stringify({
						type: "TvEpisodes",
						episode_tmdb_ids: tmdbIds,
					}),
				}
			);
			if (response.status !== 200) throw new Error("Couldn't suspect job"); // TODO: Report to user
			open.value = false;
		} finally {
			suspecting.value = false;
		}
	}
}
function clearData() {
	suspecting.value = false;
	tagType.value = null;
	selectedMovie.value = null;
	selectedTv.value = null;
	selectedSeasons.value = [];
	selectedEpisodes.value = [];
}
watch(open, () => {
	if (!open.value) {
		clearData();
	}
});

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
	if (selectedSeasons.value.length === 0) return null;

	const episodes = [];

	for (const season of selectedSeasons.value) {
		// This shouldn't really happen. Just double-checking
		if (!(season.value in appStore.tvEpisodes)) return null;

		const episodeData = appStore.tvEpisodes[season.value];
		const season_data = appStore.tvSeasonsFlat[season.value];
		const season_number = season_data.season_number;
		episodes.push(
			...Object.values(episodeData).map((item) => ({
				label: `S${season_number}E${item.episode_number} - ${item.title}`,
				value: item.tmdb_id,
			}))
		);
	}

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
watch(selectedSeasons, async () => {
	if (selectedSeasons.value.length === 0) return;
	try {
		loadingEpisodes.value = true;
		await Promise.all(
			selectedSeasons.value.map(async (selectedSeason) => {
				if (
					!(selectedSeason.value in appStore.tvEpisodes) &&
					!triggeredSeasons.has(selectedSeason.value)
				) {
					triggeredSeasons.add(selectedSeason.value);
					await appStore.getTvEpisodes(selectedSeason.value);
				}
			})
		);
	} finally {
		triggeredSeasons.clear();
		loadingEpisodes.value = false;
	}
});
</script>

<template>
	<v-dialog v-model="open">
		<v-container>
			<v-card>
				<v-card-title> Add Suspicion </v-card-title>
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
							<v-col cols="12" sm="4" v-if="tagType === 'TV' && tvSeasons !== null">
								<v-combobox
									label="Seasons"
									multiple
									chips
									:items="tvSeasons"
									item-title="label"
									:loading="loadingEpisodes"
									v-model="selectedSeasons"
								/>
							</v-col>
							<v-col cols="12" sm="4" v-if="tagType === 'TV' && tvEpisodes !== null">
								<v-combobox
									label="Episodes"
									multiple
									chips
									:items="tvEpisodes"
									item-title="label"
									v-model="selectedEpisodes"
								/>
							</v-col>
						</v-row>
					</v-container>
				</v-card-text>
				<v-card-actions>
					<v-btn @click="suspectJob" :loading="suspecting"> Confirm </v-btn>
					<v-spacer />
					<v-btn color="red" @click="open = false"> Cancel </v-btn>
				</v-card-actions>
			</v-card>
		</v-container>
	</v-dialog>
</template>
