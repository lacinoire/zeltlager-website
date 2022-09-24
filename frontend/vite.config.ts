import { sveltekit } from "@sveltejs/kit/vite";
import type { UserConfig } from "vite";

const config: UserConfig = {
	plugins: [sveltekit()],
	ssr: {
		noExternal: ["simplelightbox"],
	},
	server: {
		proxy: {
			"/api": "http://localhost:8080",
			"^/Bilder[^/]*/$": {
				target: 'http://localhost:5173/',
				rewrite: () => '/images/',
			},
			"^/Bilder[^/]*/list": "http://localhost:8080",
			"^/Bilder[^/]*/static/": "http://localhost:8080",
		},
	},
};

export default config;
