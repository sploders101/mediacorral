<script lang="ts" setup>
import { VideoFile } from "@/generated/mediacorral/server/v1/api";
import { injectKeys } from "@/scripts/config";
import { reportErrorsFactory } from "@/scripts/uiUtils";

const rpc = inject(injectKeys.rpc)!;
const reportErrors = reportErrorsFactory();
const props = defineProps<{
	videoFile: VideoFile;
}>();
onMounted(() => console.log(toRaw(props.videoFile)));

function resolveLanguage(langCode: string): string {
	switch (langCode) {
		case "eng":
			return "English";
		case "spa":
			return "Spanish";
		case "fra":
			return "French";
		case "fin":
			return "Finnish";
		case "nor":
			return "Norwegian";
		case "swe":
			return "Swedish";
		default:
			return langCode;
	}
}

interface TrackTreeLeaf {
	id: string;
	title: string;
}

const selected = shallowRef<TrackTreeLeaf[]>([]);
const items = computed(() => {
	if (props.videoFile.extendedMetadata === undefined) return [];
	return [
		{
			id: "video",
			title: "Video",
			children: props.videoFile.extendedMetadata.videoTracks.map((track, i) => {
				let segments: string[] = [];
				if (!track.enabled) {
					segments.push("Disabled");
				} else if (track.default) {
					segments.push("Default");
				}
				if (track.language) {
					segments.push(resolveLanguage(track.language));
				}
				if (track.commentary) {
					segments.push("Commentary");
				}
				const trackDetails =
					track.name === undefined
						? segments.join(" ")
						: `${track.name} - ${segments.join(" ")}`;
				return {
					id: `video-${i}`,
					title: `[${track.trackNumber}] ${trackDetails || "Main"}`,
				};
			}),
		},
		{
			id: "audio",
			title: "Audio",
			children: props.videoFile.extendedMetadata.audioTracks.map((track, i) => {
				let segments: string[] = [];
				if (!track.enabled) {
					segments.push("Disabled");
				} else if (track.default) {
					segments.push("Default");
				}
				if (track.language) {
					segments.push(resolveLanguage(track.language));
				}
				if (track.commentary) {
					segments.push("Commentary");
				}
				const trackDetails =
					track.name === undefined
						? segments.join(" ")
						: `${track.name} - ${segments.join(" ")}`;
				return {
					id: `audio-${i}`,
					title: `[${track.trackNumber}] ${trackDetails || "Main"}`,
				};
			}),
		},
		{
			id: "subtitles",
			title: "Subtitles",
			children: props.videoFile.extendedMetadata.subtitleTracks.map(
				(track, i) => {
					let segments: string[] = [];
					if (!track.enabled) {
						segments.push("Disabled");
					} else if (track.default) {
						segments.push("Default");
					}
					if (track.language) {
						segments.push(resolveLanguage(track.language));
					}
					if (track.commentary) {
						segments.push("Commentary");
					}
					const trackDetails =
						track.name === undefined
							? segments.join(" ")
							: `${track.name} - ${segments.join(" ")}`;
					return {
						id: `subtitles-${i}`,
						title: `[${track.trackNumber}] ${trackDetails}`,
					};
				}
			),
		},
	];
});
</script>

<template>
	<v-row>
		<v-col cols="12" md="6">
			<v-treeview
				v-model:selected="selected"
				:items="items"
				select-strategy="single-leaf"
				item-value="id"
				return-object
				selectable
			></v-treeview>
		</v-col>

		<v-divider vertical></v-divider>

		<v-col class="pa-6" cols="12" md="6">
			<template v-if="!selected.length">No nodes selected.</template>

			<template v-else>
				<div v-for="node in selected" :key="node.id">
					{{ node.title }}
				</div>
			</template>
		</v-col>
	</v-row>
</template>
