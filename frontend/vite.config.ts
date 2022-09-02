import { resolve } from "path";
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// https://vitejs.dev/config/
export default defineConfig({
	base: "/frontend/",
	plugins: [svelte()],
	server: {
		proxy: {
			"/admin": "http://localhost:8080",
			"/erwischt": "http://localhost:8080",
			"/static": "http://localhost:8080",
		},
	},
	build: {
		rollupOptions: {
			input: {
				betreuer: resolve(__dirname, "betreuer.html"),
				erwischt: resolve(__dirname, "erwischt.html"),
				teilnehmer: resolve(__dirname, "teilnehmer.html"),
			},
		},
	},
});
