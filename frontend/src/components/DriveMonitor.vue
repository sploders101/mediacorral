<script lang="ts" setup>
import type { DriveState, RipInstruction } from "@/apiTypes";
import { BASE_URL } from "@/scripts/config";

const props = defineProps<{
	drive: string;
}>();
const driveStatus = ref<DriveState | null>(null);
const discTitle = computed(() => {
	if (driveStatus.value === null) {
		return "Loading...";
	}
	switch (driveStatus.value.status) {
		case "Unknown":
			return "Unknown";
		case "Empty":
		case "TrayOpen":
			return props.drive;
		case "NotReady":
			return "Loading...";
		case "Loaded":
			return driveStatus.value.disc_name || props.drive;
	}
});
const currentStatus = computed(() => {
	if (driveStatus.value === null) {
		return "Fetching drive status...";
	}
	switch (driveStatus.value.active_command.type) {
		case "None":
			break;
		case "Error":
			return "Error";
		case "Ripping":
			return `Ripping - Job #${driveStatus.value.active_command.job_id}`;
		default:
			return "Unknown";
	}
	switch (driveStatus.value.status) {
		case "Unknown":
			return "Unknown3";
		case "Empty":
			return "Closed - Empty";
		case "TrayOpen":
			return "Tray Open";
		case "NotReady":
			return "Loading Disc...";
		case "Loaded":
			return "Disc loaded. Ready to rip.";
	}

	return "Unknown";
});
const showTrayAction = computed(() => {
	if (driveStatus.value === null) return [];

	if (driveStatus.value.active_command.type === "Ripping") {
		return [];
	}

	switch (driveStatus.value.status) {
		case "Unknown":
			return [];
		case "Empty":
			return ["open"];
		case "Loaded":
			return ["open", "rip"];
		case "TrayOpen":
			return ["close"];
		case "NotReady":
			return [];
	}

	return [];
});

async function openTray() {
	let response = await fetch(
		`${BASE_URL}/ripping/eject?device=${encodeURIComponent(props.drive)}`,
		{
			method: "POST",
		}
	);
	if (response.status !== 200) return; // TODO: add toast or something
	// TODO: Disable button while processing
}

async function closeTray() {
	let response = await fetch(
		`${BASE_URL}/ripping/retract?device=${encodeURIComponent(props.drive)}`,
		{
			method: "POST",
		}
	);
	if (response.status !== 200) return; // TODO: add toast or something
	// TODO: Disable button while processing
}

const assert = <T,>(item: T) => item;

async function ripDisc() {
	// TODO: Add options. Using defaults for now
	let response = await fetch(`${BASE_URL}/ripping/rip`, {
		method: "POST",
		headers: {
			"content-type": "application/json",
		},
		body: JSON.stringify(
			assert<RipInstruction>({
				autoeject: true,
				device: props.drive,
				disc_name: null,
				suspected_contents: null,
			})
		),
	});
	if (response.status !== 200) return; // TODO: add toast or something
	// TODO: Disable button while processing
}

let eventSource: EventSource | null = null;
onMounted(() => {
	eventSource = new EventSource(
		`${BASE_URL}/ripping/rip_status?device=${encodeURIComponent(props.drive)}&stream=true`
	);
	eventSource.addEventListener("message", (event) => {
		driveStatus.value = JSON.parse(event.data);
	});
});
onBeforeUnmount(() => {
	if (eventSource !== null) {
		eventSource.close();
	}
});
</script>

<template>
	<v-card>
		<v-card-title> {{ discTitle }}</v-card-title>
		<v-card-subtitle>Status: {{ currentStatus }}</v-card-subtitle>
		<v-card-text>
			<template v-if="driveStatus?.active_command.type == 'Ripping'">
				<v-label :text="`Current: ${driveStatus.active_command.cprog_title}`" />
				<v-progress-linear
					:model-value="
						(driveStatus.active_command.cprog_value /
							driveStatus.active_command.max_prog_value) *
						100
					"
					buffer-value="0"
					color="red"
					stream
				/>
				<v-label :text="`Total: ${driveStatus?.active_command.tprog_title}`" />
				<v-progress-linear
					:model-value="
						(driveStatus.active_command.tprog_value /
							driveStatus.active_command.max_prog_value) *
						100
					"
					buffer-value="0"
					color="blue"
					stream
				/>
				<v-divider style="margin-top: 0.5rem; margin-bottom: 0.5rem" />
				<pre>{{ driveStatus.active_command.logs.join("\n") }}</pre>
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
