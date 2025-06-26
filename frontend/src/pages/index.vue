<script lang="ts" setup>
import DriveMonitor from "@/components/DriveMonitor.vue";
import { AutoripStatus, type DiscDrive } from "@/generated/mediacorral/server/v1/api";
import { injectKeys } from "@/scripts/config";

const rpc = inject(injectKeys.rpc)!;
const driveSelection = ref<DiscDrive | null>(null);
const drives = ref<DiscDrive[]>([]);

onMounted(async () => {
	drives.value = (await rpc.listDrives({})).response.drives;
	driveSelection.value = drives.value[0];
});

function driveKey(drive: DiscDrive) {
	return `${drive.controller}/${drive.driveId}`;
}

const appbar = inject(injectKeys.appbar);
const autorip = ref<AutoripStatus>(AutoripStatus.UNSPECIFIED);
onMounted(() => {
	rpc
		.autoripStatus({ status: AutoripStatus.UNSPECIFIED })
		.then(({ response }) => (autorip.value = response.status));
});
async function changeAutorip(status: boolean) {
	autorip.value = AutoripStatus.UNSPECIFIED;
	await rpc.autoripStatus({
		status: status ? AutoripStatus.ENABLED : AutoripStatus.DISABLED,
	});
	autorip.value = status ? AutoripStatus.ENABLED : AutoripStatus.DISABLED;
}
</script>

<template>
	<v-tabs v-model="driveSelection" align-tabs="center">
		<v-tab v-for="drive in drives" :value="drive" :key="driveKey(drive)">
			{{ drive.name }}
		</v-tab>
	</v-tabs>
	<v-tabs-window v-model="driveSelection">
		<v-tabs-window-item
			v-for="drive in drives"
			:key="driveKey(drive)"
			:value="drive"
		>
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
