import { link } from '#lib/hexes.js';

export const FLAT = 0;
export const FACE = 1;
export const GRADIENT = 2;
export const SPRITE = 3;

export type Call = { first: number; count: number; kind: number; colour?: number[]; other?: number[]; layers?: number[]; sprite?: number };

export type Frame = {
	width: number;
	height: number;
	ratio: number;
	view: Float32Array;
	camera: number[];
	cut: number[];
	background: number[];
	calls: Call[];
};

export interface Pit {
	mesh(data: Float32Array): void;
	glyphs(frames: TexImageSource[][]): void;
	sprite(k: number, image: TexImageSource): void;
	draw(frame: Frame): void;
	destroy(): void;
}

const VERTEX = `#version 300 es
uniform mat4 u_view;
in vec3 a_position;
in vec2 a_uv;
out vec3 v_world;
out vec2 v_uv;

void main() {
	v_world = a_position;
	v_uv = a_uv;
	gl_Position = u_view * vec4(a_position, 1.0);
}`;

const FRAGMENT = (bands: number) => `#version 300 es
precision highp float;
precision highp sampler2DArray;
uniform int u_kind;
uniform vec3 u_camera;
uniform vec3 u_cut;
uniform vec4 u_colour;
uniform vec4 u_other;
uniform vec3 u_layers;
uniform bool u_ready;
uniform sampler2DArray u_glyphs;
uniform sampler2D u_sprite;
in vec3 v_world;
in vec2 v_uv;
out vec4 colour;

void main() {
	if ((u_kind == ${FLAT} || u_kind == ${FACE}) && dot(v_world.xz, u_cut.xy) >= u_cut.z) discard;
	if (u_kind == ${FLAT}) {
		if (u_camera.y >= v_world.y) discard;
		colour = u_colour;
	} else if (u_kind == ${FACE}) {
		if (dot(u_camera.xz - v_world.xz, -v_world.xz) <= 0.0) discard;
		if (u_ready) {
			colour = mix(texture(u_glyphs, vec3(v_uv, u_layers.x)), texture(u_glyphs, vec3(v_uv, u_layers.y)), u_layers.z);
		} else {
			float band = 1.0 - abs(fract(v_uv.y * ${bands.toFixed(1)}) * 2.0 - 1.0);
			colour = mix(u_colour, u_other, band);
		}
	} else if (u_kind == ${GRADIENT}) {
		colour = mix(u_colour, u_other, v_uv.y);
	} else {
		colour = texture(u_sprite, v_uv);
	}
}`;

export function pit(canvas: HTMLCanvasElement, bands: number): Pit | null {
	const gl = canvas.getContext('webgl2', { antialias: true, alpha: false });
	if (!gl) return null;
	const program = link(gl, VERTEX, FRAGMENT(bands));
	if (!program) return null;
	gl.useProgram(program);
	const at = (name: string) => gl.getUniformLocation(program, name);
	const uniforms = Object.fromEntries(['u_view', 'u_kind', 'u_camera', 'u_cut', 'u_colour', 'u_other', 'u_layers', 'u_ready', 'u_glyphs', 'u_sprite'].map((name) => [name, at(name)]));
	gl.uniform1i(uniforms.u_glyphs, 0);
	gl.uniform1i(uniforms.u_sprite, 1);

	const vao = gl.createVertexArray();
	gl.bindVertexArray(vao);
	const buffer = gl.createBuffer();
	gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
	[
		['a_position', 3, 0],
		['a_uv', 2, 12]
	].forEach(([name, size, offset]) => {
		const location = gl.getAttribLocation(program, name as string);
		gl.enableVertexAttribArray(location);
		gl.vertexAttribPointer(location, size as number, gl.FLOAT, false, 20, offset as number);
	});

	let glyphs = gl.createTexture();
	const sprites: WebGLTexture[] = [];
	let ready = false;
	const anisotropy = gl.getExtension('EXT_texture_filter_anisotropic');

	gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
	gl.depthFunc(gl.LESS);

	return {
		mesh(data) {
			gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
			gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);
		},
		glyphs(frames) {
			const layers = frames.flat();
			const [first] = layers as HTMLCanvasElement[];
			if (!first) return;
			gl.deleteTexture(glyphs);
			glyphs = gl.createTexture();
			gl.activeTexture(gl.TEXTURE0);
			gl.bindTexture(gl.TEXTURE_2D_ARRAY, glyphs);
			gl.texStorage3D(gl.TEXTURE_2D_ARRAY, 1 + Math.floor(Math.log2(Math.max(first.width, first.height))), gl.RGBA8, first.width, first.height, layers.length);
			layers.forEach((layer, k) => gl.texSubImage3D(gl.TEXTURE_2D_ARRAY, 0, 0, 0, k, first.width, first.height, 1, gl.RGBA, gl.UNSIGNED_BYTE, layer));
			gl.generateMipmap(gl.TEXTURE_2D_ARRAY);
			gl.texParameteri(gl.TEXTURE_2D_ARRAY, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
			gl.texParameteri(gl.TEXTURE_2D_ARRAY, gl.TEXTURE_WRAP_S, gl.REPEAT);
			gl.texParameteri(gl.TEXTURE_2D_ARRAY, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
			if (anisotropy) gl.texParameterf(gl.TEXTURE_2D_ARRAY, anisotropy.TEXTURE_MAX_ANISOTROPY_EXT, Math.min(8, gl.getParameter(anisotropy.MAX_TEXTURE_MAX_ANISOTROPY_EXT)));
			ready = true;
		},
		sprite(k, image) {
			const texture = (sprites[k] ??= gl.createTexture());
			gl.activeTexture(gl.TEXTURE1);
			gl.bindTexture(gl.TEXTURE_2D, texture);
			gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image);
			gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
			gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
			gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
			gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
		},
		draw({ width, height, ratio, view, camera, cut, background, calls }) {
			const [w, h] = [Math.round(width * ratio), Math.round(height * ratio)];
			if (canvas.width !== w || canvas.height !== h) [canvas.width, canvas.height] = [w, h];
			gl.viewport(0, 0, w, h);
			gl.clearColor(background[0], background[1], background[2], 1);
			gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
			gl.useProgram(program);
			gl.bindVertexArray(vao);
			gl.uniformMatrix4fv(uniforms.u_view, false, view);
			gl.uniform3fv(uniforms.u_camera, camera);
			gl.uniform3fv(uniforms.u_cut, cut);
			gl.uniform1i(uniforms.u_ready, ready ? 1 : 0);
			gl.activeTexture(gl.TEXTURE0);
			gl.bindTexture(gl.TEXTURE_2D_ARRAY, glyphs);
			calls.forEach(({ first, count, kind, colour, other, layers, sprite }) => {
				const flat = kind === GRADIENT || kind === SPRITE;
				if (kind === SPRITE && !sprites[sprite!]) return;
				gl[flat ? 'disable' : 'enable'](gl.DEPTH_TEST);
				gl[kind === SPRITE ? 'enable' : 'disable'](gl.BLEND);
				gl.uniform1i(uniforms.u_kind, kind);
				if (colour) gl.uniform4f(uniforms.u_colour, colour[0], colour[1], colour[2], 1);
				if (other) gl.uniform4f(uniforms.u_other, other[0], other[1], other[2], 1);
				if (layers) gl.uniform3fv(uniforms.u_layers, layers);
				if (kind === SPRITE) {
					gl.activeTexture(gl.TEXTURE1);
					gl.bindTexture(gl.TEXTURE_2D, sprites[sprite!]);
				}
				gl.drawArrays(gl.TRIANGLES, first, count);
			});
		},
		destroy() {
			gl.deleteBuffer(buffer);
			gl.deleteTexture(glyphs);
			sprites.forEach((texture) => gl.deleteTexture(texture));
			gl.deleteVertexArray(vao);
			gl.deleteProgram(program);
			gl.getExtension('WEBGL_lose_context')?.loseContext();
		}
	};
}
