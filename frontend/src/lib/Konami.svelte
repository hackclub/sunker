<script lang="ts">
	let { enabled }: { enabled: boolean } = $props();

	const CODE = ['ArrowUp', 'ArrowUp', 'ArrowDown', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'ArrowLeft', 'ArrowRight'];
	const TYPING = 'input, textarea, select, [contenteditable]:not([contenteditable="false"])';
	const ARROWS = ['slider', 'listbox', 'option', 'menu', 'menubar', 'menuitem', 'tablist', 'tab', 'radiogroup', 'radio', 'grid', 'tree', 'treegrid', 'combobox', 'spinbutton']
		.map((role) => `[role="${role}"]`)
		.join(', ');

	let recent: string[] = [];

	function onkeydown(event: KeyboardEvent) {
		if (!enabled || !event.isTrusted || event.repeat || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
		if (event.target instanceof Element && event.target.closest(`${TYPING}, ${ARROWS}`)) {
			recent = [];
			return;
		}
		recent = [...recent, event.key].slice(-CODE.length);
		if (recent.join() !== CODE.join()) return;
		recent = [];
		fetch('/api/onboarding/reset', { method: 'POST' }).then((response) => response.ok && location.assign('/onboarding'));
	}
</script>

<svelte:window {onkeydown} />
