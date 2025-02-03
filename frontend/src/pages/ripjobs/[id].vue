<script lang="ts" setup>
import {
	type VideoType,
	type JobInfo,
	type MatchInfoItem,
	type OstDownloadsItem,
	type TagFile,
	type VideoFilesItem,
} from "@/apiTypes";
import SuspicionDialog from "@/components/SuspicionDialog.vue";
import { BASE_URL } from "@/scripts/config";
import { useAppStore } from "@/stores/app";

const route = useRoute("/ripjobs/[id]");
const router = useRouter();
const appStore = useAppStore();

const jobInfo = ref<JobInfo | null>(null);

async function getJobInfo(jobId: number) {
	let response = await fetch(`${BASE_URL}/tagging/jobs/${jobId}`);
	if (response.status !== 200) return; // TODO: Add error reporting
	let data: JobInfo = await response.json();
	jobInfo.value = data;

	if (data.suspected_contents === null) return;
	if (data.suspected_contents.type === "Movie") {
	} else if (data.suspected_contents.type === "TvEpisodes") {
		await Promise.all(
			data.suspected_contents.episode_tmdb_ids.map((tmdbId) =>
				appStore.getTvEpisodeInfoByTmdb(tmdbId)
			)
		);
	}
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

const ostDownloads = computed(() => {
	if (jobInfo.value === null) return null;

	const downloads: Record<number, OstDownloadsItem> = {};

	for (const file of jobInfo.value.ost_subtitle_files) {
		downloads[file.id] = file;
	}

	return downloads;
});

interface VideoInfo {
	video: VideoFilesItem;
	// matches: MatchInfoItem[];
	likelyMatch: {
		name: string;
		similarity: number;
		tag: TagFile;
	} | null;
}

const datatable = computed(() => {
	if (jobInfo.value === null) return null;

	const videos: VideoInfo[] = [];

	for (const file of jobInfo.value.video_files) {
		const matches = jobInfo.value.matches
			.filter((match) => match.video_file_id === file.id)
			.sort(
				(a, b) => a.distance / a.max_distance - b.distance / b.max_distance
			);

		const info: VideoInfo = {
			video: file,
			likelyMatch: null,
		};

		const likelyMatches = matches.filter(
			(match) => match.distance / match.max_distance < 0.25
		);
		if (likelyMatches.length === 1 && ostDownloads.value !== null) {
			const ostFileInfo = ostDownloads.value[likelyMatches[0].ost_download_id];
			let name: string | null = null;
			if (ostFileInfo.video_type === "Movie") {
				if (ostFileInfo.match_id in appStore.movies) {
					name = appStore.movies[ostFileInfo.match_id].title;
				}
			} else if (ostFileInfo.video_type === "TvEpisode") {
				if (ostFileInfo.match_id in appStore.tvEpisodesFlat) {
					const episode = appStore.tvEpisodesFlat[ostFileInfo.match_id];
					const season = appStore.tvSeasonsFlat[episode.tv_season_id];
					name = `S${season.season_number}E${episode.episode_number} - ${episode.title}`;
				}
			}

			let similarity =
				100 - likelyMatches[0].distance / likelyMatches[0].max_distance;

			if (name !== null) {
				info.likelyMatch = {
					name,
					similarity: Math.floor(similarity * 100) / 100,
					tag: {
						file: file.id,
						match_id: ostFileInfo.match_id,
						video_type: ostFileInfo.video_type,
					},
				};
			}
		}

		videos.push(info);
	}

	return videos;
});

const inflightApprovals = reactive(new Set<number>());

async function approveMatch(item: VideoInfo) {
	if (item.likelyMatch === null) return;
	inflightApprovals.add(item.video.id);
	const response = await fetch(`${BASE_URL}/tagging/tag_file`, {
		method: "POST",
		headers: {
			"content-type": "application/json",
		},
		body: JSON.stringify(item.likelyMatch.tag),
	});
	try {
		getJobInfo(Number(route.params.id));
	} catch (err) {
		console.error(err);
	}
	inflightApprovals.delete(item.video.id);
	if (response.status !== 200) {
		throw new Error("Unable to tag file"); // TODO: Report to user
	}
	item.likelyMatch = null;
}

const pruning = ref(false);
async function pruneJob() {
	// TODO: Swap this out for something async
	const response = confirm(
		"Are you sure you want to prune the job? This will delete any untagged files."
	);
	if (response !== true) return;
	pruning.value = true;
	try {
		const response = await fetch(
			`${BASE_URL}/tagging/jobs/${route.params.id}/prune`,
			{
				method: "POST",
			}
		);
		if (response.status !== 200) throw new Error("Unable to prune job"); // TODO: Report to user
	} finally {
		pruning.value = false;
	}
	router.back();
}

const deleting = ref(false);
async function deleteJob() {
	// TODO: Swap this out for something async
	const confirmation = `Please delete job ${route.params.id}`;
	const response = prompt(
		`Are you sure you want to delete the job? This will delete any untagged files.\n\nTo continue, type the following:\n${confirmation}`
	);
	if (response !== confirmation) return;
	deleting.value = true;
	try {
		const response = await fetch(
			`${BASE_URL}/tagging/jobs/${route.params.id}`,
			{
				method: "DELETE",
			}
		);
		if (response.status !== 200) throw new Error("Unable to delete job"); // TODO: Report to user
	} finally {
		deleting.value = false;
	}
	router.back();
}

const suspicionDialog = ref(false);
</script>

<template>
	<v-container>
		<v-card>
			<v-card-title>
				{{ jobInfo ? jobInfo.disc_title : "Loading..." }}
			</v-card-title>
			<v-card-text>
				<v-data-table-virtual
					:loading="datatable === null"
					:headers="[
						{ title: 'Video ID', value: 'video.id' },
						{
							title: 'Likely Match',
							value: 'likelyMatch.name',
							sortable: true,
						},
						{
							title: 'Similarity',
							key: 'likelyMatch.similarity',
							value: (item) =>
								item.likelyMatch && `${item.likelyMatch.similarity}%`,
							sortable: true,
						},
						{ title: 'Actions', key: 'actions', sortable: false },
					]"
					:items="datatable || []"
				>
					<template v-slot:item.actions="{ item }">
						<v-btn
							flat
							v-if="
								item.likelyMatch !== null &&
								(item.video.video_type !== item.likelyMatch.tag.video_type ||
									item.video.match_id !== item.likelyMatch.tag.match_id)
							"
							@click="approveMatch(item)"
							:loading="inflightApprovals.has(item.video.id)"
						>
							Approve
						</v-btn>
					</template>
				</v-data-table-virtual>
			</v-card-text>
			<v-card-actions>
				<v-btn flat @click="suspicionDialog = true"> Add Suspicion </v-btn>
				<v-spacer />
				<v-btn flat color="green" @click="pruneJob()" :loading="pruning">
					Prune
				</v-btn>
				<v-btn flat color="red" @click="deleteJob()" :loading="deleting">
					Delete
				</v-btn>
			</v-card-actions>
		</v-card>
	</v-container>
	<SuspicionDialog :jobId="Number(route.params.id)" v-model="suspicionDialog" />
</template>
