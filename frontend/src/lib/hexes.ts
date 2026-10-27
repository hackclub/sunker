import { CORNERS, FRONT, WIDTH } from '#lib/map.js';

export const STRIDE = 14;

type Screen = { width: number; height: number; ratio: number };
type Plane = { width: number; height: number; scale: number; x: number; y: number };
type Camera = { perspective: number; tilt: number };

export interface Renderer {
	animated: boolean;
	resize(screen: Screen, plane: Plane, camera: Camera): void;
	draw(data: Float32Array): void;
}

const VERTEX = `#version 300 es
in vec2 a_position;
in float a_raise;
in float a_kind;
in float a_along;
in vec2 i_center;
in float i_lift;
in vec3 i_top;
in vec3 i_base;
in vec4 i_outline;
in float i_highlight;
uniform vec3 u_view;
uniform vec2 u_plane;
uniform vec2 u_screen;
uniform vec3 u_camera;
out vec2 v_local;
out float v_kind;
out float v_along;
out float v_height;
out vec3 v_top;
out vec3 v_base;
out vec4 v_outline;
out float v_highlight;
void main() {
	vec2 world = i_center + a_position - vec2(0.0, a_raise * i_lift);
	vec2 relative = world * u_view.x + u_view.yz - u_plane / 2.0;
	float w = 1.0 - relative.y * u_camera.z / u_camera.x;
	vec2 screen = vec2(relative.x, relative.y * u_camera.y) / (u_screen / 2.0);
	gl_Position = vec4(screen.x, -screen.y, 0.0, w);
	v_local = a_position;
	v_kind = a_kind;
	v_along = a_along;
	v_height = a_raise * i_lift;
	v_top = i_top;
	v_base = i_base;
	v_outline = i_outline;
	v_highlight = i_highlight;
}`;

const FRAGMENT = `#version 300 es
precision highp float;
in vec2 v_local;
in float v_kind;
in float v_along;
in float v_height;
in vec3 v_top;
in vec3 v_base;
in vec4 v_outline;
in float v_highlight;
out vec4 color;
const vec3 SEAM = vec3(14.0, 15.0, 16.0) / 255.0;
const vec3 HIGHLIGHT = vec3(134.0, 158.0, 207.0) / 255.0;
vec3 dither(vec3 rgb) {
	float noise = fract(52.9829189 * fract(dot(gl_FragCoord.xy, vec2(0.06711056, 0.00583715))));
	return rgb + (noise - 0.5) / 255.0;
}
void main() {
	if (v_kind > 0.5) {
		float edge = 0.8660254 - max(abs(v_local.x), max(abs(dot(v_local, vec2(0.5, 0.8660254))), abs(dot(v_local, vec2(0.5, -0.8660254)))));
		float aa = fwidth(edge);
		vec3 fill = mix(v_top, HIGHLIGHT, v_highlight * 0.3);
		vec3 rgb = mix(v_outline.rgb, fill, smoothstep(v_outline.a - aa, v_outline.a + aa, edge));
		color = vec4(dither(rgb), smoothstep(0.0, aa, edge));
	} else {
		float seam = min(min(v_along, 1.0 - v_along), v_height);
		float aa = fwidth(seam);
		color = vec4(dither(mix(SEAM, v_base, smoothstep(0.02 - aa, 0.02 + aa, seam))), 1.0);
	}
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

export function renderer(canvas: HTMLCanvasElement): Renderer | null {
	const gl = canvas.getContext('webgl2', { antialias: false, alpha: false });
	if (!gl) return null;
	const program = gl.createProgram();
	gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, VERTEX));
	gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, FRAGMENT));
	gl.linkProgram(program);
	if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
		console.error(gl.getProgramInfoLog(program));
		return null;
	}
	gl.useProgram(program);

	const shape = template();
	const vao = gl.createVertexArray();
	gl.bindVertexArray(vao);

	const attribute = (name: string, size: number, stride: number, offset: number, divisor: number) => {
		const location = gl.getAttribLocation(program, name);
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
	attribute('i_base', 3, STRIDE, 6, 1);
	attribute('i_outline', 4, STRIDE, 9, 1);
	attribute('i_highlight', 1, STRIDE, 13, 1);

	gl.enable(gl.BLEND);
	gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
	gl.clearColor(14 / 255, 15 / 255, 16 / 255, 1);

	const uniform = (name: string) => gl.getUniformLocation(program, name);

	return {
		animated: true,
		resize(screen, plane, camera) {
			canvas.width = Math.round(screen.width * screen.ratio);
			canvas.height = Math.round(screen.height * screen.ratio);
			gl.viewport(0, 0, canvas.width, canvas.height);
			gl.uniform3f(uniform('u_view'), plane.scale, plane.x, plane.y);
			gl.uniform2f(uniform('u_plane'), plane.width, plane.height);
			gl.uniform2f(uniform('u_screen'), screen.width, screen.height);
			const radians = (camera.tilt * Math.PI) / 180;
			gl.uniform3f(uniform('u_camera'), camera.perspective, Math.cos(radians), Math.sin(radians));
		},
		draw(data: Float32Array) {
			gl.clear(gl.COLOR_BUFFER_BIT);
			gl.bufferData(gl.ARRAY_BUFFER, data, gl.DYNAMIC_DRAW);
			gl.drawArraysInstanced(gl.TRIANGLES, 0, shape.length / 5, data.length / STRIDE);
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
		path(CORNERS.map(([x, y]) => [cx + x, cy + y - lift]));
		context.fillStyle = color(data, o + 3);
		context.fill();
		if (highlight && data[o + 13] > 0) {
			context.fillStyle = `rgb(134 158 207 / ${data[o + 13] * 0.3})`;
			context.fill();
		}
		context.save();
		context.clip();
		context.strokeStyle = color(data, o + 9);
		context.lineWidth = data[o + 12] * 2 * scale;
		context.stroke();
		context.restore();
	};

	return {
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
		draw(data) {
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
				context.restore();
			}
		}
	};
}
