export function formatRuntime(length: number): string {
	const hours = Math.floor(length / 60 / 60);
	const minutes = Math.floor(length / 60 % 60);
	const seconds = length % 60;

	let acc = [];
	if (hours > 0) acc.push(`${hours}h`);
	if (minutes > 0) acc.push(`${minutes}m`);
	if (seconds > 0) acc.push(`${seconds}s`)

	return acc.join("");
}
