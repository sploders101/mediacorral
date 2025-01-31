<script lang="ts" setup>
import { useRouter, type RouteLocation } from "vue-router";

const router = useRouter();

const drawer = ref(true);

function goto(name: RouteLocation["name"]) {
	router.push({
		name: name,
	});
	// drawer.value = false;
}
</script>

<template>
	<v-app>
		<v-app-bar>
			<template v-slot:prepend>
				<v-app-bar-nav-icon variant="text" @click.stop="drawer = !drawer" />
			</template>
			<v-app-bar-title> Mediacorral </v-app-bar-title>
		</v-app-bar>
		<v-navigation-drawer
			:location="$vuetify.display.mobile ? 'bottom' : undefined"
			:rail="drawer"
		>
			<v-list density="compact" nav>
				<v-list-item
					prepend-icon="mdi-minidisc"
					link
					title="Rip Control"
					to="/"
				/>
				<v-list-item
					prepend-icon="mdi-content-save-edit"
					link
					title="Rip Jobs"
					to="/ripjobs"
				/>
			</v-list>
		</v-navigation-drawer>
		<router-view />
	</v-app>
</template>
