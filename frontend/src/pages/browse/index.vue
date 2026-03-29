<script lang="ts" setup>

const contentType = ref("movies");

</script>

<template>
	<v-tabs v-model="contentType" align-tabs="center">
		<v-tab value="movies">Movies</v-tab>
		<v-tab value="tv">TV Shows</v-tab>
	</v-tabs>
	<v-tabs-window v-model="contentType">
		<v-tabs-window-item value="movies">
			<v-container fluid>
				<MovieBrowser :visible="contentType === 'movies'" />
			</v-container>
		</v-tabs-window-item>
		<v-tabs-window-item value="tv">
			<v-container fluid>
				<TvBrowser :visible="contentType === 'movies'" />
			</v-container>
		</v-tabs-window-item>
	</v-tabs-window>
</template
