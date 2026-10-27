const PERMUTATION = new Uint8Array(512);
{
	const p = Array.from({ length: 256 }, (_, i) => i);
	for (let i = 255; i > 0; i--) {
		const j = Math.floor(Math.random() * (i + 1));
		[p[i], p[j]] = [p[j], p[i]];
	}
	for (let i = 0; i < 512; i++) PERMUTATION[i] = p[i & 255];
}

const fade = (t: number) => t * t * t * (t * (t * 6 - 15) + 10);
const lerp = (t: number, a: number, b: number) => a + t * (b - a);
const grad = (hash: number, x: number, y: number) => (hash & 1 ? -x : x) + (hash & 2 ? -y : y);

export function perlin(x: number, y: number) {
	const fx = Math.floor(x);
	const fy = Math.floor(y);
	const X = fx & 255;
	const Y = fy & 255;
	x -= fx;
	y -= fy;
	const u = fade(x);
	const v = fade(y);
	const a = PERMUTATION[X] + Y;
	const b = PERMUTATION[X + 1] + Y;
	return lerp(
		v,
		lerp(u, grad(PERMUTATION[a], x, y), grad(PERMUTATION[b], x - 1, y)),
		lerp(u, grad(PERMUTATION[a + 1], x, y - 1), grad(PERMUTATION[b + 1], x - 1, y - 1))
	);
}
