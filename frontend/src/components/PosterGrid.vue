<script lang="ts" setup>
const props = defineProps<{
	mediaType: "movies" | "tv";
}>();
</script>

<template>
	<v-row>
		<v-col v-for="n of 15" :key="n" cols="4" sm="3" md="2" lg="1" xl="1">
			<v-tooltip location="bottom" open-delay="1000">
				<template #activator="{ props: tooltipProps }">
					<v-card
						:to="`/browse/${mediaType}/1`"
						elevation="0"
						class="bg-transparent"
						v-bind="tooltipProps"
					>
						<v-img
							src="https://www.themoviedb.org/t/p/w1280/1g0dhYtq4irTY1GPXvft6k4YLjm.jpg"
							aspect-ratio="2/3"
							class="bg-grey-lighten-2 rounded"
							cover
						>
							<template #placeholder>
								<v-row class="fill-height align-center justify-center">
									<v-progress-circular color="grey-lighten-5" indeterminate />
								</v-row>
							</template>
						</v-img>

						<v-card-text class="pa-2">
							<div class="poster-title">
								Movie Title Goes Here. This is a really obnoxiously long title
								to test the limit of the application.
							</div>
						</v-card-text>
					</v-card>
				</template>
				<span>
					Movie Title Goes Here. This is a really obnoxiously long title to test
					the limit of the application.
				</span>
			</v-tooltip>
		</v-col>
	</v-row>
</template>

<style>
.poster-title {
	font-size: 0.85rem;
	line-height: 1.2;
	text-align: center;

	/* Clamp to 2 lines like Plex */
	display: -webkit-box;
	-webkit-line-clamp: 2;
	-webkit-box-orient: vertical;

	overflow: hidden;
	text-overflow: ellipsis;
}
</style>
