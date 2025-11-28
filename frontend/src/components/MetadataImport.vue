<script lang="ts" setup>
import type { TmdbAnyTitle } from "@/generated/mediacorral/server/v1/tmdb";
import type {
	SearchTmdbMovieResponse,
	SearchTmdbMultiResponse,
	SearchTmdbTvResponse,
} from "@/generated/mediacorral/server/v1/api";
import { SearchType } from "@/scripts/commonTypes";
import { injectKeys } from "@/scripts/config";
import { reportErrorsFactory } from "@/scripts/uiUtils";

const rpc = inject(injectKeys.rpc)!;
const reportErrors = reportErrorsFactory();

const props = defineProps<{
	searchType?: SearchType;
	query?: string;
	closeBtn?: boolean;
}>();
const searchType = ref(SearchType.Unspecified);
const query = ref("");

// Prefill options from props
watch(
	[() => props.searchType, () => props.query],
	() => {
		if (props.searchType != undefined) searchType.value = props.searchType;
		if (props.query !== undefined) query.value = props.query;
	},
	{ immediate: true }
);

const emit = defineEmits<{
	dataImported: [
		{ type: "tv" | "movie"; tmdb_id: number; internal_id: bigint },
	];
	close: [];
}>();

const results = ref<
	| [SearchType.Unspecified, SearchTmdbMultiResponse]
	| [SearchType.Movie, SearchTmdbMovieResponse]
	| [SearchType.TvSeries, SearchTmdbTvResponse]
	| undefined
>();
const resultsMapped = computed(() => {
	if (results.value === undefined) return undefined;
	return results.value[1].results
		.filter(
			(result) => !("type" in result && !["movie", "tv"].includes(result.type))
		)
		.map((result) => ({
			value: result.id,
			prependAvatar: getImageUrl(result.posterPath),
			title: result.title,
			subtitle: result.overview,
		}));
});

async function submit() {
	switch (searchType.value) {
		case SearchType.Unspecified:
			const { response: responseMulti } = await reportErrors(
				rpc.searchTmdbMulti({
					query: query.value,
					page: 1,
				}),
				"Error searching TMDB"
			);
			results.value = [SearchType.Unspecified, responseMulti];
			break;
		case SearchType.Movie:
			const { response: responseMovie } = await reportErrors(
				rpc.searchTmdbMovie({
					query: query.value,
					page: 1,
				}),
				"Error searching TMDB"
			);
			results.value = [SearchType.Movie, responseMovie];
			break;
		case SearchType.TvSeries:
			const { response: responseTvSeries } = await reportErrors(
				rpc.searchTmdbTv({
					query: query.value,
					page: 1,
				}),
				"Error searching TMDB"
			);
			results.value = [SearchType.TvSeries, responseTvSeries];
			break;
	}
}

function getImageUrl(path: string | undefined) {
	if (path) return `https://image.tmdb.org/t/p/w500${path}`;
	else return undefined;
}

const selectedItem = ref<number>();
const selectedItemDetails = computed(() => {
	if (results.value === undefined) return undefined;
	return results.value[1].results.find(
		(item) => item.id === selectedItem.value
	);
});
function selectItem(item?: { id: number }) {
	selectedItem.value = item?.id;
}

const importingItem = ref(false);
async function importItem() {
	if (importingItem.value) return;
	if (selectedItemDetails.value === undefined) return;
	if (results.value === undefined) return;
	importingItem.value = true;
	try {
		switch (results.value[0]) {
			case SearchType.Unspecified:
				const multiValue = selectedItemDetails.value as TmdbAnyTitle;
				switch (multiValue.type) {
					case "movie":
						let {
							response: { movieId },
						} = await reportErrors(
							rpc.importTmdbMovie({
								tmdbId: multiValue.id,
							}),
							"Error importing movie"
						);
						emit("dataImported", {
							type: "movie",
							tmdb_id: multiValue.id,
							internal_id: movieId,
						});
						break;
					case "tv":
						let {
							response: { tvId },
						} = await reportErrors(
							rpc.importTmdbTv({
								tmdbId: multiValue.id,
							}),
							"Error importing TV series"
						);
						emit("dataImported", {
							type: "tv",
							tmdb_id: multiValue.id,
							internal_id: tvId,
						});
						break;
				}
				break;
			case SearchType.Movie:
				let {
					response: { movieId },
				} = await reportErrors(
					rpc.importTmdbMovie({ tmdbId: selectedItemDetails.value.id }),
					"Error importing movie"
				);
				emit("dataImported", {
					type: "movie",
					tmdb_id: selectedItemDetails.value.id,
					internal_id: movieId,
				});
				break;
			case SearchType.TvSeries:
				let {
					response: { tvId },
				} = await reportErrors(
					rpc.importTmdbTv({ tmdbId: selectedItemDetails.value.id }),
					"Error importing TV series"
				);
				emit("dataImported", {
					type: "tv",
					tmdb_id: selectedItemDetails.value.id,
					internal_id: tvId,
				});
				break;
		}
	} finally {
		importingItem.value = false;
	}
	selectedItem.value = undefined;
}

defineExpose({
	submit,
});
</script>

<template>
	<v-card>
		<v-card-title>
			<div class="flex row">
				Import Metadata
				<v-spacer />
				<v-btn
					v-if="props.closeBtn === true"
					density="compact"
					flat
					icon="mdi-close"
					@click="emit('close')"
				/>
			</div>
		</v-card-title>
		<v-card-text>
			<v-form @submit.prevent="submit()">
				<v-row no-gutters>
					<v-col class="pa-2">
						<v-select
							label="Content Type"
							:items="[
								{ title: 'Any', value: SearchType.Unspecified },
								{ title: 'Movies', value: SearchType.Movie },
								{ title: 'TV Series', value: SearchType.TvSeries },
							]"
							variant="outlined"
							v-model="searchType"
						/>
					</v-col>
					<v-col class="pa-2">
						<v-text-field label="Title" variant="outlined" v-model="query" />
					</v-col>
				</v-row>
				<v-row>
					<v-spacer />
					<v-col cols="auto">
						<v-btn type="submit" prepend-icon="mdi-magnify" variant="outlined">
							Search
						</v-btn>
					</v-col>
				</v-row>
				<v-list
					@click:select="selectItem($event as any)"
					:items="resultsMapped"
					lines="three"
					item-props
				></v-list>
			</v-form>
		</v-card-text>
	</v-card>

	<v-dialog
		:modelValue="!!selectedItem"
		@update:modelValue="selectedItem = undefined"
	>
		<v-card>
			<v-card-title>{{ selectedItemDetails?.title }}</v-card-title>
			<v-card-text>
				<v-img
					class="meta-import-poster"
					contain
					:src="getImageUrl(selectedItemDetails?.posterPath)"
					:height="350"
				/>
				<p>
					{{ selectedItemDetails?.overview }}
				</p>
			</v-card-text>
			<v-card-actions>
				<v-btn @click="selectItem()" color="red">Cancel</v-btn>
				<v-spacer />
				<v-btn @click="importItem()" color="green" :loading="importingItem">
					Import
				</v-btn>
			</v-card-actions>
		</v-card>
	</v-dialog>
</template>

<style lang="scss">
.meta-import-poster {
	margin-bottom: 1rem;
}
</style>
