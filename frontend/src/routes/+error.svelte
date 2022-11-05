<script lang="ts">
	import { browser } from "$app/environment";
	import { onMount } from "svelte";
	import { page } from "$app/stores";

	onMount(() => {
		let location: string = browser ? window.location.pathname : "";
		if (location.startsWith("/Bilder")) {
			// Try to reload page, svelte does not know the image pages,
			// so we need to enter the page directly, not through svelte's navigation
			window.location.pathname = window.location.pathname;
		}
	});
</script>

{#if $page.status === 404}
	<h1 class="title">404. Diese Seite gibt’s nicht 😇</h1>
{:else}
	{$page.status}: {$page.error.message}
{/if}
