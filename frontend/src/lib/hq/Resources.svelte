<script lang="ts">
	import { me } from '#lib/me.svelte.js';
	import { RESOURCES } from '#lib/resources.js';
	import { image } from '#lib/styles.svelte.js';

	let open = $state(false);

	const amount = (key: string) => (me.current?.resources?.[key] ?? 0).toLocaleString();
</script>

<svelte:window onpointerdown={(event) => open && !(event.target instanceof Element && event.target.closest('.resources')) && (open = false)} />

<section class="resources" class:open aria-label="Resources">
	<button type="button" class="row" aria-expanded={open} onclick={() => (open = !open)}>
		{#each RESOURCES as resource (resource.key)}
			<span class="count" title={resource.name}><img src={image(resource.icon)} alt={resource.name} />{amount(resource.key)}</span>
		{/each}
	</button>
	<div class="details">
		<ul>
			{#each RESOURCES as resource (resource.key)}
				<li>
					<img src={image(resource.icon)} alt="" />
					<div>
						<strong>{resource.name}</strong>
						<p>{resource.description}</p>
					</div>
				</li>
			{/each}
		</ul>
	</div>
</section>

<style>
	.resources {
		position: fixed;
		top: 16px;
		right: 16px;
		z-index: 19;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		color: var(--text);
		font: 13px 'Inter', sans-serif;
	}

	.row {
		display: flex;
		gap: 14px;
		padding: 8px 14px 8px 10px;
		border: none;
		background: none;
		color: inherit;
		font: inherit;
		text-align: inherit;
		cursor: pointer;
		touch-action: manipulation;
	}

	.count {
		display: flex;
		align-items: center;
		gap: 4px;
		font-weight: 500;
		font-variant-numeric: tabular-nums;
	}

	img {
		width: 32px;
		height: 32px;
		image-rendering: pixelated;
	}

	.details {
		display: grid;
		width: 0;
		min-width: 100%;
		grid-template-rows: 0fr;
		transition: grid-template-rows 0.25s ease;
	}

	.resources.open {
		z-index: 22;
	}

	.resources.open .details {
		grid-template-rows: 1fr;
	}

	ul {
		display: flex;
		flex-direction: column;
		gap: 10px;
		min-height: 0;
		max-height: calc(100dvh - 112px);
		margin: 0;
		padding: 0 14px 0 10px;
		overflow: hidden;
		list-style: none;
		transition: padding 0.25s ease;
	}

	.resources.open ul {
		padding-top: 12px;
		padding-bottom: 12px;
		border-top: 1px solid var(--island-border);
		overflow-y: auto;
		overscroll-behavior: contain;
	}

	@media (hover: hover) {
		.resources:hover {
			z-index: 22;
		}

		.resources:hover .details {
			grid-template-rows: 1fr;
		}

		.resources:hover ul {
			padding-top: 12px;
			padding-bottom: 12px;
			border-top: 1px solid var(--island-border);
			overflow-y: auto;
			overscroll-behavior: contain;
		}
	}

	li {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	strong {
		font-weight: 600;
	}

	p {
		margin: 2px 0 0;
		color: var(--text-muted);
		line-height: 1.35;
	}

	@media (max-width: 639px) {
		.resources {
			top: calc(env(safe-area-inset-top, 0px) + 12px);
			left: 72px;
		}

		.row {
			display: grid;
			width: 100%;
			grid-template-columns: repeat(5, 1fr);
			gap: 4px;
			padding: 6px 4px;
		}

		.count {
			flex-direction: column;
			gap: 2px;
			line-height: 16px;
			white-space: nowrap;
		}

		ul {
			max-height: calc(100dvh - 168px);
		}
	}

	@media (max-width: 639px) and (orientation: portrait) {
		ul {
			max-height: calc(100dvh - 240px - env(safe-area-inset-top, 0px) - env(safe-area-inset-bottom, 0px));
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.details,
		ul {
			transition: none;
		}
	}
</style>
