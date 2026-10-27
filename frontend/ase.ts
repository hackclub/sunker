import { existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import Aseprite from 'ase-parser';
import { PNG } from 'pngjs';
import type { Plugin } from 'vite';

function render(file: string) {
	const buffer = readFileSync(file);
	for (let o = 128, frame = 0; frame < buffer.readUInt16LE(6); frame++, o += buffer.readUInt32LE(o)) {
		if (!buffer.readUInt32LE(o + 12)) buffer.writeUInt32LE(buffer.readUInt16LE(o + 6), o + 12);
	}
	const ase = new Aseprite(buffer, file);
	ase.parse();
	const png = new PNG({ width: ase.width, height: ase.height });

	const hidden = ase.layers.map((layer, i) => {
		if (!layer.flags.visible) return true;
		for (let level = layer.layerChildLevel, j = i - 1; level > 0 && j >= 0; j--) {
			if (ase.layers[j].layerChildLevel >= level) continue;
			if (!ase.layers[j].flags.visible) return true;
			level = ase.layers[j].layerChildLevel;
		}
		return false;
	});

	const cels = ase.frames[0].cels.toSorted((a, b) => a.layerIndex - b.layerIndex);
	for (const cel of cels) {
		const layer = ase.layers[cel.layerIndex];
		if (hidden[cel.layerIndex] || !cel.rawCelData || cel.celType === 3) continue;
		const opacity = (cel.opacity / 255) * (layer.opacity / 255);
		for (let y = 0; y < cel.h; y++) {
			for (let x = 0; x < cel.w; x++) {
				const px = cel.xpos + x;
				const py = cel.ypos + y;
				if (px < 0 || py < 0 || px >= ase.width || py >= ase.height) continue;
				const i = y * cel.w + x;
				let r, g, b, a;
				if (ase.colorDepth === 32) {
					[r, g, b, a] = cel.rawCelData.subarray(i * 4, i * 4 + 4);
				} else if (ase.colorDepth === 16) {
					[r, a] = cel.rawCelData.subarray(i * 2, i * 2 + 2);
					g = b = r;
				} else {
					const index = cel.rawCelData[i];
					if (index === ase.paletteIndex && !layer.flags.background) continue;
					({ red: r, green: g, blue: b, alpha: a } = ase.palette.colors[index]);
				}
				const alpha = (a / 255) * opacity;
				if (!alpha) continue;
				const o = (py * ase.width + px) * 4;
				const below = png.data[o + 3] / 255;
				const out = alpha + below * (1 - alpha);
				for (const [k, c] of [r, g, b].entries()) {
					png.data[o + k] = Math.round((c * alpha + png.data[o + k] * below * (1 - alpha)) / out);
				}
				png.data[o + 3] = Math.round(out * 255);
			}
		}
	}

	const out = file.replace(/\.ase$/, '.png');
	if (existsSync(out)) {
		const old = PNG.sync.read(readFileSync(out));
		const same = (i: number) =>
			(!old.data[i + 3] && !png.data[i + 3]) || old.data.readUInt32BE(i) === png.data.readUInt32BE(i);
		if (old.width === png.width && old.height === png.height && png.data.every((_, i) => i % 4 || same(i))) return;
	}
	writeFileSync(out, PNG.sync.write(png));
}

function files(dir: string): string[] {
	return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
		const path = join(dir, entry.name);
		if (entry.isDirectory()) return files(path);
		return entry.name.endsWith('.ase') ? [path] : [];
	});
}

export default function ase(dir = 'src'): Plugin {
	return {
		name: 'ase',
		buildStart() {
			files(dir).forEach(render);
		},
		configureServer(server) {
			const update = (file: string) => file.endsWith('.ase') && render(file);
			server.watcher.on('add', update);
			server.watcher.on('change', update);
		}
	};
}
