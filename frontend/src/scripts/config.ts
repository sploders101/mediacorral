import type { CoordinatorApiServiceClient } from "@/generated/mediacorral/server/v1/api.client";
import type { InjectionKey } from "vue";

export const BASE_URL = "/api";

export const injectKeys = {
	rpc: Symbol() as InjectionKey<CoordinatorApiServiceClient>,
} as const;
