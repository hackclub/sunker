export type Project = {
	id: number;
	contract: string;
	ring: number;
	name: string;
	description: string;
	repo_url: string | null;
	screenshot_url: string | null;
	hackatime_projects: string[];
};

export const projects = $state<{ list: Project[] | null; broken: boolean }>({ list: null, broken: false });

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
