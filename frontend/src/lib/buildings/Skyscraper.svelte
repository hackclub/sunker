<script lang="ts" module>
	export type Vec3 = [number, number, number];

	export type SkyscraperProps = {
		x?: number;
		z?: number;
		yaw?: number;
		floors?: number;
		wallColor?: string;
		windowColor?: string;
		roofColor?: string;
		windowBorderRatio?: number;
		project: (point: Vec3) => [number, number];
		eye: Vec3;
		brightness: (normal: Vec3) => number;
	};
</script>

<script lang="ts">
	let {
		x = 0,
		z = 0,
		yaw = 0,
		floors = 10,
		wallColor = '#888888',
		windowColor = '#aaddff',
		roofColor = '#444444',
		windowBorderRatio = 0.1,
		project,
		eye,
		brightness
	}: SkyscraperProps = $props();

	const WIDTH = 96;
	const FLOOR_HEIGHT = 24;
	const COLUMNS = 4;
	const RAD = Math.PI / 180;

	const height = $derived(floors * FLOOR_HEIGHT);

	function turn([lx, ly, lz]: Vec3): Vec3 {
		const c = Math.cos(yaw * RAD);
		const s = Math.sin(yaw * RAD);
		return [lx * c + lz * s, ly, -lx * s + lz * c];
	}

	function toWorld(point: Vec3): Vec3 {
		const [tx, ty, tz] = turn(point);
		return [x + tx, ty, z + tz];
	}

	const shade = (color: string, level: number) =>
		`color-mix(in srgb, ${color} ${level * 100}%, black)`;

	type Face = { origin: Vec3; u: Vec3; v: Vec3; normal: Vec3; windows: boolean };

	const faces = $derived.by(() => {
		const hw = WIDTH / 2;
		const defs: Face[] = [
			{ origin: [-hw, height, hw], u: [WIDTH, 0, 0], v: [0, -height, 0], normal: [0, 0, 1], windows: true },
			{ origin: [hw, height, hw], u: [0, 0, -WIDTH], v: [0, -height, 0], normal: [1, 0, 0], windows: true },
			{ origin: [hw, height, -hw], u: [-WIDTH, 0, 0], v: [0, -height, 0], normal: [0, 0, -1], windows: true },
			{ origin: [-hw, height, -hw], u: [0, 0, WIDTH], v: [0, -height, 0], normal: [-1, 0, 0], windows: true },
			{ origin: [-hw, height, hw], u: [WIDTH, 0, 0], v: [0, 0, -WIDTH], normal: [0, 1, 0], windows: false }
		];

		const at = (f: Face, u: number, v: number): Vec3 =>
			toWorld([
				f.origin[0] + f.u[0] * u + f.v[0] * v,
				f.origin[1] + f.u[1] * u + f.v[1] * v,
				f.origin[2] + f.u[2] * u + f.v[2] * v
			]);

		return defs
			.map((f) => {
				const centre = at(f, 0.5, 0.5);
				const normal = turn(f.normal);
				const facing =
					normal[0] * (eye[0] - centre[0]) +
					normal[1] * (eye[1] - centre[1]) +
					normal[2] * (eye[2] - centre[2]);
				const level = brightness(normal);
				const corners = (u0: number, v0: number, u1: number, v1: number) =>
					[at(f, u0, v0), at(f, u1, v0), at(f, u1, v1), at(f, u0, v1)]
						.map((p) => project(p).join(','))
						.join('L');

				let windowPath = '';
				if (f.windows) {
					const margin = windowBorderRatio / 2;
					for (let row = 0; row < floors; row++) {
						const v0 = (row + (row === 0 ? 0 : margin)) / floors;
						const v1 = (row + 1 - (row === floors - 1 ? 0 : margin)) / floors;
						for (let col = 0; col < COLUMNS; col++) {
							const u0 = (col + (col === 0 ? 0 : margin)) / COLUMNS;
							const u1 = (col + 1 - (col === COLUMNS - 1 ? 0 : margin)) / COLUMNS;
							windowPath += `M${corners(u0, v0, u1, v1)}Z`;
						}
					}
				}

				return {
					facing,
					points: corners(0, 0, 1, 1).replaceAll('L', ' '),
					fill: shade(f.windows ? wallColor : roofColor, level),
					windowFill: shade(windowColor, level),
					windowPath
				};
			})
			.filter((f) => f.facing > 0);
	});
</script>

<g>
	{#each faces as face}
		<polygon points={face.points} style:fill={face.fill} style:stroke={face.fill} />
		{#if face.windowPath}
			<path d={face.windowPath} style:fill={face.windowFill} />
		{/if}
	{/each}
</g>

<style>
	polygon {
		stroke-width: 0.5;
		stroke-linejoin: round;
	}
</style>
