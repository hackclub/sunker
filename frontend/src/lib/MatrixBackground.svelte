<script lang="ts">
	import { untrack } from 'svelte';
	import { RING_CHARSETS, RING_COUNT, ringGeometry, ringsState } from '#lib/rings.svelte.js';

	const FONT = "14px 'JetBrains Mono', monospace";
	let LINE = 16;
	const FPS = 20;
	const MAX_COMETS = 3;
	const MAX_SPARKS = 40;
	const PULSE_MS = 8000;
	const DEFAULT_CHARS =
		'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789αβγδελμπσφψωΔΣΩ{}[]()<>/\\|=+-*&^%$#@!?:;~';
	const CHARSETS = [...RING_CHARSETS, DEFAULT_CHARS];

	type Region = {
		el: HTMLParagraphElement;
		wrap: HTMLDivElement;
		set: string;
		left: number;
		top: number;
		charWidth: number;
		cols: number;
		rows: number;
	};

	type Comet = { x: number; y: number; dx: number; dy: number; age: number; life: number; last: string };

	let container: HTMLDivElement;
	let regions: Region[] = [];
	let comets: Comet[] = [];
	let sparks = 0;
	let lit: Region | null = null;
	let mouse: [number, number] | null = null;

	const randomChar = (set: string) => set[Math.floor(Math.random() * set.length)];

	function ringAt(x: number, y: number) {
		if (!ringsState.active) return RING_COUNT;
		const { cx, cy, radii } = ringGeometry(window.innerWidth, window.innerHeight);
		const ring = radii.findIndex((r) => Math.hypot(x - cx, y - cy) < r);
		return ring === -1 ? RING_COUNT : ring;
	}

	function syncPulse(el: HTMLElement) {
		el.style.animationDelay = `${-(Number(document.timeline.currentTime) % PULSE_MS)}ms`;
	}

	function measure(text: string) {
		const p = document.createElement('p');
		const probe = document.createElement('span');
		probe.textContent = text.repeat(10);
		p.append(probe);
		container.append(p);
		const { width, height } = probe.getBoundingClientRect();
		p.remove();
		return [width / (text.length * 10), height];
	}

	function makeRegion(
		k: number,
		cell: number,
		left: number,
		top: number,
		right: number,
		bottom: number,
		ring?: { cx: number; cy: number; inner: number; outer: number }
	) {
		const set = CHARSETS[k];
		const [natural] = measure(set);
		const charWidth = cell * Math.max(1, Math.round(natural / cell));
		left = Math.floor(left / charWidth) * charWidth;
		top = Math.floor(top / LINE) * LINE;
		const cols = Math.ceil((right - left) / charWidth);
		const rows = Math.ceil((bottom - top) / LINE);
		const dim = ((RING_COUNT - k) / RING_COUNT) * 0.6;
		const wrap = document.createElement('div');
		const el = document.createElement('p');
		wrap.append(el);
		container.append(wrap);
		wrap.style.left = `${left}px`;
		wrap.style.top = `${top}px`;
		wrap.style.width = `${right - left}px`;
		wrap.style.height = `${bottom - top}px`;
		wrap.style.setProperty('--dim', String(dim));
		el.style.letterSpacing = `${charWidth - natural}px`;
		syncPulse(el);
		el.textContent = Array.from({ length: rows }, () =>
			Array.from({ length: cols }, () => randomChar(set)).join('')
		).join('\n');
		if (ring) {
			const { cx, cy, inner, outer } = ring;
			wrap.style.maskImage = `radial-gradient(circle at ${cx - left}px ${cy - top}px, transparent ${inner}px, #000 ${inner}px, #000 ${outer}px, transparent ${outer}px)`;
			const shade = document.createElement('div');
			shade.className = 'shade';
			const texture = document.createElement('div');
			texture.className = 'texture';
			const size = `${cols * charWidth}px ${rows * LINE}px`;
			for (let i = 0; i < 3; i++) {
				const noise = document.createElement('div');
				noise.style.backgroundImage = `url(${noiseImage(cols, rows, 14 * (1 - dim), 15 * (1 - dim), 16 * (1 - dim))})`;
				noise.style.backgroundSize = size;
				texture.append(noise);
			}
			wrap.prepend(shade);
			wrap.append(texture);
		}
		regions[k] = { el, wrap, set, left, top, charWidth, cols, rows };
	}

	function noiseImage(cols: number, rows: number, r: number, g: number, b: number) {
		const canvas = document.createElement('canvas');
		canvas.width = cols;
		canvas.height = rows;
		const ctx = canvas.getContext('2d')!;
		const image = ctx.createImageData(cols, rows);
		for (let i = 0; i < cols * rows; i++) {
			image.data.set([r, g, b, Math.random() * 120], i * 4);
		}
		ctx.putImageData(image, 0, 0);
		return canvas.toDataURL();
	}

	function build() {
		container.replaceChildren();
		regions = [];
		comets = [];
		sparks = 0;
		lit = null;
		const width = window.innerWidth;
		const height = window.innerHeight;
		const [cell, line] = measure('0');
		LINE = line;
		container.style.setProperty('--line', `${line}px`);
		if (!ringsState.active) {
			makeRegion(RING_COUNT, cell, 0, 0, width, height);
			return;
		}
		const { cx, cy, radii } = ringGeometry(width, height);
		radii.forEach((outer, k) => {
			makeRegion(
				k,
				cell,
				Math.max(0, cx - outer),
				Math.max(0, cy - outer),
				Math.min(width, cx + outer),
				Math.min(height, cy + outer),
				{ cx, cy, inner: k === 0 ? 0 : radii[k - 1], outer }
			);
		});
		updateHover();
	}

	function spark(region: Region, col: number, row: number) {
		let offset = row * (region.cols + 1) + col;
		for (const node of region.el.childNodes) {
			const length = node.textContent!.length;
			if (offset >= length) {
				offset -= length;
				continue;
			}
			if (node.nodeType !== Node.TEXT_NODE) return;
			const target = (node as Text).splitText(offset);
			target.splitText(1);
			const span = document.createElement('span');
			span.className = 'spark';
			span.textContent = randomChar(region.set);
			target.replaceWith(span);
			sparks++;
			span.addEventListener('animationend', () => {
				span.replaceWith(span.textContent!);
				region.el.normalize();
				sparks--;
			});
			return;
		}
	}

	function step() {
		if (comets.length < MAX_COMETS && Math.random() < 0.1) {
			const angle = Math.random() * Math.PI * 2;
			const speed = 8 + Math.random() * 8;
			comets.push({
				x: Math.random() * window.innerWidth,
				y: Math.random() * window.innerHeight,
				dx: Math.cos(angle) * speed,
				dy: Math.sin(angle) * speed,
				age: 0,
				life: 20 + Math.random() * 40,
				last: ''
			});
		}
		for (const comet of comets) {
			comet.age++;
			comet.x += comet.dx;
			comet.y += comet.dy;
			comet.last = strike(comet.x, comet.y, comet.last);
		}
		comets = comets.filter((comet) => comet.age < comet.life);
	}

	function strike(x: number, y: number, last: string) {
		const k = ringAt(x, y);
		const region = regions[k];
		if (!region || sparks >= MAX_SPARKS) return last;
		const col = Math.floor((x - region.left) / region.charWidth);
		const row = Math.floor((y - region.top) / LINE);
		if (col < 0 || col >= region.cols || row < 0 || row >= region.rows) return last;
		const key = `${k}:${col}:${row}`;
		if (key !== last) spark(region, col, row);
		return key;
	}

	function updateHover() {
		const k = mouse ? ringAt(...mouse) : RING_COUNT;
		const next = k < RING_COUNT ? regions[k] : null;
		if (next === lit) return;
		if (lit) syncPulse(lit.el);
		lit?.wrap.classList.remove('lit');
		next?.wrap.classList.add('lit');
		lit = next;
	}

	$effect(() => {
		ringsState.active;
		untrack(build);
	});

	$effect(() => {
		const onMove = (e: PointerEvent) => {
			mouse = [e.clientX, e.clientY];
			updateHover();
		};
		const onLeave = () => {
			mouse = null;
			updateHover();
		};
		document.fonts.load(FONT).then(build);
		window.addEventListener('resize', build);
		window.addEventListener('pointermove', onMove);
		document.documentElement.addEventListener('pointerleave', onLeave);
		const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
		const timer = reducedMotion ? undefined : setInterval(step, 1000 / FPS);
		return () => {
			clearInterval(timer);
			window.removeEventListener('resize', build);
			window.removeEventListener('pointermove', onMove);
			document.documentElement.removeEventListener('pointerleave', onLeave);
		};
	});
</script>

<div class="matrix" bind:this={container} aria-hidden="true"></div>

<style>
	.matrix {
		position: fixed;
		inset: 0;
		z-index: -1;
		overflow: hidden;
		pointer-events: none;
	}

	.matrix :global(div) {
		position: absolute;
		mask-repeat: no-repeat;
	}

	.matrix :global(.shade),
	.matrix :global(.texture),
	.matrix :global(.texture div) {
		inset: 0;
	}

	.matrix :global(.shade) {
		background: rgb(0 0 0 / var(--dim));
	}

	.matrix :global(.texture) {
		opacity: 0;
		transition: opacity 0.6s;
	}

	.matrix :global(.texture div) {
		background-repeat: no-repeat;
		image-rendering: pixelated;
		animation: drift 0.6s ease-in-out infinite alternate paused;
	}

	.matrix :global(.texture div:nth-child(2)) {
		animation-delay: -0.4s;
	}

	.matrix :global(.texture div:nth-child(3)) {
		animation-delay: -0.8s;
	}

	.matrix :global(.lit .texture) {
		opacity: 1;
		transition-duration: 0.1s;
	}

	.matrix :global(.lit .texture div) {
		animation-play-state: running;
	}

	.matrix :global(div:not(.lit) > p) {
		animation: pulse 8s ease-in-out infinite;
	}

	@keyframes -global-pulse {
		50% {
			opacity: 0.8;
		}
	}

	@keyframes -global-drift {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	.matrix :global(p) {
		--base: rgb(139 207 134 / calc(0.05 * (1 - var(--dim))));
		--spark: rgb(139 207 134 / calc(0.4 * (1 - var(--dim))));
		position: absolute;
		inset: 0;
		margin: 0;
		overflow: hidden;
		white-space: pre;
		font: 14px/var(--line) 'JetBrains Mono', monospace;
		color: var(--base);
		transition: color 0.6s;
	}

	.matrix :global(.lit p) {
		transition-duration: 0.1s;
		--base: rgb(110 255 120 / 0.7);
		--spark: rgb(110 255 120 / 0.03);
	}

	.matrix :global(.spark) {
		animation: spark 1s ease-out;
	}

	@keyframes -global-spark {
		5% {
			color: var(--spark);
		}
	}
</style>
