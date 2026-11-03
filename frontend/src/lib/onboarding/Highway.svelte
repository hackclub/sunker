<script lang="ts">
	import { colour, css } from '#lib/colors.js';
	import { image } from '#lib/styles.svelte.js';

	const PIXEL = 3;
	const EYE = 1.7;
	const ROAD = 8;
	const NEAR = 0.6;
	const FAR = 420;
	const BUILDINGS = ['building1', 'building2'];
	const STOREY = 9;
	const BLUR = 10;

	let innerWidth = $state(0);
	let innerHeight = $state(0);
	let canvas: HTMLCanvasElement;
	let sprites = $state<HTMLImageElement[]>([]);

	function random(n: number) {
		const h = Math.imul(0x5eed ^ Math.imul(n + 1, 2246822519), 2654435761) >>> 0;
		return ((h ^ (h >>> 16)) >>> 0) / 2 ** 32;
	}

	$effect(() => {
		Promise.all(
			BUILDINGS.map((slug) => {
				const sprite = new Image();
				sprite.src = image(slug);
				return sprite.decode().then(() => sprite);
			})
		)
			.then((loaded) => (sprites = loaded))
			.catch(console.error);
	});

	$effect(() => {
		if (!innerWidth || !innerHeight) return;
		const [w, h] = [Math.ceil(innerWidth / PIXEL), Math.ceil(innerHeight / PIXEL)];
		const scene = document.createElement('canvas');
		scene.width = w;
		scene.height = h;
		const context = scene.getContext('2d')!;
		context.imageSmoothingEnabled = false;
		const show = () => {
			canvas.width = innerWidth;
			canvas.height = innerHeight;
			const out = canvas.getContext('2d')!;
			out.imageSmoothingQuality = 'high';
			out.filter = `blur(${BLUR}px)`;
			out.drawImage(scene, -BLUR * 3, -BLUR * 3, innerWidth + BLUR * 6, innerHeight + BLUR * 6);
		};
		const horizon = Math.round(h * 0.46);
		const focal = h * 0.9;
		const across = (x: number, z: number) => w / 2 + (focal * x) / z;
		const down = (z: number) => horizon + (focal * EYE) / z;
		const quad = (x0: number, x1: number, z0: number, z1: number) => {
			context.beginPath();
			context.moveTo(across(x0, z0), down(z0));
			context.lineTo(across(x1, z0), down(z0));
			context.lineTo(across(x1, z1), down(z1));
			context.lineTo(across(x0, z1), down(z1));
			context.fill();
		};
		const blob = (x: number, y: number, rx: number, ry: number) => {
			context.beginPath();
			context.ellipse(x, y, rx, ry, 0, 0, Math.PI * 2);
			context.fill();
		};

		const sky = context.createLinearGradient(0, 0, 0, horizon);
		sky.addColorStop(0, colour('horizon'));
		sky.addColorStop(1, colour('sky'));
		context.fillStyle = sky;
		context.fillRect(0, 0, w, horizon + 1);

		const haze = context.createLinearGradient(0, horizon * 0.55, 0, horizon);
		haze.addColorStop(0, css(colour('map-fog'), 0));
		haze.addColorStop(1, css(colour('map-fog'), 0.7));
		context.fillStyle = haze;
		context.fillRect(0, 0, w, horizon + 1);

		const clouds = Array.from({ length: 46 }, (_, i) => {
			const r = (4 + random(i * 4 + 2) * 14) * (w / 420);
			return { x: random(i * 4) * w * 1.2 - w * 0.1, y: horizon - random(i * 4 + 1) ** 2 * horizon * 0.32 - r * 0.2, r };
		});
		for (const [key, dx, dy, sx, sy] of [
			['map-fog-shade', 0, 0.3, 1.6, 0.8],
			['map-fog', 0, 0, 1.5, 0.72],
			['map-fog-light', -0.3, -0.28, 0.8, 0.38]
		] as const) {
			context.fillStyle = colour(key);
			for (const { x, y, r } of clouds) blob(x + dx * r, y + dy * r, r * sx, r * sy);
		}

		context.fillStyle = colour('concrete');
		context.fillRect(0, horizon, w, h - horizon);
		for (let i = 0; i < 2400; i++) {
			const z = NEAR + random(i * 3 + 7) ** 2 * 160;
			const x = (random(i * 3 + 8) * 2 - 1) * z * (w / focal);
			const size = Math.max(1, Math.round((focal * 0.06) / z));
			context.fillStyle = colour(random(i * 3 + 9) < 0.5 ? 'concrete-speck-light' : 'concrete-speck-dark');
			context.fillRect(Math.round(across(x, z)), Math.round(down(z)), size, size);
		}

		context.fillStyle = colour('map-asphalt');
		quad(-ROAD, ROAD, NEAR, FAR * 10);
		context.fillStyle = colour('map-line');
		for (const side of [-1, 1]) quad(side * (ROAD - 0.5), side * (ROAD - 0.3), NEAR, FAR * 10);
		context.fillStyle = colour('map-paint');
		for (const x of [-0.3, 0.12]) quad(x, x + 0.18, NEAR, FAR * 10);
		context.fillStyle = colour('map-line');
		for (const side of [-1, 1])
			for (let z = NEAR; z < FAR; z += 9) quad(side * (ROAD / 2) - 0.1, side * (ROAD / 2) + 0.1, z, z + 3.5);

		const ground = context.createLinearGradient(0, horizon, 0, down(40));
		ground.addColorStop(0, css(colour('map-fog'), 0.85));
		ground.addColorStop(1, css(colour('map-fog'), 0));
		context.fillStyle = ground;
		context.fillRect(0, horizon, w, down(40) - horizon);

		if (!sprites.length) return show();
		const tint = document.createElement('canvas');
		const tinted = tint.getContext('2d', { willReadFrequently: true })!;
		const bodies = new Map(
			sprites.map((sprite) => {
				tint.width = sprite.width;
				tint.height = sprite.height;
				tinted.drawImage(sprite, 0, 0);
				const pixels = tinted.getImageData(0, 0, sprite.width, sprite.height).data;
				const solid = (x: number, y: number) => pixels[(y * sprite.width + x) * 4 + 3] >= 128;
				const middle = Math.floor(sprite.height / 2);
				let left = 0;
				let right = sprite.width - 1;
				while (left < right && !solid(left, middle)) left++;
				while (right > left && !solid(right, middle)) right--;
				let top = 0;
				while (top < middle && !Array.from({ length: sprite.width }, (_, x) => x).some((x) => solid(x, top))) top++;
				const roof = (x: number) => {
					let y = top;
					while (y < middle && !solid(x, y)) y++;
					return y;
				};
				let bottom = sprite.height - 1;
				while (bottom > middle && !solid(left, bottom)) bottom--;
								return [sprite, { left, top, bottom, width: right + 1 - left, roofs: { [-1]: roof(right), [1]: roof(left) } as Record<number, number> }];
			})
		);
		const towers = [];
		for (const side of [-1, 1]) {
			for (const [row, offset, tall] of [
				[1, 8, 1.6],
				[0, 0.6, 1]
			]) {
				const seed = side * 1000 + row * 500;
				for (let i = 0, z = 2 + random(seed + 50) * 3; z < FAR; i++) {
					const sprite = sprites[Math.floor(random(i * 5 + seed) * sprites.length)];
					const body = bodies.get(sprite)!;
					const width = STOREY * (0.8 + random(i * 5 + seed + 1) * 0.6);
					const height = width * ((body.bottom + 1 - body.roofs[side]) / body.width) * tall * (0.8 + random(i * 5 + seed + 2) * 0.8);
					const depth = width * (1.4 + random(i * 5 + seed + 4) * 0.8);
					towers.push({ sprite, body, side, row, z, width, height, depth, x: side * (ROAD + offset + random(i * 5 + seed + 3) * 3) });
					z += depth;
				}
			}
		}
		for (const { sprite, body, side, z, width, height, depth, x } of towers.sort((a, b) => b.row - a.row || b.z - a.z)) {
			const fog = css(colour('map-fog'), Math.min(0.9, (z / FAR) ** 0.6));
			const far = z + depth;
			tint.width = body.width;
			tint.height = body.bottom + 1 - body.top;
			tinted.drawImage(sprite, body.left, body.top, tint.width, tint.height, 0, 0, tint.width, tint.height);
			tinted.globalCompositeOperation = 'source-atop';
			tinted.fillStyle = fog;
			tinted.fillRect(0, 0, tint.width, tint.height);
			tinted.globalCompositeOperation = 'source-over';
			const flat = Math.max(body.roofs[-1], body.roofs[1]) - body.top;
			const [start, end] = [across(x, z), across(x, far)].sort((a, b) => a - b);
			for (let column = Math.floor(start); column < Math.ceil(end); column++) {
				const depthAt = (focal * x) / (column + 0.5 - w / 2);
				if (!(depthAt >= z && depthAt <= far)) continue;
				const along = Math.min(body.width - 1, Math.floor(((depthAt - z) / depth) * body.width));
				const top = down(depthAt) - (height * focal) / depthAt;
				context.drawImage(tint, side < 0 ? along : body.width - 1 - along, flat, 1, tint.height - flat, column, top, 1, down(depthAt) - top);
			}
			context.beginPath();
			context.moveTo(across(x, z), down(z));
			context.lineTo(across(x, z), down(z) - (height * focal) / z);
			context.lineTo(across(x, far), down(far) - (height * focal) / far);
			context.lineTo(across(x, far), down(far));
			context.fillStyle = css(colour('background'), 0.25);
			context.fill();
			const crown = ((body.roofs[side] - body.top) / (body.bottom + 1 - body.roofs[side])) * height;
			const scale = focal / z;
			const near = across(x, z);
			const edge = across(x + side * width, z);
			const bottom = down(z);
			const top = bottom - (height + crown) * scale;
			context.drawImage(tint, Math.min(near, edge), top, Math.max(1, Math.abs(edge - near)), Math.max(1, bottom - top));
		}
		show();
	});
</script>

<svelte:window bind:innerWidth bind:innerHeight />

<canvas class="highway" bind:this={canvas} aria-hidden="true"></canvas>

<style>
	.highway {
		position: fixed;
		inset: 0;
		width: 100%;
		height: 100%;
	}
</style>
