<script lang="ts" setup>
import {
	DriveStatusTag,
	type RipJobStatus,
	type DriveStatus,
	type DiscDrive,
	type RipJob
} from "@/generated/mediacorral/server/v1/api_pb";
import { injectKeys } from "@/scripts/config";
import { reportErrorsFactory } from "@/scripts/uiUtils";

const rpc = inject(injectKeys.rpc)!;
const reportErrors = reportErrorsFactory();

const props = defineProps<{
	drive: DiscDrive;
	visible: boolean;
}>();
const driveStatus = ref<DriveStatus | undefined>(undefined);
const discTitle = computed(() => {
	if (driveStatus.value === null) {
		return "Loading...";
	}
	switch (driveStatus.value?.status) {
		case undefined:
			return "Disconnected";
		case DriveStatusTag.UNSPECIFIED:
			return "Unknown";
		case DriveStatusTag.EMPTY:
		case DriveStatusTag.TRAY_OPEN:
			return props.drive.name;
		case DriveStatusTag.NOT_READY:
			return "Loading...";
		case DriveStatusTag.DISC_LOADED:
			return (
				jobInfo.value?.discTitle ||
				driveStatus.value.discName ||
				props.drive.name
			);
	}
});
const currentStatus = computed(() => {
	if (driveStatus.value === null) {
		return "Fetching drive status...";
	}
	if (driveStatus.value?.ripJob !== undefined) {
		return `Ripping - Job #${driveStatus.value.ripJob.jobId}`;
	}
	switch (driveStatus.value?.status) {
		case undefined:
			return "Disconnected"
		case DriveStatusTag.UNSPECIFIED:
			return "Unknown";
		case DriveStatusTag.EMPTY:
			return "Closed - Empty";
		case DriveStatusTag.TRAY_OPEN:
			return "Tray Open";
		case DriveStatusTag.NOT_READY:
			return "Loading Disc...";
		case DriveStatusTag.DISC_LOADED:
			return "Disc loaded. Ready to rip.";
	}
});
const showTrayAction = computed(() => {
	if (driveStatus.value === undefined) return [];

	if (driveStatus.value?.ripJob !== undefined) {
		return [];
	}

	switch (driveStatus.value.status) {
		case DriveStatusTag.UNSPECIFIED:
			return [];
		case DriveStatusTag.EMPTY:
			return ["open"];
		case DriveStatusTag.DISC_LOADED:
			return ["open", "rip"];
		case DriveStatusTag.TRAY_OPEN:
			return ["close"];
		case DriveStatusTag.NOT_READY:
			return [];
	}
	return [];
});

async function openTray() {
	await reportErrors(
		rpc.eject({
			driveId: props.drive.id,
		}),
		"Failed to eject the disc"
	);
}

async function closeTray() {
	await reportErrors(
		rpc.retract({
			driveId: props.drive.id,
		}),
		"Failed to close the drive tray"
	);
}

async function ripDisc() {
	await reportErrors(
		rpc.startRipJob({
			driveId: props.drive.id,
			autoeject: false,
		}),
		"Failed to rip disc"
	);
}

let pollInterval: number | undefined = undefined;
watch(
	() => props.visible,
	() => {
		if (props.visible) {
			pollDrive();
			pollInterval = setInterval(pollDrive, 1000);
		} else {
			if (pollInterval !== undefined) clearInterval(pollInterval);
			pollInterval = undefined;
		}
	},
	{ immediate: true }
);
onBeforeUnmount(() => {
	if (pollInterval !== undefined) clearInterval(pollInterval);
	pollInterval = undefined;
});

async function pollDrive() {
	// TODO: Add error handling for this. The current implementation will
	//       get ***REALLY*** annoying if added here.
	let result = await rpc.getDriveStatus({
		driveId: props.drive.id,
	});
	driveStatus.value = result.driveStatus;
}

const jobInfo = ref<RipJob | undefined>(undefined);
const jobStatus = computed(() => driveStatus.value?.ripJob);
watch(
	() => driveStatus.value?.ripJob?.jobId,
	async (jobId) => {
		if (jobId === undefined) {
			jobInfo.value = undefined;
			return;
		}
		let result = await reportErrors(
			rpc.getJobInfo({
				jobId: jobId,
			}),
			"Failed to get info for the active job"
		);
		jobInfo.value = result.details;
	}
);

const allowRename = computed(
	() => driveStatus.value?.ripJob !== undefined
);
async function renameJob() {
	if (driveStatus.value?.ripJob === undefined) return;
	if (jobInfo.value === undefined) return;
	const newName = prompt(
		"What would you like to name the job?",
		jobInfo.value?.discTitle || ""
	);
	if (newName === null) return;
	await reportErrors(
		rpc.renameJob({
			jobId: jobInfo.value?.id,
			newName,
		}),
		"Failed to rename job"
	);
	jobInfo.value.discTitle = newName;
}
</script>

<template>
	<v-card>
		<v-card-title>
			{{ discTitle }}
			<v-btn
				v-if="allowRename"
				density="compact"
				flat
				icon="mdi-rename"
				@click="renameJob()"
			/>
		</v-card-title>
		<v-card-subtitle>Status: {{ currentStatus }}</v-card-subtitle>
		<v-card-text>
			<template v-if="jobStatus !== undefined">
				<v-label :text="`Current: ${jobStatus.cprogTitle}`" />
				<v-progress-linear
					:model-value="
						((jobStatus.cprogValue || 0) /
							(jobStatus.maxProgValue || 1)) *
						100
					"
					buffer-value="0"
					color="red"
					stream
				/>
				<v-label :text="`Total: ${jobStatus.tprogTitle}`" />
				<v-progress-linear
					:model-value="
						((jobStatus.tprogValue || 0) /
							(jobStatus.maxProgValue || 1)) *
						100
					"
					buffer-value="0"
					color="blue"
					stream
				/>
				<v-divider style="margin-top: 0.5rem; margin-bottom: 0.5rem" />
				<pre>{{ jobStatus.logs.join("\n") }}</pre>
			</template>
		</v-card-text>
		<v-card-actions v-if="showTrayAction.length > 0">
			<v-btn v-if="showTrayAction.includes('open')" @click="openTray()">
				Open Tray
			</v-btn>
			<v-btn v-if="showTrayAction.includes('close')" @click="closeTray()">
				Close Tray
			</v-btn>
			<v-btn v-if="showTrayAction.includes('rip')" @click="ripDisc()">
				Rip Disc
			</v-btn>
		</v-card-actions>
	</v-card>
</template>
