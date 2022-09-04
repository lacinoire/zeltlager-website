import { sveltekit } from "@sveltejs/kit/vite";
import type { UserConfig } from "vite";

const config: UserConfig = {
	plugins: [sveltekit()],
	server: {
		proxy: {
			"/admin": "http://localhost:8080",
			"/erwischt": "http://localhost:8080",
			"/static": "http://localhost:8080",
		},
	},
};

export default config;
