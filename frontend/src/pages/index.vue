<script lang="ts" setup>
import DriveMonitor from "@/components/DriveMonitor.vue";
import { useAppStore } from "@/stores/app";

const appStore = useAppStore();
const driveSelection = ref<string | null>(null);
onMounted(async () => {
	await appStore.getDriveList();
	driveSelection.value = appStore.driveList[0];
});
</script>

<template>
	<v-tabs v-model="driveSelection" align-tabs="center">
		<v-tab v-for="drive in appStore.driveList" :value="drive" :key="drive">
			{{ drive }}
		</v-tab>
	</v-tabs>
	<v-tabs-window v-model="driveSelection">
		<v-tabs-window-item
			v-for="drive in appStore.driveList"
			:key="drive"
			:value="drive"
		>
			<v-container fluid>
				<DriveMonitor :drive="drive" />
			</v-container>
		</v-tabs-window-item>
	</v-tabs-window>
</template>
