import "@/styles/main.scss";
import { registerPlugins } from "@/plugins";
import App from "./App.vue";
import { createApp } from "vue";
import { injectKeys } from "./scripts/config";
import { PromptService } from "./components/PromptService.vue";
import { createConnectTransport } from "@connectrpc/connect-web";
import { createClient } from "@connectrpc/connect";
import { CoordinatorApiService } from "./generated/mediacorral/server/v1/api_pb";

const app = createApp(App);
registerPlugins(app);

const transport = createConnectTransport({
	baseUrl: "/",
});
const rpc = createClient(CoordinatorApiService, transport);
app.provide(injectKeys.rpc, rpc);
app.provide(injectKeys.promptService, new PromptService());

app.mount("#app");
