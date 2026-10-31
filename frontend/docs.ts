import rehypeShiki from '@shikijs/rehype';
import type { Root } from 'mdast';
import { toString } from 'mdast-util-to-string';
import { basename, resolve } from 'node:path';
import rehypeAutolinkHeadings from 'rehype-autolink-headings';
import rehypeRaw from 'rehype-raw';
import rehypeSlug from 'rehype-slug';
import rehypeStringify from 'rehype-stringify';
import remarkGfm from 'remark-gfm';
import remarkParse from 'remark-parse';
import remarkRehype from 'remark-rehype';
import { unified } from 'unified';
import { normalizePath, type Plugin } from 'vite';

function heading() {
	return (tree: Root, file: { data: Record<string, unknown> }) => {
		const first = tree.children.find((node) => node.type === 'heading');
		if (first) file.data.title = toString(first);
	};
}

const processor = unified()
	.use(remarkParse)
	.use(remarkGfm)
	.use(heading)
	.use(remarkRehype, { allowDangerousHtml: true })
	.use(rehypeRaw)
	.use(rehypeSlug)
	.use(rehypeAutolinkHeadings, { behavior: 'wrap' })
	.use(rehypeShiki, { theme: 'github-dark-default' })
	.use(rehypeStringify);

export default function docs(dir = 'src/lib/docs'): Plugin {
	let root = normalizePath(resolve(dir)) + '/';
	return {
		name: 'docs',
		configResolved(config) {
			root = normalizePath(resolve(config.root, dir)) + '/';
		},
		async transform(code, id) {
			if (!id.endsWith('.md') || !id.startsWith(root)) return;
			const file = await processor.process(code);
			const title = (file.data.title as string | undefined) ?? basename(id, '.md');
			return { code: `export default ${JSON.stringify({ title, html: String(file) })};`, map: null };
		}
	};
}
