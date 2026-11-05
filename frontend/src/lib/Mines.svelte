<script lang="ts" module>
	import { BASES } from '#lib/bases.js';
	import { colour, rgb } from '#lib/colors.js';
	import { RING_CHARSETS, RING_COUNT } from '#lib/rings.svelte.js';

	const BANDS = 3;
	const CELL = [9, 16];
	const TILE = [20, 8];
	const FONT = "14px 'JetBrains Mono', monospace";
	const CHURN = 0.08;
	const SHARP = 3;
	const FRAMES = 4;
	let mixed: { key: string; dirt: number[]; gray: number[] } | undefined;
	const shade = (light: number, depth = 0) => {
		const key = BASES.dirt.flat + colour('mines-depth');
		if (mixed?.key !== key) mixed = { key, dirt: rgb(BASES.dirt.flat), gray: rgb(colour('mines-depth')) };
		const { dirt, gray } = mixed;
		return dirt.map((c, i) => Math.min(1, c * light) * (1 - depth) + gray[i] * depth);
	};
	const tint = (light: number, depth = 0, alpha = 1) => `rgb(${shade(light, depth).map((c) => Math.round(c * 255)).join(' ')} / ${alpha})`;

	let baking: Promise<HTMLCanvasElement[][]> | undefined;
	let baked: HTMLCanvasElement[][] | undefined;
	let bakedFor = '';
	let encoding: Promise<string[][]> | undefined;
	let encoded: HTMLCanvasElement[][] | undefined;

	export function bake() {
		const signature = tint(1, 0.5);
		if (signature !== bakedFor) [baking, bakedFor] = [undefined, signature];
		return (baking ??= document.fonts.load(FONT).then(() => {
			let light = 0.62;
			const rings = RING_CHARSETS.toReversed().map((set, i) => {
				const pick = () => set[Math.floor(Math.random() * set.length)];
				const base = Array.from({ length: TILE[0] * TILE[1] }, pick);
				const [dark, bright, color] = [tint(light * 0.9, i / RING_COUNT), tint(light * 1.1, i / RING_COUNT), tint(light * 1.8, i / RING_COUNT, 0.5)];
				light *= 0.86;
				const draw = (cells: string[]) => {
					const [width, height] = [TILE[0] * CELL[0], TILE[1] * CELL[1]];
					const canvas = document.createElement('canvas');
					[canvas.width, canvas.height] = [width * SHARP, height * SHARP];
					const context = canvas.getContext('2d', { willReadFrequently: true })!;
					context.scale(SHARP, SHARP);
					const bands = context.createLinearGradient(0, 0, 0, height);
					for (let k = 0; k <= BANDS * 2; k++) bands.addColorStop(k / (BANDS * 2), k % 2 ? bright : dark);
					context.fillStyle = bands;
					context.fillRect(0, 0, width, height);
					context.font = FONT;
					context.textBaseline = 'top';
					context.fillStyle = color;
					cells.forEach((cell, k) => context.fillText(cell, (k % TILE[0]) * CELL[0], Math.floor(k / TILE[0]) * CELL[1]));
					return canvas;
				};
				return [draw(base), ...Array.from({ length: FRAMES }, () => draw(base.map((cell) => (Math.random() < CHURN ? pick() : cell))))];
			});
			return (baked = rings.toReversed());
		}).catch((e) => {
			baking = undefined;
			throw e;
		}));
	}

	function encode(canvases: HTMLCanvasElement[][]) {
		if (encoded !== canvases) [encoding, encoded] = [undefined, canvases];
		return (encoding ??= Promise.all(
			canvases.map((frames) =>
				Promise.all(
					frames.map(async (canvas) => {
						const url = await new Promise<string>((resolve) => canvas.toBlob((blob) => resolve(URL.createObjectURL(blob!))));
						const image = new Image();
						image.src = url;
						await image.decode();
						return `url("${url}")`;
					})
				)
			)
		));
	}
</script>

<script lang="ts">
	import { cubicOut } from 'svelte/easing';
	import { untrack } from 'svelte';
	import { fade } from 'svelte/transition';
	import { RESOURCE_NAMES } from '#lib/admin/analytics.js';
	import { PALETTE } from '#lib/colors.js';
	import Dialog from '#lib/Dialog.svelte';
	import { request } from '#lib/joel/joel.js';
	import type { Prize, Contract } from '#lib/mines.js';
	import { load } from '#lib/projects.svelte.js';
	import { image } from '#lib/styles.svelte.js';
	import { params, update } from '#lib/url.svelte.js';
	import { FLAT, GRADIENT, SPRITE, FACE, pit, type Call, type Pit } from '#lib/pit.js';

	let { onback, onproject }: { onback: () => void; onproject: () => void } = $props();

	const SIDES = 48;
	const RADIUS = 1;
	const LEDGE = 0.11;
	const WALL = 0.13;
	const RIM = 0.8;
	const SKY = [16, 6, 4];
	const PERSPECTIVE = 1.2;
	const SHEET = 0.45;
	const PANEL = 402;
	const HANG = 100;
	const FAR = 0.2;
	const PITCH = 14;
	const YAW = 20;
	const FORWARD = 0.8;
	const DEPTH = 0.1;
	const RISE = 0.16;
	const NEAR = 0.15;
	const STEP = 700;
	const BUILDINGS = ['building1', 'building2'];
	const TOWERS = [
		[-3.6, 0.55],
		[-3, 0.75],
		[-2.3, 0.48],
		[-1.4, 0.85],
		[-0.6, 0.52],
		[0.3, 0.7],
		[1, 0.46],
		[1.9, 0.62]
	];
	const TABS = [
		['projects', 'Contracts'],
		['prizes', 'Prizes']
	] as const;
	const RINGS = Array.from({ length: RING_COUNT }, (_, i) => RING_COUNT - 1 - i);
	const ENTER = 450;
	const LEAVE = 250;
	const SLICE = Math.tan(Math.PI / SIDES) * 2;
	const CORNER = 1 / Math.cos(Math.PI / SIDES);
	const SEGMENTS = 192;
	const ASPECT = (TILE[0] * CELL[0]) / (TILE[1] * CELL[1]);

	const ANGLES = Array.from({ length: SIDES }, (_, k) => (k * 360) / SIDES - 180);
	const radians = (degrees: number) => (degrees * Math.PI) / 180;
	const around = (radius: number) => ANGLES.filter((angle) => radius * Math.cos(radians(angle - YAW)) + FORWARD * Math.cos(radians(YAW)) > NEAR);

	function annulus(outer: number, inner: number) {
		const circle = (r: number) => `M ${outer - r} ${outer} a ${r} ${r} 0 1 0 ${2 * r} 0 a ${r} ${r} 0 1 0 ${-2 * r} 0 Z`;
		return {
			width: `${2 * outer}px`,
			height: `${outer + Math.min(outer, (FORWARD * Math.cos(radians(YAW)) - NEAR) * unit)}px`,
			clip: `path(evenodd, '${circle(outer)} ${circle(inner)}')`,
			transform: `rotateY(${YAW}deg) rotateX(90deg) translate(${-outer}px, ${-outer}px)`
		};
	}

	let innerWidth = $state(window.innerWidth);
	let innerHeight = $state(window.innerHeight);
	const sheet = $derived(innerWidth <= 1024 && innerHeight >= innerWidth);
	const room = $derived(sheet ? innerHeight * SHEET : innerHeight);
	const unit = $derived(Math.min(room, innerWidth * 0.75));
	const perspective = $derived(PERSPECTIVE * room);
	const centre = $derived([sheet ? innerWidth / 2 : Math.min((innerWidth * 2) / 3, innerWidth - PANEL + HANG - FAR * unit), room * 0.62]);

	let canvases = $state.raw(baked);
	let textures = $state<string[][]>([]);
	let tick = $state(0);
	let gpu = $state(true);
	let canvas = $state<HTMLCanvasElement>();
	let painter = $state.raw<Pit>();
	let aspects = $state<number[]>([]);
	const still = matchMedia('(prefers-reduced-motion: reduce)').matches;
	const touch = matchMedia('(hover: none)').matches;
	let rise = 0;

	$effect(() => {
		bake()
			.then((drawn) => (canvases = drawn))
			.catch(() => {});
	});

	$effect(() => {
		if (gpu || !canvases) return;
		encode(canvases)
			.then((urls) => (textures = urls))
			.catch(() => {});
	});

	$effect(() => {
		if (gpu || still) return;
		const id = setInterval(() => tick++, STEP);
		return () => clearInterval(id);
	});

	$effect(() => {
		const surface = canvas;
		if (!surface) return;
		const gl = pit(surface, BANDS);
		if (!gl) {
			gpu = false;
			return;
		}
		const lost = () => (gpu = false);
		surface.addEventListener('webglcontextlost', lost);
		painter = gl;
		BUILDINGS.forEach((name, k) => {
			const sprite = new Image();
			sprite.onload = () => {
				gl.sprite(k, sprite);
				aspects[k] = sprite.naturalWidth / sprite.naturalHeight;
			};
			sprite.src = image(name);
		});
		return () => {
			surface.removeEventListener('webglcontextlost', lost);
			painter = undefined;
			gl.destroy();
		};
	});

	$effect(() => {
		if (painter && canvases) painter.glyphs(canvases);
	});

	const phase = (ring: number) => (tick + ring) % 4;
	const overlay = (ring: number) => textures[ring]?.[1 + (Math.floor((tick + ring) / 4) % FRAMES)];

	const terraces = $derived.by(() => {
		let light = 1;
		return Array.from({ length: RING_COUNT }, (_, i) => {
			const outer = (RADIUS - i * LEDGE) * unit;
			const top = i * WALL * unit;
			const wall = 0.62 * light;
			light *= 0.86;
			const ring = RING_COUNT - 1 - i;
			return {
				ring,
				angles: around(RADIUS - i * LEDGE),
				outer,
				inner: outer - LEDGE * unit,
				top,
				bottom: top + WALL * unit,
				dark: tint(wall * 0.9, i / RING_COUNT),
				light: tint(wall * 1.1, i / RING_COUNT),
				ledge: tint(light, (i + 1) / RING_COUNT),
				lit: tint(light * 1.3, (i + 1) / RING_COUNT)
			};
		});
	});

	const rim = $derived(annulus((RADIUS + RIM) * unit, RADIUS * unit - 1));

	const scene = $derived.by(() => {
		const data: number[] = [];
		const calls: (Call & { ring?: number; lit?: number[] })[] = [];
		const piece = (call: Omit<Call, 'first' | 'count'> & { ring?: number; lit?: number[] }, build: () => void) => {
			const first = data.length / 5;
			build();
			calls.push({ ...call, first, count: data.length / 5 - first });
		};
		const quad = (...corners: number[][]) => [0, 1, 2, 0, 2, 3].forEach((k) => data.push(...corners[k]));
		const plane = (matrix: DOMMatrix, [left, top, right, bottom]: number[]) => {
			const corner = (x: number, y: number, u: number, v: number) => {
				const { x: px, y: py, z: pz } = matrix.transformPoint({ x, y });
				return [px, py, pz, u, v];
			};
			quad(corner(left, top, 0, 0), corner(right, top, 1, 0), corner(right, bottom, 1, 1), corner(left, bottom, 0, 1));
		};
		const ring = (y: number, inner: number, outer: number) => {
			for (let k = 0; k < SEGMENTS; k++) {
				const [a, b] = [k, k + 1].map((j) => (j / SEGMENTS) * Math.PI * 2);
				quad([inner * Math.sin(a), y, inner * Math.cos(a), 0, 0], [outer * Math.sin(a), y, outer * Math.cos(a), 0, 0], [outer * Math.sin(b), y, outer * Math.cos(b), 0, 0], [inner * Math.sin(b), y, inner * Math.cos(b), 0, 0]);
			}
		};

		const sky = new DOMMatrix(`rotateY(${YAW}deg) translateZ(${-SKY[2] * unit}px) translate(${(-SKY[0] * unit) / 2}px, ${-SKY[1] * unit}px)`);
		piece({ kind: GRADIENT, colour: rgb(PALETTE.horizon), other: rgb(PALETTE.sky) }, () => plane(sky, [0, 0, SKY[0] * unit, SKY[1] * unit]));
		TOWERS.forEach(([x, height], i) => {
			const aspect = aspects[i % BUILDINGS.length];
			if (!aspect) return;
			const [middle, tall] = [(SKY[0] / 2 + x) * unit, height * unit];
			piece({ kind: SPRITE, sprite: i % BUILDINGS.length }, () => plane(sky, [middle - (tall * aspect) / 2, SKY[1] * unit - tall, middle + (tall * aspect) / 2, SKY[1] * unit]));
		});
		piece({ kind: FLAT, colour: shade(1) }, () => ring(0, RADIUS * unit - 1, (RADIUS + RIM) * unit));

		let light = 1;
		terraces.forEach(({ ring: index, outer, inner, top, bottom }, i) => {
			const wall = 0.62 * light;
			light *= 0.86;
			const [width, tall] = [outer * SLICE, bottom - top];
			piece({ kind: FACE, ring: index, colour: shade(wall * 0.9, i / RING_COUNT), other: shade(wall * 1.1, i / RING_COUNT) }, () =>
				ANGLES.forEach((angle) => {
					const shift = ((angle * SIDES) / 360) * width;
					const repeat = tall * ASPECT;
					plane(new DOMMatrix(`translateY(${top}px) rotateY(${angle}deg) translateZ(${-outer}px) translate(${-width / 2}px, 0)`), [0, 0, width, tall]);
					const corners = data.length - 30;
					[0, 1, 2, 0, 2, 3].forEach((corner, k) => (data[corners + k * 5 + 3] = ((corner === 1 || corner === 2 ? width : 0) - shift) / repeat));
				})
			);
			const last = i === RING_COUNT - 1;
			piece({ kind: FLAT, ring: index, colour: shade(light, (i + 1) / RING_COUNT), lit: shade(light * 1.3, (i + 1) / RING_COUNT) }, () => ring(bottom, last ? 0 : inner, outer * CORNER + (last ? 2 : 1)));
		});
		return { data: new Float32Array(data), calls };
	});

	$effect(() => {
		painter?.mesh(scene.data);
	});

	function matrix(depth: number) {
		const [width, height, distance] = [innerWidth, innerHeight, perspective];
		const [near, far] = [unit * 0.01, unit * 20];
		const [x, y] = [(2 * centre[0]) / width - 1, 1 - (2 * centre[1]) / height];
		const [a, b] = [(far + near) / (far - near), (-2 * far * near) / (far - near)];
		const projection = DOMMatrix.fromFloat32Array(new Float32Array([(2 * distance) / width, 0, 0, 0, 0, (-2 * distance) / height, 0, 0, -x, -y, -a, -1, x * distance, y * distance, a * distance + b, distance]));
		return projection.multiply(new DOMMatrix(drone(depth)));
	}

	const depth = () => DEPTH - RISE * rise;

	function pick(x: number, y: number) {
		const inverse = matrix(depth()).inverse();
		const [from, to] = [-1, 1].map((z) => {
			const point = inverse.transformPoint({ x: (x / innerWidth) * 2 - 1, y: 1 - (y / innerHeight) * 2, z, w: 1 });
			return [point.x / point.w, point.y / point.w, point.z / point.w];
		});
		const ray = to.map((value, k) => value - from[k]);
		const camera = depth() * unit;
		let best = Infinity;
		let found = null as number | null;
		const hit = (t: number, ring: number | null) => t > 0 && t < best && ((best = t), (found = ring));
		const flat = (level: number, inner: number, outer: number, ring: number | null) => {
			if (camera >= level || !ray[1]) return;
			const t = (level - from[1]) / ray[1];
			const r = Math.hypot(from[0] + ray[0] * t, from[2] + ray[2] * t);
			if (r >= inner && r <= outer) hit(t, ring);
		};
		flat(0, RADIUS * unit - 1, (RADIUS + RIM) * unit, null);
		terraces.forEach(({ ring, outer, inner, top, bottom }, i) => {
			const last = i === RING_COUNT - 1;
			flat(bottom, last ? 0 : inner, outer * CORNER + (last ? 2 : 1), ring);
			const [a, b, c] = [ray[0] ** 2 + ray[2] ** 2, 2 * (from[0] * ray[0] + from[2] * ray[2]), from[0] ** 2 + from[2] ** 2 - outer ** 2];
			const t = (-b + Math.sqrt(b * b - 4 * a * c)) / (2 * a);
			const level = from[1] + ray[1] * t;
			if (level >= top && level <= bottom) hit(t, ring);
		});
		return found;
	}

	$effect(() => {
		const gl = painter;
		if (!gl) return;
		const { calls } = scene;
		const [width, height, ratio] = [innerWidth, innerHeight, devicePixelRatio];
		const background = shade(0.45, 0.4);
		const cut = [Math.sin(radians(YAW)), Math.cos(radians(YAW)), (FORWARD * Math.cos(radians(YAW)) - NEAR) * unit];
		const start = performance.now();
		const glyph = (ring: number, now: number) => {
			const x = still ? 0 : (now - start) / STEP;
			const tick = Math.floor(x);
			const shown = (at: number) => (((at + ring) % 4) + 4) % 4 < 2 ? 1 : 0;
			const f = still ? 1 : Math.min(1, ((x - tick) * STEP) / 700);
			const eased = f * f * (3 - 2 * f);
			const base = ring * (1 + FRAMES);
			return [base, base + 1 + (Math.floor((tick + ring) / 4) % FRAMES), shown(tick - 1) + (shown(tick) - shown(tick - 1)) * eased];
		};
		let frame = requestAnimationFrame(function draw(now) {
			frame = requestAnimationFrame(draw);
			const level = depth();
			gl.draw({
				width,
				height,
				ratio,
				view: matrix(level).toFloat32Array(),
				camera: [0, level * unit, FORWARD * unit],
				cut,
				background,
				calls: calls.map((call) => (call.kind === FACE ? { ...call, layers: glyph(call.ring!, now) } : call.lit && call.ring === glow ? { ...call, colour: call.lit } : call))
			});
		});
		return () => cancelAnimationFrame(frame);
	});

	let hovered = $state<number | null>(null);
	let pointer = $state({ x: window.innerWidth / 2, y: window.innerHeight / 2 });
	let cardHeight = $state(0);

	const ring = $derived(terraces.find(({ ring }) => String(ring) === params().get('ring'))?.ring);
	const glow = $derived(hovered ?? (touch ? (ring ?? null) : null));

	let mines = $state<{ contracts: Contract[]; quarry: Contract[]; prizes: Prize[] } | null>(null);
	let broken = $state(false);
	let claiming = $state<string | null>(null);
	let claimed = $state(false);
	let failure = $state('');
	let tab = $state<(typeof TABS)[number][0]>('projects');
	let list = $state<HTMLElement>();
	const sections: Record<number, HTMLElement> = {};

	$effect(() => {
		fetch('/api/mines', { cache: 'no-cache' })
			.then((response) => (response.ok ? response.json() : Promise.reject()))
			.then((data) => (mines = data))
			.catch(() => (broken = true));
	});

	$effect(() => {
		tab;
		if (mines) untrack(() => ring !== undefined && reveal(ring, false));
	});

	function reveal(ring: number, smooth: boolean) {
		const section = sections[ring];
		if (!list || !section) return;
		list.scrollTo({ top: section.offsetTop, behavior: smooth && duration(false) ? 'smooth' : 'instant' });
	}

	function choose(ring: number) {
		update({ ring }, { replace: true });
		reveal(ring, true);
	}

	async function embark(contract: Contract) {
		claiming = contract.slug;
		try {
			await request('POST', '/api/projects', { contract: contract.slug });
			contract.active = (contract.active ?? 0) + 1;
			failure = '';
			load();
		} catch (e) {
			failure = e instanceof Error ? e.message : String(e);
		}
		claiming = null;
		claimed = true;
	}

	function onescape(event: KeyboardEvent) {
		if (event.key === 'Escape' && !claimed) onback();
	}

	function onkeydown(event: KeyboardEvent, ring: number) {
		if (event.key !== 'Enter' && event.key !== ' ') return;
		event.preventDefault();
		choose(ring);
	}

	function drone(depth: number) {
		return `translateZ(${perspective}px) rotateX(${-PITCH}deg) rotateY(${-YAW}deg) translate3d(0, ${-depth * unit}px, ${-FORWARD * unit}px)`;
	}

	const duration = (back: boolean) => (matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : back ? LEAVE : ENTER);

	function fly(_: Element, { back = false } = {}) {
		return { duration: duration(back), easing: cubicOut, css: (t: number) => `transform: ${drone(DEPTH - RISE * (1 - t))}` };
	}

	function lift(_: Element, { back = false } = {}) {
		return { duration: duration(back), easing: cubicOut, tick: (t: number) => (rise = 1 - t) };
	}

	function hover(event: PointerEvent) {
		pointer = { x: event.clientX, y: event.clientY };
		if (gpu) hovered = pick(event.clientX, event.clientY);
	}
</script>

{#snippet offer(contract: Contract)}
	<article class="item">
		{#if contract.image_url}
			<img src={contract.image_url} alt="" />
		{/if}
		<h3>{contract.name}</h3>
		<p class="count">{contract.active ?? 0} active</p>
		{#if contract.description}
			<p>{contract.description}</p>
		{/if}
		{#if contract.criteria.length}
			<ul class="criteria">
				{#each contract.criteria as criterion, i (i)}
					<li>{criterion}</li>
				{/each}
			</ul>
		{/if}
		{#if Object.keys(contract.resources).length}
			<ul class="resources">
				{#each Object.entries(contract.resources) as [name, amount] (name)}
					<li>+{amount} {RESOURCE_NAMES[name] ?? name}</li>
				{/each}
			</ul>
		{/if}
		{#if contract.video_url}
			<a href={contract.video_url} target="_blank" rel="noreferrer">Watch the video</a>
		{/if}
		<button class="button primary embark" disabled={claiming !== null} onclick={() => embark(contract)}>{claiming === contract.slug ? 'Embarking...' : 'Embark'}</button>
	</article>
{/snippet}

<svelte:window bind:innerWidth bind:innerHeight onkeydown={onescape} />

<div class="mines" style:--pit={tint(0.45, 0.4)} style:--sky={PALETTE.sky} style:--horizon={PALETTE.horizon} style:--x="{centre[0]}px" style:--y="{centre[1]}px" style:--room="{room}px" transition:fade={{ duration: duration(false) / 2 }}>
	<div class="stage" style:perspective="{perspective}px" role="presentation" onpointermove={hover}>
		{#if gpu}
			<canvas
				bind:this={canvas}
				style:cursor={hovered === null ? null : 'pointer'}
				in:lift|global
				out:lift|global={{ back: true }}
				onclick={(event) => {
					const ring = pick(event.clientX, event.clientY);
					if (ring !== null) choose(ring);
				}}
				onpointerleave={() => (hovered = null)}
			></canvas>
		{:else}
			<div class="world" style:transform={drone(DEPTH)} in:fly|global out:fly|global={{ back: true }}>
				<div class="sky" style:width="{SKY[0] * unit}px" style:height="{SKY[1] * unit}px" style:transform="rotateY({YAW}deg) translateZ({-SKY[2] * unit}px) translate(-50%, -100%)">
					{#each TOWERS as [x, height], i}
						<img class="tower" src={image(BUILDINGS[i % BUILDINGS.length])} alt="" style:left="{(SKY[0] / 2 + x) * unit}px" style:height="{height * unit}px" />
					{/each}
				</div>
				<div class="ledge" style:--ledge={tint(1)} style:width={rim.width} style:height={rim.height} style:clip-path={rim.clip} style:transform={rim.transform}></div>
				{#each terraces as terrace, i (terrace.ring)}
					{@const { ring, angles, outer, inner, top, bottom } = terrace}
					{@const last = i === RING_COUNT - 1}
					<div
						class="terrace"
						class:lit={glow === ring}
						style:--ledge={terrace.ledge}
						style:--lit={terrace.lit}
						style:--dark={terrace.dark}
						style:--light={terrace.light}
						style:--band="{(WALL * unit) / BANDS}px"
						class:shown={phase(ring) < 2}
						style:--glyphs={textures[ring]?.[0]}
						style:--overlay={overlay(ring)}
						role="button"
						tabindex="0"
						aria-label="Ring {ring}"
						onclick={() => choose(ring)}
						onkeydown={(event) => onkeydown(event, ring)}
						onpointerenter={() => (hovered = ring)}
						onpointerleave={() => hovered === ring && (hovered = null)}
						onfocus={(event) => {
							hovered = ring;
							if (event.currentTarget.matches(':focus-visible')) pointer = { x: innerWidth / 2, y: innerHeight / 2 };
						}}
						onblur={() => hovered === ring && (hovered = null)}
					>
						{#each angles as angle}
							<div
								class="wall"
								style:width="{outer * SLICE + 3}px"
								style:background-position-x="{((angle * SIDES) / 360) * outer * SLICE}px"
								style:height="{bottom - top + 1}px"
								style:transform="translateY({top}px) rotateY({angle}deg) translateZ({-outer}px) translate(-50%, 0)"
							></div>
						{/each}
						{#if last}
							<div class="floor" style:width="{outer * CORNER * 2 + 4}px" style:height="{outer * CORNER * 2 + 4}px" style:transform="translateY({bottom}px) rotateX(90deg) translate(-50%, -50%)"></div>
						{:else}
							{@const ledge = annulus(outer * CORNER + 1, inner)}
							<div class="ledge" style:width={ledge.width} style:height={ledge.height} style:clip-path={ledge.clip} style:transform="translateY({bottom}px) {ledge.transform}"></div>
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	</div>

	{#if gpu}
		<div class="rings">
			{#each terraces as { ring } (ring)}
				<button
					aria-label="Ring {ring}"
					onclick={() => choose(ring)}
					onfocus={(event) => {
						hovered = ring;
						if (event.currentTarget.matches(':focus-visible')) pointer = { x: innerWidth / 2, y: innerHeight / 2 };
					}}
					onblur={() => hovered === ring && (hovered = null)}
				></button>
			{/each}
		</div>
	{/if}

	{#if hovered !== null}
		<div class="card" class:flipped={pointer.x > innerWidth - 280} class:above={pointer.y + 16 + cardHeight > innerHeight} style:translate="{pointer.x}px {pointer.y}px" bind:clientHeight={cardHeight}>
			<h2>Ring {hovered}</h2>
		</div>
	{/if}

	<aside class="panel">
		<div class="tabs" role="tablist">
			{#each TABS as [key, label] (key)}
				<button role="tab" aria-selected={tab === key} class:active={tab === key} onclick={() => (tab = key)}>{label}</button>
			{/each}
		</div>
		<div class="list" bind:this={list}>
			{#if tab === 'projects' && mines?.quarry.length}
				<section>
					<h2 class="heading">Quarry</h2>
					<p class="empty">For general projects, rather than these specific projects that dive in to a particular category</p>
					{#each mines.quarry as contract (contract.slug)}
						{@render offer(contract)}
					{/each}
				</section>
			{/if}
			{#each RINGS as section (section)}
				{@const contracts = mines?.contracts.filter((contract) => contract.ring === section) ?? []}
				{@const prizes = mines?.prizes.filter((prize) => prize.ring === section) ?? []}
				<section bind:this={sections[section]}>
					<h2 class="heading" class:selected={ring === section}>Ring {section}</h2>
					{#if broken}
						<p class="empty">Couldn't load the mines</p>
					{:else if mines && !(tab === 'projects' ? contracts : prizes).length}
						<p class="empty">Nothing here yet</p>
					{/if}
					{#if tab === 'projects'}
						{#each contracts as contract (contract.slug)}
							{@render offer(contract)}
						{/each}
					{:else}
						<div class="prizes">
							{#each prizes as prize (prize.id)}
								<article class="item prize">
									<div class="picture" class:blank={!prize.image}>
										{#if prize.image}
											<img src={prize.image} alt="" onerror={(event) => ((event.currentTarget as HTMLElement).hidden = true)} />
										{/if}
									</div>
									{#if prize.tagline}
										<p class="tagline">{prize.tagline}</p>
									{/if}
									<h3>{prize.name}</h3>
									{#if prize.description}
										<p>{prize.description}</p>
									{/if}
								</article>
							{/each}
						</div>
					{/if}
				</section>
			{/each}
		</div>
	</aside>

	<Dialog bind:open={claimed} title={failure ? "Couldn't claim that contract" : 'Contract claimed'}>
		<p class="claimed">{failure || 'Contract claimed and project created, check the main HQ dashboard!'}</p>
		{#if !failure}
			<footer>
				<button class="button primary" onclick={onproject}>Go to project</button>
			</footer>
		{/if}
	</Dialog>

	<h1 class="title">The Mine of Projects</h1>

	<button class="back" aria-label="Back" onclick={onback}>
		<svg viewBox="0 0 24 24"><path d="M15 5l-7 7 7 7" /></svg>
	</button>
</div>

<style>
	.mines {
		position: fixed;
		inset: 0;
		z-index: 30;
		overflow: hidden;
		background: var(--pit);
		color: var(--text);
		font: 13px/1.5 'Inter', sans-serif;
		color-scheme: dark;
	}

	.mines :global(:focus-visible) {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.stage {
		position: absolute;
		inset: 0;
		perspective-origin: var(--x) var(--y);
	}

	canvas {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}

	.rings {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
	}

	.world {
		position: absolute;
		top: var(--y);
		left: var(--x);
		transform-style: preserve-3d;
	}

	.terrace {
		position: absolute;
		transform-style: preserve-3d;
		cursor: pointer;
	}

	.mines .terrace:focus-visible {
		outline: none;
	}

	.sky {
		position: absolute;
		transform-origin: 0 0;
		background: linear-gradient(var(--horizon), var(--sky));
	}

	.tower {
		position: absolute;
		bottom: 0;
		image-rendering: pixelated;
		transform: translateX(-50%);
	}

	.wall,
	.ledge,
	.floor {
		position: absolute;
		transform-origin: 0 0;
		backface-visibility: hidden;
	}

	.wall {
		background-image: var(--glyphs, repeating-linear-gradient(var(--dark), var(--light) calc(var(--band) / 2), var(--dark) var(--band)));
		background-size: auto 100%;
		transform-style: preserve-3d;
	}

	.wall::after {
		content: '';
		position: absolute;
		inset: 0;
		background-image: var(--overlay, none);
		background-size: auto 100%;
		background-position-x: inherit;
		backface-visibility: hidden;
		opacity: 0;
		transform: translateZ(1px);
		transition: opacity 0.7s ease-in-out;
	}

	.shown .wall::after {
		opacity: 1;
	}

	.ledge {
		background: var(--ledge);
	}

	.floor {
		border-radius: 50%;
		background: var(--ledge);
	}

	.lit .ledge,
	.lit .floor {
		background: var(--lit);
	}

	.card {
		position: fixed;
		top: 0;
		left: 0;
		min-width: 120px;
		padding: 10px 14px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		box-shadow: 0 16px 48px var(--shadow);
		pointer-events: none;
		transform: translate(var(--dx, 16px), var(--dy, 16px));
	}

	.card.flipped {
		--dx: calc(-100% - 16px);
	}

	.card.above {
		--dy: calc(-100% - 16px);
	}

	.card h2 {
		margin: 0;
		font: 18px 'Share Tech', sans-serif;
	}

	.panel {
		position: fixed;
		top: 16px;
		right: 16px;
		bottom: 16px;
		display: flex;
		flex-direction: column;
		gap: 12px;
		width: min(360px, calc(100vw - 32px));
		padding: 12px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
	}

	@media (orientation: portrait) and (max-width: 1024px) {
		.panel {
			top: var(--room);
			right: calc(8px + env(safe-area-inset-right, 0px));
			bottom: calc(8px + env(safe-area-inset-bottom, 0px));
			left: calc(8px + env(safe-area-inset-left, 0px));
			width: auto;
		}
	}

	.tabs {
		display: grid;
		grid-template-columns: 1fr 1fr;
		flex-shrink: 0;
		height: 40px;
		border: 1px solid var(--accent);
		border-radius: 6px;
		overflow: hidden;
	}

	.tabs button {
		border: none;
		background: none;
		color: var(--accent);
		font: 500 13px 'Inter', sans-serif;
		cursor: pointer;
	}

	.tabs button:not(.active):hover {
		background: color-mix(in srgb, var(--accent) 15%, transparent);
	}

	.tabs button.active {
		background: var(--accent);
		color: var(--background);
	}

	.list {
		position: relative;
		flex: 1;
		overflow-y: auto;
		overscroll-behavior: contain;
	}

	section {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding-bottom: 16px;
	}

	.heading {
		position: sticky;
		top: 0;
		z-index: 1;
		margin: 0;
		padding: 4px 0 8px;
		background: var(--island);
		color: var(--text-muted);
		font: 18px 'Share Tech', sans-serif;
	}

	.heading.selected {
		color: var(--accent);
	}

	.empty {
		margin: 0;
		color: var(--text-muted);
	}

	.item {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 12px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
	}

	.item img {
		width: 100%;
		aspect-ratio: 16 / 9;
		border-radius: 4px;
		object-fit: cover;
	}

	.prizes {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(calc(50% - 4px), 1fr));
		gap: 8px;
	}

	.prize {
		aspect-ratio: 1;
	}

	.picture {
		display: grid;
		place-items: center;
		aspect-ratio: 4 / 3;
		border-radius: 4px;
		overflow: hidden;
	}

	.blank {
		background: var(--background);
	}

	.item .picture img {
		width: 100%;
		height: 100%;
		aspect-ratio: auto;
		object-fit: contain;
	}

	.item h3 {
		margin: 0;
		font: 18px 'Share Tech', sans-serif;
	}

	.item p,
	.item ul {
		margin: 0;
	}

	.item ul {
		padding-left: 18px;
	}

	.tagline {
		color: var(--text-muted);
		font-style: italic;
	}

	.item .resources {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
		padding: 0;
		list-style: none;
	}

	.resources li {
		padding: 2px 6px;
		border-radius: 4px;
		background: var(--background);
		color: var(--positive);
		font-size: 12px;
	}

	.item a {
		color: var(--accent);
	}

	.embark {
		align-self: flex-start;
		margin-top: 4px;
	}

	@media (pointer: coarse) {
		.tabs {
			height: 44px;
		}

		.embark,
		footer .button {
			min-height: 44px;
		}

		.item a {
			display: inline-flex;
			align-items: center;
			align-self: flex-start;
			min-height: 44px;
		}
	}

	.claimed {
		margin: 0;
	}

	.count {
		color: var(--text-muted);
	}

	footer {
		display: flex;
		justify-content: flex-end;
	}

	.title {
		position: fixed;
		top: calc(16px + env(safe-area-inset-top, 0px));
		left: 50%;
		display: grid;
		place-items: center;
		height: 64px;
		margin: 0;
		padding: 0 28px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		font: 32px 'Share Tech', sans-serif;
		white-space: nowrap;
		transform: translateX(-50%);
	}

	@media (orientation: landscape) and (max-width: 1114px) {
		.title {
			left: calc(72px + env(safe-area-inset-left, 0px));
			transform: none;
		}
	}

	@media (max-width: 600px), (orientation: landscape) and (max-width: 900px) {
		.title {
			height: 48px;
			padding: 0 14px;
			font-size: 18px;
		}
	}

	.back {
		position: fixed;
		top: calc(16px + env(safe-area-inset-top, 0px));
		left: calc(16px + env(safe-area-inset-left, 0px));
		display: grid;
		place-items: center;
		width: 48px;
		height: 48px;
		padding: 0;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		color: inherit;
		cursor: pointer;
	}

	.back:hover {
		background: var(--hover);
	}

	.back svg {
		width: 24px;
		height: 24px;
		fill: none;
		stroke: currentColor;
		stroke-width: 2.5;
		stroke-linecap: round;
		stroke-linejoin: round;
	}

	@media (hover: none) {
		.mines {
			touch-action: manipulation;
		}

		.stage {
			touch-action: none;
		}

		.card {
			display: none;
		}
	}
</style>
