export interface Base {
	glsl: string;
	flat: string;
	functions?: string;
}

export const LIBRARY = `
uniform float u_time;
vec2 g_flow = vec2(0.327, 0.945);
vec2 g_flows[7];
float g_weights[7];
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

export const BASES: Record<string, Base> = {
	plain: { glsl: 'return vec3(34.0, 43.0, 61.0) / 255.0;', flat: '#222b3d' },
	grass: {
		flat: '#3ba250',
		functions: `
float grass_blade(vec2 q, float side, float radius) {
	vec2 v = q - vec2(side * radius, 0.0);
	float theta = atan(-v.y, -side * v.x);
	if (theta >= 0.0 && theta <= 0.9) return abs(length(v) - radius);
	vec2 tip = vec2(side * radius * (1.0 - cos(0.9)), -radius * sin(0.9));
	return min(length(q), length(q - tip));
}`,
		glsl: `
	float n = 0.5 * perlin(world * 0.5) + 0.3 * perlin(world * 1.2) + 0.2 * perlin(world * 2.6) + 0.12 * perlin(world * 6.0) + 0.08 * perlin(world * 14.0);
	vec3 color = mix(vec3(44.0, 144.0, 65.0), vec3(75.0, 181.0, 96.0), clamp(n * 0.9 + 0.5, 0.0, 1.0)) / 255.0;
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
	return mix(vec3(21.0, 159.0, 49.0) / 255.0, color, smoothstep(0.018 - aa, 0.018 + aa, blades));`
	},
	savannah: {
		flat: '#8f9c50',
		glsl: `
	float n = 0.5 * perlin(world * 0.5) + 0.3 * perlin(world * 1.2) + 0.2 * perlin(world * 2.6) + 0.12 * perlin(world * 6.0) + 0.08 * perlin(world * 14.0);
	vec3 color = mix(vec3(118.0, 140.0, 66.0), vec3(168.0, 176.0, 94.0), clamp(n * 0.9 + 0.5, 0.0, 1.0)) / 255.0;
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
	return mix(vec3(186.0, 180.0, 100.0) / 255.0, color, smoothstep(0.016 - aa, 0.016 + aa, blades));`
	},
	sand: {
		flat: '#dfcda1',
		glsl: `
	vec2 p = world * 2.5;
	float wave = sin(p.y * 3.0 + p.x * 0.8 + perlin(p * 0.6) * 4.0);
	float aa = fwidth(wave);
	return mix(vec3(223.0, 205.0, 161.0), vec3(206.0, 193.0, 164.0), smoothstep(0.7 - aa, 0.7 + aa, wave)) / 255.0;`
	},
	concrete: {
		flat: '#6c6e71',
		glsl: `
	float n = 0.6 * perlin(world * 1.5) + 0.3 * perlin(world * 5.0) + 0.1 * perlin(world * 20.0);
	vec3 color = vec3(108.0, 110.0, 113.0) / 255.0 * (1.0 + n * 0.08);
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
	vec3 speck = tone > 0.5 ? vec3(131.0, 138.0, 147.0) / 255.0 : vec3(88.0, 90.0, 94.0) / 255.0;
	color = mix(color, speck, (1.0 - smoothstep(-aa, aa, specks)) * 0.7);
	vec2 grid = abs(fract(world / 1.5 + 0.5) - 0.5) * 1.5;
	float joint = min(grid.x, grid.y);
	float ja = fwidth(joint);
	return mix(color * 0.82, color, smoothstep(0.008 - ja, 0.008 + ja, joint));`
	},
	dirt: {
		flat: '#3e3633',
		glsl: `
	float n = 0.5 * perlin(world * 0.5) + 0.3 * perlin(world * 1.2) + 0.2 * perlin(world * 2.6) + 0.12 * perlin(world * 6.0) + 0.08 * perlin(world * 14.0);
	vec3 color = mix(vec3(56.0, 50.0, 48.0), vec3(68.0, 58.0, 55.0), clamp(n * 0.9 + 0.5, 0.0, 1.0)) / 255.0;
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
	return mix(vec3(35.0, 29.0, 27.0) / 255.0, color, smoothstep(-aa, aa, specks));`
	},
	ocean: {
		flat: '#305375',
		glsl: `
	vec2 p = world * 1.6;
	float warp = perlin(p * 0.5 + vec2(u_time * 0.04, 0.0)) * 2.2 + perlin(p * 1.3 + 11.0 - vec2(0.0, u_time * 0.07)) * 0.6;
	float wave = 0.0;
	float total = 0.0;
	for (int i = 0; i < 7; i++) {
		if (g_weights[i] <= 0.0) continue;
		wave += g_weights[i] * (0.5 + 0.5 * sin(dot(p, g_flows[i]) * 2.75 + warp * 2.0 - u_time * 0.9));
		total += g_weights[i];
	}
	wave /= total;
	float ramp = smoothstep(0.15, 0.95, wave);
	vec3 color = mix(vec3(48.0, 83.0, 117.0), vec3(54.0, 99.0, 128.0), ramp) / 255.0;
	float grain = 0.6 * perlin(world * 9.0 + u_time * 0.15) + 0.4 * perlin(world * 23.0 - u_time * 0.2);
	return color * (1.0 + grain * 0.035);`
	}
};

export const NAMES = Object.keys(BASES);
