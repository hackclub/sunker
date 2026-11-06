<script lang="ts">
	import PrizeCard from '#lib/hq/PrizeCard.svelte';
	import { catalog, type Prize } from '#lib/contracts.svelte.js';
	import { RING_COUNT } from '#lib/rings.svelte.js';

	let { claims, onclaim }: { claims: Record<number, number>; onclaim: (prize: Prize) => void } = $props();

	const RINGS = Array.from({ length: RING_COUNT }, (_, i) => RING_COUNT - 1 - i);

	let list = $state<HTMLElement>();
	let fill = $state(0);
	const sections: HTMLElement[] = [];

	function track() {
		if (!list) return;
		const end = list.scrollHeight - list.clientHeight;
		const stops = sections.map((section) => Math.min(section.offsetTop, end));
		const at = list.scrollTop;
		let progress = 0;
		for (let i = 0; i < stops.length - 1; i++) {
			if (at >= stops[i + 1]) progress = i + 1;
			else if (at > stops[i] && stops[i + 1] > stops[i]) progress = i + (at - stops[i]) / (stops[i + 1] - stops[i]);
		}
		fill = end > 0 ? progress / (stops.length - 1) : 1;
	}

	function jump(i: number) {
		const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
		list?.scrollTo({ top: sections[i].offsetTop, behavior: reduced ? 'instant' : 'smooth' });
	}

	$effect(() => {
		if (!list) return;
		const observer = new ResizeObserver(track);
		observer.observe(list);
		return () => observer.disconnect();
	});
</script>

<div class="all">
	<nav class="bar" style:--fill={fill}>
		<div class="track"><div class="filled"></div></div>
		{#each RINGS as ring, i (ring)}
			<button onclick={() => jump(i)}><span class="long">Ring {ring}</span><span class="short">R{ring}</span></button>
		{/each}
	</nav>
	<div class="list" bind:this={list} onscroll={track}>
		{#each RINGS as ring, i (ring)}
			{@const prizes = catalog.prizes.filter((prize) => prize.ring === ring)}
			{@const left = claims[ring] ?? 0}
			<section bind:this={sections[i]}>
				<h3>Ring {ring}</h3>
				<p class="muted claimable">You can claim {left} {left === 1 ? 'prize' : 'prizes'} from Ring {ring}</p>
				{#if prizes.length}
					<div class="grid">
						{#each prizes as prize (prize.id)}
							<PrizeCard {prize}>
								<button class="button claim" disabled={!left} onclick={() => onclaim(prize)}>Claim</button>
							</PrizeCard>
						{/each}
					</div>
				{:else}
					<p class="muted">Nothing here yet</p>
				{/if}
			</section>
		{/each}
	</div>
</div>

<style>
	.all {
		display: flex;
		gap: 16px;
		height: 100%;
	}

	.bar {
		position: relative;
		display: flex;
		flex: none;
		flex-direction: column;
		justify-content: space-between;
		width: 72px;
		padding: 0 0 0 14px;
	}

	.track {
		position: absolute;
		top: 12px;
		bottom: 12px;
		left: 4px;
		width: 4px;
		border-radius: 2px;
		background: var(--background);
	}

	.filled {
		height: calc(var(--fill) * 100%);
		border-radius: 2px;
		background: var(--accent);
	}

	.bar button {
		display: flex;
		align-items: center;
		min-height: 24px;
		padding: 0;
		border: none;
		background: none;
		color: var(--text-muted);
		font: 14px 'Share Tech', sans-serif;
		text-align: left;
		cursor: pointer;
	}

	.bar button:hover,
	.bar button:focus-visible {
		color: var(--accent);
		outline: none;
	}

	.short {
		display: none;
	}

	.list {
		position: relative;
		flex: 1;
		min-width: 0;
		overflow-y: auto;
		overscroll-behavior: contain;
	}

	section {
		padding-bottom: 16px;
	}

	h3 {
		position: sticky;
		top: 0;
		z-index: 1;
		margin: 0;
		padding: 0 0 8px;
		background: var(--island);
		color: var(--text-muted);
		font: 18px 'Share Tech', sans-serif;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
		gap: 8px;
	}

	.muted {
		margin: 0;
		color: var(--text-muted);
	}

	.claimable {
		margin-bottom: 10px;
		opacity: 0.7;
		font-size: 13px;
	}

	.claim {
		margin-top: auto;
		padding-top: 4px;
	}

	@media (max-width: 600px), (max-height: 480px) {
		.all {
			gap: 10px;
		}

		.bar {
			width: 30px;
			padding-left: 10px;
		}

		.track {
			left: 2px;
		}

		.long {
			display: none;
		}

		.short {
			display: inline;
		}

		.grid {
			grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
		}
	}

	@media (pointer: coarse) {
		.bar button {
			min-height: 44px;
		}
	}
</style>
