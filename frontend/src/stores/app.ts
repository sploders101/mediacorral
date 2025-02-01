// Utilities
import { BASE_URL } from "@/scripts/config";
import { defineStore } from "pinia";

export const useAppStore = defineStore("app", () => {
	const driveList = ref<string[]>([]);

	async function getDriveList() {
		const response = await (await fetch(`${BASE_URL}/ripping/list_drives`)).json();
		driveList.value = response;
		console.log(response);
	}

	return { driveList, getDriveList };
});
