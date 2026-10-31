import { goto } from '$app/navigation';
import { page } from '$app/state';

type Value = string | number | boolean | null | undefined;

export const params = () => (page.shallow?.url ?? page.url).searchParams;

export function href(changes: Record<string, Value>) {
	const url = new URL((page.shallow?.url ?? page.url).href);
	for (const [key, value] of Object.entries(changes)) {
		if (value === null || value === undefined || value === false) url.searchParams.delete(key);
		else url.searchParams.set(key, value === true ? '' : String(value));
	}
	const search = url.searchParams.toString().replaceAll('%2F', '/').replace(/=(?=&|$)/g, '');
	return `${url.pathname}${search ? `?${search}` : ''}`;
}

export function update(changes: Record<string, Value>, { replace = false, state }: { replace?: boolean; state?: App.PageState } = {}) {
	return goto(href(changes), { shallow: true, replace, state: state ?? (replace ? page.state : {}) });
}
