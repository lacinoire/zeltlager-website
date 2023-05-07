<script lang="ts">
	import { onMount, tick } from "svelte";
	// glightbox from https://github.com/biati-digital/glightbox under MIT
	import { GlightboxInit } from "./glightbox/glightbox";

	export let settings: any;
	let lightbox: GlightboxInit;
	let showLightbox = false;

	export function reload() {
		lightbox.reload();
	}

	function download() {
		console.log("Download image", lightbox.elements[lightbox.index].slideConfig.href);
		const link = document.createElement("a");
		link.href = lightbox.elements[lightbox.index].slideConfig.href;
		link.download = "";
		document.body.appendChild(link);
		link.click();
		document.body.removeChild(link);
	}

	onMount(() => {
		const allSettings = { ...settings };
		allSettings.showLightbox = async (show: boolean) => {
			showLightbox = show;
			await tick();
		};
		allSettings.svg = {
			download: `<?xml version="1.0" encoding="UTF-8"?>
                <svg width="512" height="512" version="1.1" viewBox="0 0 135.47 135.47" xmlns="http://www.w3.org/2000/svg">
                <g transform="translate(0 -161.53)">
                <rect x="18.521" y="273.19" width="97.896" height="7.9375" ry="0"/>
                <g transform="translate(-1.0583)">
                <rect x="66.146" y="175.29" width="5.2917" height="76.729"/>
                <rect transform="rotate(39.421)" x="209.93" y="117.57" width="5.2917" height="31.75"/>
                <rect transform="matrix(-.7725 .63501 .63501 .7725 0 0)" x="103.65" y="204.94" width="5.2917" height="31.75"/>
                </g>
                </g>
                </svg>
            `,
		};

		lightbox = new GlightboxInit(allSettings);
		lightbox.init();
		return () => lightbox.destroy();
	});
</script>

{#if showLightbox && lightbox}
	{#if $$slots.default}
		<slot />
	{:else}
		<div
			id="glightbox-body"
			class="glightbox-container glightbox-clean"
			tabindex="-1"
			role="dialog"
			aria-hidden="false">
			<div class="gloader visible" />
			<div class="goverlay" />
			<div class="gcontainer">
				<div id="glightbox-slider" class="gslider" />
				<button
					class="gbutton gdownload gbtn"
					on:click={download}
					aria-label="Donwload"
					data-taborder="4">
					{@html lightbox.settings.svg.download}
				</button>
				<button class="gclose gbtn" aria-label="Close" data-taborder="3">
					{@html lightbox.settings.svg.close}
				</button>
				<button class="gprev gbtn" aria-label="Previous" data-taborder="2">
					{@html lightbox.settings.svg.prev}
				</button>
				<button class="gnext gbtn" aria-label="Next" data-taborder="1">
					{@html lightbox.settings.svg.next}
				</button>
			</div>
		</div>
	{/if}
{/if}
