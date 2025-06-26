<script lang="ts" setup>
import type { TmdbAnyTitle } from "@/generated/mediacorral/common/tmdb/v1/main";
import type {
	SearchTmdbMovieResponse,
	SearchTmdbMultiResponse,
	SearchTmdbTvResponse,
} from "@/generated/mediacorral/server/v1/api";
import { injectKeys } from "@/scripts/config";

const rpc = inject(injectKeys.rpc)!;

enum SearchType {
	Unspecified = 0,
	Movie = 1,
	TvSeries = 2,
}
const searchType = ref(SearchType.Unspecified);
const query = ref("");

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
			(result) =>
				!("type" in result && !["movie", "tv"].includes(result.type))
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
			const { response: responseMulti } = await rpc.searchTmdbMulti({
				query: query.value,
			});
			results.value = [SearchType.Unspecified, responseMulti];
			break;
		case SearchType.Movie:
			const { response: responseMovie } = await rpc.searchTmdbMovie({
				query: query.value,
			});
			results.value = [SearchType.Movie, responseMovie];
			break;
		case SearchType.TvSeries:
			const { response: responseTvSeries } = await rpc.searchTmdbTv({
				query: query.value,
			});
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
						await rpc.importTmdbMovie({
							tmdbId: multiValue.id,
						});
						break;
					case "tv":
						await rpc.importTmdbTv({
							tmdbId: multiValue.id,
						});
						break;
				}
				break;
			case SearchType.Movie:
				await rpc.importTmdbMovie({ tmdbId: selectedItemDetails.value.id });
				break;
			case SearchType.TvSeries:
				await rpc.importTmdbTv({ tmdbId: selectedItemDetails.value.id });
				break;
		}
	} catch(err) {
		alert(err);
	} finally {
		importingItem.value = false;
	}
	selectedItem.value = undefined;
}
</script>

<template>
	<v-card>
		<v-card-title>Import Metadata</v-card-title>
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
