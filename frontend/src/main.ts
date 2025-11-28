import "@/styles/main.scss";
import { registerPlugins } from "@/plugins";
import App from "./App.vue";
import { createApp } from "vue";
import { TwirpFetchTransport } from "@protobuf-ts/twirp-transport";
import { CoordinatorApiServiceClient } from "./generated/mediacorral/server/v1/api.client";
import { injectKeys } from "./scripts/config";
import { PromptService } from "./components/PromptService.vue";

const app = createApp(App);
registerPlugins(app);

const transport = new TwirpFetchTransport({
	baseUrl: "/twirp",
	sendJson: import.meta.env.DEV,
});
const rpc = new CoordinatorApiServiceClient(transport);
app.provide(injectKeys.rpc, rpc);
app.provide(injectKeys.promptService, new PromptService());

app.mount("#app");
