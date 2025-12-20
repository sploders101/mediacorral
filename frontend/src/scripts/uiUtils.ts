import { RpcError } from "@protobuf-ts/runtime-rpc";
import { injectKeys } from "./config";

export function reportErrorsFactory() {
	const prompter = inject(injectKeys.promptService)!;

	return async <T>(prom: PromiseLike<T>, alertTitle?: string) => {
		try {
			const result = await prom;
			return result;
		} catch (error) {
			if (error instanceof RpcError) {
				if (error.code === "CANCELLED") throw error;
				await prompter.alert(
					`Code: ${error.code}\n\nMessage:\n${decodeURIComponent(error.message)}`,
					alertTitle
				);
			}
			throw error;
		}
	};
}
