import { MAP } from '#lib/colors.js';

export interface Tile {
	height?: number;
	base?: string;
	side?: string;
	variants?: string[];
}

export interface Region {
	name: string;
	tiles: [number, number][];
}

export interface HexMap {
	rows: (Tile | null)[][];
	regions?: Region[];
}

export const ENEMIES = ['chunker', 'crawler', 'hopper'] as const;
export type Enemy = (typeof ENEMIES)[number];
export type Force = Partial<Record<Enemy, number>>;

export const enemyName = (enemy: Enemy) => `${enemy.charAt(0).toUpperCase()}${enemy.slice(1)}`;

export interface Territory {
	row: number;
	col: number;
	occupied: boolean;
	enemies: Force;
}

export interface PlacedTile {
	tile: Required<Tile>;
	row: number;
	col: number;
	x: number;
	y: number;
}

export const DEFAULT_TILE: Required<Tile> = { height: 0.5, base: 'plain', side: MAP.side, variants: [] };

export const SIZE = 1;
export const WIDTH = Math.sqrt(3) * SIZE;
export const HEIGHT = 2 * SIZE;

export const CORNERS = Array.from({ length: 6 }, (_, i) => {
	const angle = (Math.PI / 3) * i - Math.PI / 2;
	return [SIZE * Math.cos(angle), SIZE * Math.sin(angle)];
});

export const FRONT = [1, 2, 3, 4].map((i) => CORNERS[i]);

export function neighbours(row: number, col: number) {
	const odd = row & 1;
	return [
		[row - 1, col + odd],
		[row, col + 1],
		[row + 1, col + odd],
		[row + 1, col + odd - 1],
		[row, col - 1],
		[row - 1, col + odd - 1]
	];
}

export function center(row: number, col: number) {
	return { x: WIDTH * (col + (row & 1) / 2), y: HEIGHT * 0.75 * row };
}

export function place(map: HexMap): PlacedTile[] {
	return map.rows.flatMap((tiles, row) =>
		tiles.flatMap((tile, col) =>
			tile ? [{ tile: { ...DEFAULT_TILE, ...tile }, row, col, ...center(row, col) }] : []
		)
	);
}

export function bounds(tiles: PlacedTile[]) {
	const xs = tiles.map((t) => t.x);
	const ys = tiles.map((t) => t.y);
	const left = Math.min(...xs) - WIDTH / 2;
	const top = Math.min(...ys) - HEIGHT / 2;
	return {
		left,
		top,
		width: Math.max(...xs) + WIDTH / 2 - left,
		height: Math.max(...ys) + HEIGHT / 2 - top
	};
}
