<script lang="ts" setup>
import { BASE_URL } from "@/scripts/config";

interface DriveStatus {
	active_command: ActiveDriveCommand;
}
type ActiveDriveCommand =
	| { type: "None" }
	| { type: "Error"; message: string }
	| {
			type: "Ripping";
			cprog_title: string;
			cprog_value: number;
			tprog_title: string;
			tprog_value: number;
			max_prog_value: number;
			logs: string[];
	  };

const props = defineProps<{
	drive: string;
}>();
const driveStatus = ref<DriveStatus | null>(null);
const currentStatus = computed(() => {
	switch (driveStatus.value?.active_command.type) {
		case null:
		case undefined:
			return "Fetching drive status...";
		case "None":
			return "Inactive";
		case "Error":
			return "Error";
		case "Ripping":
			return "Ripping";
		default:
			return "Unknown";
	}
});

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
		<v-card-title> {{ props.drive }}</v-card-title>
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
	</v-card>
</template>
