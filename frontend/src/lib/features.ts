export type Stroke = { d: string; fill?: boolean; width?: number };
export type Shape = { slug: string; name: string; reach: number; strokes: Stroke[] };

export const PHOTO = { width: 100, height: 128 };

const circle = (x: number, y: number, r: number) => `M ${x - r} ${y} a ${r} ${r} 0 1 0 ${r * 2} 0 a ${r} ${r} 0 1 0 ${-r * 2} 0`;

export const FACE: Shape[] = [
	{ slug: 'eye', name: 'Eye', reach: 6, strokes: [{ d: circle(0, 0, 5), width: 1.8 }, { d: circle(0, 0, 2), fill: true }] },
	{ slug: 'eye-closed', name: 'Closed eye', reach: 7, strokes: [{ d: 'M -6 -1 Q 0 4 6 -1', width: 2 }] },
	{ slug: 'brow', name: 'Eyebrow', reach: 8, strokes: [{ d: 'M -7 2 Q 0 -4 7 2', width: 2.5 }] },
	{ slug: 'nose', name: 'Nose', reach: 9, strokes: [{ d: 'M 1 -8 L -4 4 L 3 4', width: 2 }] },
	{ slug: 'nose-round', name: 'Round nose', reach: 5, strokes: [{ d: 'M -4 0 a 4 3 0 1 0 8 0 a 4 3 0 1 0 -8 0', fill: true }] },
	{ slug: 'smile', name: 'Smile', reach: 11, strokes: [{ d: 'M -10 -2 Q 0 8 10 -2', width: 2.5 }] },
	{ slug: 'mouth-flat', name: 'Flat mouth', reach: 9, strokes: [{ d: 'M -8 0 L 8 0', width: 2.5 }] },
	{ slug: 'mouth-open', name: 'Open mouth', reach: 10, strokes: [{ d: 'M -9 -3 Q 0 12 9 -3 Z', fill: true }] },
	{ slug: 'ear', name: 'Ear', reach: 10, strokes: [{ d: 'M -2 -9 Q 7 -10 5 0 Q 4 9 -2 8', width: 2 }] },
	{ slug: 'hair-spiky', name: 'Spiky hair', reach: 31, strokes: [{ d: 'M -30 8 L -26 -8 L -17 1 L -11 -13 L -3 -1 L 3 -14 L 10 -1 L 17 -12 L 22 1 L 30 -7 L 30 8 Z', fill: true }] },
	{ slug: 'hair-bob', name: 'Bob', reach: 33, strokes: [{ d: 'M -30 22 Q -33 -20 0 -21 Q 33 -20 30 22 L 22 22 Q 22 -6 0 -6 Q -22 -6 -22 22 Z', fill: true }] },
	{ slug: 'top-hat', name: 'Top hat', reach: 30, strokes: [{ d: 'M -26 12 L 26 12 L 26 7 L 16 7 L 16 -20 L -16 -20 L -16 7 L -26 7 Z', fill: true }] },
	{ slug: 'cap', name: 'Cap', reach: 37, strokes: [{ d: 'M -22 7 Q -22 -16 0 -16 Q 22 -16 22 7 L 36 7 L 36 11 L -22 11 Z', fill: true }] },
	{ slug: 'glasses', name: 'Glasses', reach: 18, strokes: [{ d: `${circle(-10, 0, 7)} ${circle(10, 0, 7)} M -3 0 L 3 0`, width: 2 }] },
	{ slug: 'moustache', name: 'Moustache', reach: 13, strokes: [{ d: 'M 0 -2 Q -6 -7 -12 1 Q -6 3 0 1 Q 6 3 12 1 Q 6 -7 0 -2 Z', fill: true }] },
	{ slug: 'beard', name: 'Beard', reach: 24, strokes: [{ d: 'M -22 -10 Q -22 21 0 23 Q 22 21 22 -10 L 16 -10 Q 14 9 0 9 Q -14 9 -16 -10 Z', fill: true }] }
];

export const PRIMITIVES: Shape[] = [
	{ slug: 'circle', name: 'Circle', reach: 9, strokes: [{ d: circle(0, 0, 8), fill: true }] },
	{ slug: 'ring', name: 'Ring', reach: 10, strokes: [{ d: circle(0, 0, 8), width: 2.5 }] },
	{ slug: 'oval', name: 'Oval', reach: 12, strokes: [{ d: 'M -11 0 a 11 6 0 1 0 22 0 a 11 6 0 1 0 -22 0', fill: true }] },
	{ slug: 'square', name: 'Square', reach: 10, strokes: [{ d: 'M -7 -7 H 7 V 7 H -7 Z', fill: true }] },
	{ slug: 'triangle', name: 'Triangle', reach: 9, strokes: [{ d: 'M 0 -8 L 8 6 L -8 6 Z', fill: true }] },
	{ slug: 'half', name: 'Half circle', reach: 10, strokes: [{ d: 'M -9 3 A 9 9 0 0 1 9 3 Z', fill: true }] },
	{ slug: 'line', name: 'Line', reach: 11, strokes: [{ d: 'M -10 0 L 10 0', width: 3 }] },
	{ slug: 'curve', name: 'Curve', reach: 11, strokes: [{ d: 'M -10 3 Q 0 -7 10 3', width: 3 }] }
];

export const SHAPES = new Map([...FACE, ...PRIMITIVES].map((shape) => [shape.slug, shape]));

export const SWATCHES = Array.from({ length: 10 }, (_, i) => `feature-${i + 1}`);
