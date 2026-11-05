<script lang="ts">
	import { linear } from 'svelte/easing';
	import { untrack } from 'svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import Dialog from '#lib/admin/Dialog.svelte';
	import Passport from '#lib/onboarding/Passport.svelte';
	import { HEAD, HUE, SHAPES, tint } from '#lib/features.js';
	import type { Character, Guild } from '#lib/me.svelte.js';
	import { params, update } from '#lib/url.svelte.js';
	import type { Rect, Stage } from '#lib/view.js';

	let { shape = null, avoid = null }: { shape?: Stage['crowd']; avoid?: Rect | null } = $props();

	type Person = { id: number; character: Character; guild: Guild | null };

	const TABLE = { width: 1024, height: 256 };

	let people = $state<Person[]>([]);
	let ready = false;
	const viewing = $derived(people.find((person) => String(person.id) === params().get('passport')));

	type Point = { angle: number; x: number; y: number };
	type Spot = Point & { bob: number; delay: number; shift: number; walking: number; start: number };

	const POLL = 15000;
	const OVERLAP = 64;
	const spots = new SvelteMap<number, Spot>();
	let width = $state(0);

	function point(angle: number): Point {
		if (shape) return horseshoe(shape, angle);
		const reach = 1 + Math.random() * 0.12;
		const x = 512 - Math.cos(angle) * 470 * reach;
		return { angle, x, y: 150 - Math.sin(angle) * 96 * reach - ((x - 512) * 14) / 833 };
	}

	function horseshoe({ left, right, top, bottom, person, depth, pad }: NonNullable<Stage['crowd']>, angle: number): Point {
		const [xl, xr, yl, yf] = [left - person * 0.6, right + person * 0.6, bottom - 8, top + 6];
		const [up, across] = [yl - yf, xr - xl];
		const jitter = Math.random() * depth;
		const fit = (x: number) => Math.max(person / 2 + pad, Math.min(innerWidth - person / 2 - pad, x));
		let along = (angle / Math.PI) * (2 * up + across);
		if (along < up) return { angle, x: fit(xl - jitter), y: yl - along };
		along -= up;
		if (along < across) return { angle, x: xl + along, y: yf - jitter };
		return { angle, x: fit(xr + jitter), y: yf + along - across };
	}

	const apart = (a: Point, b: Point) => (shape ? Math.max(Math.abs(a.x - b.x) / shape.person, Math.abs(a.y - b.y) / (shape.person * 0.87)) : Math.hypot(a.x - b.x, (a.y - b.y) * 4));

	function free(spot: Point) {
		if (!shape || !avoid) return true;
		const [half, tall] = [shape.person / 2, shape.person * 1.74];
		return spot.x + half < avoid.x || spot.x - half > avoid.x + avoid.width || spot.y < avoid.y || spot.y - tall > avoid.y + avoid.height;
	}

	function pick(id: number, near?: number, clear = (_: Point) => true) {
		const taken = [...spots].filter(([other]) => other !== id).map(([, spot]) => spot);
		let best: Point | null = null;
		let gap = -1;
		for (let k = 0; k < 24; k++) {
			const angle = near === undefined ? Math.random() * Math.PI : Math.min(Math.PI, Math.max(0, near + (Math.random() - 0.5) * 0.8));
			const candidate = point(angle);
			if (!clear(candidate) || !free(candidate)) continue;
			const nearest = Math.min(Infinity, ...taken.map((other) => apart(other, candidate)));
			if (nearest > gap) [best, gap] = [candidate, nearest];
		}
		return best;
	}

	function place(id: number) {
		const bob = 2.4 + Math.random() * 1.6;
		const spot = pick(id) ?? point(Math.random() * Math.PI);
		spots.set(id, { ...spot, bob, delay: -Math.random() * bob, shift: (Math.random() - 0.5) * 10, walking: 0, start: spot.x });
	}

	function clear(from: Spot, to: Point, others: Spot[]) {
		if (shape) return others.every((other) => apart(to, other) >= 0.7 && apart({ angle: 0, x: (from.x + to.x) / 2, y: (from.y + to.y) / 2 }, other) >= 0.5);
		const [low, high] = [Math.min(from.x, to.x) - OVERLAP, Math.max(from.x, to.x) + OVERLAP];
		return others.every((other) => {
			if (other.walking) return [other.x, other.start].every((x) => x < low || x > high);
			if (other.y < Math.min(from.y, to.y) - 1 || other.y > Math.max(from.y, to.y) + 1) return true;
			const t = to.y === from.y ? 0.5 : Math.min(1, Math.max(0, (other.y - from.y) / (to.y - from.y)));
			return Math.abs(from.x + t * (to.x - from.x) - other.x) >= OVERLAP;
		});
	}

	function wander() {
		const idle = people.filter((person) => !spots.get(person.id)?.walking);
		if (!idle.length) return;
		const { id } = idle[Math.floor(Math.random() * idle.length)];
		const from = spots.get(id)!;
		const others = [...spots].filter(([other]) => other !== id).map(([, spot]) => spot);
		const to = pick(id, from.angle, (to) => clear(from, to, others));
		if (!to) return;
		const walking = Math.max(800, ((Math.hypot(to.x - from.x, to.y - from.y) * (shape ? 1 : width / TABLE.width)) / 120) * 1000);
		spots.set(id, { ...from, ...to, walking, start: from.x });
		setTimeout(() => {
			const spot = spots.get(id);
			if (spot) spots.set(id, { ...spot, walking: 0 });
		}, walking);
	}

	$effect(() => {
		let pending: ReturnType<typeof setTimeout>[] = [];
		const poll = (spread: boolean) =>
			fetch('/api/online', { cache: 'no-cache' })
				.then((response) => (response.ok ? response.json() : Promise.reject()))
				.then((list: Person[]) => {
					pending.forEach(clearTimeout);
					pending = [];
					const next = new Map(list.map((person) => [person.id, person]));
					const present = new Set(people.map((person) => person.id));
					people = people.map((person) => next.get(person.id) ?? person);
					const changes = [
						...list
							.filter((person) => !present.has(person.id))
							.map((person) => () => {
								place(person.id);
								people.push(person);
							}),
						...people
							.filter((person) => !next.has(person.id))
							.map(({ id }) => () => {
								spots.delete(id);
								people = people.filter((person) => person.id !== id);
							})
					];
					for (const change of changes) {
						if (spread) pending.push(setTimeout(change, Math.random() * POLL));
						else change();
					}
				})
				.catch(() => {});
		poll(false).then(() => requestAnimationFrame(() => (ready = true)));
		const timer = setInterval(() => document.hidden || poll(true), POLL);
		let roam: ReturnType<typeof setTimeout>;
		const stroll = () => {
			if (!document.hidden) wander();
			roam = setTimeout(stroll, (20000 + Math.random() * 20000) / Math.max(1, people.length));
		};
		roam = setTimeout(stroll, 4000);
		return () => {
			clearInterval(timer);
			clearTimeout(roam);
			pending.forEach(clearTimeout);
		};
	});

	const signature = $derived(shape ? [shape.left, shape.right, shape.top, shape.bottom, shape.person, avoid?.x, avoid?.y, avoid?.width, avoid?.height].join() : '');

	$effect(() => {
		signature;
		untrack(() => {
			spots.clear();
			for (const person of people) place(person.id);
		});
	});

	function walk(node: HTMLElement) {
		if (!ready) return { duration: 0 };
		const distance = innerWidth - node.getBoundingClientRect().left + 20;
		const duration = Math.max(1500, (distance / 240) * 1000);
		const steps = Math.round(duration / 280);
		return {
			duration,
			easing: linear,
			css: (t: number) => {
				const phase = t * steps * Math.PI;
				const bob = -Math.abs(Math.sin(phase)) * 10;
				const meander = -Math.abs(Math.sin(t * Math.PI * 3)) * 24 * (1 - t);
				return `transform: translate(${(1 - t) * distance}px, ${bob + meander}px) rotate(${Math.sin(phase) * 9}deg)`;
			}
		};
	}
</script>

<div class="crowd" class:upright={!!shape} bind:clientWidth={width}>
	{#each people as person (person.id)}
		{@const spot = spots.get(person.id)!}
		<button
			class="person"
			class:walking={spot.walking}
			style:--walk="{spot.walking}ms"
			style:left={shape ? `${spot.x}px` : `${(spot.x / TABLE.width) * 100}%`}
			style:top={shape ? `${spot.y}px` : `${(spot.y / TABLE.height) * 100}%`}
			style:width={shape ? `${shape.person}px` : null}
			style:z-index={Math.round(spot.y)}
			aria-label="{person.character.handle}'s passport"
			in:walk
			out:walk
			onclick={() => update({ passport: person.id })}
		>
			<svg viewBox="0 0 100 174" aria-hidden="true" style:--body={tint('avatar', person.character.attributes.hue ?? HUE + spot.shift)}>
				<ellipse class="body" cx="50" cy="130" rx="28" ry="43" />
				<g class="head" style:animation-duration="{spot.bob}s" style:animation-delay="{spot.delay}s">
					<path class="body" d={HEAD} />
					{#each person.character.attributes.features as feature, j (j)}
						{@const shape = SHAPES.get(feature.shape)}
						{#if shape}
							<g transform="translate({feature.x * 100} {feature.y * 128}) rotate({feature.rotation})">
								{#each shape.strokes as stroke, k (k)}
									<path
										d={stroke.d}
										fill={stroke.fill ? feature.colour : 'none'}
										stroke={stroke.fill ? 'none' : feature.colour}
										stroke-width={stroke.width}
										stroke-linecap="round"
										stroke-linejoin="round"
									/>
								{/each}
							</g>
						{/if}
					{/each}
				</g>
			</svg>
			<span class="tip">{person.character.handle}{#if person.guild}<small>Guild of {person.guild.name}</small>{/if}</span>
		</button>
	{/each}
</div>

{#if viewing}
	<Dialog title="{viewing.character.handle}'s passport" onclose={() => update({ passport: null })} --width="680px">
		<div class="booklet"><Passport draft={viewing.character} guild={viewing.guild} readonly fold /></div>
	</Dialog>
{/if}

<style>
	.crowd {
		position: fixed;
		top: 50%;
		left: 50%;
		width: calc(100vw / 2);
		aspect-ratio: 4;
		z-index: 1;
		transform: translate(-50%, -50%);
		pointer-events: none;
	}

	.crowd.upright {
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		aspect-ratio: auto;
		transform: none;
	}

	.person {
		position: absolute;
		width: 11%;
		padding: 0;
		border: none;
		background: none;
		cursor: pointer;
		translate: -50% -100%;
		transform-origin: 50% 100%;
		pointer-events: auto;
		transition:
			left var(--walk) linear,
			top var(--walk) linear,
			z-index var(--walk) linear;
	}

	svg {
		display: block;
		width: 100%;
		transform-origin: 50% 100%;
	}

	.walking svg {
		animation: waddle 560ms ease-in-out infinite;
	}

	@keyframes waddle {
		25% {
			transform: translateY(-10px) rotate(9deg);
		}
		75% {
			transform: translateY(-10px) rotate(-9deg);
		}
	}

	.body {
		fill: var(--body);
	}

	.head {
		animation: bob ease-in-out infinite alternate;
	}

	@keyframes bob {
		to {
			transform: translateY(-2.5px);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.head,
		.walking svg {
			animation: none;
		}
	}

	.booklet {
		margin-inline: -24px;
		padding-inline: 24px;
		overflow-x: clip;
	}

	.tip {
		position: absolute;
		bottom: calc(100% + 6px);
		left: 50%;
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: 6px 10px;
		border: 1px solid var(--tip-border);
		border-radius: 6px;
		background: var(--island);
		color: var(--text);
		font: 13px 'Inter', sans-serif;
		white-space: nowrap;
		transform: translateX(-50%);
		pointer-events: none;
		visibility: hidden;
	}

	.tip small {
		color: var(--text-muted);
		font-size: 12px;
	}

	.person:focus-visible .tip {
		visibility: visible;
	}

	@media (hover: hover) {
		.person:hover .tip {
			visibility: visible;
		}
	}
</style>
