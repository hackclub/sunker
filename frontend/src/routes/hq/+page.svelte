<script lang="ts">
	import plank from '#lib/assets/images/plank.png';
	import plank2 from '#lib/assets/images/plank2.png';
	import plank3 from '#lib/assets/images/plank3.png';
	import table from '#lib/assets/images/table.png';
	import projectsIcon from '#lib/assets/images/projects-icon.png';
	import propagandaIcon from '#lib/assets/images/propaganda-icon.png';
	import helpIcon from '#lib/assets/images/help-icon.png';
	import chatIcon from '#lib/assets/images/chat-icon.png';
	import battleIcon from '#lib/assets/images/battle-icon.png';
	import profileIcon from '#lib/assets/images/profile.png';
	import xIcon from '#lib/assets/images/x-icon.png';
	import SidePanel from '#lib/SidePanel.svelte';
	import Projects from '#lib/hq/Projects.svelte';
	import Propaganda from '#lib/hq/Propaganda.svelte';
	import Help from '#lib/hq/Help.svelte';
	import Chat from '#lib/hq/Chat.svelte';
	import Missions from '#lib/hq/Missions.svelte';
	import Profile from '#lib/hq/Profile.svelte';
	import type { Component } from 'svelte';

	type Side = 'left' | 'right';
	type Section = { label: string; icon: string; component: Component; disabled: boolean; dot: boolean };

	const panels: Record<Side, Section[]> = $state({
		left: [
			{ label: 'Projects', icon: projectsIcon, component: Projects, disabled: false, dot: false },
			{ label: 'Propaganda', icon: propagandaIcon, component: Propaganda, disabled: false, dot: false },
			{ label: 'Help', icon: helpIcon, component: Help, disabled: false, dot: false }
		],
		right: [
			{ label: 'Chat', icon: chatIcon, component: Chat, disabled: false, dot: false },
			{ label: 'Missions', icon: battleIcon, component: Missions, disabled: false, dot: false },
			{ label: 'Profile', icon: profileIcon, component: Profile, disabled: false, dot: false }
		]
	});

	const open: Record<Side, boolean> = $state({ left: false, right: false });
	const active: Record<Side, number> = $state({ left: 0, right: 0 });

	function toggle(side: Side, index: number) {
		open[side] = !(open[side] && active[side] === index);
		active[side] = index;
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') open.left = open.right = false;
	}

	const PLANKS = [plank, plank2, plank3];
	const SCALE = 4;
	const WIDTH = 128 * SCALE;
	const HEIGHT = 16 * SCALE;

	const seed = 0x5eed;

	function pick(row: number, col: number) {
		const h = Math.imul(seed ^ (row * 73856093) ^ (col * 19349663), 2654435761) >>> 0;
		return PLANKS[((h ^ (h >>> 16)) >>> 0) % PLANKS.length];
	}

	let innerWidth = $state(0);
	let innerHeight = $state(0);
	const rows = $derived(Math.ceil(innerHeight / HEIGHT));
	const cols = $derived(Math.ceil(innerWidth / WIDTH) + 1);
</script>

<svelte:window bind:innerWidth bind:innerHeight {onkeydown} />

<svelte:head>
	<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500&display=swap" />
</svelte:head>

<div class="hq">
	<div class="floor" style:--width="{WIDTH}px" style:--height="{HEIGHT}px">
		{#each { length: rows }, row}
			<div class="row" style:margin-left="{((row % 5) * WIDTH) / 5 - WIDTH}px">
				{#each { length: cols }, col}
					<img src={pick(row, col)} alt="" />
				{/each}
			</div>
		{/each}
	</div>

	<img class="table" src={table} alt="" />

	{#each ['left', 'right'] as const as side}
		{@const section = panels[side][active[side]]}
		<SidePanel {side} open={open[side]}>
			{#each panels[side] as { label, icon, disabled, dot }, i}
				<button
					aria-label={label}
					aria-expanded={open[side] && active[side] === i}
					tabindex={i * 2 + (side === 'left' ? 1 : 2)}
					{disabled}
					onclick={() => toggle(side, i)}
				>
					<img src={icon} alt="" />
					{#if dot}<span class="dot"></span>{/if}
					<span class="tip {side}">{label}</span>
				</button>
			{/each}
			{#snippet dialog()}
				<header class={side}>
					<h2>{section.label}</h2>
					<button class="close" aria-label="Close" onclick={() => (open[side] = false)}>
						<img src={xIcon} alt="" />
					</button>
				</header>
				<div class="body"><section.component /></div>
			{/snippet}
		</SidePanel>
	{/each}
</div>

<style>
	.hq {
		--island: #1e1f22;
		--island-border: #2b2d30;
		--hover: #2b2d30;
		--text: #dfe1e5;
		--accent: #869ecf;
		color: var(--text);
		color-scheme: dark;
		font: 13px 'Inter', sans-serif;
	}

	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 14px 12px 20px;
	}

	header.left {
		flex-direction: row-reverse;
		padding: 12px 20px 12px 14px;
	}

	h2 {
		margin: 0;
		font-size: 24px;
		line-height: 24px;
		font-weight: 500;
	}

	.close {
		width: 24px;
		height: 24px;
		border-radius: 6px;
	}

	.close img {
		width: 24px;
		height: 24px;
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
		background: transparent;
		color: inherit;
		cursor: pointer;
	}

	button img {
		width: 64px;
		height: 64px;
	}

	.tip {
		position: absolute;
		top: 50%;
		padding: 6px 10px;
		border: 1px solid #393b40;
		border-radius: 6px;
		background: var(--island);
		white-space: nowrap;
		transform: translateY(-50%);
		pointer-events: none;
		visibility: hidden;
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
		background: #3574f0;
	}

	button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
	}

	.floor {
		position: fixed;
		inset: 0;
		overflow: hidden;
	}

	.row {
		display: flex;
		height: var(--height);
	}

	img {
		image-rendering: pixelated;
	}

	.row img {
		flex: none;
		width: var(--width);
		height: var(--height);
	}

	.table {
		position: fixed;
		top: 50%;
		left: 50%;
		width: calc(100vw / 2);
		transform: translate(-50%, -50%);
		--px: calc(100vw / 2 / 512);
		filter: drop-shadow(var(--px) 0 #2e1a10) drop-shadow(calc(-1 * var(--px)) 0 #2e1a10)
			drop-shadow(0 var(--px) #2e1a10) drop-shadow(0 calc(-1 * var(--px)) #2e1a10);
	}
</style>
