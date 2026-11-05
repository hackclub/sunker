<script module lang="ts">
	export const FOLD = 700;

	const RISE = 70;
	const WAVES = [1, -1].flatMap((sign) =>
		Array.from({ length: Math.ceil((200 + 2 * RISE) / 7) }, (_, n) => {
			const points = Array.from({ length: 101 }, (_, i) => {
				const x = i * 4;
				return `${x},${(n * 7 - RISE + sign * RISE * Math.tanh((x - 200) / 70)).toFixed(1)}`;
			});
			return `<path d="M${points.join('L')}"/>`;
		})
	);
	const GUILLOCHE = `url("data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 200" preserveAspectRatio="none" fill="none" stroke="black" stroke-width="0.4">${WAVES.join('')}</svg>`)}")`;
</script>

<script lang="ts">
	import Headshot from '#lib/onboarding/Headshot.svelte';
	import type { Character, Guild } from '#lib/me.svelte.js';
	import { image } from '#lib/styles.svelte.js';

	let {
		draft = $bindable(),
		guild = null,
		onboarding = false,
		stamping = false,
		error = '',
		editing = $bindable(false),
		fold = false,
		closed = false,
		readonly = false
	}: { draft: Character; guild?: Guild | null; onboarding?: boolean; stamping?: boolean; error?: string; editing?: boolean; fold?: boolean; closed?: boolean; readonly?: boolean } = $props();

	let root: HTMLDivElement;
	let opened = $state(!fold);
	let coverHeight = $state(0);
	let pageHeight = $state(0);
	$effect(() => {
		if (opened) return;
		Promise.all([...root.querySelectorAll('img')].map((img) => img.decode().catch(() => {}))).then(() =>
			requestAnimationFrame(() => requestAnimationFrame(() => (opened = true)))
		);
	});

	const LIMITS = { handle: 32, age: 20, occupation: 64, backstory: 500 };

	const missing = (value: string) => onboarding && !value.trim();
	const machine = (value: string) => value.toUpperCase().replace(/[^A-Z0-9]+/g, '<');
	const zone = $derived([
		`P<SNK${machine(draft.handle)}`.padEnd(44, '<').slice(0, 44),
		`${machine(draft.attributes.occupation)}<<${machine(guild?.name ?? '')}`.padEnd(44, '<').slice(0, 44)
	]);
</script>

<div class="passport" bind:this={root}>
	<div class="upper" class:shut={!opened || closed} style:--fold="{FOLD}ms" style:--fit={coverHeight && pageHeight ? pageHeight / coverHeight : 1} bind:offsetHeight={coverHeight}>
		<div class="guilloche" style:--guilloche={GUILLOCHE}></div>
		{#if guild}
			<img class="stamp" class:stamping style:animation-delay={fold ? `${FOLD}ms` : null} src={image(`${guild.slug}-patch`)} alt="Guild of {guild.name} stamp" />
		{/if}
	</div>

	<div class="data" bind:offsetHeight={pageHeight}>
		<header>
			<strong>Sunker</strong>
			<span>Passport</span>
		</header>
		<div class="body">
			<div class="photo">
				<Headshot bind:features={draft.attributes.features} bind:hue={draft.attributes.hue} bind:open={editing} missing={onboarding && !draft.attributes.features.length} {readonly} />
				{#if onboarding}<span class="aside pointer">Craft your character!</span>{/if}
			</div>
			<div class="fields">
				<label>
					<span>Name</span>
					<input class:missing={missing(draft.handle)} bind:value={draft.handle} {readonly} maxlength={LIMITS.handle} placeholder="Name your character" autocomplete="off" />
					{#if error}<em class="error">{error}</em>{/if}
				</label>
				<label>
					<span>Age</span>
					<input class:missing={missing(draft.attributes.age)} bind:value={draft.attributes.age} {readonly} maxlength={LIMITS.age} placeholder="Age" autocomplete="off" />
				</label>
				<label>
					<span>Occupation</span>
					<input class:missing={missing(draft.attributes.occupation)} bind:value={draft.attributes.occupation} {readonly} maxlength={LIMITS.occupation} placeholder="Occupation" autocomplete="off" />
				</label>
				<label>
					<span>Backstory{#if !readonly}<b>{draft.attributes.backstory.length}/{LIMITS.backstory}</b>{/if}</span>
					<textarea class:missing={missing(draft.attributes.backstory)} bind:value={draft.attributes.backstory} {readonly} maxlength={LIMITS.backstory} rows="3" placeholder="It all started when..."></textarea>
					{#if onboarding}<span class="aside">Add some pizzazz, feel free to embellish and really play into this character as you go!</span>{/if}
				</label>
			</div>
		</div>
		<div class="zone" aria-hidden="true">
			{#each zone as line, i (i)}<span>{line}</span>{/each}
		</div>
	</div>
</div>

<style>
	.passport {
		container-type: inline-size;
		display: grid;
		grid-template-rows: 1fr 1fr;
		width: 100%;
		color: var(--passport-ink);
		font: 12px 'Inter', sans-serif;
	}

	.upper {
		position: relative;
		z-index: 1;
		display: flex;
		align-items: center;
		border: 1px solid var(--passport-guilloche);
		border-radius: 6px 6px 0 0;
		background: linear-gradient(to bottom, transparent 60%, color-mix(in srgb, var(--passport-ink) 10%, transparent)) var(--passport-paper-shade);
		transform: perspective(1400px) rotateX(-14deg) scaleY(1);
		transform-origin: bottom;
		transform-style: preserve-3d;
		transition: transform var(--fold) cubic-bezier(0.4, 0, 0.2, 1);
		will-change: transform;
	}

	.upper.shut {
		transform: perspective(1400px) rotateX(-179deg) scaleY(var(--fit));
	}

	.upper::after {
		content: '';
		position: absolute;
		inset: -1px;
		border-radius: 6px 6px 0 0;
		background: var(--passport-cover);
		transform: translateZ(-1px) rotateX(180deg);
		backface-visibility: hidden;
	}

	.guilloche {
		position: absolute;
		inset: 0;
		background: var(--passport-guilloche);
		mask: var(--guilloche) 0 0 / 100% 100% no-repeat;
		opacity: 0.8;
	}

	.stamp {
		position: absolute;
		right: 7%;
		width: 28cqw;
		opacity: 0.88;
		transform: rotate(-12deg);
		image-rendering: pixelated;
	}

	.stamping {
		animation: stamp 0.55s cubic-bezier(0.5, 0, 0.75, 0) both;
	}

	@keyframes stamp {
		0% {
			opacity: 0;
			transform: rotate(-4deg) scale(2.6);
		}
		70% {
			opacity: 0.88;
			transform: rotate(-12deg) scale(0.94);
		}
		100% {
			transform: rotate(-12deg) scale(1);
		}
	}

	.data {
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding: 14px 4% 12px;
		border: 1px solid var(--passport-guilloche);
		border-top: none;
		border-radius: 0 0 6px 6px;
		background: linear-gradient(to bottom, color-mix(in srgb, var(--passport-ink) 8%, transparent), transparent 28px) var(--passport-paper);
	}

	header {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		padding-bottom: 8px;
		border-bottom: 1px solid var(--passport-red);
		font-size: 15px;
	}

	header strong {
		color: var(--passport-red);
		font: 600 18px 'Inter', sans-serif;
	}

	header span {
		color: var(--passport-label);
	}

	.body {
		display: grid;
		grid-template-columns: 34% 1fr;
		gap: 4%;
		align-items: start;
	}

	.fields {
		display: flex;
		flex-direction: column;
		gap: 10px;
		min-width: 0;
	}

	label {
		position: relative;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	label span {
		display: flex;
		color: var(--passport-label);
		font-weight: 500;
	}

	label b {
		margin-left: auto;
		font-weight: 400;
	}

	.photo {
		position: relative;
	}

	label span.aside,
	.aside {
		position: absolute;
		top: 0;
		left: calc(100% + 4cqw + 28px);
		display: block;
		width: 220px;
		padding: 10px 14px;
		border: 1px solid var(--island-border);
		border-radius: 8px;
		background: var(--island);
		color: var(--text);
		font: 13px/1.4 'Inter', sans-serif;
	}

	.aside::before {
		content: '';
		position: absolute;
		top: 14px;
		left: -7px;
		width: 12px;
		height: 12px;
		border-bottom: 1px solid var(--island-border);
		border-left: 1px solid var(--island-border);
		background: var(--island);
		transform: rotate(45deg);
	}

	.pointer {
		right: calc(100% + 4cqw + 28px);
		left: auto;
		width: auto;
		white-space: nowrap;
	}

	.pointer::before {
		right: -7px;
		left: auto;
		transform: rotate(-135deg);
	}

	input,
	textarea {
		box-sizing: border-box;
		width: 100%;
		padding: 6px 8px;
		border: 1px solid var(--passport-guilloche);
		border-radius: 3px;
		background: var(--passport-paper-shade);
		color: var(--passport-ink);
		font: 13px 'Inter', sans-serif;
	}

	textarea {
		resize: vertical;
	}

	input::placeholder,
	textarea::placeholder {
		color: var(--passport-label);
		opacity: 0.7;
	}

	input:focus,
	textarea:focus {
		border-color: var(--passport-ink);
		outline: none;
	}

	@media (pointer: coarse) {
		input,
		textarea {
			padding: 10px;
		}

		input {
			min-height: 44px;
		}
	}

	.missing {
		border-color: var(--passport-missing);
		box-shadow: 0 0 0 1px var(--passport-missing);
	}

	.error {
		color: var(--passport-red);
		font-size: 11px;
		font-style: normal;
	}

	.zone {
		display: flex;
		flex-direction: column;
		overflow: hidden;
		color: var(--passport-label);
		font: 2.6cqw/1.3 'Inter', sans-serif;
		white-space: pre;
	}

	@container (max-width: 380px) {
		.body {
			grid-template-columns: 1fr;
			row-gap: 12px;
		}

		.photo {
			width: min(64%, 220px);
			margin: 0 auto;
		}
	}

	@media (max-width: 1240px) {
		label span.aside,
		.aside {
			position: static;
			width: auto;
		}

		.pointer {
			margin-top: 8px;
			white-space: normal;
		}

		.aside::before {
			display: none;
		}
	}

	@media (max-width: 640px), (pointer: coarse) {
		.passport {
			grid-template-rows: auto auto;
		}

		.upper {
			height: min(42cqw, 38dvh);
		}

		.stamp {
			width: auto;
			height: 80%;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.stamping {
			animation: none;
		}

		.upper {
			transition: none;
		}
	}
</style>
