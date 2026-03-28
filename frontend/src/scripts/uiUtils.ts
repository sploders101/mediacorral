import { Code, ConnectError } from "@connectrpc/connect";
import { injectKeys } from "./config";

export function reportErrorsFactory() {
	const prompter = inject(injectKeys.promptService)!;

	return async <T>(prom: PromiseLike<T>, alertTitle?: string) => {
		try {
			const result = await prom;
			return result;
		} catch (error) {
			if (error instanceof ConnectError) {
				if (error.code === Code.Canceled) throw error;
				await prompter.alert(
					`Code: ${error.code}\n\nMessage:\n${decodeURIComponent(error.message)}`,
					alertTitle
				);
			}
			throw error;
		}
	};
}
