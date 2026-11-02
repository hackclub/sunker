import { watch } from '#lib/styles.svelte.js';
import type { LayoutLoad } from './$types';

export const ssr = false;

export const load: LayoutLoad = ({ fetch }) => watch(fetch);
