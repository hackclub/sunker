import { colour, vec3 } from '#lib/colors.js';

export interface Base {
	glsl: string;
	flat: string;
	functions?: string;
}

export const NOISE = `
uniform float u_time;
vec2 hash2(vec2 p) {
	p = vec2(dot(p, vec2(127.1, 311.7)), dot(p, vec2(269.5, 183.3)));
	return fract(sin(p) * 43758.5453);
}
float perlin(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
	float a = dot(hash2(i) * 2.0 - 1.0, f);
	float b = dot(hash2(i + vec2(1.0, 0.0)) * 2.0 - 1.0, f - vec2(1.0, 0.0));
	float c = dot(hash2(i + vec2(0.0, 1.0)) * 2.0 - 1.0, f - vec2(0.0, 1.0));
	float d = dot(hash2(i + vec2(1.0, 1.0)) * 2.0 - 1.0, f - vec2(1.0, 1.0));
	return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}`;

export const LIBRARY = `${NOISE}
bool g_rock = false;
float grass_blade(vec2 q, float side, float radius) {
	vec2 v = q - vec2(side * radius, 0.0);
	float theta = atan(-v.y, -side * v.x);
	if (theta >= 0.0 && theta <= 0.9) return abs(length(v) - radius);
	vec2 tip = vec2(side * radius * (1.0 - cos(0.9)), -radius * sin(0.9));
	return min(length(q), length(q - tip));
}`;

export const BASES: Record<string, Base> = {
	plain: {
		get flat() {
			return colour('plain');
		},
		get glsl() {
			return `return ${vec3(colour('plain'))};`;
		}
	},
	grass: {
		get flat() {
			return colour('grass');
		},
		get glsl() {
			return `
	float n = 0.5 * perlin(world * 0.5) + 0.3 * perlin(world * 1.2) + 0.2 * perlin(world * 2.6) + 0.12 * perlin(world * 6.0) + 0.08 * perlin(world * 14.0);
	vec3 color = mix(${vec3(colour('grass-low'))}, ${vec3(colour('grass-high'))}, clamp(n * 0.9 + 0.5, 0.0, 1.0));
	float blades = 1e9;
	vec2 cell = floor(world / 0.4);
	for (int y = -1; y <= 1; y++) {
		for (int x = -1; x <= 1; x++) {
			vec2 id = cell + vec2(x, y);
			vec2 h = hash2(id);
			if (fract(h.x * 7.13) > 0.6) continue;
			vec2 base = (id + 0.2 + h * 0.6) * 0.4;
			float angle = (h.y - 0.5) * 0.7;
			vec2 q = mat2(cos(angle), sin(angle), -sin(angle), cos(angle)) * (world - base);
			blades = min(blades, grass_blade(q + vec2(0.035, 0.0), -1.0, 0.12));
			blades = min(blades, grass_blade(q, h.x < 0.5 ? -1.0 : 1.0, 0.15));
			blades = min(blades, grass_blade(q - vec2(0.035, 0.0), 1.0, 0.12));
		}
	}
	float aa = fwidth(blades);
	return mix(${vec3(colour('grass-blade'))}, color, smoothstep(0.018 - aa, 0.018 + aa, blades));`;
		}
	},
	brush: {
		get flat() {
			return colour('brush');
		},
		get glsl() {
			return `
	float n = 0.5 * perlin(world * 0.5) + 0.3 * perlin(world * 1.2) + 0.2 * perlin(world * 2.6) + 0.12 * perlin(world * 6.0) + 0.08 * perlin(world * 14.0);
	vec3 color = mix(${vec3(colour('brush-low'))}, ${vec3(colour('brush-high'))}, clamp(n * 0.9 + 0.5, 0.0, 1.0));
	float blades = 1e9;
	vec2 cell = floor(world / 0.4);
	for (int y = -1; y <= 1; y++) {
		for (int x = -1; x <= 1; x++) {
			vec2 id = cell + vec2(x, y);
			vec2 h = hash2(id + 41.7);
			if (fract(h.x * 7.13) > 0.4) continue;
			vec2 base = (id + 0.2 + h * 0.6) * 0.4;
			float angle = (h.y - 0.5) * 1.2;
			vec2 q = mat2(cos(angle), sin(angle), -sin(angle), cos(angle)) * (world - base);
			blades = min(blades, grass_blade(q + vec2(0.035, 0.0), -1.0, 0.1));
			blades = min(blades, grass_blade(q, h.x < 0.5 ? -1.0 : 1.0, 0.13));
			blades = min(blades, grass_blade(q - vec2(0.035, 0.0), 1.0, 0.1));
		}
	}
	float aa = fwidth(blades);
	return mix(${vec3(colour('brush-blade'))}, color, smoothstep(0.016 - aa, 0.016 + aa, blades));`;
		}
	},
	sand: {
		get flat() {
			return colour('sand');
		},
		get glsl() {
			return `
	vec2 p = world * 2.5;
	float wave = sin(p.y * 3.0 + p.x * 0.8 + perlin(p * 0.6) * 4.0);
	float aa = fwidth(wave);
	return mix(${vec3(colour('sand'))}, ${vec3(colour('sand-ripple'))}, smoothstep(0.7 - aa, 0.7 + aa, wave));`;
		}
	},
	concrete: {
		get flat() {
			return colour('concrete');
		},
		get glsl() {
			return `
	float n = 0.6 * perlin(world * 1.5) + 0.3 * perlin(world * 5.0) + 0.1 * perlin(world * 20.0);
	vec3 color = ${vec3(colour('concrete'))} * (1.0 + n * 0.08);
	float specks = 1e9;
	float tone = 0.0;
	vec2 cell = floor(world / 0.1);
	for (int y = -1; y <= 1; y++) {
		for (int x = -1; x <= 1; x++) {
			vec2 id = cell + vec2(x, y);
			vec2 h = hash2(id);
			if (fract(h.x * 5.31) > 0.4) continue;
			float d = length(world - (id + h) * 0.1) - (0.006 + fract(h.y * 3.77) * 0.012);
			if (d < specks) {
				specks = d;
				tone = fract(h.x * 11.3);
			}
		}
	}
	float aa = fwidth(specks);
	vec3 speck = tone > 0.5 ? ${vec3(colour('concrete-speck-light'))} : ${vec3(colour('concrete-speck-dark'))};
	color = mix(color, speck, (1.0 - smoothstep(-aa, aa, specks)) * 0.7);
	vec2 grid = abs(fract(world / 1.5 + 0.5) - 0.5) * 1.5;
	float joint = min(grid.x, grid.y);
	float ja = fwidth(joint);
	return mix(color * 0.82, color, smoothstep(0.008 - ja, 0.008 + ja, joint));`;
		}
	},
	dirt: {
		get flat() {
			return colour('dirt');
		},
		get glsl() {
			return `
	float n = 0.5 * perlin(world * 0.5) + 0.3 * perlin(world * 1.2) + 0.2 * perlin(world * 2.6) + 0.12 * perlin(world * 6.0) + 0.08 * perlin(world * 14.0);
	vec3 color = mix(${vec3(colour('dirt-low'))}, ${vec3(colour('dirt-high'))}, clamp(n * 0.9 + 0.5, 0.0, 1.0));
	float specks = 1e9;
	vec2 cell = floor(world / 0.1);
	for (int y = -1; y <= 1; y++) {
		for (int x = -1; x <= 1; x++) {
			vec2 id = cell + vec2(x, y);
			vec2 h = hash2(id);
			if (fract(h.x * 5.31) > 0.4) continue;
			specks = min(specks, length(world - (id + h) * 0.1) - (0.006 + fract(h.y * 3.77) * 0.012));
		}
	}
	float aa = fwidth(specks);
	return mix(${vec3(colour('dirt-speck'))}, color, smoothstep(-aa, aa, specks));`;
		}
	},
	ocean: {
		get flat() {
			return colour('ocean');
		},
		get glsl() {
			return `
	vec2 at = world;
	float r2 = dot(local, local);
	if (g_rock && r2 > 0.0) at += mix(local * vec2(0.16, -0.16) / max(r2, 0.09), vec2(0.0), smoothstep(0.45, 0.85, sqrt(r2)));
	vec2 p = at * 0.9;
	float n = perlin(p + u_time * vec2(0.06, 0.025) + perlin(p * 0.5 - u_time * vec2(0.02, 0.03)) * 0.6);
	if (g_rock) n = max(n, 0.36 + 0.05 * sin(u_time * 0.6 + atan(local.y, local.x) * 3.0) - sqrt(r2));
	float aa = fwidth(n);
	return mix(${vec3(colour('ocean-deep'))}, ${vec3(colour('ocean-light'))}, smoothstep(0.05 - aa, 0.05 + aa, n));`;
		}
	}
};

export const NAMES = Object.keys(BASES);
