export const RING_COUNT = 6;
export const RING_WIDTH = 90;
export const VIEW_HEIGHT = 600;
export const CENTER_OFFSET = 40;

export const RING_CHARSETS = [
	'01',
	'0123456789ABCDEF',
	'{}[]();:<>*&|^~!%=+-/\\',
	'αβγδεζηθλμξπρσφψωΓΔΘΛΞΠΣΦΨΩ',
	'·∙•●◦○',
	'░▒▓█▀▄▌▐■□▪▫'
];

export const ringsState = $state({ active: false });

export function ringGeometry(width: number, height: number) {
	const scale = height / VIEW_HEIGHT;
	return {
		cx: width / 2,
		cy: height - CENTER_OFFSET * scale,
		radii: Array.from({ length: RING_COUNT }, (_, k) => RING_WIDTH * (k + 1) * scale)
	};
}
