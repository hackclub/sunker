import { BASES, LIBRARY, NAMES } from '#lib/bases.js';
import { CORNERS, FRONT, WIDTH } from '#lib/map.js';
import mountain from '#lib/assets/images/mountain.png';
import building1 from '#lib/assets/images/building1.png';
import building2 from '#lib/assets/images/building2.png';
import house1 from '#lib/assets/images/house1.png';
import house2 from '#lib/assets/images/house2.png';

export const STRIDE = 16;
export const THIN = 1;
export const CONTOUR = 2;
export const TREE_STRIDE = 6;

const SPRITES = [
	{ src: mountain, size: [256, 256], anchor: [112, 200], unit: 1.9 / 256, flat: true },
	{ src: building1, size: [64, 128], anchor: [28.5, 128], unit: 0.016, flat: false },
	{ src: building2, size: [64, 128], anchor: [33, 128], unit: 0.016, flat: false },
	{ src: house1, size: [256, 256], anchor: [132, 234], unit: 0.003, flat: false },
	{ src: house2, size: [256, 256], anchor: [125, 218], unit: 0.003, flat: false }
];
const float = (n: number) => (Number.isInteger(n) ? `${n}.0` : `${n}`);
const SPRITE_CONSTANTS = `
const vec2 SPRITE_SIZES[${SPRITES.length}] = vec2[${SPRITES.length}](${SPRITES.map(({ size }) => `vec2(${size.map(float)})`)});
const vec2 SPRITE_ANCHORS[${SPRITES.length}] = vec2[${SPRITES.length}](${SPRITES.map(({ anchor }) => `vec2(${anchor.map(float)})`)});
const float SPRITE_UNITS[${SPRITES.length}] = float[${SPRITES.length}](${SPRITES.map(({ unit }) => float(unit))});
const bool SPRITE_FLAT[${SPRITES.length}] = bool[${SPRITES.length}](${SPRITES.map(({ flat }) => flat)});`;

const BORDERS = { [THIN]: { width: 0.015, color: 'rgb(180 184 191)' }, [CONTOUR]: { width: 0.08, color: 'rgb(255 255 255)' } };

type Screen = { width: number; height: number; ratio: number };
type Plane = { width: number; height: number; scale: number; x: number; y: number };
type Camera = { perspective: number; tilt: number };

export interface Renderer {
	animated: boolean;
	resize(screen: Screen, plane: Plane, camera: Camera): void;
	plant(trees: Float32Array): void;
	draw(data: Float32Array): void;
}

const PROJECT = `
uniform vec3 u_view;
uniform vec2 u_plane;
uniform vec2 u_screen;
uniform vec3 u_camera;
vec4 project(vec2 world) {
	vec2 relative = world * u_view.x + u_view.yz - u_plane / 2.0;
	float w = 1.0 - relative.y * u_camera.z / u_camera.x;
	vec2 screen = vec2(relative.x, relative.y * u_camera.y) / (u_screen / 2.0);
	return vec4(screen.x, -screen.y, 0.0, w);
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
${PROJECT}
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
flat out vec3 v_nodes;
void main() {
	vec2 position = a_kind > 0.5 ? a_position * 1.005 : a_position;
	gl_Position = project(i_center + position - vec2(0.0, a_raise * i_lift));
	v_local = position;
	v_kind = a_kind;
	v_along = a_along;
	v_height = a_raise * i_lift;
	v_top = i_top;
	v_side = i_side;
	v_outline = i_outline;
	v_highlight = i_highlight;
	v_world = i_center + position;
	v_surface = i_surface;
	v_edges = i_edges;
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
flat in vec3 v_nodes;
out vec4 color;
const vec3 SEAM = vec3(14.0, 15.0, 16.0) / 255.0;
const vec3 HIGHLIGHT = vec3(134.0, 158.0, 207.0) / 255.0;
const vec3 THIN = vec3(180.0, 184.0, 191.0) / 255.0;
const vec3 ASPHALT = vec3(54.0, 56.0, 59.0) / 255.0;
const vec3 PAINT = vec3(238.0, 241.0, 146.0) / 255.0;
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
		int variant = packed / 100;
		vec3 base = surface(float(packed % 100), v_world, v_local, v_top);
		int edges = int(v_edges);
		if (variant == 3) base = mine(v_local, v_world.y, base, (edges >> 12) & 63);
		vec3 fill = mix(base, HIGHLIGHT, v_highlight * 0.3);
		vec3 rgb = v_outline.a > 0.0 ? mix(v_outline.rgb, fill, smoothstep(v_outline.a - aa, v_outline.a + aa, edge)) : fill;
		float thin = 0.0;
		float contour = 0.0;
		float open = 1.0;
		for (int i = 0; i < 6; i++) {
			int kind = (edges >> (2 * i)) & 3;
			float distance = 0.8660254 - dot(v_local, NORMALS[i]);
			float da = fwidth(distance);
			if (kind == 1 && !(variant == 3 && ((edges >> (12 + i)) & 1) == 1)) thin = max(thin, 1.0 - smoothstep(0.015 - da, 0.015 + da, distance));
			if (kind != 1) open = min(open, distance);
			if (kind == 2) contour = max(contour, 1.0 - smoothstep(0.08 - da, 0.08 + da, distance));
		}
		rgb = mix(rgb, THIN, thin);
		int roads = (int(v_nodes[2]) >> 15) & 127;
		if (roads != 0) {
			vec2 nodes[7];
			bool highways[7];
			for (int k = 0; k < 7; k++) {
				int bits = (int(v_nodes[k / 3]) >> (8 * (k % 3))) & 255;
				nodes[k] = (k > 0 ? NORMALS[k - 1] * 1.7320508 : vec2(0.0)) + vec2(float((bits & 15) - 4) * 0.2165064, float(((bits >> 4) & 7) - 2) * 0.375);
				highways[k] = (bits >> 7) == 1;
			}
			int arms = ((edges >> 12) | (edges >> 18)) & 63;
			int hubs = (int(v_nodes[2]) >> 8) & 127;
			bool own = (roads & 1) == 1;
			float road = own ? length(v_local - nodes[0]) - (variant == 1 ? 0.36 : 0.2) : 1e9;
			float line = 1e9;
			for (int i = 0; i < 6; i++) {
				if (!own && ((roads >> (i + 1)) & 1) == 1) {
					int after = (i + 1) % 6 + 1;
					road = min(road, length(v_local - nodes[i + 1]) - (highways[i + 1] ? 0.36 : 0.2));
					if (((roads >> after) & 1) == 0) continue;
					bool both = highways[i + 1] && highways[after];
					float d = segment(v_local, nodes[i + 1], nodes[after]);
					road = min(road, d - (both ? 0.36 : 0.2));
					if (both) line = min(line, d);
				}
				if (!own || ((arms >> i) & 1) == 0) continue;
				int next = (i + 1) % 6;
				int previous = (i + 5) % 6;
				bool linked = ((arms >> next) & 1) == 1;
				bool wide = highways[0] && highways[i + 1];
				float d = segment(v_local, nodes[0], nodes[i + 1]);
				if (!(linked && dropped(nodes, hubs, 0, i + 1, next + 1)) && !(((arms >> previous) & 1) == 1 && dropped(nodes, hubs, 0, i + 1, previous + 1))) {
					road = min(road, d - (wide ? 0.36 : 0.2));
					if (wide) line = min(line, d);
				}
				if (!linked || dropped(nodes, hubs, i + 1, next + 1, 0)) continue;
				if (!dropped(nodes, hubs, 0, i + 1, next + 1) && !dropped(nodes, hubs, 0, next + 1, i + 1) && inside(v_local, nodes[0], nodes[i + 1], nodes[next + 1])) road = min(road, -1.0);
				wide = highways[i + 1] && highways[next + 1];
				d = segment(v_local, nodes[i + 1], nodes[next + 1]);
				road = min(road, d - (wide ? 0.36 : 0.2));
				if (wide) line = min(line, d);
			}
			float ra = length(fwidth(v_local));
			rgb = mix(rgb, mix(PAINT, ASPHALT, smoothstep(0.025 - ra, 0.025 + ra, line)), 1.0 - smoothstep(-ra, ra, road));
		}
		rgb = mix(rgb, vec3(1.0), contour);
		color = vec4(dither(rgb), smoothstep(0.0, fwidth(open), open));
	} else {
		float seam = min(min(v_along, 1.0 - v_along), v_height);
		float aa = fwidth(seam);
		color = vec4(dither(mix(SEAM, v_side, smoothstep(0.02 - aa, 0.02 + aa, seam))), 1.0);
	}
}`;

const TREE_VERTEX = `#version 300 es
in vec2 a_corner;
in vec3 i_root;
in vec3 i_tree;
uniform float u_shadow;${SPRITE_CONSTANTS}
${PROJECT}
out vec2 v_offset;
out vec2 v_uv;
flat out float v_seed;
flat out float v_kind;
void main() {
	vec2 root = i_root.xy - vec2(0.0, i_root.z);
	if (u_shadow > 0.5) {
		gl_Position = i_tree.z > 0.5 ? vec4(0.0) : project(root + a_corner * vec2(0.26, 0.1) * i_tree.x);
		v_offset = a_corner;
	} else {
		vec2 uv = vec2(a_corner.x * 0.5 + 0.5, 0.5 - a_corner.y * 0.5);
		int sprite = int(i_tree.z + 0.5) - 2;
		vec2 offset = sprite >= 0 ? (uv * SPRITE_SIZES[sprite] - SPRITE_ANCHORS[sprite]) * SPRITE_UNITS[sprite] : i_tree.z > 0.5 ? vec2(a_corner.x * 0.75, mix(0.05, -0.8, a_corner.y * 0.5 + 0.5)) : vec2(a_corner.x * 0.28, mix(0.03, -1.05, a_corner.y * 0.5 + 0.5));
		gl_Position = project(root);
		gl_Position.xy += vec2(offset.x, -offset.y) * i_tree.x * u_view.x / (u_screen / 2.0);
		if (sprite >= 0 && SPRITE_FLAT[sprite]) gl_Position = project(root + offset * i_tree.x);
		v_offset = offset;
		v_uv = uv;
	}
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
		return vec4(vec3(60.0, 61.0, 63.0) / 255.0 * lit * mix(0.85, 1.0, smoothstep(0.0, 0.012, seam)), smoothstep(-aa, aa, d) * smoothstep(0.0, -0.3, p.y));
	}
	return vec4(0.0);
}
float pine(vec2 q, float seed) {
	float tiers = 2.0 + floor(fract(seed * 7.31) * 3.0);
	float girth = 0.11 * (0.8 + 0.4 * fract(seed * 13.17));
	float rise = 0.06 + 0.025 * fract(seed * 5.73);
	float taper = 0.35 + 0.2 * fract(seed * 3.37);
	float stem = 0.04 + 0.03 * fract(seed * 11.9);
	float tall = rise * 1.35;
	float trunk = max(abs(q.x) - 0.018, max(q.y, -stem - 0.02 - q.y));
	float leaves = 1e9;
	for (int k = 0; k < 4; k++) {
		if (float(k) >= tiers) break;
		float bottom = -stem - float(k) * rise;
		float wide = girth * (1.0 - 0.6 * float(k) / tiers);
		float edge = mix(wide, wide * taper, clamp((bottom - q.y) / tall, 0.0, 1.0));
		leaves = min(leaves, max(abs(q.x) - edge, max(q.y - bottom, bottom - tall - q.y)));
	}
	float top = -stem - (tiers - 1.0) * rise - tall * 0.75;
	float edge = girth * (1.0 - 0.6 * (tiers - 1.0) / tiers) * taper * 1.1 * (1.0 - clamp((top - q.y) / 0.1, 0.0, 1.0));
	leaves = min(leaves, max(abs(q.x) - edge, max(q.y - top, top - 0.1 - q.y)));
	return leaves < trunk ? -leaves - 1.0 : trunk;
}
void main() {
	if (u_shadow > 0.5) {
		color = vec4(0.0, 0.0, 0.0, (1.0 - smoothstep(0.3, 1.0, length(v_offset))) * 0.3);
		return;
	}
	if (v_kind > 1.5) {
		int sprite = int(v_kind + 0.5) - 2;
${SPRITES.map((_, k) => `		if (sprite == ${k}) color = texture(u_sprites[${k}], v_uv);`).join('\n')}
		return;
	}
	if (v_kind > 0.5) {
		color = rock(v_offset, v_seed);
		return;
	}
	vec2 q = v_offset / 2.0;
	float tree = pine(q, v_seed);
	bool leafy = tree < -0.5;
	float d = leafy ? -tree - 1.0 : tree;
	float aa = length(fwidth(q));
	float tone = clamp(perlin(q * 40.0 + v_seed * 91.0) * 1.5 + v_seed, 0.0, 1.0);
	vec3 paint = leafy ? mix(vec3(61.0, 101.0, 59.0), vec3(62.0, 95.0, 59.0), tone) / 255.0 : vec3(68.0, 63.0, 55.0) / 255.0;
	paint *= mix(1.12, 0.78, smoothstep(-0.06, 0.06, q.x)) * mix(0.82, 1.0, smoothstep(0.0, 0.008, -d));
	color = vec4(paint, 1.0 - smoothstep(-aa, aa, d));
}`;

function template() {
	const vertices: number[] = [];
	const push = ([x, y]: number[], raise: number, kind: number, along: number) => vertices.push(x, y, raise, kind, along);
	for (let i = 0; i < FRONT.length - 1; i++) {
		const [a, b] = [FRONT[i], FRONT[i + 1]];
		for (const [corner, raise, along] of [[a, 0, 0], [b, 0, 1], [b, 1, 1], [a, 0, 0], [b, 1, 1], [a, 1, 0]] as const) {
			push(corner, raise, 0, along);
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

export function rgb(color: string) {
	const hex = color.replace('#', '');
	const full = hex.length === 3 ? [...hex].map((c) => c + c).join('') : hex;
	return [0, 2, 4].map((i) => parseInt(full.slice(i, i + 2), 16) / 255);
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
	const gl = canvas.getContext('webgl2', { antialias: false, alpha: false });
	if (!gl) return null;
	const program = link(gl, VERTEX, FRAGMENT);
	const forest = link(gl, TREE_VERTEX, TREE_FRAGMENT);
	if (!program || !forest) return null;

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

	const grove = gl.createVertexArray();
	gl.bindVertexArray(grove);
	gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
	gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, 1, 1, -1, -1, 1, 1, -1, 1]), gl.STATIC_DRAW);
	attribute('a_corner', 2, 2, 0, 0, forest);
	const trees = gl.createBuffer();
	gl.bindBuffer(gl.ARRAY_BUFFER, trees);
	attribute('i_root', 3, TREE_STRIDE, 0, 1, forest);
	attribute('i_tree', 3, TREE_STRIDE, 3, 1, forest);
	let planted = 0;

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
	gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
	gl.clearColor(14 / 255, 15 / 255, 16 / 255, 1);

	const uniforms = (set: (uniform: (name: string) => WebGLUniformLocation | null) => void) => {
		for (const each of [program, forest]) {
			gl.useProgram(each);
			set((name) => gl.getUniformLocation(each, name));
		}
	};

	return {
		animated: true,
		resize(screen, plane, camera) {
			canvas.width = Math.round(screen.width * screen.ratio);
			canvas.height = Math.round(screen.height * screen.ratio);
			gl.viewport(0, 0, canvas.width, canvas.height);
			const radians = (camera.tilt * Math.PI) / 180;
			uniforms((uniform) => {
				gl.uniform3f(uniform('u_view'), plane.scale, plane.x, plane.y);
				gl.uniform2f(uniform('u_plane'), plane.width, plane.height);
				gl.uniform2f(uniform('u_screen'), screen.width, screen.height);
				gl.uniform3f(uniform('u_camera'), camera.perspective, Math.cos(radians), Math.sin(radians));
			});
		},
		plant(data) {
			gl.bindBuffer(gl.ARRAY_BUFFER, trees);
			gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);
			planted = data.length / TREE_STRIDE;
		},
		draw(data: Float32Array) {
			gl.clear(gl.COLOR_BUFFER_BIT);
			gl.useProgram(program);
			gl.bindVertexArray(vao);
			gl.bindBuffer(gl.ARRAY_BUFFER, instances);
			gl.bufferData(gl.ARRAY_BUFFER, data, gl.DYNAMIC_DRAW);
			gl.drawArraysInstanced(gl.TRIANGLES, 0, shape.length / 5, data.length / STRIDE);
			if (!planted) return;
			gl.useProgram(forest);
			gl.bindVertexArray(grove);
			const shadow = gl.getUniformLocation(forest, 'u_shadow');
			for (const pass of [1, 0]) {
				gl.uniform1f(shadow, pass);
				gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, planted);
			}
		}
	};
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
	let last: Float32Array | undefined;
	const sprites = SPRITES.map(({ src }) => {
		const sprite = new Image();
		sprite.onload = () => {
			stale = true;
			if (last) renderer.draw(last);
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
		const arms = ((data[o + 15] >> 12) | (data[o + 15] >> 18)) & 63;
		const nodes = Array.from({ length: 7 }, (_, k) => {
			const bits = (data[o + 9 + Math.floor(k / 3)] >> (8 * (k % 3))) & 255;
			const [x, y] = k > 0 ? [CORNERS[k - 1][0] + CORNERS[k % 6][0], CORNERS[k - 1][1] + CORNERS[k % 6][1]] : [0, 0];
			return { x: cx + x + ((bits & 15) - 4) * 0.2165064, y: cy + y + (((bits >> 4) & 7) - 2) * 0.375, highway: bits >> 7 === 1 };
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
				context.fillStyle = 'rgb(54 56 59)';
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
		stroke(false, 0.4, 'rgb(54 56 59)');
		stroke(true, 0.72, 'rgb(54 56 59)');
		stroke(true, 0.05, 'rgb(238 241 146)');
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
			context.strokeStyle = 'rgb(14 15 16)';
			context.lineWidth = 0.04 * scale;
			context.stroke();
			context.restore();
		}
		const variant = Math.floor(data[o + 14] / 100);
		path(CORNERS.map(([x, y]) => [cx + x, cy + y - lift]));
		context.fillStyle = color(data, o + 3);
		context.fill();
		if (variant === 3) {
			mine(cx, cy - lift, [...data.subarray(o + 3, o + 6)].map((c) => c * 255), (data[o + 15] >> 12) & 63);
			path(CORNERS.map(([x, y]) => [cx + x, cy + y - lift]));
		}
		if (highlight && data[o + 13] > 0) {
			context.fillStyle = `rgb(134 158 207 / ${data[o + 13] * 0.3})`;
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
			if (variant === 3 && edge < 6 && (data[o + 15] >> (12 + edge)) & 1) continue;
			const border = BORDERS[kind];
			const [a, b] = [CORNERS[edge % 6], CORNERS[(edge + 1) % 6]];
			context.beginPath();
			context.moveTo(...project(cx + a[0], cy + a[1] - lift));
			context.lineTo(...project(cx + b[0], cy + b[1] - lift));
			context.strokeStyle = border.color;
			context.lineWidth = border.width * 2 * scale;
			context.stroke();
		}
		context.restore();
	};

	const forest = () => {
		for (let i = 0; i < planted.length; i += TREE_STRIDE) {
			const [x, y, lift, size] = planted.subarray(i, i + 4);
			if (planted[i + 5] > 0.5) continue;
			path(Array.from({ length: 16 }, (_, k) => [x + Math.cos((k / 8) * Math.PI) * 0.2 * size, y - lift + Math.sin((k / 8) * Math.PI) * 0.08 * size]));
			context.fillStyle = 'rgb(0 0 0 / 0.2)';
			context.fill();
		}
		for (let i = 0; i < planted.length; i += TREE_STRIDE) {
			const [x, y, lift, size, seed, kind] = planted.subarray(i, i + TREE_STRIDE);
			const leaves = [61, 101 - seed * 6, 59];
			const [rx, ry] = project(x, y - lift);
			const span = (project(x + 1, y - lift)[0] - rx) * size;
			if (kind > 1.5) {
				const sprite = sprites[Math.round(kind) - 2];
				const { size: [width, height], anchor, unit, flat } = SPRITES[Math.round(kind) - 2];
				if (!sprite.complete) continue;
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
					fade.addColorStop(0, shade([60, 61, 63], lit));
					fade.addColorStop(1, shade([60, 61, 63], lit).replace(')', ' / 0)'));
					context.fillStyle = fade;
					context.fill();
				});
				continue;
			}
			const part = (points: number[][], fill: string) => {
				context.beginPath();
				for (const [px, py] of points) context.lineTo(rx + px * span, ry + py * span);
				context.closePath();
				context.fillStyle = fill;
				context.fill();
			};
			const fract = (k: number) => (seed * k) % 1;
			const tiers = 2 + Math.floor(fract(7.31) * 3);
			const girth = 0.22 * (0.8 + 0.4 * fract(13.17));
			const rise = 0.12 + 0.05 * fract(5.73);
			const taper = 0.35 + 0.2 * fract(3.37);
			const stem = 0.08 + 0.06 * fract(11.9);
			const tall = rise * 1.35;
			const top = -stem - (tiers - 1) * rise - tall * 0.75;
			const tip = girth * (1 - (0.6 * (tiers - 1)) / tiers) * taper * 1.1;
			for (const side of [-1, 1]) {
				const light = side < 0 ? 1.12 : 0.78;
				part([[0, 0], [side * 0.036, 0], [side * 0.036, -stem - 0.04], [0, -stem - 0.04]], shade([68, 63, 55], light));
				for (let k = 0; k < tiers; k++) {
					const [bottom, wide] = [-stem - k * rise, girth * (1 - (0.6 * k) / tiers)];
					part([[0, bottom], [side * wide, bottom], [side * wide * taper, bottom - tall], [0, bottom - tall]], shade(leaves, light));
				}
				part([[0, top], [side * tip, top], [0, top - 0.2]], shade(leaves, light));
			}
		}
	};

	const renderer: Renderer = {
		animated: false,
		resize(screen, plane, camera) {
			canvas.width = cache.width = Math.round(screen.width * screen.ratio);
			canvas.height = cache.height = Math.round(screen.height * screen.ratio);
			context.setTransform(screen.ratio, 0, 0, screen.ratio, 0, 0);
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
		draw(data) {
			last = data;
			const count = data.length / STRIDE;
			const lit = [];
			for (let i = 0; i < count; i++) if (data[i * STRIDE + 13] > 0) lit.push(i);
			if (stale) {
				context.fillStyle = 'rgb(14 15 16)';
				context.fillRect(0, 0, canvas.width, canvas.height);
				reach = 0;
				for (let i = 0; i < count; i++) {
					hex(data, i, false);
					reach = Math.max(reach, data[i * STRIDE + 2]);
				}
				forest();
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
				context.restore();
			}
		}
	};
	return renderer;
}
