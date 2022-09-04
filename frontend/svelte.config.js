import adapter from "@sveltejs/adapter-static";
import preprocess from "svelte-preprocess";
import { markdown } from "svelte-preprocess-markdown";

/** @type {import('@sveltejs/kit').Config} */
const config = {
	extensions: [".svelte", ".md"],
	// Consult https://github.com/sveltejs/svelte-preprocess
	// for more information about preprocessors
	preprocess: [markdown(), preprocess()],

	kit: {
		adapter: adapter(),
		trailingSlash: "always",
	},
};

export default config;
