<script lang="ts" setup>
import DriveMonitor from "@/components/DriveMonitor.vue";
import {
	AutoripStatus,
	type DiscDrive,
} from "@/generated/mediacorral/server/v1/api_pb";
import { injectKeys } from "@/scripts/config";
import { reportErrorsFactory } from "@/scripts/uiUtils";

const rpc = inject(injectKeys.rpc)!;
const reportErrors = reportErrorsFactory();
const driveSelection = ref<DiscDrive | undefined>(undefined);
const drives = ref<DiscDrive[]>([]);

function keyExtractSort<T>(keyExtractor: (i: T) => any) {
	return (a: T, b: T) => {
		const aKey = keyExtractor(a);
		const bKey = keyExtractor(b);
		if (aKey > bKey) return 1;
		if (aKey < bKey) return -1;
		return 0;
	};
}

/**
 * Updates the list of drives in the least destructive way possible.
 * This preserves components that check for equality.
 */
function updateDrives(newDrives: DiscDrive[]) {
	newDrives.forEach((newDrive) => {
		const oldDrive = drives.value.find((drive) => drive.id === newDrive.id);
		if (oldDrive === undefined) {
			drives.value.push(newDrive);
		} else {
			oldDrive.name = newDrive.name;
		}
	});
	drives.value = drives.value.filter(
		(oldDrive) =>
			newDrives.findIndex((newDrive) => newDrive.id === oldDrive.id) != -1
	);

	drives.value.sort(keyExtractSort((drive) => drive.id));
}

let listTrackerAbort: AbortController = new AbortController();
onMounted(async () => {
	listTrackerAbort.abort("CANCELLED");
	listTrackerAbort = new AbortController();
	let response = rpc.streamDrivesList({});
	await reportErrors(
		(async () => {
			for await (const update of response) {
				updateDrives(update.drives);
			}
		})(),
		"Error while streaming job list"
	);
});
onBeforeUnmount(() => listTrackerAbort.abort());
watch(
	() => drives.value,
	() => {
		if (driveSelection.value === undefined && drives.value.length > 0) {
			driveSelection.value = drives.value[0];
		} else if (driveSelection.value !== undefined && !drives.value.includes(driveSelection.value)) {
			// Prefer selecting nothing to prevent mis-clicks
			driveSelection.value = undefined;
		}
	},
	{ immediate: true, deep: true }
);

const appbar = inject(injectKeys.appbar);
const autorip = ref<AutoripStatus>(AutoripStatus.UNSPECIFIED);
onMounted(() => {
	reportErrors(
		rpc
			.autoripStatus({ status: AutoripStatus.UNSPECIFIED })
			.then((response) => (autorip.value = response.status))
	);
});
async function changeAutorip(status: boolean) {
	autorip.value = AutoripStatus.UNSPECIFIED;
	await reportErrors(
		rpc.autoripStatus({
			status: status ? AutoripStatus.ENABLED : AutoripStatus.DISABLED,
		}),
		"Error changing autorip status"
	);
	autorip.value = status ? AutoripStatus.ENABLED : AutoripStatus.DISABLED;
}
</script>

<template>
	<v-tabs v-model="driveSelection" align-tabs="center">
		<v-tab v-for="drive in drives" :value="drive" :key="drive.id">
			{{ drive.name }}
		</v-tab>
	</v-tabs>
	<v-tabs-window v-model="driveSelection">
		<v-tabs-window-item v-for="drive in drives" :key="drive.id" :value="drive">
			<v-container fluid>
				<DriveMonitor :drive="drive" :visible="driveSelection === drive" />
			</v-container>
		</v-tabs-window-item>
	</v-tabs-window>
	<teleport :to="appbar">
		<v-switch
			label="Autorip"
			hide-details
			:loading="autorip === AutoripStatus.UNSPECIFIED"
			:modelValue="autorip === AutoripStatus.ENABLED"
			@update:modelValue="changeAutorip($event as boolean)"
		/>
	</teleport>
</template>
