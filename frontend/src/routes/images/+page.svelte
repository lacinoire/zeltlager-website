<script lang="ts">
	import { splitImageName } from "$lib/utils";
	import { onMount, tick } from "svelte";

	let GLightbox: (options: any) => any;
	let subName = "";
	let imageList: string[] = [];
	let error: string | undefined;
	let isLoading = true;
	let gallery: HTMLDivElement | undefined;

	const lightboxHTML = `
		<div id="glightbox-body" class="glightbox-container" tabindex="-1" role="dialog" aria-hidden="false">
			<div class="gloader visible"></div>
			<div class="goverlay"></div>
			<div class="gcontainer">
				<div id="glightbox-slider" class="gslider"></div>
				<button class="gclose gbtn" aria-label="Close" data-taborder="3">{closeSVG}</button>
				<button class="gprev gbtn" aria-label="Previous" data-taborder="2">{prevSVG}</button>
				<button class="gnext gbtn" aria-label="Next" data-taborder="1">{nextSVG}</button>
			</div>
		</div>`;

	async function listImages() {
		let path = window.location.pathname;
		if (path.endsWith("/")) path = path.slice(0, -1);
		error = undefined;
		imageList = [];
		let response: Response;
		try {
			response = await fetch(path + "/list");
		} catch (e) {
			console.error("Failed to list images", path, e);
			error = "Verbindung fehlgeschlagen. Ist das Internet erreichbar?";
			return;
		}
		let respText: string;
		try {
			respText = await response.text();
		} catch (e) {
			console.error("Failed to read image list", path, e);
			error = "Verbindung abgebrochen";
			return;
		}
		try {
			const resp = JSON.parse(respText);
			// List successful
			imageList = resp;
			isLoading = false;
			await tick();
			if (gallery !== undefined) {
				const lightbox = GLightbox({
					touchNavigation: true,
					// loop: true,
					// TODO zoomable?
					lightboxHTML,
				});
			}
		} catch (e) {
			console.error("Failed to convert image list to json", path, e);
			error = respText;
			return;
		}
	}

	onMount(async () => {
		GLightbox = (await import("glightbox/dist/js/glightbox")).default;
		let path = window.location.pathname;
		if (!path.startsWith("/Bilder")) return;
		path = path.slice("/Bilder".length);
		if (path.endsWith("/")) path = path.slice(0, -1);
		subName = splitImageName(path);
		listImages();
	});
</script>

<svelte:head>
	<title>Bilder {subName} – Zeltlager – FT München Gern e.V.</title>
</svelte:head>

<h3 class="title is-3">Bilder {subName}</h3>

{#if error !== undefined}
	<article class="message is-danger">
		<div class="message-body">
			{error}
		</div>
	</article>
{/if}

{#if error === undefined && isLoading}
	<progress class="progress is-small is-primary">Loading</progress>
{/if}

<div bind:this={gallery} class="is-flex galleryContainer">
	{#each imageList as image}
		{#if image.toLowerCase().endsWith(".jpg") || image.toLowerCase().endsWith(".jpeg") || image
				.toLowerCase()
				.endsWith(".png")}
			<a
				href={`static/${image}`}
				class="glightbox box">
				<!-- svelte-ignore a11y-missing-attribute -->
				<img src={`static/thumbs/${image}`} />
			</a>
		{:else}
			<a href={`static/${image}`} target="_blank" class="box">
				<div class="document">
					<img src="/img/file.svg" alt="Document" />
					{image}
				</div>
			</a>
		{/if}
	{/each}
</div>

<style lang="scss">
	@import "glightbox/dist/css/glightbox.min.css";

	.galleryContainer {
		gap: 0.75em;
		flex-wrap: wrap;

		.box {
			padding: 0.5em;
		}
	}

	.document {
		display: flex;
		flex-direction: column;
		text-align: center;

		img {
			height: 10em;
		}
	}
</style>
