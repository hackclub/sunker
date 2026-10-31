import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import ase from './ase.ts';
import docs from './docs.ts';

export default defineConfig({
	plugins: [
		ase(),
		docs(),
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},

			adapter: adapter({ fallback: 'index.html' })
		})
	],
	envDir: '..',
	server: {
		proxy: {
			'/api': 'http://localhost:4400',
			'/login': 'http://localhost:4400',
			'/callback': 'http://localhost:4400',
			'/logout': 'http://localhost:4400'
		}
	}
});
