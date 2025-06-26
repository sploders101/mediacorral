<script lang="ts" setup>
import type {
	GetJobCatalogueInfoResponse,
	RipJob,
} from "@/generated/mediacorral/server/v1/api";
import { injectKeys } from "@/scripts/config";
import { formatRuntime } from "@/scripts/utils";

const route = useRoute("/rip_jobs/[id]");
const rpc = inject(injectKeys.rpc)!;
const data = ref("");
const jobInfo = ref<RipJob | undefined>();
const catInfo = ref<GetJobCatalogueInfoResponse>();

onMounted(async () => {
	const jobInfoResponse = await rpc.getJobInfo({ jobId: route.params.id });
	const catInfoResponse = await rpc.getJobCatalogueInfo({
		jobId: route.params.id,
	});
	jobInfo.value = jobInfoResponse.response.details;
	catInfo.value = catInfoResponse.response;
	data.value = JSON.stringify(
		{
			jobInfo: jobInfoResponse.response.details,
			catInfo: catInfoResponse.response,
		},
		null,
		4
	);
});

const tableItems = computed(() => {
	if (jobInfo.value === undefined || catInfo.value === undefined) return [];

	const items = [];
	for (const videoFile of catInfo.value.videoFiles) {
		items.push({
			id: videoFile.id,
			runtime:
				videoFile.length === undefined
					? "?"
					: formatRuntime(videoFile.length),
		});
	}

	return items;
});
</script>

<template>
	<v-data-table :items="tableItems" hide-default-footer></v-data-table>
	<pre>{{ data }}</pre>
</template>

<style lang="scss"></style>
