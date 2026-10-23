<script lang="ts">
	import City from '#lib/buildings/City.svelte';

	const palettes = [
		{ wallColor: '#888888', windowColor: '#aaddff', roofColor: '#444444', windowBorderRatio: 0.1 },
		{ wallColor: '#b5651d', windowColor: '#ffe9a8', roofColor: '#5a2d0c', windowBorderRatio: 0.1 },
		{ wallColor: '#2f4f6f', windowColor: '#cdeaff', roofColor: '#1b2a3a', windowBorderRatio: 0.3 },
		{ wallColor: '#6b8e5a', windowColor: '#f4fff0', roofColor: '#33452b', windowBorderRatio: 0.5 },
		{ wallColor: '#a33333', windowColor: '#ffd0d0', roofColor: '#551111', windowBorderRatio: 0.2 }
	];

	const floors = [6, 14, 9, 20, 11, 7, 16, 5, 12, 8, 18, 10];

	const buildings = floors.map((count, i) => ({
		x: ((i % 4) - 1.5) * 144,
		z: (Math.floor(i / 4) - 1) * 144,
		yaw: 30,
		floors: count,
		...palettes[i % palettes.length]
	}));
</script>

<City {buildings} cameraPitch={30} />
