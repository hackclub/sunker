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
	import SidePanel from '#lib/SidePanel.svelte';

	const PANELS = {
		left: [
			{ label: 'Projects', icon: projectsIcon },
			{ label: 'Propaganda', icon: propagandaIcon },
			{ label: 'Help', icon: helpIcon }
		],
		right: [
			{ label: 'Chat', icon: chatIcon },
			{ label: 'Missions', icon: battleIcon },
			{ label: 'Profile', icon: profileIcon }
		]
	};

	const PLANKS = [plank, plank2, plank3];
	const SCALE = 4;
	const WIDTH = 128 * SCALE;
	const HEIGHT = 16 * SCALE;

	const seed = Math.floor(Math.random() * 2 ** 32);

	function pick(row: number, col: number) {
		const h = Math.imul(seed ^ (row * 73856093) ^ (col * 19349663), 2654435761) >>> 0;
		return PLANKS[((h ^ (h >>> 16)) >>> 0) % PLANKS.length];
	}

	let innerWidth = $state(0);
	let innerHeight = $state(0);
	const rows = $derived(Math.ceil(innerHeight / HEIGHT));
	const cols = $derived(Math.ceil(innerWidth / WIDTH) + 1);
</script>

<svelte:window bind:innerWidth bind:innerHeight />

<svelte:head>
	<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Inter:wght@400&display=swap" />
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
		<SidePanel {side}>
			{#each PANELS[side] as { label, icon }}
				<button aria-label={label}>
					<img src={icon} alt="" />
					<span class="tip {side}">{label}</span>
				</button>
			{/each}
		</SidePanel>
	{/each}
</div>

<style>
	.hq {
		--island: #1e1f22;
		--island-border: #2b2d30;
		--hover: #2b2d30;
		--text: #dfe1e5;
		--accent: #8bcf86;
		color: var(--text);
		color-scheme: dark;
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
		font: 13px 'Inter', sans-serif;
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

	button:hover {
		background: var(--hover);
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
