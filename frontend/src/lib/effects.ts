import type { Tile } from '#lib/map.js';

export interface Effect {
	name: string;
	value?: number | string;
	positive: boolean;
}

export const EFFECTS: Record<string, Effect[]> = {};

export function describe(tile: Required<Tile>) {
	const key = tile.variant || tile.base;
	return { title: key.charAt(0).toUpperCase() + key.slice(1), effects: EFFECTS[key] ?? [] };
}

export function amount(value: number | string) {
	return typeof value === 'number' ? (value > 0 ? `+${value}` : `${value}`) : value;
}
