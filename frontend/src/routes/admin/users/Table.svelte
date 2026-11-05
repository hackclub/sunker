<script lang="ts">
	import type { Snippet } from 'svelte';
	import { format, isPerson, label, personName, type Row } from './users';

	let {
		rows,
		columns,
		onopen,
		actions
	}: { rows: Row[]; columns?: string[]; onopen: (id: number) => void; actions?: Snippet<[Row]> } = $props();

	const keys = $derived(columns ?? [...new Set(['id', ...rows.flatMap((row) => Object.keys(row))])].filter((key) => rows.some((row) => key in row)));
</script>

{#if rows.length === 0}
	<p class="empty">None</p>
{:else}
	<div class="scroll">
		<table>
			<thead>
				<tr>
					{#if actions}<th></th>{/if}
					{#each keys as key (key)}<th>{label(key)}</th>{/each}
				</tr>
			</thead>
			<tbody>
				{#each rows as row, i (row.id ?? i)}
					<tr class:deleted={!!row.deleted_at}>
						{#if actions}<td class="actions">{@render actions(row)}</td>{/if}
						{#each keys as key (key)}
							{@const value = row[key]}
							<td>
								{#if isPerson(value)}
									<button class="link" onclick={() => onopen(value.id)}>{personName(value)}</button>
								{:else if value === null || value === undefined}
									<span class="none">none</span>
								{:else}
									{format(value)}
								{/if}
							</td>
						{/each}
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
{/if}

<style>
	.scroll {
		overflow-x: auto;
		border: 1px solid var(--island-border);
		border-radius: 8px;
	}

	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 12px;
	}

	th,
	td {
		padding: 6px 8px;
		border-bottom: 1px solid var(--island-border);
		text-align: left;
		vertical-align: top;
		white-space: nowrap;
	}

	td {
		max-width: 320px;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	th {
		color: var(--text-muted);
		font-weight: 500;
		text-transform: capitalize;
	}

	tbody tr:last-child td {
		border-bottom: none;
	}

	tr.deleted td {
		opacity: 0.5;
	}

	tr.deleted td.actions {
		opacity: 1;
	}

	.actions :global(button + button) {
		margin-left: 4px;
	}

	.none,
	.empty {
		color: var(--text-muted);
	}

	.empty {
		margin: 0;
		font-size: 13px;
	}

	.link {
		padding: 0;
		border: none;
		background: none;
		color: var(--accent);
		font: inherit;
		cursor: pointer;
	}

	.link:hover {
		text-decoration: underline;
	}

	@media (pointer: coarse) {
		.link {
			padding: 8px 0;
		}
	}
</style>
