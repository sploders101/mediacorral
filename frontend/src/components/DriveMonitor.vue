<script lang="ts" setup>
import {
	DriveStatusTag,
	JobStatus,
	RipStatus,
	type DriveState,
} from "@/generated/mediacorral/drive_controller/v1/main";
import type { DiscDrive, RipJob } from "@/generated/mediacorral/server/v1/api";
import { injectKeys } from "@/scripts/config";
import { RpcError } from "@protobuf-ts/runtime-rpc";

const rpc = inject(injectKeys.rpc)!;

const props = defineProps<{
	drive: DiscDrive;
	visible: boolean;
}>();
const driveStatus = ref<DriveState | null>(null);
const discTitle = computed(() => {
	if (driveStatus.value === null) {
		return "Loading...";
	}
	switch (driveStatus.value.status) {
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
	if (driveStatus.value.activeRipJob !== undefined) {
		return `Ripping - Job #${driveStatus.value.activeRipJob}`;
	}
	switch (driveStatus.value.status) {
		case DriveStatusTag.UNSPECIFIED:
			return "Unknown3";
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
	if (driveStatus.value === null) return [];

	if (driveStatus.value.activeRipJob !== undefined) {
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
});

async function openTray() {
	await rpc.eject({
		drive: props.drive,
	});
}

async function closeTray() {
	await rpc.retract({ drive: props.drive });
}

async function ripDisc() {
	await rpc.startRipJob({
		drive: props.drive,
		autoeject: false,
	});
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
	let result = await rpc.getDriveState({
		controllerId: props.drive.controller,
		driveId: props.drive.driveId,
	});
	driveStatus.value = result.response;
}

const jobInfo = ref<RipJob | undefined>(undefined);
const jobStatus = ref<RipStatus | undefined>(undefined);
watch(
	() => driveStatus.value?.activeRipJob,
	async (jobId) => {
		if (jobId === undefined) {
			jobInfo.value = undefined;
			return;
		}
		let result = await rpc.getJobInfo({
			jobId: jobId,
		});
		jobInfo.value = result.response.details;
	}
);

let jobTrackerAbort: AbortController = new AbortController();
async function trackJob(jobId: string) {
	jobTrackerAbort.abort("Tracker obsoleted");
	jobTrackerAbort = new AbortController();
	let response = rpc.streamRipJobUpdates(
		{ jobId },
		{
			abort: jobTrackerAbort.signal,
		}
	);
	jobStatus.value = {
		jobId,
		status: JobStatus.UNSPECIFIED,
		logs: [],
		cprogTitle: "",
		tprogTitle: "",
		progress: {
			cprogValue: 0,
			tprogValue: 0,
			maxValue: Infinity,
		},
	};
	try {
		for await (const update of response.responses) {
			switch (update.ripUpdate.oneofKind) {
				case "status":
					jobStatus.value.status = update.ripUpdate.status;
					break;
				case "logMessage":
					jobStatus.value.logs.push(update.ripUpdate.logMessage);
					break;
				case "cprogTitle":
					jobStatus.value.cprogTitle = update.ripUpdate.cprogTitle;
					break;
				case "tprogTitle":
					jobStatus.value.tprogTitle = update.ripUpdate.tprogTitle;
					break;
				case "progressValues":
					jobStatus.value.progress = update.ripUpdate.progressValues;
					break;
			}
		}
	} catch (err) {
		if (!(err instanceof RpcError)) {
			throw err;
		}
	}
}
watch(
	() => jobInfo.value?.id,
	(id) => {
		if (id === undefined) {
			jobTrackerAbort.abort("Job no longer active");
		} else {
			trackJob(id);
		}
	},
	{ immediate: true }
);
onBeforeUnmount(() => jobTrackerAbort.abort());

const allowRename = computed(
	() => driveStatus.value?.activeRipJob !== undefined
);
async function renameJob() {
	if (driveStatus.value?.activeRipJob === undefined) return;
	if (jobInfo.value === undefined) return;
	const newName = prompt(
		"What would you like to name the job?",
		jobInfo.value?.discTitle || ""
	);
	if (newName === null) return;
	await rpc.renameJob({
		jobId: jobInfo.value?.id,
		newName,
	});
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
						(jobStatus.progress!.cprogValue / jobStatus.progress!.maxValue) *
						100
					"
					buffer-value="0"
					color="red"
					stream
				/>
				<v-label :text="`Total: ${jobStatus.tprogTitle}`" />
				<v-progress-linear
					:model-value="
						(jobStatus.progress!.tprogValue / jobStatus.progress!.maxValue) *
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
