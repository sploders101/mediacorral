<script lang="ts">
type Prompt = AlertPrompt | ConfirmPrompt | TextPrompt;
export interface AlertPrompt {
	type: "alert";
	title?: string;
	message: string;
	okLabel?: string;
	callback: () => void;
}
export interface ConfirmPrompt {
	type: "confirm";
	title?: string;
	message: string;
	yesLabel?: string;
	noLabel?: string;
	callback: (result: boolean) => void;
}
export interface TextPrompt {
	type: "text";
	title: string;
	message?: string;
	confirmLabel?: string;
	cancelLabel?: string;
	value?: string;
	callback: (result: string | null) => void;
}

export class PromptService {
	promptStack: Prompt[] = reactive([]);

	alert(message: string, title?: string) {
		return new Promise<void>((callback) =>
			this.promptStack.push({ type: "alert", title, message, callback })
		);
	}
	alertCustom(message: Omit<AlertPrompt, "callback" | "type">) {
		return new Promise<void>((callback) =>
			this.promptStack.push({ ...message, type: "alert", callback })
		);
	}

	confirm(message: string, title?: string) {
		return new Promise<boolean>((callback) =>
			this.promptStack.push({ type: "confirm", title, message, callback })
		);
	}
	confirmCustom(prompt: Omit<ConfirmPrompt, "callback" | "type">) {
		return new Promise<boolean>((callback) =>
			this.promptStack.push({ ...prompt, type: "confirm", callback })
		);
	}

	prompt(title: string, message?: string) {
		return new Promise<string | null>((callback) =>
			this.promptStack.push({ type: "text", title, message, callback })
		);
	}
	promptCustom(prompt: Omit<TextPrompt, "callback" | "type">) {
		return new Promise<string | null>((callback) =>
			this.promptStack.push({ ...prompt, type: "text", callback })
		);
	}
}
</script>

<script lang="ts" setup>
import { injectKeys } from "@/scripts/config";

const inputValues = reactive(new WeakMap<TextPrompt, string>());

const promptService = inject(injectKeys.promptService)!;
const visiblePrompt = computed(() => {
	if (promptService.promptStack.length === 0) return undefined;
	return promptService.promptStack[promptService.promptStack.length - 1];
});
</script>

<template>
	<v-dialog :modelValue="!!visiblePrompt" persistent>
		<template v-if="visiblePrompt?.type === 'alert'">
			<v-card>
				<v-card-title>
					{{ visiblePrompt.title || "Confirm" }}
				</v-card-title>
				<v-card-text> {{ visiblePrompt.message }} </v-card-text>
				<v-card-actions>
					<v-spacer />
					<v-btn
						@click="
							visiblePrompt.callback();
							promptService.promptStack.pop();
						"
						>{{ visiblePrompt.okLabel || "Ok" }}</v-btn
					>
				</v-card-actions>
			</v-card>
		</template>
		<template v-else-if="visiblePrompt?.type === 'confirm'">
			<v-card>
				<v-card-title>
					{{ visiblePrompt.title || "Confirm" }}
				</v-card-title>
				<v-card-text> {{ visiblePrompt.message }} </v-card-text>
				<v-card-actions>
					<v-spacer />
					<v-btn
						@click="
							visiblePrompt.callback(true);
							promptService.promptStack.pop();
						"
						>{{ visiblePrompt.yesLabel || "Yes" }}</v-btn
					>
					<v-btn
						@click="
							visiblePrompt.callback(false);
							promptService.promptStack.pop();
						"
						>{{ visiblePrompt.noLabel || "No" }}</v-btn
					>
				</v-card-actions>
			</v-card>
		</template>
		<template v-else-if="visiblePrompt?.type === 'text'">
			<v-form
				@submit="
					if (inputValues.has(visiblePrompt)) {
						visiblePrompt.callback(inputValues.get(visiblePrompt)!);
						promptService.promptStack.pop();
					} else {
						visiblePrompt.callback(visiblePrompt.value || '');
						promptService.promptStack.pop();
					}
				"
			>
				<v-card>
					<v-card-title> {{ visiblePrompt.title }} </v-card-title>
					<v-card-text>
						<p
							v-if="visiblePrompt.message !== undefined"
							class="pre-wrap ma-5"
						>
							{{ visiblePrompt.message }}
						</p>
						<v-text-field
							autofocus
							@keydown.esc.stopPropogation="
								visiblePrompt.callback(null);
								promptService.promptStack.pop();
							"
							variant="outlined"
							:modelValue="
								inputValues.get(visiblePrompt) ||
								visiblePrompt.value
							"
							@update:modelValue="
								inputValues.set(visiblePrompt, $event)
							"
						/>
					</v-card-text>
					<v-card-actions>
						<v-btn
							@click="
								visiblePrompt.callback(null);
								promptService.promptStack.pop();
							"
						>
							{{ visiblePrompt.cancelLabel || "Cancel" }}
						</v-btn>
						<v-spacer />
						<v-btn type="submit">
							{{ visiblePrompt.confirmLabel || "Confirm" }}
						</v-btn>
					</v-card-actions>
				</v-card>
			</v-form>
		</template>
		<v-skeleton-loader v-else type="card" />
	</v-dialog>
</template>
