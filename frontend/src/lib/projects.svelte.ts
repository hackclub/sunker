import { catalog } from '#lib/contracts.svelte.js';
import { RING_COUNT } from '#lib/rings.svelte.js';

export type Project = {
	id: number;
	contract: string | null;
	ring: number | null;
	criteria: string[] | null;
	resources: Record<string, number> | null;
	criteria_ticked: number[];
	name: string;
	description: string;
	repo_url: string | null;
	screenshot_url: string | null;
	hackatime_projects: string[];
};

export type Hackatime = { linked: boolean; projects?: { name: string; seconds: number }[] };

export const CHIPROCK_PER_HOUR = 5;

export const projects = $state<{ list: Project[] | null; broken: boolean; hackatime: Hackatime | null }>({ list: null, broken: false, hackatime: null });

export async function load() {
	try {
		const response = await fetch('/api/projects', { cache: 'no-cache' });
		if (!response.ok) throw new Error();
		projects.list = (await response.json()).projects;
		projects.broken = false;
	} catch {
		projects.broken = true;
	}
}

export async function link() {
	try {
		const response = await fetch('/api/hackatime/projects', { cache: 'no-cache' });
		if (!response.ok) throw new Error();
		projects.hackatime = await response.json();
	} catch {
		projects.hackatime = { linked: false };
	}
}

export function hours(project: Project) {
	const seconds = (projects.hackatime?.projects ?? []).filter(({ name }) => project.hackatime_projects.includes(name)).reduce((total, { seconds }) => total + seconds, 0);
	return seconds / 3600;
}

export function ring(project: Project) {
	if (project.ring !== null) return project.ring;
	const logged = hours(project);
	return Math.min(RING_COUNT - 1, ...catalog.rings.filter(({ min_hours }) => logged >= min_hours).map(({ ring }) => ring));
}

export function remaining(project: Project) {
	const next = catalog.rings.find(({ ring: other }) => other === ring(project) - 1);
	return next ? next.min_hours - hours(project) : null;
}
