export type Feature = { shape: string; colour: string; x: number; y: number; rotation: number };
export type Attributes = { age: string; occupation: string; backstory: string; features: Feature[]; hue?: number | null };
export type Character = { handle: string; attributes: Attributes };
export type Guild = { slug: string; name: string };
export type Stage = null | 'passport' | 'guild' | 'hq' | 'done';
export type Me = { stage: Stage; superadmin: boolean; banned: boolean; character: Character | null; guild: Guild | null; resources: Record<string, number> };

export const me = $state<{ current: Me | null }>({ current: null });

export async function load(fetcher: typeof fetch = fetch) {
	const response = await fetcher('/api/me', { cache: 'no-cache' });
	if (response.status === 401) return null;
	if (!response.ok) throw new Error(`${response.status}`);
	me.current = await response.json();
	return me.current;
}

export async function advance(step: Exclude<Stage, null>) {
	const response = await fetch('/api/onboarding', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ step }) });
	if (!response.ok) throw new Error((await response.text()) || `${response.status}`);
	if (me.current) me.current.stage = step;
}

export async function save(character: Character) {
	const response = await fetch('/api/character', {
		method: 'PUT',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ handle: character.handle, ...character.attributes })
	});
	if (!response.ok) throw Object.assign(new Error((await response.text()) || `${response.status}`), { status: response.status });
	const saved: Character = await response.json();
	if (me.current) me.current.character = saved;
	return saved;
}

export function draft(character: Character | null | undefined): Character {
	const copy = structuredClone($state.snapshot(character));
	return { handle: copy?.handle ?? '', attributes: { age: '', occupation: '', backstory: '', features: [], ...copy?.attributes } };
}

export const complete = ({ handle, attributes }: Character) =>
	[handle, attributes.age, attributes.occupation, attributes.backstory].every((value) => value.trim()) && attributes.features.length > 0;
