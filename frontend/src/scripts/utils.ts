import { VideoStereoMode } from "@/generated/mediacorral/analysis/v1/main";

export function formatRuntime(length: number): string {
	const hours = Math.floor(length / 60 / 60);
	const minutes = Math.floor((length / 60) % 60);
	const seconds = length % 60;

	let acc = [];
	if (hours > 0) acc.push(`${hours}h`);
	if (minutes > 0) acc.push(`${minutes}m`);
	if (seconds > 0) acc.push(`${seconds}s`);

	return acc.join("");
}

export function toHex(buf: Uint8Array) {
	return Array.prototype.map
		.call(buf, (n: number) => n.toString(16).padStart(2, "0"))
		.join("")
		.toUpperCase();
}

export function capitalize(mystr: string): string {
	if (mystr.length > 1) {
		return mystr[0].toUpperCase() + mystr.substring(1);
	} else {
		return mystr.toUpperCase();
	}
}

export function resolveLanguage(langCode: string): string {
	switch (langCode) {
		case "eng":
			return "English";
		case "spa":
			return "Spanish";
		case "fra":
			return "French";
		case "fin":
			return "Finnish";
		case "nor":
			return "Norwegian";
		case "swe":
			return "Swedish";
		default:
			return langCode;
	}
}

export function resolveStereoMode(stereoMode: VideoStereoMode): string {
	switch (stereoMode) {
		case VideoStereoMode.UNSPECIFIED:
			return "Unknown";
		case VideoStereoMode.MONO:
			return "2D";
		case VideoStereoMode.SIDE_BY_SIDE_LEFT_EYE_FIRST:
			return "Side by side (left eye first)";
		case VideoStereoMode.TOP_BOTTOM_RIGHT_EYE_FIRST:
			return "Top - bottom (right eye is first)";
		case VideoStereoMode.TOP_BOTTOM_LEFT_EYE_FIRST:
			return "Top - bottom (left eye is first)";
		case VideoStereoMode.CHECKBOARD_RIGHT_EYE_FIRST:
			return "Checkboard (right eye is first)";
		case VideoStereoMode.CHECKBOARD_LEFT_EYE_FIRST:
			return "Checkboard (left eye is first)";
		case VideoStereoMode.ROW_INTERLEAVED_RIGHT_EYE_FIRST:
			return "Row interleaved (right eye is first)";
		case VideoStereoMode.ROW_INTERLEAVED_LEFT_EYE_FIRST:
			return "Row interleaved (left eye is first)";
		case VideoStereoMode.COLUMN_INTERLEAVED_RIGHT_EYE_FIRST:
			return "Column interleaved (right eye is first)";
		case VideoStereoMode.COLUMN_INTERLEAVED_LEFT_EYE_FIRST:
			return "Column interleaved (left eye is first)";
		case VideoStereoMode.ANAGLYPH_CYAN_RED:
			return "Anaglyph (cyan/red)";
		case VideoStereoMode.SIDE_BY_SIDE_RIGHT_EYE_FIRST:
			return "Side by side (right eye first)";
		case VideoStereoMode.ANAGLYPH_GREEN_MAGENTA:
			return "Anaglyph (green/magenta)";
		case VideoStereoMode.LACED_LEFT_EYE_FIRST:
			return "Both eyes laced in one Block (left eye is first)";
		case VideoStereoMode.LACED_RIGHT_EYE_FIRST:
			return "Both eyes laced in one Block (right eye is first)";
	}
}
