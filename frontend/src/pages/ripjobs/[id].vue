<script lang="ts" setup>
import type { JobInfo } from "@/apiTypes";
import { BASE_URL } from "@/scripts/config";

const route = useRoute("/ripjobs/[id]");

const jobInfo = ref<JobInfo | null>(null);

async function getJobInfo(jobId: number) {
	let response = await fetch(`${BASE_URL}/tagging/jobs/${jobId}`);
	if (response.status !== 200) return; // TODO: Add error reporting
	let data: JobInfo = await response.json();
	jobInfo.value = data;
}

watch(
	() => route.params.id,
	() => {
		const jobId = Number(route.params.id);
		if (Number.isNaN(jobId)) return;
		getJobInfo(jobId);
	},
	{ immediate: true }
);

function stringify(value: any) {
	return JSON.stringify(value, null, "    ");
}
</script>

<template>
	<pre>{{ stringify(jobInfo) }}</pre>
</template>
