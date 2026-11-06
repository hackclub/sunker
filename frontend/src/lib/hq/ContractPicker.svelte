<script lang="ts">
	import { RESOURCE_NAMES } from '#lib/admin/analytics.js';
	import { catalog } from '#lib/contracts.svelte.js';

	let { busy, failure, onstart }: { busy: boolean; failure: string; onstart: (slug: string) => void } = $props();

	let started = $state<string | null>(null);

	const rings = $derived([...new Set(catalog.contracts.map(({ ring }) => ring))].toSorted((a, b) => b - a));
</script>

{#if catalog.broken}
	<p class="muted">Couldn't load the contracts</p>
{:else if !catalog.loaded}
	<p class="muted">Loading the contracts...</p>
{:else if !catalog.contracts.length}
	<p class="muted">No contracts yet</p>
{:else}
	{#each rings as ring (ring)}
		<section>
			<h3 class="heading">Ring {ring}</h3>
			<div class="grid">
				{#each catalog.contracts.filter((contract) => contract.ring === ring) as contract (contract.slug)}
					<article class="contract">
						{#if contract.image_url}<img src={contract.image_url} alt="" />{/if}
						<strong>{contract.name}</strong>
						{#if contract.description}<p>{contract.description}</p>{/if}
						{#if Object.keys(contract.resources).length}
							<ul class="resources">
								{#each Object.entries(contract.resources) as [name, amount] (name)}<li>+{amount} {RESOURCE_NAMES[name] ?? name}</li>{/each}
							</ul>
						{/if}
						<p class="muted">On Shipping, you can claim 1 prize from Ring {contract.ring}</p>
						{#if contract.video_url}<a href={contract.video_url} target="_blank" rel="noreferrer">Watch the video</a>{/if}
						<footer>
							{#if contract.criteria.length}
								<span class="label">Contract Criteria</span>
								<ul class="criteria">
									{#each contract.criteria as criterion, i (i)}<li>{criterion}</li>{/each}
								</ul>
							{/if}
							{#if failure && started === contract.slug}<p class="error">{failure}</p>{/if}
							<button class="button primary start" disabled={busy} onclick={() => onstart((started = contract.slug))}>
								{busy && started === contract.slug ? 'Starting...' : 'Start'}
							</button>
						</footer>
					</article>
				{/each}
			</div>
		</section>
	{/each}
{/if}

<style>
	section {
		padding-bottom: 16px;
	}

	.heading {
		margin: 0 0 8px;
		color: var(--text-muted);
		font: 18px 'Share Tech', sans-serif;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(260px, 100%), 1fr));
		gap: 8px;
	}

	.contract {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 12px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
	}

	img {
		width: 100%;
		aspect-ratio: 16 / 9;
		border-radius: 4px;
		object-fit: cover;
	}

	strong {
		font: 18px 'Share Tech', sans-serif;
		font-weight: normal;
	}

	p,
	ul {
		margin: 0;
	}

	footer {
		display: flex;
		flex-direction: column;
		gap: 8px;
		margin-top: auto;
		padding-top: 4px;
	}

	.label {
		font-weight: 500;
	}

	.criteria {
		padding-left: 18px;
	}

	.resources {
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

	a {
		align-self: flex-start;
		color: var(--accent);
	}

	.muted {
		color: var(--text-muted);
	}

	.error {
		color: var(--negative);
		font-size: 12px;
	}

	.start {
		align-self: flex-start;
	}

	@media (pointer: coarse) {
		a {
			display: inline-flex;
			align-items: center;
			min-height: 44px;
		}
	}
</style>
