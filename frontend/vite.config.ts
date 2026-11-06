import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, loadEnv } from 'vite';
import ase from './ase.ts';
import docs from './docs.ts';

export default defineConfig(({ mode }) => {
	const { STYLING_FROM, STYLING_HEADER } = loadEnv(mode, '..', 'STYLING_');
	const [name, ...value] = STYLING_HEADER?.split(':') ?? [];
	return {
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
				...(STYLING_FROM && {
					'/api/styling': {
						target: STYLING_FROM,
						changeOrigin: true,
						headers: name ? { [name.trim()]: value.join(':').trim() } : {},
						configure: (proxy) => proxy.on('proxyReq', (request) => request.removeHeader('cookie'))
					}
				}),
				'/api': 'http://localhost:4400',
				'/login': 'http://localhost:4400',
				'/callback': 'http://localhost:4400',
				'/logout': 'http://localhost:4400',
				'/hackatime': 'http://localhost:4400'
			}
		}
	};
});
