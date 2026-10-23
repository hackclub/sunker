<script lang="ts">
	import Skyscraper, { type SkyscraperProps, type Vec3 } from './Skyscraper.svelte';

	let {
		buildings,
		cameraPitch = 60,
		cameraHeight = 1200,
		ambientLight = 0.3,
		directionalLight = 1,
		lightDirection = [-1, -1, -1]
	}: {
		buildings: Omit<SkyscraperProps, 'project' | 'eye' | 'brightness'>[];
		cameraPitch?: number;
		cameraHeight?: number;
		ambientLight?: number;
		directionalLight?: number;
		lightDirection?: Vec3;
	} = $props();

	const RAD = Math.PI / 180;

	const camera = $derived.by(() => {
		const sin = Math.sin(cameraPitch * RAD);
		const cos = Math.cos(cameraPitch * RAD);
		const eye: Vec3 = [0, cameraHeight, (cameraHeight * cos) / sin];
		const focal = cameraHeight / sin;

		function project([px, py, pz]: Vec3): [number, number] {
			const rx = px - eye[0];
			const ry = py - eye[1];
			const rz = pz - eye[2];
			const scale = focal / (-ry * sin - rz * cos);
			return [rx * scale, -(ry * cos - rz * sin) * scale];
		}

		return { eye, project };
	});

	function brightness(normal: Vec3): number {
		const len = Math.hypot(...lightDirection) || 1;
		const [lx, ly, lz] = lightDirection;
		const diffuse = Math.max(0, -(normal[0] * lx + normal[1] * ly + normal[2] * lz) / len);
		return Math.min(1, ambientLight + directionalLight * diffuse);
	}

	const farToNear = $derived(
		buildings.toSorted((a, b) => {
			const distance = (s: { x?: number; z?: number }) =>
				Math.hypot((s.x ?? 0) - camera.eye[0], (s.z ?? 0) - camera.eye[2]);
			return distance(b) - distance(a);
		})
	);
</script>

<svg viewBox="-500 -650 1000 800">
	{#each farToNear as building}
		<Skyscraper {...building} project={camera.project} eye={camera.eye} {brightness} />
	{/each}
</svg>

<style>
	svg {
		display: block;
		width: 100%;
		height: auto;
	}
</style>
