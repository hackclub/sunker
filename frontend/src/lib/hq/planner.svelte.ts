export type Hex = [number, number];

export const planner = $state<{ selectable: Hex[] | null; selected: Hex | null; path: Hex[] | null; pick: ((hex: Hex) => void) | null }>({
	selectable: null,
	selected: null,
	path: null,
	pick: null
});
