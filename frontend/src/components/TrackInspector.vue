<script lang="ts" setup>
import { VideoFile } from "@/generated/mediacorral/server/v1/api";
import { injectKeys } from "@/scripts/config";
import { reportErrorsFactory } from "@/scripts/uiUtils";

const rpc = inject(injectKeys.rpc)!;
const reportErrors = reportErrorsFactory();
const props = defineProps<{
	videoFile: VideoFile;
}>();

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

const trackDetails = computed(() => {
	if (selected.value.length === 0) {
		return undefined;
	}
	const value = selected.value[0];
	if (value.id.startsWith("video-")) {
		const track =
			props.videoFile.extendedMetadata?.videoTracks[Number(value.id.slice(6))];
		if (track === undefined) return undefined;
		return {
			type: "video",
			track,
		} as const;
	} else if (value.id.startsWith("audio-")) {
		const track =
			props.videoFile.extendedMetadata?.audioTracks[Number(value.id.slice(6))];
		if (track === undefined) return undefined;
		return {
			type: "audio",
			track,
		} as const;
	} else if (value.id.startsWith("subtitles-")) {
		const track =
			props.videoFile.extendedMetadata?.subtitleTracks[
				Number(value.id.slice(10))
			];
		if (track === undefined) return undefined;
		return {
			type: "subtitles",
			track,
		} as const;
	}
});
</script>

<template>
	<v-row>
		<v-col cols="12" md="6">
			<v-treeview
				v-model:activated="selected"
				:items="items"
				active-strategy="single-leaf"
				indent-lines="default"
				item-value="id"
				return-object
				activatable
				open-all
				open-on-click
			></v-treeview>
		</v-col>

		<v-divider vertical></v-divider>

		<v-col class="pa-6" cols="12" md="6">
			<template v-if="trackDetails === undefined">No track selected.</template>

			<div v-else-if="trackDetails.type === 'video'">
				Video Track
			</div>
			<div v-else-if="trackDetails.type === 'audio'">
				Audio Track
			</div>
			<div v-else-if="trackDetails.type === 'subtitles'">
				Subtitles Track
			</div>
		</v-col>
	</v-row>
</template>
