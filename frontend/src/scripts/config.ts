import type { CoordinatorApiServiceClient } from "@/generated/mediacorral/server/v1/api.client";
import type { InjectionKey } from "vue";
import type { MetaCache } from "./commonTypes";
import type { PromptService } from "@/components/PromptService.vue";

export const injectKeys = {
	rpc: Symbol() as InjectionKey<CoordinatorApiServiceClient>,
	appbar: Symbol() as InjectionKey<Ref<HTMLDivElement | undefined>>,
	metaCache: Symbol() as InjectionKey<MetaCache>,
	promptService: Symbol() as InjectionKey<PromptService>,
} as const;
