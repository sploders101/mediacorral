<script lang="ts" setup>
import PromptService from "./components/PromptService.vue";
import { injectKeys } from "./scripts/config";

const drawer = ref(false);
const appbar = ref<HTMLDivElement | undefined>();
provide(injectKeys.appbar, appbar);
</script>

<template>
	<v-app>
		<v-app-bar>
			<template v-slot:prepend>
				<v-app-bar-nav-icon variant="text" @click.stop="drawer = !drawer" />
			</template>
			<v-app-bar-title>
				<div class="mc-appbar" ref="appbar">
					Mediacorral
					<v-spacer />
				</div>
			</v-app-bar-title>
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
				<v-list-item
					prepend-icon="mdi-monitor-arrow-down"
					link
					title="Metadata Importer"
					to="/meta_import"
				/>
				<v-list-item
					prepend-icon="mdi-archive-edit"
					link
					title="Catalogue"
					to="/catalogue"
				/>
			</v-list>
		</v-navigation-drawer>
		<v-main>
			<router-view />
		</v-main>
		<PromptService/>
	</v-app>
</template>
