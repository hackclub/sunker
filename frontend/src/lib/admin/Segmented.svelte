<script lang="ts" generics="T extends string">
	let {
		options,
		value,
		label,
		onchange
	}: {
		options: { value: T; label: string }[];
		value: T;
		label: string;
		onchange: (value: T) => void;
	} = $props();
</script>

<span class="segmented" role="group" aria-label={label}>
	{#each options as option (option.value)}
		<button
			type="button"
			aria-pressed={value === option.value}
			onclick={() => onchange(option.value)}
		>
			{option.label}
		</button>
	{/each}
</span>

<style>
	.segmented {
		display: inline-flex;
		max-width: 100%;
		padding: 2px;
		overflow-x: auto;
		scrollbar-width: none;
		border: 1px solid var(--island-border);
		border-radius: 8px;
		background: var(--background);
	}

	button {
		padding: 4px 10px;
		border: none;
		border-radius: 6px;
		background: none;
		color: var(--text-muted);
		font: inherit;
		font-size: 13px;
		white-space: nowrap;
		cursor: pointer;
	}

	button:hover {
		color: var(--text);
	}

	button[aria-pressed='true'] {
		background: var(--hover);
		color: var(--text);
	}

	button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
	}

	@media (pointer: coarse) {
		button {
			min-height: 44px;
			padding: 0 14px;
		}
	}
</style>
