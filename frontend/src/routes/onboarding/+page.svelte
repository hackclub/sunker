<script lang="ts">
	import { goto, preloadCode, preloadData } from '$app/navigation';
	import { untrack } from 'svelte';
	import { fade } from 'svelte/transition';
	import Konami from '#lib/Konami.svelte';
	import Highway from '#lib/onboarding/Highway.svelte';
	import Passport, { FOLD } from '#lib/onboarding/Passport.svelte';
	import Robosquirrel from '#lib/onboarding/Robosquirrel.svelte';
	import Speech from '#lib/onboarding/Speech.svelte';
	import { advance, complete, draft, load, me, save } from '#lib/me.svelte.js';
	import { INTRO, tilt as turn } from '#lib/robosquirrel.js';
	import { image, styles } from '#lib/styles.svelte.js';

	let { data } = $props();

	type Step = { kind: 'line'; text: string } | { kind: 'passport' };

	const STEPS: Step[] = [...INTRO.map((text) => ({ kind: 'line' as const, text })), { kind: 'passport' }];
	const PASSPORT = INTRO.length;
	const STORED = 'onboarding-passport';
	const BLOCKING = 'button, a, input, textarea, select, label, [role="button"], [role="dialog"], [contenteditable]';

	const initial = untrack(() => data.me);
	let index = $state(initial.stage ? PASSPORT : 0);
	const step = $derived(STEPS[index]);
	let tilt = $state(0);
	let speech = $state<Speech>();
	let done = $state(false);
	let character = $state(draft(stored() ?? initial.character));
	let editing = $state(false);
	let saving = $state(false);
	let error = $state('');
	let failure = $state('');
	let leaving = $state(false);
	let closing = $state(false);

	function stored() {
		try {
			return JSON.parse(localStorage.getItem(STORED) ?? 'null');
		} catch {
			return null;
		}
	}

	$effect(() => {
		const snapshot = JSON.stringify(character);
		if (leaving) return;
		try {
			localStorage.setItem(STORED, snapshot);
		} catch {}
	});

	$effect(() => {
		const warmed: HTMLImageElement[] = [];
		const id = (window.requestIdleCallback ?? setTimeout)(() => {
			preloadCode('/hq').catch(() => {});
			for (const slug of Object.keys(styles.images)) {
				const sprite = new Image();
				sprite.fetchPriority = 'low';
				sprite.src = image(slug);
				warmed.push(sprite);
			}
		});
		return () => (window.cancelIdleCallback ?? clearTimeout)(id);
	});

	const message = (e: unknown) => (e instanceof Error ? e.message : String(e));
	const fold = () => {
		closing = true;
		return new Promise((resolve) => setTimeout(resolve, FOLD));
	};

	function go(to: number) {
		index = to;
		editing = false;
		if (to === PASSPORT) closing = false;
		if (to === PASSPORT && !me.current?.stage) advance('passport').catch((e) => (failure = message(e)));
	}

	function next() {
		if (leaving || step.kind === 'passport') return;
		if (!done) return speech?.finish();
		tilt = turn(tilt);
		go(index + 1);
	}

	async function submit() {
		saving = true;
		error = failure = '';
		try {
			await save(character);
			if (me.current?.stage === 'passport') await advance('guild');
			await load();
			preloadData('/hq').catch(() => {});
			await fold();
			leaving = true;
			try {
				localStorage.removeItem(STORED);
			} catch {}
			setTimeout(() => goto('/hq'), 600);
		} catch (e) {
			if ((e as { status?: number }).status === 409) error = message(e);
			else failure = message(e);
		}
		saving = false;
	}

	async function back() {
		if (step.kind === 'passport') {
			if (closing) return;
			await fold();
		}
		go(index - 1);
	}

	function onclick(event: MouseEvent) {
		if (event.target instanceof Element && event.target.closest(BLOCKING)) return;
		next();
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key !== ' ' || (event.target instanceof Element && event.target.closest(BLOCKING))) return;
		event.preventDefault();
		next();
	}
</script>

<svelte:window {onclick} {onkeydown} />

<svelte:head>
	<title>Welcome - Hack Club Sunker</title>
</svelte:head>

<Konami enabled={me.current?.superadmin ?? false} />

<div class="onboarding">
	<Highway />

	{#if step.kind === 'line'}
		<div class="scene" transition:fade={{ duration: 250 }}>
			<div class="squirrel"><Robosquirrel {tilt} size="min(30vw, 40vh, 340px)" /></div>
			<div class="bubble">
				{#key index}<Speech bind:this={speech} bind:done text={step.text} />{/key}
			</div>
		</div>
	{:else}
		<div class="sheet" transition:fade={{ duration: 250 }}>
			<h1 class="tip">Create the character that you will inhabit for Sunker. Be creative!</h1>
			<div class="booklet">
				<Passport bind:draft={character} bind:editing onboarding fold closed={closing} {error} />
			</div>
			<button class="continue" disabled={!complete(character) || saving} onclick={submit}>{saving ? 'Saving...' : 'Continue'}</button>
			<p class="subtitle">All characteristics can be changed later</p>
			{#if failure}<p class="failure">{failure}</p>{/if}
		</div>
	{/if}

	{#if index > 0 && !leaving}
		<button class="back" onclick={back}>
			<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 5l-7 7 7 7" /></svg>
			Back
		</button>
	{/if}

	<div class="fade" class:on={leaving}></div>
</div>

<style>
	.onboarding {
		position: fixed;
		inset: 0;
		overflow: hidden;
		background: var(--background);
		color: var(--text);
		color-scheme: dark;
		font: 13px 'Inter', sans-serif;
		user-select: none;
		cursor: pointer;
	}

	.scene {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 2vw;
		padding: 0 5vw;
	}

	.squirrel {
		flex-shrink: 0;
		filter: drop-shadow(0 16px 24px var(--shadow));
	}

	.bubble {
		position: relative;
		flex: 0 1 520px;
		padding: 20px 24px;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: var(--island);
		box-shadow: 0 16px 48px var(--shadow);
	}

	.bubble::before {
		content: '';
		position: absolute;
		top: 32px;
		left: -12px;
		width: 22px;
		height: 22px;
		border-bottom: 1px solid var(--island-border);
		border-left: 1px solid var(--island-border);
		background: var(--island);
		transform: rotate(45deg);
	}

	.sheet {
		position: fixed;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 12px;
		overflow-y: auto;
		padding: 40px 16px 88px;
		cursor: default;
		user-select: text;
	}

	.booklet {
		width: min(640px, 100%);
		margin-top: 20px;
		flex-shrink: 0;
	}

	.tip {
		max-width: 640px;
		margin: 0 0 4px;
		font: 500 26px/1.3 'Inter', sans-serif;
		text-align: center;
		text-shadow: 0 1px 8px var(--background);
	}

	.continue {
		min-width: 180px;
		margin-top: 8px;
		padding: 10px 20px;
		border: 1px solid var(--accent);
		border-radius: 6px;
		background: var(--accent);
		color: var(--background);
		font: 500 15px 'Inter', sans-serif;
		cursor: pointer;
	}

	.continue:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.subtitle {
		margin: 0;
		padding: 6px 12px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		color: var(--text);
	}

	.failure {
		margin: 0;
		color: var(--negative);
	}

	.back {
		position: fixed;
		bottom: 16px;
		left: 16px;
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 8px 14px 8px 8px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		color: var(--text);
		font: 500 13px 'Inter', sans-serif;
		cursor: pointer;
	}

	.back:hover {
		background: var(--hover);
	}

	.back svg {
		width: 18px;
		height: 18px;
		fill: none;
		stroke: currentColor;
		stroke-width: 2.5;
		stroke-linecap: round;
		stroke-linejoin: round;
	}

	.fade {
		position: fixed;
		inset: 0;
		background: var(--background);
		opacity: 0;
		pointer-events: none;
		transition: opacity 0.5s ease;
	}

	.fade.on {
		opacity: 1;
		pointer-events: auto;
	}

	@media (max-width: 640px) {
		.scene {
			flex-direction: column;
			gap: 12px;
		}

		.bubble {
			flex: none;
			width: 100%;
			box-sizing: border-box;
		}

		.bubble::before {
			top: -12px;
			left: 50%;
			border: none;
			border-top: 1px solid var(--island-border);
			border-left: 1px solid var(--island-border);
			transform: translateX(-50%) rotate(45deg);
		}

		.squirrel :global(.robosquirrel) {
			--size: min(52vw, 32vh) !important;
		}
	}
</style>
