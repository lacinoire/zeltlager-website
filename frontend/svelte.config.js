import sveltePreprocess from "svelte-preprocess";

export default {
	// Consult https://github.com/sveltejs/svelte-preprocess
	// for more information about preprocessors
	preprocess: sveltePreprocess({
		scss: {
			includePaths: ["src", "node_modules"],
			// prependData is for preproc >= 4.X
			prependData: `
				@use "sass:math";
			`,
		},
	}),
};
