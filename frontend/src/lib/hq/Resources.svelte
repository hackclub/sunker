<script lang="ts">
	import { me } from '#lib/me.svelte.js';
	import { RESOURCES } from '#lib/resources.js';
	import { image } from '#lib/styles.svelte.js';

	const amount = (key: string) => (me.current?.resources?.[key] ?? 0).toLocaleString();
</script>

<section class="resources" aria-label="Resources">
	<div class="row">
		{#each RESOURCES as resource (resource.key)}
			<span class="count" title={resource.name}><img src={image(resource.icon)} alt={resource.name} />{amount(resource.key)}</span>
		{/each}
	</div>
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

	.resources:hover {
		z-index: 22;
	}

	.resources:hover .details {
		grid-template-rows: 1fr;
	}

	ul {
		display: flex;
		flex-direction: column;
		gap: 10px;
		min-height: 0;
		margin: 0;
		padding: 0 14px 0 10px;
		overflow: hidden;
		list-style: none;
		transition: padding 0.25s ease;
	}

	.resources:hover ul {
		padding-top: 12px;
		padding-bottom: 12px;
		border-top: 1px solid var(--island-border);
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

	@media (prefers-reduced-motion: reduce) {
		.details,
		ul {
			transition: none;
		}
	}
</style>
