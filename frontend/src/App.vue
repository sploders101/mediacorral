<script lang="ts" setup>
import { useRouter, type RouteLocation } from "vue-router";
import vuetify from "./plugins/vuetify";

const router = useRouter();
const drawer = ref(false);
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
			location="left"
			:rail="$vuetify.display.mobile ? false : !drawer"
			:modelValue="$vuetify.display.mobile ? drawer : true"
		>
			<v-list density="compact" nav>
				<v-list-item
					prepend-icon="mdi-minidisc"
					link
					title="Rip Control"
					to="/"
				/>
			</v-list>
		</v-navigation-drawer>
		<v-main>
			<router-view />
		</v-main>
	</v-app>
</template>
