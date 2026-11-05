<script lang="ts">
	import { ENEMIES, enemyName, type Force } from '#lib/map.js';
	import { image, rendering } from '#lib/styles.svelte.js';

	let { force, disabled = false, available, onchange }: { force: Force; disabled?: boolean; available?: Force; onchange: (force: Force) => void } = $props();

	function set(enemy: (typeof ENEMIES)[number], value: number) {
		const next = { ...force };
		if (value > 0) next[enemy] = Math.floor(value);
		else delete next[enemy];
		onchange(next);
	}
</script>

<div class="force">
	{#each ENEMIES as enemy (enemy)}
		<label title={enemyName(enemy)}>
			<img src={image(enemy)} alt="" style:image-rendering={rendering(enemy)} />
			<input type="number" inputmode="numeric" min="0" value={force[enemy] ?? 0} {disabled} oninput={(event) => set(enemy, Number(event.currentTarget.value))} />
			<span>{enemyName(enemy)}{#if available}<em> / {available[enemy] ?? 0}</em>{/if}</span>
		</label>
	{/each}
</div>

<style>
	.force {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(100px, 1fr));
		gap: 6px;
	}

	label {
		display: grid;
		grid-template-columns: 22px 1fr;
		grid-template-rows: auto auto;
		align-items: center;
		column-gap: 6px;
		padding: 6px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--background);
	}

	img {
		grid-row: span 2;
		width: 22px;
	}

	input {
		width: 100%;
		min-width: 0;
		box-sizing: border-box;
		padding: 2px 4px;
		border: 1px solid var(--island-border);
		border-radius: 4px;
		background: var(--island);
		color: var(--text);
		font: inherit;
		font-size: 13px;
	}

	span {
		color: var(--text-muted);
		font-size: 11px;
	}

	em {
		font-style: normal;
		opacity: 0.8;
	}

	@media (pointer: coarse) {
		input {
			min-height: 44px;
			padding: 6px 6px;
		}
	}
</style>
