<script lang="ts" setup>
import { RipJob } from "@/generated/mediacorral/server/v1/api";
import { injectKeys } from "@/scripts/config";

const rpc = inject(injectKeys.rpc)!;
const rows = ref<RipJob[]>([]);
onMounted(async () => {
	const { response: untaggedJobs } = await rpc.getUntaggedJobs({
		skip: 0,
		limit: 500,
	});
	rows.value = untaggedJobs.ripJobs;
});

function formatTime(time: number) {
	const result = new Date(time * 1000).toLocaleString();
	return result;
}
function getRipStatus(job: RipJob) {
	if (!job.ripFinished) return "In progress";
	if (!job.imported) return "Importing";
	return "Finished";
}
</script>

<template>
	<v-data-table-virtual
		:items="rows"
		items-per-page="-1"
		:headers="[
			{ title: 'ID', value: 'id' },
			{
				title: 'Start Time',
				key: 'startTime',
				value: (row) => formatTime(Number(row.startTime)),
			},
			{
				title: 'Rip Status',
				key: 'status',
				value: (row) => getRipStatus(row as RipJob),
			},
			{ title: 'Title', value: 'discTitle' },
			{ title: 'Actions', key: 'actions', sortable: false },
		]"
		hide-default-footer
	>
		<template v-slot:item.actions="{ item }">
			<v-btn flat @click="$router.push(`/catalogue/${item.id}`)">Open</v-btn>
		</template>
	</v-data-table-virtual>
</template>

<style lang="scss"></style>
