<script lang="ts" setup>
import { VideoStereoMode } from "@/generated/mediacorral/analysis/v1/main";
import { VideoFile } from "@/generated/mediacorral/server/v1/api";
import { injectKeys } from "@/scripts/config";
import { reportErrorsFactory } from "@/scripts/uiUtils";
import {
	capitalize,
	resolveLanguage,
	resolveStereoMode,
	toHex,
} from "@/scripts/utils";

const rpc = inject(injectKeys.rpc)!;
const reportErrors = reportErrorsFactory();
const props = defineProps<{
	videoFile: VideoFile;
}>();

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

			<div v-else>
				<v-table striped="odd">
					<tbody>
						<tr>
							<td>Track Number</td>
							<td>{{ trackDetails.track.trackNumber }}</td>
						</tr>
						<tr>
							<td>Track UID</td>
							<td>{{ trackDetails.track.trackUid }}</td>
						</tr>
						<tr>
							<td>Hash</td>
							<td>{{ toHex(trackDetails.track.hash) }}</td>
						</tr>
						<tr>
							<td>Name</td>
							<td>{{ trackDetails.track.name || "Untitled Track" }}</td>
						</tr>
						<tr>
							<td>Type</td>
							<td>{{ capitalize(trackDetails.type) }}</td>
						</tr>
						<tr v-if="trackDetails.track.language !== undefined">
							<td>Language</td>
							<td>{{ resolveLanguage(trackDetails.track.language) }}</td>
						</tr>
						<tr>
							<td>Flags</td>
							<td>
								<v-tooltip location="top" text="Disabled">
									<template v-slot:activator="{ props }">
										<v-icon
											v-bind="props"
											v-if="!trackDetails.track.enabled"
											icon="mdi-cancel"
										/>
									</template>
								</v-tooltip>
								<v-tooltip location="top" text="Default">
									<template v-slot:activator="{ props }">
										<v-icon
											v-bind="props"
											v-if="trackDetails.track.default"
											icon="mdi-selection"
										/>
									</template>
								</v-tooltip>
								<v-tooltip location="top" text="Commentary">
									<template v-slot:activator="{ props }">
										<v-icon
											v-bind="props"
											v-if="trackDetails.track.commentary"
											icon="mdi-comment-quote"
										/>
									</template>
								</v-tooltip>
								<v-tooltip location="top" text="Visually Impaired">
									<template v-slot:activator="{ props }">
										<v-icon
											v-bind="props"
											v-if="trackDetails.track.visualImpaired"
										/>
									</template>
								</v-tooltip>
							</td>
						</tr>
						<tr v-if="trackDetails.type === 'video'">
							<td>Resolution</td>
							<td>
								{{ trackDetails.track.displayWidth }}x{{
									trackDetails.track.displayHeight
								}}
							</td>
						</tr>
						<tr v-if="trackDetails.type === 'video'">
							<td>Stereoscopy (3D) Mode</td>
							<td>{{ resolveStereoMode(trackDetails.track.stereoMode) }}</td>
						</tr>
						<tr v-if="trackDetails.type === 'audio'">
							<td>Channel Count</td>
							<td>{{ trackDetails.track.channels }}</td>
						</tr>
					</tbody>
				</v-table>
			</div>
		</v-col>
	</v-row>
</template>
