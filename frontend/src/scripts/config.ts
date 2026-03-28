import type { InjectionKey } from "vue";
import type { MetaCache } from "./commonTypes";
import type { PromptService } from "@/components/PromptService.vue";
import type { Client } from "@connectrpc/connect";
import { CoordinatorApiService } from "@/generated/mediacorral/server/v1/api_pb";

export const injectKeys = {
	rpc: Symbol() as InjectionKey<Client<typeof CoordinatorApiService>>,
	appbar: Symbol() as InjectionKey<Ref<HTMLDivElement | undefined>>,
	metaCache: Symbol() as InjectionKey<MetaCache>,
	promptService: Symbol() as InjectionKey<PromptService>,
} as const;
