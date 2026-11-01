import { BASES, LIBRARY, NAMES, NOISE } from '#lib/bases.js';
import { MAP, css, rgb, vec3 } from '#lib/colors.js';
import { CORNERS, FRONT, HEIGHT, WIDTH } from '#lib/map.js';
import { perlin } from '#lib/noise.js';
import mountain from '#lib/assets/images/mountain.png';
import building1 from '#lib/assets/images/building1.png';
import building2 from '#lib/assets/images/building2.png';
import house1 from '#lib/assets/images/house1.png';
import house2 from '#lib/assets/images/house2.png';
import tree from '#lib/assets/images/tree.png';
import cactus from '#lib/assets/images/cactus.png';
import chunker from '#lib/assets/images/chunker.png';
import crawler from '#lib/assets/images/crawler.png';
import hopper from '#lib/assets/images/hopper.png';

export const STRIDE = 18;
export const THIN = 1;
export const CONTOUR = 2;
export const HIDDEN = 3;
export const TREE_STRIDE = 7;
export const RAMP_STRIDE = 12;
export const FOG_STRIDE = 4;
export const ROCK = 1;

const SPRITES = [
	{ name: 'mountain', src: mountain, size: [256, 256], anchor: [112, 200], unit: 1.9 / 256, flat: true },
	{ name: 'building1', src: building1, size: [64, 128], anchor: [28.5, 128], unit: 0.016, flat: false },
	{ name: 'building2', src: building2, size: [64, 128], anchor: [33, 128], unit: 0.016, flat: false },
	{ name: 'house1', src: house1, size: [256, 256], anchor: [132, 234], unit: 0.003, flat: false },
	{ name: 'house2', src: house2, size: [256, 256], anchor: [125, 218], unit: 0.003, flat: false },
	{ name: 'tree', src: tree, size: [128, 128], anchor: [68, 127], unit: 0.0034, flat: false },
	{ name: 'cactus', src: cactus, size: [128, 128], anchor: [63, 127], unit: 0.004, flat: false },
	{ name: 'chunker', src: chunker, size: [128, 128], anchor: [66, 124], unit: 0.011, flat: false },
	{ name: 'crawler', src: crawler, size: [128, 128], anchor: [62, 112], unit: 0.0055, flat: false },
	{ name: 'hopper', src: hopper, size: [128, 128], anchor: [66, 110], unit: 0.0055, flat: false }
] as const;
export const sprite = (name: (typeof SPRITES)[number]['name']) => 2 + SPRITES.findIndex((sprite) => sprite.name === name);
const TREE = sprite('tree');

const VARIANTS = ['highway', 'street', 'mine', 'beach', 'farmland', 'sunker'];
const flag = (variant: string) => 1 << VARIANTS.indexOf(variant);
const hovering = (value: number) => value > 0 || value < -1.5;
export const pack = (variants: string[]) => variants.reduce((mask, variant) => (VARIANTS.includes(variant) ? mask | flag(variant) : mask), 0);
const FLAGS = VARIANTS.map((variant) => `const int ${variant.toUpperCase()} = ${flag(variant)};`).join('\n');

const float = (n: number) => (Number.isInteger(n) ? `${n}.0` : `${n}`);
const SPRITE_CONSTANTS = `
const vec2 SPRITE_SIZES[${SPRITES.length}] = vec2[${SPRITES.length}](${SPRITES.map(({ size }) => `vec2(${size.map(float)})`)});
const vec2 SPRITE_ANCHORS[${SPRITES.length}] = vec2[${SPRITES.length}](${SPRITES.map(({ anchor }) => `vec2(${anchor.map(float)})`)});
const float SPRITE_UNITS[${SPRITES.length}] = float[${SPRITES.length}](${SPRITES.map(({ unit }) => float(unit))});
const bool SPRITE_FLAT[${SPRITES.length}] = bool[${SPRITES.length}](${SPRITES.map(({ flat }) => flat)});`;

const BORDERS: Record<number, { width: number; focus: number; alpha: number }> = { [THIN]: { width: 0.015, focus: 0.035, alpha: 0.4 }, [CONTOUR]: { width: 0.08, focus: 0.1, alpha: 1 } };

type Screen = { width: number; height: number; ratio: [number, number] };
type Plane = { width: number; height: number; scale: number; x: number; y: number };
type Camera = { perspective: number; tilt: number };

export interface Renderer {
	animated: boolean;
	resize(screen: Screen, plane: Plane, camera: Camera): void;
	plant(trees: Float32Array): void;
	pave(ramps: Float32Array): void;
	shroud(clouds: Float32Array): void;
	fill(data: Float32Array): void;
	draw(time?: number): void;
	destroy(): void;
}

const PROJECT = `
uniform vec3 u_view;
uniform vec2 u_plane;
uniform vec2 u_screen;
uniform vec3 u_camera;
uniform vec2 u_depth;
vec4 project(vec2 world) {
	vec2 relative = world * u_view.x + u_view.yz - u_plane / 2.0;
	float w = 1.0 - relative.y * u_camera.z / u_camera.x;
	vec2 screen = vec2(relative.x, relative.y * u_camera.y) / (u_screen / 2.0);
	return vec4(screen.x, -screen.y, 0.0, w);
}
const float OVERLAY = 0.0002;
vec4 layer(vec4 position, float y, float bias) {
	position.z = ((1.0 - 2.0 * clamp((y - u_depth.x) / u_depth.y, 0.0, 1.0)) * 0.999 - bias) * position.w;
	return position;
}`;

const VERTEX = `#version 300 es
in vec2 a_position;
in float a_raise;
in float a_kind;
in float a_along;
in vec2 i_center;
in float i_lift;
in vec3 i_top;
in vec3 i_side;
in vec4 i_outline;
in float i_highlight;
in float i_surface;
in float i_edges;
in float i_slopes;
in float i_arms;
uniform vec2 u_seed;
${PROJECT}
${NOISE}
out vec2 v_local;
out float v_kind;
out float v_along;
out float v_height;
out vec3 v_top;
out vec3 v_side;
out vec4 v_outline;
out float v_highlight;
out vec2 v_world;
flat out float v_surface;
flat out float v_edges;
flat out float v_slopes;
flat out float v_arms;
flat out vec3 v_nodes;
void main() {
	float lift = i_lift;
	vec3 top = i_top;
	vec3 side = i_side;
	if (i_surface < 0.5) {
		float t = u_time * 12.5;
		vec2 p = i_center + u_seed;
		float n = 0.65 * perlin(p * 0.07 + t * vec2(0.012, 0.006)) + 0.35 * perlin(p * 0.16 + vec2(31.0, 17.0) + t * vec2(-0.008, 0.01));
		n = clamp(n * 1.4 + 0.5, 0.0, 1.0);
		float v = (20.0 + n * 22.0) / 255.0;
		lift *= n;
		top = v + vec3(0.0, 1.0, 2.0) / 255.0;
		side = v * 0.7 + vec3(0.0, 0.0, 1.0) / 255.0;
	}
	vec2 position = a_kind > 0.5 ? a_position * 1.005 : a_position;
	gl_Position = layer(project(i_center + position - vec2(0.0, a_raise * lift)), i_center.y, 0.0);
	v_local = position;
	v_kind = a_kind;
	v_along = a_along;
	v_height = a_raise * lift;
	v_top = top;
	v_side = side;
	v_outline = i_outline;
	v_highlight = i_highlight;
	v_world = i_center + position;
	v_surface = i_surface;
	v_edges = i_edges;
	v_slopes = i_slopes;
	v_arms = i_arms;
	v_nodes = i_outline.rgb;
}`;

const SURFACES = [LIBRARY, ...NAMES.map((name, i) => `${BASES[name].functions ?? ''}\nvec3 base_${i}(vec2 world, vec2 local) { ${BASES[name].glsl} }`)].join('\n');
const DISPATCH = NAMES.map((_, i) => `\tif (id < ${i + 1.5}) return base_${i}(world, local);`).join('\n');

const FRAGMENT = `#version 300 es
precision highp float;
in vec2 v_local;
in float v_kind;
in float v_along;
in float v_height;
in vec3 v_top;
in vec3 v_side;
in vec4 v_outline;
in float v_highlight;
in vec2 v_world;
flat in float v_surface;
flat in float v_edges;
flat in float v_slopes;
flat in float v_arms;
flat in vec3 v_nodes;
out vec4 color;
const vec3 SEAM = ${vec3(MAP.background)};
const vec3 HIGHLIGHT = ${vec3(MAP.highlight)};
const vec3 HOSTILE = ${vec3(MAP.hostile)};
const vec3 HOSTILE_BORDER = ${vec3(MAP.hostileBorder)};
const vec3 HOSTILE_HOVER = ${vec3(MAP.hostileHover)};
const vec3 HOSTILE_LINE = ${vec3(MAP.hostileLine)};
const vec3 LINE = ${vec3(MAP.line)};
const vec3 ASPHALT = ${vec3(MAP.asphalt)};
const vec3 PAINT = ${vec3(MAP.paint)};
const vec3 HEDGE = ${vec3(MAP.hedge)};
const vec3 CROPS[${MAP.crops.length}] = vec3[${MAP.crops.length}](${MAP.crops.map(vec3)});
${FLAGS}
const vec2 NORMALS[6] = vec2[6](vec2(0.5, -0.8660254), vec2(1.0, 0.0), vec2(0.5, 0.8660254), vec2(-0.5, 0.8660254), vec2(-1.0, 0.0), vec2(-0.5, -0.8660254));
vec3 dither(vec3 rgb) {
	float noise = fract(52.9829189 * fract(dot(gl_FragCoord.xy, vec2(0.06711056, 0.00583715))));
	return rgb + (noise - 0.5) / 255.0;
}
${SURFACES}
float hexagon(vec2 p) {
	return max(abs(p.x), max(abs(dot(p, vec2(0.5, 0.8660254))), abs(dot(p, vec2(0.5, -0.8660254)))));
}
const vec2 HEX[6] = vec2[6](vec2(0.0, -1.0), vec2(0.8660254, -0.5), vec2(0.8660254, 0.5), vec2(0.0, 1.0), vec2(-0.8660254, 0.5), vec2(-0.8660254, -0.5));
float segment(vec2 p, vec2 a, vec2 b) {
	vec2 ab = b - a;
	return length(p - a - ab * clamp(dot(p - a, ab) / max(dot(ab, ab), 1e-6), 0.0, 1.0));
}
vec2 lane(vec2 p, vec2 a, vec2 b) {
	vec2 ab = b - a;
	float t = clamp(dot(p - a, ab) / max(dot(ab, ab), 1e-6), 0.0, 1.0);
	return vec2(length(p - a - ab * t), t);
}
void pave(inout float road, inout float line, vec2 p, vec2 a, vec2 b, bool wa, bool wb) {
	vec2 l = lane(p, a, b);
	road = min(road, l.x - (wa && wb ? 0.36 : wa || wb ? mix(0.36, 0.2, smoothstep(0.0, 0.3, wa ? l.y : 1.0 - l.y)) : 0.2));
	if (wa && wb) line = min(line, l.x);
}
bool longest(vec2 a, vec2 b, vec2 c) {
	return distance(a, b) > distance(a, c) + 0.01 && distance(a, b) > distance(b, c) + 0.01;
}
bool inside(vec2 p, vec2 a, vec2 b, vec2 c) {
	float u = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
	float v = (c.x - b.x) * (p.y - b.y) - (c.y - b.y) * (p.x - b.x);
	float w = (a.x - c.x) * (p.y - c.y) - (a.y - c.y) * (p.x - c.x);
	return (u >= 0.0 && v >= 0.0 && w >= 0.0) || (u <= 0.0 && v <= 0.0 && w <= 0.0);
}
bool dropped(vec2 nodes[7], int hubs, int a, int b, int c) {
	int count = ((hubs >> a) & 1) + ((hubs >> b) & 1) + ((hubs >> c) & 1);
	return count == 1 ? ((hubs >> c) & 1) == 1 : longest(nodes[a], nodes[b], nodes[c]);
}
float pit(vec2 p, int bits) {
	float d = 1e9;
	int nearest = -1;
	float best = dot(p, p);
	for (int i = 0; i < 6; i++) {
		vec2 center = NORMALS[i] * 1.7320508;
		if (dot(p - center, p - center) < best) {
			best = dot(p - center, p - center);
			nearest = i;
		}
		vec2 a = HEX[i];
		vec2 b = HEX[(i + 1) % 6];
		if (((bits >> i) & 1) == 0) {
			d = min(d, segment(p, a, b));
			continue;
		}
		int after = (i + 1) % 6;
		int before = (i + 5) % 6;
		if (((bits >> after) & 1) == 0) d = min(d, segment(p, b, center + NORMALS[after] * 1.7320508 - b));
		if (((bits >> before) & 1) == 0) d = min(d, segment(p, a, center + NORMALS[before] * 1.7320508 - a));
	}
	return nearest < 0 || ((bits >> nearest) & 1) == 1 ? d : -d;
}
vec3 mine(vec2 p, float y, vec3 rgb, int bits) {
	float aa = length(fwidth(p));
	vec3 result = rgb;
	vec2 o = vec2(0.0);
	float shade = 1.0;
	for (int i = 0; i < 4; i++) {
		float depth = 0.116 + float(i) * 0.17;
		float rim = smoothstep(-aa, aa, pit(p - o, bits) - depth);
		vec3 wall = rgb * shade * 0.62 * (1.0 + 0.1 * sin(y * 270.0));
		o.y += 0.07;
		shade *= 0.86;
		float ground = smoothstep(-aa, aa, pit(p - o, bits) - depth);
		result = mix(result, mix(wall, rgb * shade, ground), rim);
	}
	return result;
}
vec3 farmland(vec2 w) {
	vec2 cell = floor(w / 1.4);
	float first = 1e9;
	float second = 1e9;
	vec2 id = cell;
	for (int y = -1; y <= 1; y++) {
		for (int x = -1; x <= 1; x++) {
			vec2 c = cell + vec2(x, y);
			float d = length(w - (c + 0.15 + hash2(c) * 0.7) * 1.4);
			if (d < first) {
				second = first;
				first = d;
				id = c;
			} else if (d < second) second = d;
		}
	}
	vec2 h = hash2(id + 7.3);
	vec3 crop = CROPS[h.x < 0.35 ? 0 : h.x < 0.6 ? 1 : h.x < 0.85 ? 2 : 3];
	float angle = floor(h.y * 4.0) * 0.785398;
	float u = dot(w, vec2(cos(angle), sin(angle))) / 0.16;
	float aa = fwidth(u);
	float f = fract(u);
	float row = smoothstep(0.2 - aa, 0.2 + aa, f) * (1.0 - smoothstep(0.75 - aa, 0.75 + aa, f));
	float grain = 0.6 * perlin(w * 14.0) + 0.4 * perlin(w * 33.0);
	vec3 rgb = mix(crop * 0.72, crop, row) * (1.0 + grain * 0.06);
	float edge = second - first;
	float ea = fwidth(edge);
	return mix(HEDGE, rgb, smoothstep(0.05 - ea, 0.05 + ea, edge));
}
vec2 spot(int bits) {
	int x = bits & 15;
	return x > 8 ? vec2(sin(float(x - 9) * 1.0471976), -cos(float(x - 9) * 1.0471976)) : vec2(float(x - 4) * 0.2165064, float(((bits >> 4) & 7) - 2) * 0.375);
}
vec3 paved(vec3 rgb, vec2 local, int edges, bool highway) {
	int roads = (int(v_nodes[2]) >> 15) & 127;
	if (roads != 0) {
		vec2 nodes[7];
		bool highways[7];
		for (int k = 0; k < 7; k++) {
			int bits = (int(v_nodes[k / 3]) >> (8 * (k % 3))) & 255;
			nodes[k] = (k > 0 ? NORMALS[k - 1] * 1.7320508 : vec2(0.0)) + spot(bits);
			highways[k] = (bits >> 7) == 1;
		}
		int arms = ((highway ? int(v_arms) >> 12 : 0) | (edges >> 18)) & 63;
		int hubs = (int(v_nodes[2]) >> 8) & 127;
		bool own = (roads & 1) == 1;
		bool mixed = ((int(v_nodes[0]) >> 4) & 1) == 1;
		float road = own && !mixed ? length(local - nodes[0]) - (highway ? 0.36 : 0.2) : 1e9;
		float line = 1e9;
		for (int i = 0; i < 6; i++) {
			if (!own && ((roads >> (i + 1)) & 1) == 1) {
				int after = (i + 1) % 6 + 1;
				road = min(road, length(local - nodes[i + 1]) - (highways[i + 1] ? 0.36 : 0.2));
				if (((roads >> after) & 1) == 0 || ((int(v_slopes) >> i) & 1) == 1) continue;
				pave(road, line, local, nodes[i + 1], nodes[after], highways[i + 1], highways[after]);
			}
			if (!own || ((arms >> i) & 1) == 0) continue;
			int next = (i + 1) % 6;
			int previous = (i + 5) % 6;
			bool linked = ((arms >> next) & 1) == 1;
			bool sloped = ((int(v_slopes) >> i) & 1) == 1;
			if (!sloped && !(linked && dropped(nodes, hubs, 0, i + 1, next + 1)) && !(((arms >> previous) & 1) == 1 && dropped(nodes, hubs, 0, i + 1, previous + 1))) pave(road, line, local, nodes[0], nodes[i + 1], highways[0], highways[i + 1]);
			if (mixed || !linked || dropped(nodes, hubs, i + 1, next + 1, 0)) continue;
			if (!dropped(nodes, hubs, 0, i + 1, next + 1) && !dropped(nodes, hubs, 0, next + 1, i + 1) && inside(local, nodes[0], nodes[i + 1], nodes[next + 1])) road = min(road, -1.0);
			pave(road, line, local, nodes[i + 1], nodes[next + 1], highways[i + 1], highways[next + 1]);
		}
		float ra = length(fwidth(local));
		rgb = mix(rgb, mix(PAINT, ASPHALT, smoothstep(0.025 - ra, 0.025 + ra, line)), 1.0 - smoothstep(-ra, ra, road));
	}
	return rgb;
}
vec3 surface(float id, vec2 world, vec2 local, vec3 top) {
	if (id < 0.5) return top;
${DISPATCH}
	return top;
}
void main() {
	if (v_kind > 0.5) {
		float edge = 0.8660254 - max(abs(v_local.x), max(abs(dot(v_local, vec2(0.5, 0.8660254))), abs(dot(v_local, vec2(0.5, -0.8660254)))));
		float aa = fwidth(edge);
		int packed = int(v_surface + 0.5);
		int variants = packed / 1000;
		g_rock = (variants & SUNKER) != 0;
		vec3 base = (variants & FARMLAND) != 0 ? farmland(v_world) : surface(float(packed % 1000), v_world, v_local, v_top);
		if (v_highlight < 0.0) base = mix(base, HOSTILE, 0.3);
		int edges = int(v_edges);
		if ((variants & MINE) != 0) base = mine(v_local, v_world.y, base, int(v_arms) & 63);
		if ((variants & BEACH) != 0) {
			vec3 sand = surface(${NAMES.indexOf('sand') + 1}.0, v_world, v_local, v_top);
			float shore = 0.0;
			for (int i = 0; i < 6; i++) {
				if (((int(v_arms) >> (6 + i)) & 1) == 0) continue;
				float distance = 0.8660254 - dot(v_local, NORMALS[i]);
				float da = fwidth(distance);
				float width = 0.2 + 0.05 * perlin(v_world * 3.0);
				shore = max(shore, 1.0 - smoothstep(width - da, width + da, distance));
			}
			base = mix(base, sand, shore);
		}
		bool hostile = v_highlight < 0.0;
		float focus = hostile ? step(1.5, -v_highlight) : v_highlight;
		vec3 fill = mix(base, hostile ? HOSTILE : HIGHLIGHT, focus * 0.3);
		vec3 rgb = v_outline.a > 0.0 ? mix(v_outline.rgb, fill, smoothstep(v_outline.a - aa, v_outline.a + aa, edge)) : fill;
		float thin = 0.0;
		float contour = 0.0;
		float border = 0.0;
		float open = 1.0;
		float thinWidth = mix(${float(BORDERS[THIN].width)}, ${float(BORDERS[THIN].focus)}, focus);
		float contourWidth = mix(${float(BORDERS[CONTOUR].width)}, ${float(BORDERS[CONTOUR].focus)}, focus);
		for (int i = 0; i < 6; i++) {
			int kind = (edges >> (2 * i)) & 3;
			float distance = 0.8660254 - dot(v_local, NORMALS[i]);
			float da = fwidth(distance);
			if (kind == ${THIN}) thin = max(thin, 1.0 - smoothstep(thinWidth - da, thinWidth + da, distance));
			if ((kind & 1) == 0) open = min(open, distance);
			if (kind == ${CONTOUR}) contour = max(contour, 1.0 - smoothstep(contourWidth - da, contourWidth + da, distance));
			if (((int(v_arms) >> (18 + i)) & 1) == 1) border = max(border, 1.0 - smoothstep(contourWidth - da, contourWidth + da, distance));
		}
		vec3 line = hostile ? mix(HOSTILE_LINE, HOSTILE_HOVER, focus) : mix(LINE, HIGHLIGHT, focus);
		rgb = mix(rgb, line, thin * mix(${float(BORDERS[THIN].alpha)}, 1.0, focus));
		rgb = mix(rgb, line, contour);
		rgb = mix(rgb, HOSTILE_BORDER, border);
		rgb = paved(rgb, v_local, edges, (variants & HIGHWAY) != 0);
		color = vec4(dither(rgb), smoothstep(0.0, fwidth(open), open));
	} else {
		float seam = min(min(v_along, 1.0 - v_along), v_height);
		float aa = fwidth(seam);
		int edge = int(-v_kind + 0.5);
		bool step = ((int(v_edges) >> (2 * edge)) & 1) == 1;
		int packed = int(v_surface + 0.5);
		int variants = packed / 1000;
		vec3 top = (variants & FARMLAND) != 0 ? farmland(v_world) : surface(float(packed % 1000), v_world, v_local, v_top);
		bool highway = (variants & HIGHWAY) != 0;
		bool sloped = ((int(v_slopes) >> edge) & 1) == 1;
		bool hostile = v_highlight < 0.0;
		float focus = hostile ? float(-v_highlight >= 1.5) : v_highlight;
		vec3 line = hostile ? mix(HOSTILE_LINE, HOSTILE_HOVER, focus) : mix(LINE, HIGHLIGHT, focus);
		vec3 face = ((int(v_edges) >> (2 * edge)) & 3) == ${THIN} ? mix(top, line, mix(${float(BORDERS[THIN].alpha)}, 1.0, focus)) : top;
		vec3 side = step ? (sloped ? face : paved(face, v_local, int(v_edges), highway)) : mix(SEAM, v_side, smoothstep(0.02 - aa, 0.02 + aa, seam));
		color = vec4(dither(side), 1.0);
	}
}`;

const RAMP_VERTEX = `#version 300 es
in vec2 a_corner;
in vec3 i_from;
in vec3 i_to;
in float i_wide;
in vec2 i_bend;
in float i_tile;
in vec2 i_slope;
${PROJECT}
out float v_across;
out float v_along;
out vec2 v_spot;
flat out float v_length;
flat out float v_wide;
flat out vec4 v_arms;
void main() {
	int wide = int(i_wide + 0.5);
	v_wide = i_wide;
	if (wide >= 4) {
		v_spot = a_corner * 0.7;
		v_arms = vec4(i_to.xy, i_bend) - i_from.xyxy;
		gl_Position = layer(project(i_from.xy + v_spot - vec2(0.0, i_from.z + dot(v_spot, i_slope))), i_tile, OVERLAY);
		return;
	}
	vec2 widths = vec2((wide & 1) == 1 ? 0.36 : 0.2, (wide & 2) == 2 ? 0.36 : 0.2);
	float width = a_corner.x < 0.0 ? widths.x : widths.y;
	float span = max(distance(i_to.xy, i_from.xy), 1e-4);
	vec2 along = (i_to.xy - i_from.xy) / span;
	float reach = a_corner.x < 0.0 ? -width : span + width;
	vec3 point = mix(i_from, i_to, reach / span);
	gl_Position = layer(project(point.xy + vec2(-along.y, along.x) * a_corner.y * width - vec2(0.0, point.z)), i_tile, OVERLAY);
	v_across = a_corner.y * width;
	v_along = reach;
	v_length = span;
}`;

const RAMP_FRAGMENT = `#version 300 es
precision highp float;
in float v_across;
in float v_along;
in vec2 v_spot;
flat in float v_length;
flat in float v_wide;
flat in vec4 v_arms;
uniform float u_paint;
out vec4 color;
const vec3 ASPHALT = ${vec3(MAP.asphalt)};
const vec3 PAINT = ${vec3(MAP.paint)};
float arm(vec2 p, vec2 b) {
	return length(p - b * clamp(dot(p, b) / dot(b, b), 0.0, 1.0));
}
void main() {
	int kind = int(v_wide + 0.5);
	if (kind >= 4) {
		if (u_paint > 0.5) discard;
		float a = arm(v_spot, v_arms.xy) - ((kind & 1) == 1 ? 0.36 : 0.2);
		float b = arm(v_spot, v_arms.zw) - ((kind & 2) == 2 ? 0.36 : 0.2);
		float h = clamp(0.5 + 0.5 * (b - a) / 0.15, 0.0, 1.0);
		float s = mix(b, a, h) - 0.15 * h * (1.0 - h);
		float edge = fwidth(s);
		color = vec4(ASPHALT, 1.0 - smoothstep(-edge, 0.0, s));
		return;
	}
	float d = length(vec2(max(max(-v_along, v_along - v_length), 0.0), v_across));
	float aa = fwidth(d);
	if (u_paint > 0.5) {
		if (int(v_wide + 0.5) != 3) discard;
		color = vec4(PAINT, 1.0 - smoothstep(0.025 - aa, 0.025 + aa, d));
		return;
	}
	int wide = int(v_wide + 0.5);
	float t = clamp(v_along / v_length, 0.0, 1.0);
	float width = wide == 3 ? 0.36 : wide == 0 ? 0.2 : mix(0.36, 0.2, smoothstep(0.0, 0.3, wide == 1 ? t : 1.0 - t));
	color = vec4(ASPHALT, 1.0 - smoothstep(width - aa, width, d));
}`;

const TREE_VERTEX = `#version 300 es
in vec2 a_corner;
in vec3 i_root;
in vec3 i_tree;
in float i_tile;
uniform float u_shadow;${SPRITE_CONSTANTS}
${PROJECT}
out vec2 v_offset;
out vec2 v_uv;
flat out float v_seed;
flat out float v_kind;
void main() {
	vec2 root = i_root.xy - vec2(0.0, i_root.z);
	if (u_shadow > 0.5) {
		gl_Position = i_tree.z > 0.5 && i_tree.z < ${TREE - 0.5} ? vec4(0.0) : project(root + a_corner * vec2(0.26, 0.1) * i_tree.x);
		v_offset = a_corner;
	} else {
		vec2 uv = vec2(a_corner.x * 0.5 + 0.5, 0.5 - a_corner.y * 0.5);
		int sprite = int(i_tree.z + 0.5) - 2;
		vec2 offset = sprite >= 0 ? (uv * SPRITE_SIZES[sprite] - SPRITE_ANCHORS[sprite]) * SPRITE_UNITS[sprite] : vec2(a_corner.x * 0.75, mix(0.05, -0.8, a_corner.y * 0.5 + 0.5));
		gl_Position = project(root);
		gl_Position.xy += vec2(offset.x, -offset.y) * i_tree.x * u_view.x / (u_screen / 2.0);
		if (sprite >= 0 && SPRITE_FLAT[sprite]) gl_Position = project(root + offset * i_tree.x);
		v_offset = offset;
		v_uv = uv;
	}
	gl_Position = layer(gl_Position, i_tile, OVERLAY);
	v_seed = i_tree.y;
	v_kind = i_tree.z;
}`;

const TREE_FRAGMENT = `#version 300 es
precision highp float;
in vec2 v_offset;
in vec2 v_uv;
flat in float v_seed;
flat in float v_kind;
uniform float u_shadow;
uniform sampler2D u_sprites[${SPRITES.length}];
out vec4 color;
${LIBRARY}
float cross2(vec2 a, vec2 b) {
	return a.x * b.y - a.y * b.x;
}
vec4 rock(vec2 p, float seed) {
	vec2 center = vec2(0.0, -0.3);
	vec2 peak = center + vec2(-0.1, -0.12);
	vec2 points[7];
	for (int k = 0; k < 7; k++) {
		vec2 h = hash2(vec2(float(k), seed * 91.0));
		float theta = 6.2831853 * float(k) / 7.0 + (h.x - 0.5) * 0.5;
		vec2 v = center + vec2(cos(theta) * 0.5, sin(theta) * 0.36) * (0.85 + h.y * 0.3);
		points[k] = vec2(v.x, min(v.y, 0.0));
	}
	float aa = length(fwidth(p));
	vec2 light = normalize(vec2(-0.6, -0.8));
	for (int k = 0; k < 7; k++) {
		vec2 a = points[k];
		vec2 b = points[(k + 1) % 7];
		float s = sign(cross2(a - peak, b - peak));
		if (s * cross2(a - peak, p - peak) < 0.0 || s * cross2(peak - b, p - b) < 0.0) continue;
		float d = s * cross2(b - a, p - a) / length(b - a);
		float lit = 0.95 + 0.35 * dot(normalize((a + b) / 2.0 - peak), light);
		float seam = min(abs(cross2(normalize(a - peak), p - peak)), abs(cross2(normalize(b - peak), p - peak)));
		return vec4(${vec3(MAP.rock)} * lit * mix(0.85, 1.0, smoothstep(0.0, 0.012, seam)), smoothstep(-aa, aa, d) * smoothstep(0.0, -0.3, p.y));
	}
	return vec4(0.0);
}
void main() {
	if (u_shadow > 0.5) {
		color = vec4(${vec3(MAP.shadow)}, (1.0 - smoothstep(0.3, 1.0, length(v_offset))) * 0.3);
		return;
	}
	if (v_kind > 1.5) {
		int sprite = int(v_kind + 0.5) - 2;
		vec2 uv = v_uv;
		if (sprite == ${TREE - 2}) uv.x += sin(u_time * 1.1 + v_seed * 6.2831853) * 0.012 * (1.0 - uv.y);
${SPRITES.map((_, k) => `		if (sprite == ${k}) color = texture(u_sprites[${k}], uv);`).join('\n')}
		return;
	}
	color = rock(v_offset, v_seed);
}`;

const FOG_VERTEX = `#version 300 es
in vec2 a_corner;
uniform vec4 u_box;
uniform float u_lift;
${PROJECT}
out vec2 v_world;
void main() {
	v_world = mix(u_box.xy, u_box.zw, a_corner * 0.5 + 0.5);
	gl_Position = project(v_world - vec2(0.0, u_lift + 0.15));
}`;

const FOG_FRAGMENT = `#version 300 es
precision highp float;
in vec2 v_world;
uniform sampler2D u_fog;
uniform ivec2 u_origin;
out vec4 color;
${NOISE}
float puff(vec2 w) {
	vec2 drift = u_time * vec2(0.05, 0.02);
	return 0.5 + 0.55 * perlin(w * 0.85 + drift) + 0.25 * perlin(w * 1.9 - drift * 1.4);
}
float density(int row, int col) {
	ivec2 cell = ivec2(col, row) - u_origin;
	if (any(lessThan(cell, ivec2(0))) || any(greaterThanEqual(cell, textureSize(u_fog, 0)))) return 0.0;
	return texelFetch(u_fog, cell, 0).r;
}
vec2 field(vec2 w) {
	int row = int(floor(w.y / ${float(HEIGHT * 0.75)} + 0.5));
	vec2 total = vec2(0.0);
	for (int dr = -2; dr <= 2; dr++) {
		int r = row + dr;
		float shift = float(r & 1) * 0.5;
		int col = int(floor(w.x / ${float(WIDTH)} - shift + 0.5));
		for (int dc = -2; dc <= 2; dc++) {
			float d = density(r, col + dc);
			if (d <= 0.0) continue;
			vec2 center = vec2(${float(WIDTH)} * (float(col + dc) + shift), ${float(HEIGHT * 0.75)} * float(r));
			float k = 1.0 - smoothstep(0.0, 1.9, length(w - center));
			total += vec2(d, step(1.0, d)) * k;
		}
	}
	return total;
}
void main() {
	vec2 mass = field(v_world);
	if (mass.x < 0.05) discard;
	float n = puff(v_world);
	float lit = n - puff(v_world + vec2(0.18, 0.22));
	float body = min(mass.x + (n - 0.5) * 0.5 - 0.32, n + mass.y * 0.9 - 0.55);
	float aa = fwidth(body);
	float edge = smoothstep(-aa, aa, body);
	if (edge <= 0.0) discard;
	float la = fwidth(lit);
	vec3 tone = mix(${vec3(MAP.fog)}, ${vec3(MAP.fogLight)}, smoothstep(0.03 - la, 0.03 + la, lit));
	float rim = 1.0 - smoothstep(0.1 - aa, 0.1 + aa, body);
	color = vec4(mix(tone, ${vec3(MAP.fogShade)}, rim), edge);
}`;

function template() {
	const vertices: number[] = [];
	const push = ([x, y]: number[], raise: number, kind: number, along: number) => vertices.push(x, y, raise, kind, along);
	for (let i = 0; i < FRONT.length - 1; i++) {
		const [a, b] = [FRONT[i], FRONT[i + 1]];
		for (const [corner, raise, along] of [[a, 0, 0], [b, 0, 1], [b, 1, 1], [a, 0, 0], [b, 1, 1], [a, 1, 0]] as const) {
			push(corner, raise, -(i + 1), along);
		}
	}
	for (let i = 0; i < 6; i++) {
		push([0, 0], 1, 1, 0);
		push(CORNERS[i], 1, 1, 0);
		push(CORNERS[(i + 1) % 6], 1, 1, 0);
	}
	return new Float32Array(vertices);
}

function compile(gl: WebGL2RenderingContext, type: number, source: string) {
	const shader = gl.createShader(type)!;
	gl.shaderSource(shader, source);
	gl.compileShader(shader);
	if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) console.error(gl.getShaderInfoLog(shader));
	return shader;
}

function link(gl: WebGL2RenderingContext, vertex: string, fragment: string) {
	const program = gl.createProgram();
	gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, vertex));
	gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, fragment));
	gl.linkProgram(program);
	if (gl.getProgramParameter(program, gl.LINK_STATUS)) return program;
	console.error(gl.getProgramInfoLog(program));
	return null;
}

export function renderer(canvas: HTMLCanvasElement): Renderer | null {
	const gl = canvas.getContext('webgl2', { antialias: true, alpha: false });
	if (!gl) return null;
	const program = link(gl, VERTEX, FRAGMENT);
	const forest = link(gl, TREE_VERTEX, TREE_FRAGMENT);
	const asphalt = link(gl, RAMP_VERTEX, RAMP_FRAGMENT);
	const mist = link(gl, FOG_VERTEX, FOG_FRAGMENT);
	if (!program || !forest || !asphalt || !mist) return null;

	const shape = template();
	const vao = gl.createVertexArray();
	gl.bindVertexArray(vao);

	const attribute = (name: string, size: number, stride: number, offset: number, divisor: number, from = program) => {
		const location = gl.getAttribLocation(from, name);
		gl.enableVertexAttribArray(location);
		gl.vertexAttribPointer(location, size, gl.FLOAT, false, stride * 4, offset * 4);
		gl.vertexAttribDivisor(location, divisor);
	};

	gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
	gl.bufferData(gl.ARRAY_BUFFER, shape, gl.STATIC_DRAW);
	attribute('a_position', 2, 5, 0, 0);
	attribute('a_raise', 1, 5, 2, 0);
	attribute('a_kind', 1, 5, 3, 0);
	attribute('a_along', 1, 5, 4, 0);

	const instances = gl.createBuffer();
	gl.bindBuffer(gl.ARRAY_BUFFER, instances);
	attribute('i_center', 2, STRIDE, 0, 1);
	attribute('i_lift', 1, STRIDE, 2, 1);
	attribute('i_top', 3, STRIDE, 3, 1);
	attribute('i_side', 3, STRIDE, 6, 1);
	attribute('i_outline', 4, STRIDE, 9, 1);
	attribute('i_highlight', 1, STRIDE, 13, 1);
	attribute('i_surface', 1, STRIDE, 14, 1);
	attribute('i_edges', 1, STRIDE, 15, 1);
	attribute('i_slopes', 1, STRIDE, 16, 1);
	attribute('i_arms', 1, STRIDE, 17, 1);

	const grove = gl.createVertexArray();
	gl.bindVertexArray(grove);
	gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
	gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, 1, 1, -1, -1, 1, 1, -1, 1]), gl.STATIC_DRAW);
	attribute('a_corner', 2, 2, 0, 0, forest);
	const trees = gl.createBuffer();
	gl.bindBuffer(gl.ARRAY_BUFFER, trees);
	attribute('i_root', 3, TREE_STRIDE, 0, 1, forest);
	attribute('i_tree', 3, TREE_STRIDE, 3, 1, forest);
	attribute('i_tile', 1, TREE_STRIDE, 6, 1, forest);
	let planted = 0;

	const road = gl.createVertexArray();
	gl.bindVertexArray(road);
	gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
	gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, 1, 1, -1, -1, 1, 1, -1, 1]), gl.STATIC_DRAW);
	attribute('a_corner', 2, 2, 0, 0, asphalt);
	const ramps = gl.createBuffer();
	gl.bindBuffer(gl.ARRAY_BUFFER, ramps);
	attribute('i_from', 3, RAMP_STRIDE, 0, 1, asphalt);
	attribute('i_to', 3, RAMP_STRIDE, 3, 1, asphalt);
	attribute('i_wide', 1, RAMP_STRIDE, 6, 1, asphalt);
	attribute('i_bend', 2, RAMP_STRIDE, 7, 1, asphalt);
	attribute('i_tile', 1, RAMP_STRIDE, 9, 1, asphalt);
	attribute('i_slope', 2, RAMP_STRIDE, 10, 1, asphalt);
	let paved = 0;

	const fog = gl.createVertexArray();
	gl.bindVertexArray(fog);
	gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
	gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, 1, 1, -1, -1, 1, 1, -1, 1]), gl.STATIC_DRAW);
	attribute('a_corner', 2, 2, 0, 0, mist);
	const clouds = gl.createTexture();
	gl.useProgram(mist);
	gl.uniform1i(gl.getUniformLocation(mist, 'u_fog'), SPRITES.length);
	gl.activeTexture(gl.TEXTURE0 + SPRITES.length);
	gl.bindTexture(gl.TEXTURE_2D, clouds);
	gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
	gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
	let shrouded = 0;
	let count = 0;

	gl.useProgram(program);
	gl.uniform2f(gl.getUniformLocation(program, 'u_seed'), Math.random() * 100, Math.random() * 100);
	gl.useProgram(forest);
	gl.uniform1iv(gl.getUniformLocation(forest, 'u_sprites'), SPRITES.map((_, k) => k));
	SPRITES.forEach(({ src }, k) => {
		const texture = gl.createTexture();
		gl.activeTexture(gl.TEXTURE0 + k);
		gl.bindTexture(gl.TEXTURE_2D, texture);
		gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array(4));
		const sprite = new Image();
		sprite.onload = () => {
			gl.activeTexture(gl.TEXTURE0 + k);
			gl.bindTexture(gl.TEXTURE_2D, texture);
			gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, sprite);
			gl.generateMipmap(gl.TEXTURE_2D);
			gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
			gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
			gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
		};
		sprite.src = src;
	});

	gl.enable(gl.BLEND);
	gl.enable(gl.DEPTH_TEST);
	gl.depthFunc(gl.LEQUAL);
	gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
	gl.clearColor(...(rgb(MAP.background) as [number, number, number]), 1);

	const uniforms = (set: (uniform: (name: string) => WebGLUniformLocation | null) => void) => {
		for (const each of [program, forest, asphalt, mist]) {
			gl.useProgram(each);
			set((name) => gl.getUniformLocation(each, name));
		}
	};

	return {
		animated: true,
		resize(screen, plane, camera) {
			const [width, height] = [Math.round(screen.width * screen.ratio[0]), Math.round(screen.height * screen.ratio[1])];
			if (canvas.width !== width || canvas.height !== height) {
				canvas.width = width;
				canvas.height = height;
				gl.viewport(0, 0, width, height);
			}
			const radians = (camera.tilt * Math.PI) / 180;
			uniforms((uniform) => {
				gl.uniform3f(uniform('u_view'), plane.scale, plane.x, plane.y);
				gl.uniform2f(uniform('u_plane'), plane.width, plane.height);
				gl.uniform2f(uniform('u_screen'), screen.width, screen.height);
				gl.uniform3f(uniform('u_camera'), camera.perspective, Math.cos(radians), Math.sin(radians));
				gl.uniform2f(uniform('u_depth'), -plane.y / plane.scale, plane.height / plane.scale);
			});
		},
		plant(data) {
			gl.bindBuffer(gl.ARRAY_BUFFER, trees);
			gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);
			planted = data.length / TREE_STRIDE;
		},
		pave(data) {
			gl.bindBuffer(gl.ARRAY_BUFFER, ramps);
			gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);
			paved = data.length / RAMP_STRIDE;
		},
		shroud(data) {
			shrouded = data.length / FOG_STRIDE;
			if (!shrouded) return;
			const cells = Array.from({ length: shrouded }, (_, i) => {
				const [x, y, lift, density] = data.subarray(i * FOG_STRIDE, (i + 1) * FOG_STRIDE);
				const row = Math.round(y / (HEIGHT * 0.75));
				return { x, y, lift, density, row, col: Math.round(x / WIDTH - (row & 1) / 2) };
			});
			const [top, left] = [Math.min(...cells.map((c) => c.row)), Math.min(...cells.map((c) => c.col))];
			const [rows, cols] = [Math.max(...cells.map((c) => c.row)) - top + 1, Math.max(...cells.map((c) => c.col)) - left + 1];
			const bytes = new Uint8Array(rows * cols);
			for (const { row, col, density } of cells) bytes[(row - top) * cols + col - left] = Math.round(density * 255);
			gl.useProgram(mist);
			gl.activeTexture(gl.TEXTURE0 + SPRITES.length);
			gl.bindTexture(gl.TEXTURE_2D, clouds);
			gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
			gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, cols, rows, 0, gl.RED, gl.UNSIGNED_BYTE, bytes);
			gl.uniform2i(gl.getUniformLocation(mist, 'u_origin'), left, top);
			const [xs, ys] = [cells.map((c) => c.x), cells.map((c) => c.y)];
			gl.uniform4f(gl.getUniformLocation(mist, 'u_box'), Math.min(...xs) - 2, Math.min(...ys) - 2, Math.max(...xs) + 2, Math.max(...ys) + 2);
			gl.uniform1f(gl.getUniformLocation(mist, 'u_lift'), cells.reduce((sum, c) => sum + c.lift, 0) / shrouded);
		},
		fill(data) {
			gl.bindBuffer(gl.ARRAY_BUFFER, instances);
			gl.bufferData(gl.ARRAY_BUFFER, data, gl.DYNAMIC_DRAW);
			count = data.length / STRIDE;
		},
		draw(time = 0) {
			gl.depthMask(true);
			gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
			gl.useProgram(program);
			gl.uniform1f(gl.getUniformLocation(program, 'u_time'), time);
			gl.bindVertexArray(vao);
			gl.drawArraysInstanced(gl.TRIANGLES, 0, shape.length / 5, count);
			gl.depthMask(false);
			if (paved) {
				gl.useProgram(asphalt);
				gl.bindVertexArray(road);
				const paint = gl.getUniformLocation(asphalt, 'u_paint');
				gl.uniform1f(paint, 0);
				gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, paved);
				gl.uniform1f(paint, 1);
				gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, paved);
			}
			if (planted) {
				gl.useProgram(forest);
				gl.uniform1f(gl.getUniformLocation(forest, 'u_time'), time);
				gl.bindVertexArray(grove);
				const shadow = gl.getUniformLocation(forest, 'u_shadow');
				for (const pass of [1, 0]) {
					gl.uniform1f(shadow, pass);
					gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, planted);
				}
			}
			if (!shrouded) return;
			gl.disable(gl.DEPTH_TEST);
			gl.useProgram(mist);
			gl.uniform1f(gl.getUniformLocation(mist, 'u_time'), time);
			gl.bindVertexArray(fog);
			gl.drawArrays(gl.TRIANGLES, 0, 6);
			gl.enable(gl.DEPTH_TEST);
		},
		destroy() {
			gl.getExtension('WEBGL_lose_context')?.loseContext();
		}
	};
}

function noise(x: number, y: number) {
	const n = 0.65 * perlin(x * 0.07, y * 0.07) + 0.35 * perlin(x * 0.16 + 31, y * 0.16 + 17);
	return Math.min(1, Math.max(0, n * 1.4 + 0.5));
}

export function fallback(canvas: HTMLCanvasElement): Renderer {
	let context = canvas.getContext('2d', { alpha: false });
	if (!context) {
		const fresh = canvas.cloneNode() as HTMLCanvasElement;
		canvas.replaceWith(fresh);
		canvas = fresh;
		context = canvas.getContext('2d', { alpha: false })!;
	}
	const cache = document.createElement('canvas');
	let project = (x: number, y: number): [number, number] => [x, y];
	let scale = 1;
	let stale = true;
	let reach = 0;
	let planted: Float32Array = new Float32Array();
	let paved: Float32Array = new Float32Array();
	let shrouded: Float32Array = new Float32Array();
	let source: Float32Array | undefined;
	let last: Float32Array | undefined;
	const sprites = SPRITES.map(({ src }) => {
		const sprite = new Image();
		sprite.onload = () => {
			stale = true;
			renderer.draw();
		};
		sprite.src = src;
		return sprite;
	});

	const color = (data: Float32Array, o: number) => `rgb(${data[o] * 255} ${data[o + 1] * 255} ${data[o + 2] * 255})`;

	const path = (points: number[][]) => {
		context.beginPath();
		for (const [x, y] of points) context.lineTo(...project(x, y));
		context.closePath();
	};

	const silhouette = (cx: number, cy: number, lift: number) =>
		path([
			...[4, 5, 0, 1].map((i) => [cx + CORNERS[i][0], cy + CORNERS[i][1] - lift]),
			...FRONT.map(([x, y]) => [cx + x, cy + y])
		]);

	const shade = (color: number[], light: number) => `rgb(${color.map((c) => c * light).join(' ')})`;

	const road = (cx: number, cy: number, data: Float32Array, o: number) => {
		const arms = ((((data[o + 9] >> 7) & 1) === 1 ? data[o + 17] >> 12 : 0) | (data[o + 15] >> 18)) & 63;
		const nodes = Array.from({ length: 7 }, (_, k) => {
			const bits = (data[o + 9 + Math.floor(k / 3)] >> (8 * (k % 3))) & 255;
			const [x, y] = k > 0 ? [CORNERS[k - 1][0] + CORNERS[k % 6][0], CORNERS[k - 1][1] + CORNERS[k % 6][1]] : [0, 0];
			const [dx, dy] = (bits & 15) > 8 ? CORNERS[(bits & 15) - 9] : [((bits & 15) - 4) * 0.2165064, (((bits >> 4) & 7) - 2) * 0.375];
			return { x: cx + x + dx, y: cy + y + dy, highway: bits >> 7 === 1 };
		});
		const gap = (a: number, b: number) => Math.hypot(nodes[a].x - nodes[b].x, nodes[a].y - nodes[b].y);
		const hubs = (data[o + 11] >> 8) & 127;
		const roads = (data[o + 11] >> 15) & 127;
		const hub = (k: number) => ((hubs >> k) & 1) === 1;
		const longest = (a: number, b: number, c: number) =>
			[a, b, c].filter(hub).length === 1 ? hub(c) : gap(a, b) > gap(a, c) + 0.01 && gap(a, b) > gap(b, c) + 0.01;
		const linked = (k: number) => ((arms >> ((k + 6) % 6)) & 1) === 1;
		const pairs = roads & 1 ? [[0, 0]] : [];
		for (let i = 0; i < 6; i++) {
			if (!(roads & 1) && (roads >> (i + 1)) & 1) {
				pairs.push([i + 1, i + 1]);
				if ((roads >> (((i + 1) % 6) + 1)) & 1) pairs.push([i + 1, ((i + 1) % 6) + 1]);
			}
			if (!(roads & 1) || !linked(i)) continue;
			const [next, previous] = [((i + 1) % 6) + 1, ((i + 5) % 6) + 1];
			if (!(linked(i + 1) && longest(0, i + 1, next)) && !(linked(i - 1) && longest(0, i + 1, previous))) pairs.push([0, i + 1]);
			if (linked(i + 1) && !longest(i + 1, next, 0)) pairs.push([i + 1, next]);
			if (linked(i + 1) && !longest(i + 1, next, 0) && !longest(0, i + 1, next) && !longest(0, next, i + 1)) {
				path([nodes[0], nodes[i + 1], nodes[next]].map(({ x, y }) => [x, y]));
				context.fillStyle = css(MAP.asphalt);
				context.fill();
			}
		}
		const stroke = (wide: boolean, width: number, color: string) => {
			context.beginPath();
			for (const [a, b] of pairs) {
				if ((nodes[a].highway && nodes[b].highway) !== wide) continue;
				context.moveTo(...project(nodes[a].x, nodes[a].y));
				context.lineTo(...project(nodes[b].x, nodes[b].y));
			}
			context.lineCap = 'round';
			context.lineWidth = width * scale;
			context.strokeStyle = color;
			context.stroke();
			context.lineCap = 'butt';
		};
		stroke(false, 0.4, css(MAP.asphalt));
		stroke(true, 0.72, css(MAP.asphalt));
		stroke(true, 0.05, css(MAP.paint));
	};

	const inset = (cx: number, cy: number, open: boolean[], depth: number) => {
		let points = CORNERS.map(([x, y]) => [x, y]);
		open.forEach((cut, k) => {
			if (!cut) return;
			const [nx, ny] = [(CORNERS[k][0] + CORNERS[(k + 1) % 6][0]) / 1.7320508, (CORNERS[k][1] + CORNERS[(k + 1) % 6][1]) / 1.7320508];
			const side = ([x, y]: number[]) => x * nx + y * ny - 0.8660254 + depth;
			points = points.flatMap((p, i) => {
				const q = points[(i + 1) % points.length];
				const [a, b] = [side(p), side(q)];
				const kept = a <= 0 ? [p] : [];
				if (a * b < 0) kept.push([p[0] + ((q[0] - p[0]) * a) / (a - b), p[1] + ((q[1] - p[1]) * a) / (a - b)]);
				return kept;
			});
		});
		return points.map(([x, y]) => [cx + x, cy + y]);
	};

	const mine = (cx: number, cy: number, rgb: number[], bits: number) => {
		const mined = (k: number) => ((bits >> ((k + 6) % 6)) & 1) === 1;
		const trace = (depth: number, offset: number) => {
			const regions = [inset(cx, cy + offset, CORNERS.map((_, k) => !mined(k)), depth)];
			CORNERS.forEach(([x, y], i) => {
				if (!mined(i)) return;
				const open = CORNERS.map((_, k) => (k === (i + 2) % 6 && !mined(i + 1)) || (k === (i + 4) % 6 && !mined(i - 1)));
				regions.push(inset(cx + x + CORNERS[(i + 1) % 6][0], cy + offset + y + CORNERS[(i + 1) % 6][1], open, depth));
			});
			context.beginPath();
			for (const region of regions) {
				region.forEach(([x, y], k) => (k ? context.lineTo(...project(x, y)) : context.moveTo(...project(x, y))));
				context.closePath();
			}
		};
		context.save();
		path(CORNERS.map(([x, y]) => [cx + x, cy + y]));
		context.clip();
		let [offset, light] = [0, 1];
		for (let i = 0; i < 4; i++) {
			const depth = 0.116 + i * 0.17;
			trace(depth, offset);
			context.fillStyle = shade(rgb, light * 0.62);
			context.fill();
			context.clip();
			offset += 0.07;
			light *= 0.86;
			trace(depth, offset);
			context.fillStyle = shade(rgb, light);
			context.fill();
		}
		context.restore();
	};

	const hex = (data: Float32Array, i: number, highlight: boolean) => {
		const o = i * STRIDE;
		const [cx, cy, lift] = data.subarray(o, o + 3);
		if (lift > 0) {
			const raised = FRONT.map(([x, y]) => [cx + x, cy + y - lift]);
			const ground = FRONT.map(([x, y]) => [cx + x, cy + y]);
			path([...raised, ...ground.toReversed()]);
			context.fillStyle = color(data, o + 6);
			context.fill();
			context.save();
			context.clip();
			context.beginPath();
			raised.forEach(([x, y], j) => {
				context.moveTo(...project(x, y));
				context.lineTo(...project(x, ground[j][1]));
			});
			context.moveTo(...project(ground[0][0], ground[0][1]));
			for (const [x, y] of ground) context.lineTo(...project(x, y));
			context.strokeStyle = css(MAP.background);
			context.lineWidth = 0.04 * scale;
			context.stroke();
			context.restore();
			for (let j = 0; j < FRONT.length - 1; j++) {
				if (((data[o + 15] >> (2 * (j + 1))) & 1) === 0) continue;
				path([raised[j], raised[j + 1], ground[j + 1], ground[j]]);
				context.fillStyle = color(data, o + 3);
				context.fill();
			}
		}
		const variants = Math.floor(data[o + 14] / 1000);
		const focus = highlight && hovering(data[o + 13]);
		const hostile = data[o + 13] < 0;
		path(CORNERS.map(([x, y]) => [cx + x, cy + y - lift]));
		context.fillStyle = color(data, o + 3);
		context.fill();
		if (variants & flag('farmland')) {
			context.save();
			context.clip();
			context.fillStyle = css(MAP.field);
			context.fill();
			context.strokeStyle = css(MAP.crops[0]);
			context.lineWidth = 0.13 * scale;
			for (let k = -12; k <= 12; k++) {
				const [a, b] = [-4, 4].map((t) => project(cx + 0.9397 * t - 0.342 * k * 0.2, cy - lift + 0.342 * t + 0.9397 * k * 0.2));
				context.beginPath();
				context.moveTo(...a);
				context.lineTo(...b);
				context.stroke();
			}
			context.restore();
		}
		if (hostile) {
			context.fillStyle = css(MAP.hostile, 0.3);
			context.fill();
		}
		if (variants & flag('mine')) {
			mine(cx, cy - lift, [...data.subarray(o + 3, o + 6)].map((c) => c * 255), data[o + 17] & 63);
			path(CORNERS.map(([x, y]) => [cx + x, cy + y - lift]));
		}
		if (variants & flag('beach')) {
			const sand = BASES.sand.flat;
			context.save();
			context.clip();
			context.strokeStyle = sand;
			context.lineWidth = 0.4 * scale;
			for (let i = 0; i < 6; i++) {
				if (((data[o + 17] >> (6 + i)) & 1) === 0) continue;
				const [a, b] = [CORNERS[i], CORNERS[(i + 1) % 6]];
				context.beginPath();
				context.moveTo(...project(cx + a[0], cy + a[1] - lift));
				context.lineTo(...project(cx + b[0], cy + b[1] - lift));
				context.stroke();
			}
			context.restore();
		}
		if (focus) {
			context.fillStyle = css(hostile ? MAP.hostile : MAP.highlight, 0.3);
			context.fill();
		}
		context.save();
		context.clip();
		if (data[o + 12] > 0) {
			context.strokeStyle = color(data, o + 9);
			context.lineWidth = data[o + 12] * 2 * scale;
			context.stroke();
		}
		for (let edge = 0; edge < 12; edge++) {
			if (edge === 6 && (data[o + 11] >> 15) & 127) road(cx, cy - lift, data, o);
			const kind = (data[o + 15] >> (2 * (edge % 6))) & 3;
			if (kind !== (edge < 6 ? THIN : CONTOUR)) continue;
			const border = BORDERS[kind];
			const [a, b] = [CORNERS[edge % 6], CORNERS[(edge + 1) % 6]];
			context.beginPath();
			context.moveTo(...project(cx + a[0], cy + a[1] - lift));
			context.lineTo(...project(cx + b[0], cy + b[1] - lift));
			context.strokeStyle = focus ? css(hostile ? MAP.hostileHover : MAP.highlight) : css(hostile ? MAP.hostileLine : MAP.line, border.alpha);
			context.lineWidth = (focus ? border.focus : border.width) * 2 * scale;
			context.stroke();
		}
		for (let i = 0; i < 6; i++) {
			if (((data[o + 17] >> (18 + i)) & 1) === 0) continue;
			const [a, b] = [CORNERS[i], CORNERS[(i + 1) % 6]];
			context.beginPath();
			context.moveTo(...project(cx + a[0], cy + a[1] - lift));
			context.lineTo(...project(cx + b[0], cy + b[1] - lift));
			context.strokeStyle = css(MAP.hostileBorder);
			context.lineWidth = BORDERS[CONTOUR].width * 2 * scale;
			context.stroke();
		}
		context.restore();
	};

	const taper = (wide: number, t: number) => {
		if (wide === 3 || wide === 0) return wide === 3 ? 0.36 : 0.2;
		const x = Math.min(1, Math.max(0, (wide === 1 ? t : 1 - t) / 0.3));
		return 0.36 - 0.16 * x * x * (3 - 2 * x);
	};

	const fillet = (at: number[], a: number[], b: number[], wa: number, wb: number, lift: number) => {
		const unit = ([x, y]: number[]) => [(x - at[0]) / Math.hypot(x - at[0], y - at[1]), (y - at[1]) / Math.hypot(x - at[0], y - at[1])];
		const [da, db] = [unit(a), unit(b)];
		const toward = ([x, y]: number[], [ox, oy]: number[]) => (-y * ox + x * oy < 0 ? [y, -x] : [-y, x]);
		const [na, nb] = [toward(da, db), toward(db, da)];
		const det = db[0] * da[1] - da[0] * db[1];
		if (Math.abs(det) < 1e-6) return;
		const [dx, dy] = [nb[0] * wb - na[0] * wa, nb[1] * wb - na[1] * wa];
		const s = (db[0] * dy - dx * db[1]) / det;
		const corner = [at[0] + na[0] * wa + da[0] * s, at[1] + na[1] * wa + da[1] * s];
		const point = (d: number[]) => project(corner[0] + d[0] * 0.15, corner[1] + d[1] * 0.15 - lift);
		context.beginPath();
		context.moveTo(...project(corner[0], corner[1] - lift));
		context.lineTo(...point(da));
		context.quadraticCurveTo(...project(corner[0], corner[1] - lift), ...point(db));
		context.closePath();
		context.fill();
	};

	const ramps = () => {
		context.fillStyle = css(MAP.asphalt);
		for (let i = 0; i < paved.length; i += RAMP_STRIDE) {
			const [x0, y0, l0, x1, y1, l1, wide, bx, by] = paved.subarray(i, i + RAMP_STRIDE);
			if (wide >= 4) {
				fillet([x0, y0], [x1, y1], [bx, by], wide & 1 ? 0.36 : 0.2, wide & 2 ? 0.36 : 0.2, l0);
				continue;
			}
			if (l0 === l1) continue;
			const length = Math.hypot(x1 - x0, y1 - y0);
			const [nx, ny] = [(y0 - y1) / length, (x1 - x0) / length];
			const side = (sign: number) =>
				Array.from({ length: 21 }, (_, k) => {
					const t = k / 20;
					const width = taper(wide, t) * sign;
					return [x0 + (x1 - x0) * t + nx * width, y0 + (y1 - y0) * t + ny * width - l0 - (l1 - l0) * t];
				});
			path([...side(1), ...side(-1).toReversed()]);
			context.fill();
		}
	};

	const clouds = () => {
		for (let i = 0; i < shrouded.length; i += FOG_STRIDE) {
			const [x, y, lift, density] = shrouded.subarray(i, i + FOG_STRIDE);
			const [cx, cy] = project(x, y - lift - 0.15);
			const radius = project(x + 1.4, y - lift - 0.15)[0] - cx;
			const squash = (project(x, y - lift + 1.25)[1] - cy) / radius;
			context.save();
			context.translate(cx, cy);
			context.scale(1, squash);
			for (const [tone, size] of [[MAP.fogShade, 1], [MAP.fog, 0.85], [MAP.fogLight, 0.5]] as const) {
				context.fillStyle = css(tone, density);
				context.beginPath();
				context.arc(0, 0, radius * size, 0, Math.PI * 2);
				context.fill();
			}
			context.restore();
		}
	};

	const forest = () => {
		for (let i = 0; i < planted.length; i += TREE_STRIDE) {
			const [x, y, lift, size] = planted.subarray(i, i + 4);
			if (planted[i + 5] > 0.5 && planted[i + 5] < TREE - 0.5) continue;
			path(Array.from({ length: 16 }, (_, k) => [x + Math.cos((k / 8) * Math.PI) * 0.2 * size, y - lift + Math.sin((k / 8) * Math.PI) * 0.08 * size]));
			context.fillStyle = css(MAP.shadow, 0.2);
			context.fill();
		}
		for (let i = 0; i < planted.length; i += TREE_STRIDE) {
			const [x, y, lift, size, seed, kind] = planted.subarray(i, i + TREE_STRIDE);
			const [rx, ry] = project(x, y - lift);
			const span = (project(x + 1, y - lift)[0] - rx) * size;
			if (kind > 1.5) {
				const sprite = sprites[Math.round(kind) - 2];
				const { size: [width, height], anchor, unit, flat } = SPRITES[Math.round(kind) - 2];
				if (!sprite.complete || !sprite.naturalWidth) continue;
				context.save();
				if (flat) {
					const corner = (u: number, v: number) => project(x + (u - anchor[0]) * unit * size, y - lift + (v - anchor[1]) * unit * size);
					const [origin, right, down] = [corner(0, 0), corner(width, 0), corner(0, height)];
					context.transform((right[0] - origin[0]) / width, (right[1] - origin[1]) / width, (down[0] - origin[0]) / height, (down[1] - origin[1]) / height, origin[0], origin[1]);
				} else {
					context.transform(span * unit, 0, 0, span * unit, rx - anchor[0] * span * unit, ry - anchor[1] * span * unit);
				}
				context.drawImage(sprite, 0, 0);
				context.restore();
				continue;
			}
			if (kind > 0.5) {
				const fract = (v: number) => v - Math.floor(v);
				const points = Array.from({ length: 7 }, (_, k) => {
					const [hx, hy] = [fract(Math.sin(k * 127.1 + seed * 91 * 311.7) * 43758.5453), fract(Math.sin(k * 269.5 + seed * 91 * 183.3) * 43758.5453)];
					const theta = (Math.PI * 2 * k) / 7 + (hx - 0.5) * 0.5;
					const r = 0.85 + hy * 0.3;
					return [Math.cos(theta) * 0.5 * r, Math.min(-0.3 + Math.sin(theta) * 0.36 * r, 0)];
				});
				const peak = [-0.1, -0.42];
				points.forEach((a, k) => {
					const b = points[(k + 1) % 7];
					const [mx, my] = [(a[0] + b[0]) / 2 - peak[0], (a[1] + b[1]) / 2 - peak[1]];
					const lit = 0.95 + (0.35 * (mx * -0.6 + my * -0.8)) / Math.hypot(mx, my);
					context.beginPath();
					for (const [px, py] of [peak, a, b]) context.lineTo(rx + px * span, ry + py * span);
					context.closePath();
					const fade = context.createLinearGradient(0, ry - 0.3 * span, 0, ry);
					const rock = rgb(MAP.rock).map((c) => c * 255);
					fade.addColorStop(0, shade(rock, lit));
					fade.addColorStop(1, shade(rock, lit).replace(')', ' / 0)'));
					context.fillStyle = fade;
					context.fill();
				});
				continue;
			}
		}
	};

	const renderer: Renderer = {
		animated: false,
		resize(screen, plane, camera) {
			canvas.width = cache.width = Math.round(screen.width * screen.ratio[0]);
			canvas.height = cache.height = Math.round(screen.height * screen.ratio[1]);
			context.setTransform(screen.ratio[0], 0, 0, screen.ratio[1], 0, 0);
			const radians = (camera.tilt * Math.PI) / 180;
			const [cos, sin] = [Math.cos(radians), Math.sin(radians)];
			scale = plane.scale;
			project = (x, y) => {
				const rx = x * plane.scale + plane.x - plane.width / 2;
				const ry = y * plane.scale + plane.y - plane.height / 2;
				const w = 1 - (ry * sin) / camera.perspective;
				return [rx / w + screen.width / 2, (ry * cos) / w + screen.height / 2];
			};
			stale = true;
		},
		plant(data) {
			planted = data;
			stale = true;
		},
		pave(data) {
			paved = data;
			stale = true;
		},
		shroud(data) {
			shrouded = data;
			stale = true;
		},
		fill(data) {
			if (data !== source) {
				source = data;
				last = data.slice();
				for (let o = 0; o < last.length; o += STRIDE) {
					if (last[o + 14] >= 0.5) continue;
					const n = noise(last[o], last[o + 1]);
					const v = (20 + n * 22) / 255;
					const b = v * 0.7;
					last.set([n * last[o + 2], v, v + 1 / 255, v + 2 / 255, b, b, b + 1 / 255], o + 2);
				}
				stale = true;
			}
			for (let o = 13; o < data.length; o += STRIDE) last![o] = data[o];
		},
		draw() {
			if (!last) return;
			const data = last;
			const count = data.length / STRIDE;
			const lit = [];
			for (let i = 0; i < count; i++) if (hovering(data[i * STRIDE + 13])) lit.push(i);
			if (stale) {
				context.fillStyle = css(MAP.background);
				context.fillRect(0, 0, canvas.width, canvas.height);
				reach = 0;
				for (let i = 0; i < count; i++) {
					hex(data, i, false);
					reach = Math.max(reach, data[i * STRIDE + 2]);
				}
				ramps();
				forest();
				clouds();
				cache.getContext('2d')!.drawImage(canvas, 0, 0);
				stale = false;
			}
			context.save();
			context.resetTransform();
			context.drawImage(cache, 0, 0);
			context.restore();
			for (const i of lit) {
				const [x, y, lift] = data.subarray(i * STRIDE, i * STRIDE + 3);
				context.save();
				silhouette(x, y, lift);
				context.clip();
				for (let j = i; j < count; j++) {
					const o = j * STRIDE;
					if (data[o + 1] - reach - 1 > y + 1) break;
					if (Math.abs(data[o] - x) < WIDTH) hex(data, j, true);
				}
				forest();
				clouds();
				context.restore();
			}
		},
		destroy() {}
	};
	return renderer;
}
