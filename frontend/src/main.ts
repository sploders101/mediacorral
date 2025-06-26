import "@/styles/main.scss";
import { registerPlugins } from "@/plugins";
import App from "./App.vue";
import { createApp } from "vue";
import { GrpcWebFetchTransport } from "@protobuf-ts/grpcweb-transport";
import { CoordinatorApiServiceClient } from "./generated/mediacorral/server/v1/api.client";
import { BASE_URL, injectKeys } from "./scripts/config";

const app = createApp(App);
registerPlugins(app);

const transport = new GrpcWebFetchTransport({
	baseUrl: BASE_URL,
	format: "binary",
});
const rpc = new CoordinatorApiServiceClient(transport);
app.provide(injectKeys.rpc, rpc);

app.mount("#app");
