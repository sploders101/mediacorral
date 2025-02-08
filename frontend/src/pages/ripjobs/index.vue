<script lang="ts" setup>
import type { RipJobsItem } from "@/apiTypes";
import { BASE_URL } from "@/scripts/config";

const ripJobs = ref<RipJobsItem[] | null>(null);

async function getRipJobs() {
	const request = await fetch(`${BASE_URL}/tagging/get_untagged_jobs`);
	const data: RipJobsItem[] = await request.json();
	ripJobs.value = data;
}

function formatTime(timestamp: number) {
	return new Date(timestamp * 1000).toLocaleString();
}

onMounted(() => {
	getRipJobs();
});

const thing = ref(null);

watch(thing, () => alert(thing.value));
</script>

<template>
	<v-data-table-virtual
		:loading="ripJobs === null"
		:headers="[
			{ title: 'ID', value: 'id' },
			{ title: 'Disc Name', value: 'disc_title' },
			{
				title: 'Started At',
				key: 'start_time',
				value: (item) => formatTime(item.start_time),
			},
			{ title: 'Actions', key: 'actions', sortable: false },
		]"
		:items="ripJobs || []"
	>
		<template v-slot:item.actions="{ item }">
			<v-btn flat @click="$router.push(`/ripjobs/${item.id}`)">Open</v-btn>
		</template>
	</v-data-table-virtual>
</template>

<style lang="scss" scoped></style>
