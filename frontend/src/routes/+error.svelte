<script lang="ts">
	import { browser } from '$app/environment';
	import { onMount } from 'svelte';
	import { page } from '$app/stores';

	onMount(() => {
		let location: string = browser ? window.location.pathname : "";
		if (location.startsWith("/Bilder")) {
			// Try to reload page, svelte does not know the image pages,
			// so we need to enter the page directly, not through svelte's navigation
			window.location.pathname = window.location.pathname;
		}
	});
</script>

<h1>{$page.status}: {$page.error.message}</h1>
