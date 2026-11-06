<script lang="ts">
	import SidePanel, { DIALOG_WIDTH, SPLIT, offset } from '#lib/SidePanel.svelte';
	import { PALETTE } from '#lib/colors.js';
	import { image, rendering, smooth } from '#lib/styles.svelte.js';
	import WorldMap from '#lib/WorldMap.svelte';
	import { params, update } from '#lib/url.svelte.js';
	import { page } from '$app/state';
	import { place } from '#lib/map.js';
	import { BAR, FLIGHT, PHONE, crest, footprint, halo, inside, island, reveal, stage, transform, unseat } from '#lib/view.js';
	import Projects from '#lib/hq/Projects.svelte';
	import Propaganda from '#lib/hq/Propaganda.svelte';
	import Prizes from '#lib/hq/Prizes.svelte';
	import Chat from '#lib/hq/Chat.svelte';
	import Operations from '#lib/hq/Operations.svelte';
	import { planner } from '#lib/hq/planner.svelte.js';
	import Crowd from '#lib/hq/Crowd.svelte';
	import Profile from '#lib/hq/Profile.svelte';
	import Resources from '#lib/hq/Resources.svelte';
	import Konami from '#lib/Konami.svelte';
	import Robosquirrel from '#lib/onboarding/Robosquirrel.svelte';
	import Speech from '#lib/onboarding/Speech.svelte';
	import Passport, { FOLD } from '#lib/onboarding/Passport.svelte';
	import { advance, draft, me } from '#lib/me.svelte.js';
	import { FAREWELL, GUILDS, QUIPS, TOUR, tilt as turn } from '#lib/robosquirrel.js';
	import { goto } from '$app/navigation';
	import { tick, type Component } from 'svelte';
	import { fade } from 'svelte/transition';
	import { MediaQuery } from 'svelte/reactivity';

	let { data } = $props();

	type Side = 'left' | 'right';
	type Section = { label: string; icon: string; component: Component; disabled: boolean; dot: boolean };

	const panels: Record<Side, Section[]> = $state({
		left: [
			{ label: 'Projects', icon: 'projects-icon', component: Projects, disabled: false, dot: false },
			{ label: 'Propaganda', icon: 'propaganda-icon', component: Propaganda, disabled: false, dot: false },
			{ label: 'Prizes', icon: 'orders-icon', component: Prizes, disabled: false, dot: false }
		],
		right: [
			{ label: 'Chat', icon: 'chat-icon', component: Chat, disabled: false, dot: false },
			{ label: 'Operations', icon: 'battle-icon', component: Operations, disabled: false, dot: false },
			{ label: 'Profile', icon: 'profile', component: Profile, disabled: false, dot: false }
		]
	});

	const slug = (section: Section) => section.label.toLowerCase();
	const chosen = (side: Side) => panels[side].findIndex((section) => slug(section) === params().get(side));
	const open: Record<Side, boolean> = $derived({ left: chosen('left') >= 0, right: chosen('right') >= 0 });
	const active: Record<Side, number> = $state({ left: 0, right: 0 });
	$effect.pre(() => {
		for (const side of ['left', 'right'] as const) if (open[side]) active[side] = chosen(side);
	});

	function toggle(side: Side, index: number) {
		const shut = open[side] && active[side] === index;
		const changes: Record<string, string | null> = { [side]: shut ? null : slug(panels[side][index]) };
		if (!shut && innerWidth < SPLIT) changes[side === 'left' ? 'right' : 'left'] = null;
		update(changes);
	}

	$effect(() => {
		if (innerWidth && innerWidth < SPLIT && open.left && open.right) update({ [chatting ? 'left' : 'right']: null }, { replace: true });
	});

	type Stop = { text?: string; area?: 'map'; spots: string[] };

	const STOPS: Stop[] = [
		{ text: TOUR[0], spots: ['Projects', 'Prizes'] },
		{ text: TOUR[1], area: 'map', spots: ['Operations'] },
		{ text: TOUR[2], spots: ['Chat'] },
		{ text: GUILDS, spots: [] },
		{ spots: [] },
		{ text: FAREWELL, spots: [] }
	];
	const FINALE = STOPS.length - 1;
	const STAMP = FINALE - 1;

	const arriving = me.current?.stage === 'guild' || me.current?.stage === 'hq';
	let stop = $state<number | null>(me.current?.stage === 'guild' ? 0 : me.current?.stage === 'hq' ? STAMP : null);
	const current = $derived(stop === null ? null : STOPS[stop]);
	const chatting = $derived(current?.spots.includes('Chat') ?? false);
	let spoken = $state(false);
	let tilt = $state(0);
	const passport = draft(me.current?.character ?? null);
	let quip = $state<string | null>(null);
	let quipping: ReturnType<typeof setTimeout> | undefined;
	let bag: string[] = [];
	let said: string | null = null;

	function chatter() {
		if (!bag.length) {
			bag = [...QUIPS];
			for (let i = bag.length - 1; i > 0; i--) {
				const j = Math.floor(Math.random() * (i + 1));
				[bag[i], bag[j]] = [bag[j], bag[i]];
			}
			if (bag.length > 1 && bag[bag.length - 1] === said) [bag[0], bag[bag.length - 1]] = [bag[bag.length - 1], bag[0]];
		}
		quip = said = bag.pop()!;
		clearTimeout(quipping);
		quipping = setTimeout(() => (quip = null), 10000);
	}

	let closing = $state(false);
	let assigned = $state(0);
	let booklet = $state(0);

	async function shut() {
		closing = true;
		await new Promise((resolve) => setTimeout(resolve, FOLD));
		closing = false;
	}

	async function onward() {
		if (stop === null || closing) return;
		if (current?.text && !spoken) return speech?.finish();
		if (stop === STAMP) await shut();
		tilt = turn(tilt);
		if (stop === FINALE) {
			stop = null;
			advance('hq')
				.then(() => advance('done'))
				.catch(console.error);
		} else stop++;
		chat();
	}

	async function backward() {
		if (stop === null || closing) return;
		if (stop === STAMP) await shut();
		tilt = turn(tilt);
		if (stop === 0) goto('/onboarding?passport');
		else stop--;
		chat();
	}

	function chat() {
		if (chatting || params().get('right') === 'chat') update({ right: chatting ? 'chat' : null }, { replace: true });
	}

	let speech = $state<Speech>();

	function onkeydown(event: KeyboardEvent) {
		if (current) {
			if (event.key !== ' ' || (event.target instanceof Element && event.target.closest('input, textarea, [contenteditable]'))) return;
			event.preventDefault();
			return onward();
		}
		if (event.key !== 'Escape' || (event.target instanceof Element && event.target.closest('dialog'))) return;
		if (wanted) close();
		else if (open.left || open.right) update({ left: null, right: null });
	}

	const PLANKS = ['plank', 'plank2', 'plank3'];
	const BUILDINGS = ['building1', 'building2'];
	const SCALE = 1 / 768;
	const WIDTH = 128;
	const SPACING = 96;
	const MULLION = 3;
	const RAIL = 5;
	const REFRACTION = 8;
	const ANGLE = 40;
	const SMOOTH = false;

	const seed = 0x5eed;

	function pick(row: number, col: number, count: number) {
		const h = Math.imul(seed ^ (row * 73856093) ^ (col * 19349663), 2654435761) >>> 0;
		return ((h ^ (h >>> 16)) >>> 0) % count;
	}

	function random(n: number) {
		const h = Math.imul(seed ^ Math.imul(n + 1, 2246822519), 2654435761) >>> 0;
		return ((h ^ (h >>> 16)) >>> 0) / 2 ** 32;
	}

	let innerWidth = $state(0);
	let innerHeight = $state(0);
	let viewportTop = $state(0);
	let viewportBottom = $state(0);

	$effect(() => {
		const visual = window.visualViewport;
		if (!visual) return;
		const sync = () => {
			const zoomed = visual.scale > 1.01;
			viewportTop = zoomed ? 0 : Math.round(visual.offsetTop);
			viewportBottom = zoomed ? 0 : Math.max(0, Math.round(window.innerHeight - visual.offsetTop - visual.height));
		};
		sync();
		visual.addEventListener('resize', sync);
		visual.addEventListener('scroll', sync);
		return () => {
			visual.removeEventListener('resize', sync);
			visual.removeEventListener('scroll', sync);
		};
	});
	let probe: HTMLElement;
	let insets = $state({ top: 0, bottom: 0 });
	const portrait = $derived(innerHeight > innerWidth);
	const dock = $derived(portrait && innerWidth <= PHONE ? BAR + insets.bottom : 0);
	let sheet = $state(0);
	let ceiling = $state(0);
	const hidden = $derived(sheet ? sheet + 8 + dock : 0);
	$effect(() => {
		innerHeight;
		if (!dock || planner.selectable === null) return void (sheet = ceiling = 0);
		tick().then(() => {
			sheet = document.querySelector<HTMLElement>('.dialog.peek')?.offsetHeight ?? 0;
			ceiling = document.querySelector('.resources')?.getBoundingClientRect().bottom ?? 0;
		});
	});
	const covered = $derived(planner.selectable !== null && innerWidth < DIALOG_WIDTH * 2 && !dock);
	const hud = $derived(innerWidth >= 640 ? 82 : 118);
	const span = $derived(portrait ? Math.max(innerWidth, innerHeight * 0.9) : innerWidth);

	$effect(() => {
		innerWidth;
		innerHeight;
		const style = getComputedStyle(probe);
		insets = { top: parseFloat(style.paddingTop) || 0, bottom: parseFloat(style.paddingBottom) || 0 };
	});

	const shrink = $derived(booklet ? Math.min(1, (innerHeight - 72 - (dock ? dock + 52 : 0) - assigned) / booklet) : 1);
	let ratio = $state(1);
	let natural = $state(0);
	let floor: HTMLCanvasElement;
	let windows: HTMLCanvasElement;
	let images: HTMLImageElement[] = $state([]);
	const blend = $derived(PLANKS.every(smooth));
	let buildings: HTMLImageElement[] = $state([]);

	function load(src: string) {
		const image = new Image();
		image.src = src;
		return image.decode().then(() => image);
	}

	async function settle(sources: string[]) {
		const results = await Promise.allSettled(sources.map(load));
		return results.flatMap((result) => {
			if (result.status === 'fulfilled') return [result.value];
			console.error(result.reason);
			return [];
		});
	}

	const tiles = $derived(place(data.map));
	const layout = $derived(stage(tiles, innerWidth, innerHeight, insets.top, insets.bottom, hud, portrait));
	const wall = $derived(Math.round(layout.wall * ratio));
	const pose = $derived(transform(layout.pose));
	const outline = $derived(island(tiles, innerWidth, innerHeight));
	const clip = $derived(footprint(outline));
	const border = $derived(halo(outline, layout.pose, innerWidth, innerHeight));
	const top = $derived(crest(outline, layout.pose, innerWidth, innerHeight));
	let hovering = $state<'map' | null>(null);
	const shown = $derived(current ? (current.area ?? null) : hovering);

	function over(event: PointerEvent | MouseEvent) {
		const point = unseat(layout.pose, innerWidth, innerHeight)([event.clientX, event.clientY]);
		return inside(outline, point) ? 'map' : null;
	}

	function onpointermove(event: PointerEvent) {
		hovering = !docked || (event.target instanceof Element && event.target.closest('.crowd')) ? null : over(event);
	}

	function release(event: PointerEvent) {
		if (event.pointerType !== 'mouse') hovering = null;
	}

	$effect(() => {
		document.documentElement.style.overscrollBehavior = 'none';
		return () => document.documentElement.style.removeProperty('overscroll-behavior');
	});

	const wanted = $derived(params().has('map'));
	let settled = page.url.searchParams.has('map');
	let docked = $state(!settled);
	let flight: HTMLElement;
	let animations: Animation[] = [];
	let keyframes: ReturnType<typeof reveal> | undefined;

	$effect(() => {
		const size = [tiles, innerWidth, innerHeight, layout.pose] as const;
		keyframes = undefined;
		if (!innerWidth || !innerHeight) return;
		const later = window.requestIdleCallback ?? setTimeout;
		const cancel = window.cancelIdleCallback ?? clearTimeout;
		const id = later(() => (keyframes = reveal(...size)));
		return () => cancel(id);
	});

	const touch = new MediaQuery('(hover: none)');

	function enter() {
		update({ map: true }, { state: { map: true } });
	}

	function onclick(event: MouseEvent) {
		event.preventDefault();
		if (wanted || current) return;
		if (event.detail === 0 || over(event)) enter();
	}

	function close() {
		if (page.state.map) history.back();
		else update({ map: null, operation: null }, { replace: true, state: {} });
	}

	$effect(() => {
		if (current && wanted) update({ map: null, operation: null }, { replace: true, state: {} });
	});

	$effect(() => {
		if (params().has('operation') && !wanted) update({ map: true }, { replace: true, state: {} });
	});

	async function fly(opening: boolean) {
		if (opening) docked = false;
		if (animations.length) {
			const now = performance.now();
			animations.forEach((animation) => {
				const start = animation.startTime;
				animation.playbackRate = -animation.playbackRate;
				if (start === null) return;
				const at = Math.max(0, Math.min(FLIGHT, (now - Number(start)) * -animation.playbackRate));
				animation.startTime = now - at / animation.playbackRate;
			});
		} else {
			const { grow, spread } = keyframes ?? reveal(tiles, innerWidth, innerHeight, layout.pose);
			const options: KeyframeAnimationOptions = { duration: FLIGHT, fill: 'both', direction: opening ? 'normal' : 'reverse' };
			animations = [flight.animate(grow, options), flight.animate(spread, options)];
		}
		const current = animations;
		await Promise.all(current.map((animation) => animation.finished));
		if (current !== animations || animations.some((animation) => animation.playState === 'running')) return;
		animations.forEach((animation) => animation.cancel());
		animations = [];
		docked = !settled;
	}

	$effect(() => {
		if (wanted === settled) return;
		settled = wanted;
		fly(wanted);
	});

	$effect(() => {
		settle(PLANKS.map(image)).then((loaded) => (images = loaded));
		settle(BUILDINGS.map(image)).then((loaded) => (buildings = loaded));
	});

	$effect(() => {
		const query = matchMedia(`(resolution: ${ratio}dppx)`);
		const update = () => (ratio = devicePixelRatio);
		update();
		query.addEventListener('change', update);
		return () => query.removeEventListener('change', update);
	});

	const view = $derived.by(() => {
		const dpr = ratio;
		const width = Math.round(innerWidth * dpr);
		const height = Math.round(innerHeight * dpr);
		const focal = height;
		const pitch = (ANGLE * Math.PI) / 180;
		const camera = (focal * Math.sin(pitch)) / (span * SCALE * dpr);

		const project = (y: number) => {
			const tilt = Math.atan((y + 0.5 - height / 2) / focal);
			const ray = pitch + tilt;
			return { distance: camera / Math.tan(ray), stretch: (camera * Math.cos(tilt)) / Math.sin(ray) / focal };
		};

		const far = project(wall);
		const lift = (y: number) => {
			const tilt = Math.atan((y + 0.5 - height / 2) / focal);
			const ray = pitch + tilt;
			return { z: camera - far.distance * Math.tan(ray), depth: (far.distance * Math.cos(tilt)) / Math.cos(ray) };
		};

		return { width, height, focal, project, far, lift };
	});

	$effect(() => {
		if (!images.length) return;
		const { width, height, project, far } = view;
		const near = project(height - 1);
		const [{ naturalWidth: plankWidth, naturalHeight: plankHeight }] = images;
		const density = plankWidth / WIDTH;
		const flat = document.createElement('canvas');
		flat.width = Math.ceil(width * far.stretch * density) + 2;
		flat.height = Math.ceil((far.distance - near.distance) * density) + 2;
		const flatContext = flat.getContext('2d')!;
		for (let row = 0; row * plankHeight < flat.height; row++) {
			const offset = Math.round(((row % 5) * plankWidth) / 5) - plankWidth;
			for (let col = 0; col * plankWidth + offset < flat.width; col++) {
				flatContext.drawImage(images[pick(row, col, images.length)], col * plankWidth + offset, row * plankHeight);
			}
		}

		floor.width = width;
		floor.height = height;
		const context = floor.getContext('2d')!;
		context.imageSmoothingEnabled = blend;
		for (let y = wall; y < height; y++) {
			const { distance, stretch } = project(y);
			const source = blend ? (far.distance - distance) * density : Math.floor((far.distance - distance) * density);
			context.drawImage(flat, flat.width / 2 - (width / 2) * stretch * density, source, width * stretch * density, 1, 0, y, width, 1);
		}
	});

	$effect(() => {
		if (!buildings.length) return;
		const { width, focal, lift } = view;
		const snap = (n: number) => (SMOOTH ? n : Math.round(n));
		const span = (left: number, right: number, y: number, rows: number) => {
			const start = snap(left);
			context.fillRect(start, y, snap(right) - start, rows);
		};

		windows.width = width;
		windows.height = wall;
		const context = windows.getContext('2d')!;
		context.imageSmoothingEnabled = SMOOTH;

		const unit = focal / lift(wall / 2).depth;
		const margin = Math.ceil(unit * REFRACTION);
		const backdrop = document.createElement('canvas');
		backdrop.width = width + margin * 2;
		backdrop.height = wall;
		const backdropContext = backdrop.getContext('2d')!;

		const sky = backdropContext.createRadialGradient(backdrop.width / 2, wall * 1.5, 0, backdrop.width / 2, wall * 1.5, wall * 2);
		sky.addColorStop(0, PALETTE.sky);
		sky.addColorStop(0.75, PALETTE.horizon);
		backdropContext.fillStyle = sky;
		backdropContext.fillRect(0, 0, backdrop.width, wall);

		const skyline = [];
		for (let i = 0, x = -unit * 32; x < backdrop.width + unit * 32; i++) {
			const image = buildings[Math.floor(random(i * 3) * buildings.length)];
			const size = unit * (0.5 + random(i * 3 + 1) * 0.7);
			skyline.push({ image, size, x });
			x += image.width * size * (0.4 + random(i * 3 + 2) * 0.5);
		}
		for (const { image, size, x } of skyline.sort((a, b) => a.size - b.size)) {
			const left = snap(x);
			const top = snap(wall - image.height * size);
			backdropContext.imageSmoothingEnabled = smooth(BUILDINGS[buildings.indexOf(image)]);
			backdropContext.drawImage(image, left, top, snap(left + image.width * size) - left, wall - top);
		}

		const pane = (focal / lift(wall).depth) * SPACING;
		for (let k = Math.floor(-width / 2 / pane); width / 2 + k * pane < width; k++) {
			const left = snap(width / 2 + k * pane);
			const right = snap(width / 2 + (k + 1) * pane);
			const shift = snap((random(10000 + k) * 2 - 1) * margin);
			context.drawImage(backdrop, left + margin + shift, 0, right - left, wall, left, 0, right - left, wall);
		}

		context.fillStyle = getComputedStyle(windows).getPropertyValue('--island');
		for (let y = 0; y < wall; y++) {
			const { z } = lift(y);
			if (z < RAIL) context.fillRect(0, y, width, 1);
		}
		const scale = focal / lift(wall).depth;
		const half = width / 2 / scale + MULLION;
		for (let k = Math.ceil(-half / SPACING); k * SPACING <= half; k++) {
			span(width / 2 + (k * SPACING - MULLION / 2) * scale, width / 2 + (k * SPACING + MULLION / 2) * scale, 0, wall);
		}
	});
</script>

<svelte:window
	bind:innerWidth
	bind:innerHeight
	{onkeydown}
	{onpointermove}
	onpointerup={release}
	onpointercancel={release}
	onpointerdown={(event) => {
		if (!(event.target instanceof Element && event.target.closest('.corner'))) quip = null;
	}}
/>

<Konami enabled={me.current?.superadmin ?? false} />

<svelte:head>
	<title>{wanted ? 'Map' : 'HQ'} - Hack Club Sunker</title>
</svelte:head>

<div
	class="hq"
	class:chatting
	class:covered
	class:portrait
	class:phone={dock > 0}
	style:--slots={panels.left.length + panels.right.length}
	style:--vv-top="{viewportTop}px"
	style:--vv-bottom="{viewportBottom}px"
>
	<div class="insets" bind:this={probe}></div>
	<canvas class="floor" class:smooth={blend} bind:this={floor}></canvas>
	<canvas class="windows" class:smooth={SMOOTH} style:height="{wall / ratio}px" bind:this={windows}></canvas>

	<div class="corner" class:touring={!!current} class:away={!docked} style:top="{wall / ratio}px" style:--guide={layout.guide && `${layout.guide.size}px`}>
		<Robosquirrel mode="bob" {tilt} size={layout.guide ? `${layout.guide.size}px` : 'clamp(96px, 11vw, 180px)'} onclick={current ? undefined : chatter} />
		{#if current?.text}
			<div class="bubble">{#key stop}<Speech bind:this={speech} bind:done={spoken} text={current.text} />{/key}</div>
		{:else if !current && quip}
			<div class="bubble" out:fade={{ duration: 250 }}>{#key quip}<Speech text={quip} />{/key}</div>
		{/if}
	</div>

	<button class="help" class:spotlit={current?.spots.includes('Help')} aria-label="Help"><img src={image('help-icon')} alt="" style:--rendering={rendering('help-icon')} /><span class="tip left">Help</span></button>
	<Resources />

	<Crowd shape={layout.crowd} avoid={layout.guide && { x: 4, y: layout.guide.top - 14, width: layout.guide.size + 8, height: layout.guide.size + 14 }} />
	{#if layout.box}
		<div
			class="table upright"
			style:--px="{layout.unit}px"
			style:left="{layout.box.x}px"
			style:top="{layout.box.y}px"
			style:width="{layout.box.width}px"
			style:height="{layout.box.height}px"
			style:border-image-source="url({image('table')})"
			style:--rendering={rendering('table')}
		></div>
	{:else}
		<img class="table" src={image('table')} alt="" style:--rendering={rendering('table')} style:--px={natural ? `calc(100vw / 2 / ${natural})` : null} bind:naturalWidth={natural} />
	{/if}
	{#if shown && docked}
		<svg class="outline" viewBox="0 0 {innerWidth} {innerHeight}"><path d={border} stroke-width={innerWidth / 256} /></svg>
		<span class="label" class:spotlit={!!current} style:left="{top[0]}px" style:top="{top[1]}px">Map</span>
	{/if}
	<a class="enter" href="?map" aria-label="Open map" inert={!!current} class:hovering {onclick} onfocus={() => (hovering = !docked ? null : 'map')} onblur={() => (hovering = null)}></a>

	{#if touch.current && docked && !wanted && !current}
		<div class="chips">
			<button class="chip" onpointerdown={() => (hovering = 'map')} onclick={enter}>Map</button>
		</div>
	{/if}

	{#each ['left', 'right'] as const as side}
		{@const section = panels[side][active[side]]}
		<SidePanel {side} open={open[side]} peek={side === 'right' && planner.selectable !== null} count={panels[side].length}>
			{#snippet dialog()}
				<header class={side}>
					<h2>{section.label}</h2>
					<button class="close" aria-label="Close" onclick={() => update({ [side]: null })}>
						<svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
							<path d="M2 2L12 12M12 2L2 12" />
						</svg>
					</button>
				</header>
				<div class="body"><section.component /></div>
			{/snippet}
		</SidePanel>
	{/each}

	{#each ['left', 'right'] as const as side, s}
		{#each panels[side] as { label, icon, disabled, dot }, i}
			<button
				class="tab {side}"
				class:spotlit={current?.spots.includes(label)}
				class:open={open[side] && !covered}
				aria-label={label}
				aria-expanded={open[side] && active[side] === i}
				style:--offset="{offset(panels[side].length, i)}px"
				style:--slot={s * panels.left.length + i}
				style:--dialog-width="{DIALOG_WIDTH}px"
				{disabled}
				onclick={() => toggle(side, i)}
			>
				<img src={image(icon)} alt="" style:--rendering={rendering(icon)} />
				{#if dot}<span class="dot"></span>{/if}
				<span class="tip {side}">{label}</span>
			</button>
		{/each}
	{/each}

	<div class="flight" class:open={wanted} style:transform={wanted ? null : pose} style:--clip={wanted ? null : clip} bind:this={flight}>
		<WorldMap
			map={data.map}
			territory={data.territory}
			table={!wanted}
			covered={hidden}
			cleared={ceiling}
			selectable={planner.selectable}
			selected={planner.selected}
			route={planner.path}
			onpick={(hex) => planner.pick?.(hex)}
		/>
		{#if wanted}
			<button class="back" aria-label="Back to HQ" onclick={close}>
				<svg viewBox="0 0 24 24"><path d="M15 5l-7 7 7 7" /></svg>
			</button>
		{/if}
	</div>

	{#if current}
		<button class="dim" aria-label="Continue" onclick={onward}>
			<svg viewBox="0 0 {innerWidth} {innerHeight}" aria-hidden="true">
				<mask id="spotlight">
					<rect width={innerWidth} height={innerHeight} fill="white" />
					{#if current.area}<path d={border} fill="black" />{/if}
				</mask>
				<rect class="shade" width={innerWidth} height={innerHeight} mask="url(#spotlight)" />
			</svg>
		</button>
		{#if stop === STAMP}
			<div class="stamp" in:fade={{ duration: 250 }}>
				<p class="assigned" role="status" bind:offsetHeight={assigned}>You have been assigned to the Guild of <strong>{me.current?.guild?.name}</strong></p>
				<div class="booklet" inert bind:offsetHeight={booklet} style:transform="scale({shrink})" style:margin-bottom="{(shrink - 1) * booklet}px"><Passport draft={passport} guild={me.current?.guild} stamping fold closed={closing} /></div>
			</div>
		{/if}
		<button class="rewind" onclick={backward}>
			<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 5l-7 7 7 7" /></svg>
			Back
		</button>
	{/if}
	{#if arriving}
		<div class="arrive"></div>
	{/if}
</div>

<style>
	.hq {
		--bar: 0px;
		color: var(--text);
		color-scheme: dark;
		font: 13px 'Inter', sans-serif;
		touch-action: manipulation;
		-webkit-tap-highlight-color: transparent;
	}

	.hq :global(.button) {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 7px 14px;
		border: 1px solid var(--accent);
		border-radius: 6px;
		background: transparent;
		color: var(--accent);
		font: 500 13px 'Inter', sans-serif;
		text-decoration: none;
		white-space: nowrap;
		cursor: pointer;
	}

	.hq :global(.button:not(.primary, .danger):hover:not(:disabled)) {
		background: color-mix(in srgb, var(--accent) 15%, transparent);
	}

	.hq :global(.button.primary) {
		background: var(--accent);
		color: var(--background);
	}

	.hq :global(.button.danger) {
		border-color: var(--negative);
		background: var(--negative);
		color: var(--background);
	}

	.hq :global(:is(.button.primary, .button.danger):hover:not(:disabled)) {
		filter: brightness(1.1);
	}

	.hq :global(.button:disabled) {
		opacity: 0.5;
		cursor: default;
	}

	.hq :global(.input) {
		box-sizing: border-box;
		width: 100%;
		padding: 7px 10px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--background);
		color: var(--text);
		font: 13px 'Inter', sans-serif;
	}

	.hq :global(.input::placeholder) {
		color: var(--text-muted);
	}

	.hq :global(.input:focus) {
		border-color: var(--accent);
		outline: none;
	}

	.hq :global(textarea.input) {
		resize: vertical;
	}

	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 14px 12px 20px;
	}

	h2 {
		margin: 0;
		font: 24px/24px 'Share Tech', sans-serif;
	}

	.close {
		width: 24px;
		height: 24px;
		border-radius: 6px;
	}

	.close path {
		stroke: var(--text-muted);
		stroke-width: 1.5;
		stroke-linecap: round;
	}

	@media (pointer: coarse) {
		.hq :global(.button) {
			min-height: 44px;
		}

		.close {
			width: 44px;
			height: 44px;
			margin: -10px -8px -10px 0;
		}
	}

	.body {
		padding: 0 20px 16px;
	}

	button {
		position: relative;
		display: grid;
		place-items: center;
		width: 64px;
		height: 64px;
		padding: 0;
		border: none;
		border-radius: 6px;
		background: transparent;
		color: inherit;
		cursor: pointer;
	}

	button img {
		width: 64px;
		height: 64px;
	}

	.tip,
	.label {
		position: absolute;
		top: 50%;
		padding: 6px 10px;
		border: 1px solid var(--tip-border);
		border-radius: 6px;
		background: var(--island);
		white-space: nowrap;
		transform: translateY(-50%);
		pointer-events: none;
		visibility: hidden;
	}

	.label {
		position: fixed;
		left: 50%;
		z-index: 11;
		transform: translate(-50%, calc(-100% - 16px));
		visibility: visible;
	}

	.tip.left {
		left: calc(100% + 8px);
	}

	.tip.right {
		right: calc(100% + 8px);
	}

	button:hover .tip,
	button:focus-visible .tip {
		visibility: visible;
	}

	button:hover:enabled {
		background: var(--hover);
	}

	button:disabled {
		cursor: default;
	}

	button:disabled img {
		opacity: 0.3;
	}

	.dot {
		position: absolute;
		top: 8px;
		left: 8px;
		width: 20px;
		height: 20px;
		border-radius: 50%;
		background: var(--dot);
	}

	.tab {
		position: fixed;
		top: calc(50% + var(--offset));
		z-index: 21;
		transition: transform 0.25s ease;
	}

	.tab.left {
		left: 0;
	}

	.tab.right {
		right: 0;
	}

	.tab.left.open {
		transform: translateX(calc(min(var(--dialog-width), 100vw - 144px) + 8px));
	}

	.tab.right.open {
		transform: translateX(calc(-1 * (min(var(--dialog-width), 100vw - 144px) + 8px)));
	}

	.covered :global(.dialog.open.left) {
		transform: translateX(calc(-100% - 8px));
	}

	.covered :global(.dialog.open.right) {
		transform: translateX(calc(100% + 8px));
	}

	.covered :global(.panel.open) {
		--shift: 0px;
	}

	button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
	}

	.floor {
		position: fixed;
		inset: 0;
		width: 100%;
		height: 100%;
		image-rendering: pixelated;
	}

	.windows {
		position: fixed;
		top: 0;
		left: 0;
		width: 100%;
		image-rendering: pixelated;
	}

	.smooth {
		image-rendering: auto;
	}

	img {
		image-rendering: var(--rendering);
	}

	.outline {
		position: fixed;
		inset: 0;
		z-index: 3;
		width: 100%;
		height: 100%;
		pointer-events: none;
		fill: var(--outline);
		stroke: var(--outline);
		stroke-linejoin: round;
	}

	.flight {
		position: fixed;
		inset: 0;
		z-index: 10;
		pointer-events: none;
		will-change: transform;
		clip-path: var(--clip, none);
	}

	.flight.open {
		pointer-events: auto;
	}

	.back {
		position: fixed;
		top: calc(16px + env(safe-area-inset-top, 0px));
		left: calc(16px + env(safe-area-inset-left, 0px));
		width: 48px;
		height: 48px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		touch-action: manipulation;
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

	@media (max-height: 460px) and (orientation: landscape) {
		.back {
			left: calc(72px + env(safe-area-inset-left, 0px));
		}
	}

	.enter {
		position: fixed;
		inset: 0;
	}

	.enter.hovering {
		cursor: pointer;
	}

	.chips {
		position: fixed;
		right: 16px;
		bottom: 26px;
		z-index: 12;
		display: flex;
		gap: 8px;
	}

	.phone .chips {
		bottom: calc(var(--bar) + 22px);
	}

	@media (orientation: landscape) and (max-height: 520px) {
		.chips {
			right: 88px;
		}
	}

	.chip {
		width: auto;
		height: 44px;
		padding: 0 18px;
		border: 1px solid var(--island-border);
		background: var(--island);
		font: 500 13px 'Inter', sans-serif;
	}

	.chip:active {
		background: var(--hover);
	}

	.corner {
		position: fixed;
		left: 96px;
		z-index: 19;
		transform: translateY(-62%);
		pointer-events: none;
	}

	.corner.touring {
		z-index: 31;
	}

	.corner.away {
		z-index: 9;
	}

	.corner:not(.touring) :global(.robosquirrel),
	.corner:not(.touring) .bubble {
		pointer-events: auto;
	}

	.corner :global(button.robosquirrel:hover) {
		background: none;
	}

	.corner .bubble {
		position: absolute;
		top: 10%;
		left: calc(100% + 12px);
		width: min(420px, calc(100vw - 100% - 140px));
		padding: 14px 18px;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: var(--island);
		box-shadow: 0 16px 48px var(--shadow);
	}

	.corner .bubble::before {
		content: '';
		position: absolute;
		top: 22px;
		left: -9px;
		width: 16px;
		height: 16px;
		border-bottom: 1px solid var(--island-border);
		border-left: 1px solid var(--island-border);
		background: var(--island);
		transform: rotate(45deg);
	}

	.corner .bubble :global(.speech) {
		font-size: 15px;
	}

	.portrait .corner {
		left: 12px;
	}

	.portrait .corner .bubble {
		top: -40%;
		width: min(420px, calc(100vw - var(--guide) - 36px));
	}

	.portrait .corner .bubble::before {
		top: calc(var(--guide) * 0.65 - 8px);
	}

	.dim {
		position: fixed;
		inset: 0;
		z-index: 30;
		width: auto;
		height: auto;
		padding: 0;
		border-radius: 0;
		background: none;
	}

	.dim:hover:enabled {
		background: none;
	}

	.dim svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}

	.shade {
		fill: color-mix(in srgb, var(--background) 60%, transparent);
	}

	.spotlit {
		z-index: 31;
		pointer-events: none;
	}

	.chatting :global(:is(.dialog, .panel).right) {
		z-index: 31;
	}

	.tab.spotlit,
	.help.spotlit {
		z-index: 31;
		background: var(--hover);
	}

	.spotlit .tip {
		visibility: visible;
	}

	.stamp {
		position: fixed;
		inset: 0;
		z-index: 31;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 12px;
		padding: 16px;
		pointer-events: none;
	}

	.booklet {
		width: min(560px, 100%);
		transform-origin: top;
	}

	.assigned {
		margin: 0;
		padding: 16px 24px;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: var(--island);
		font-size: 18px;
		text-align: center;
	}

	.assigned strong {
		color: var(--accent);
	}

	.rewind {
		position: fixed;
		bottom: 96px;
		left: 16px;
		z-index: 32;
		display: flex;
		align-items: center;
		gap: 4px;
		width: auto;
		height: auto;
		padding: 8px 14px 8px 8px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		color: var(--text);
		font: 500 13px 'Inter', sans-serif;
		cursor: pointer;
	}

	.rewind:hover:enabled {
		background: var(--hover);
	}

	.rewind svg {
		width: 18px;
		height: 18px;
		fill: none;
		stroke: currentColor;
		stroke-width: 2.5;
		stroke-linecap: round;
		stroke-linejoin: round;
	}

	@media (pointer: coarse) {
		.rewind {
			min-height: 44px;
		}
	}

	.help {
		position: fixed;
		bottom: 16px;
		left: 16px;
		z-index: 19;
		border: 1px solid var(--island-border);
		background: var(--island);
	}

	.help img {
		width: 44px;
		height: 44px;
	}

	.help .tip {
		top: 50%;
	}

	.phone .help {
		bottom: calc(var(--bar) + 12px);
	}

	.phone:has(.flight.open) .help {
		bottom: calc(var(--bar) + 16px);
		left: calc(16px + env(safe-area-inset-left, 0px));
		width: 48px;
		height: 48px;
	}

	.phone:has(.flight.open) .help img {
		width: 32px;
		height: 32px;
	}

	@media (orientation: landscape) and (max-height: 460px) {
		.hq:has(.flight.open) .help {
			bottom: calc(16px + env(safe-area-inset-bottom, 0px));
			left: calc(72px + env(safe-area-inset-left, 0px));
			width: 48px;
			height: 48px;
		}

		.hq:has(.flight.open) .help img {
			width: 32px;
			height: 32px;
		}
	}

	.phone .rewind {
		right: 16px;
		bottom: calc(var(--bar) + 22px);
		left: auto;
	}

	.phone.chatting .rewind {
		top: calc(max(8px, env(safe-area-inset-top, 0px)) + 4px);
		right: auto;
		bottom: auto;
		left: 50%;
		translate: -50% 0;
	}

	.phone .stamp {
		padding-bottom: calc(var(--bar) + 68px);
	}

	.arrive {
		position: fixed;
		inset: 0;
		z-index: 40;
		background: var(--background);
		pointer-events: none;
		animation: arrive 0.7s ease-out forwards;
	}

	@keyframes arrive {
		to {
			opacity: 0;
		}
	}

	@media (orientation: landscape) and (max-height: 520px) {
		.help,
		.rewind {
			left: 88px;
		}
	}

	.table {
		position: fixed;
		top: 50%;
		left: 50%;
		z-index: 2;
		width: calc(100vw / 2);
		transform: translate(-50%, -50%);
		filter: drop-shadow(var(--px) 0 var(--table-edge)) drop-shadow(calc(-1 * var(--px)) 0 var(--table-edge))
			drop-shadow(0 var(--px) var(--table-edge)) drop-shadow(0 calc(-1 * var(--px)) var(--table-edge));
		pointer-events: none;
	}

	.table.upright {
		box-sizing: border-box;
		transform: none;
		border-style: solid;
		border-color: transparent;
		border-width: calc(56 * var(--px)) calc(250 * var(--px)) calc(110 * var(--px)) calc(210 * var(--px));
		border-image-slice: 56 250 110 210 fill;
		border-image-width: 1;
		border-image-repeat: stretch;
		image-rendering: var(--rendering);
	}

	.insets {
		position: fixed;
		top: 0;
		left: 0;
		width: 0;
		height: 0;
		padding-top: env(safe-area-inset-top);
		padding-bottom: env(safe-area-inset-bottom);
		visibility: hidden;
		pointer-events: none;
	}

	@media (pointer: coarse), (orientation: portrait) and (max-width: 600px) {
		header {
			flex: none;
		}

		.body {
			display: flex;
			flex: 1;
			flex-direction: column;
			min-height: 0;
			overflow: auto;
			overscroll-behavior: contain;
		}
	}

	@media (hover: none) {
		.tab {
			display: flex;
			flex-direction: column;
			align-items: center;
			justify-content: center;
			gap: 2px;
		}

		.tab img {
			width: 40px;
			height: 40px;
			image-rendering: auto;
		}

		.tab .tip {
			position: static;
			padding: 0;
			border: none;
			background: none;
			font: 500 10px/12px 'Inter', sans-serif;
			transform: none;
			visibility: visible;
		}

		.tab .dot {
			top: 6px;
			left: 6px;
			width: 14px;
			height: 14px;
		}
	}

	@media (orientation: portrait) and (max-width: 600px) {
		.hq {
			--bar: calc(64px + env(safe-area-inset-bottom, 0px));
		}

		.tab,
		.tab.left,
		.tab.right {
			top: auto;
			right: auto;
			bottom: 0;
			left: calc(var(--slot) * 100% / var(--slots));
			box-sizing: border-box;
			display: flex;
			flex-direction: column;
			align-items: center;
			justify-content: center;
			gap: 2px;
			width: calc(100% / var(--slots));
			height: var(--bar);
			padding: 0 0 env(safe-area-inset-bottom, 0px);
			border-radius: 0;
			-webkit-tap-highlight-color: transparent;
		}

		.tab.left.open,
		.tab.right.open {
			transform: none;
		}

		.tab img {
			box-sizing: content-box;
			width: 36px;
			height: 36px;
			padding: 2px 6px;
			border-radius: 12px;
			image-rendering: auto;
		}

		.tab .tip {
			position: static;
			padding: 0;
			border: none;
			background: none;
			font: 500 clamp(9px, 2.7vw, 11px) / 12px 'Inter', sans-serif;
			transform: none;
			visibility: visible;
		}

		.tab .dot {
			top: 4px;
			left: calc(50% + 6px);
			width: 12px;
			height: 12px;
		}

		.tab:hover:enabled,
		.tab.spotlit {
			background: none;
		}

		.tab[aria-expanded='true'] img,
		.tab:active:enabled img,
		.tab.spotlit img {
			background: var(--hover);
		}

		.tab[aria-expanded='true'] .tip {
			color: var(--accent);
		}

		.chatting :global(.panel.right) {
			z-index: 20;
		}
	}

	@media (orientation: portrait) and (max-width: 600px) and (hover: hover) {
		.tab:hover:enabled img {
			background: var(--hover);
		}
	}
</style>
