import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { markdown } from "svelte-preprocess-markdown";

const config = {
	extensions: [".svelte", ".md"],
	// Consult https://kit.svelte.dev/docs/integrations#preprocessors
	// for more information about preprocessors
	preprocess: [markdown(), vitePreprocess()],
	kit: { adapter: adapter() }
};

export default config;
