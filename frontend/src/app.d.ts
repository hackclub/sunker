// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		interface PageState {
			map?: boolean;
			mines?: boolean;
			ring?: boolean;
		}
		// interface Platform {}
	}
}

export {};
