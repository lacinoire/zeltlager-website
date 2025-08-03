<script lang="ts">
	// This is a horizontally scrollable container that adds an extra scrollbar that is fixed at
	// the bottom of the screen.
	import { onMount, tick } from "svelte";
	import type { Snippet } from "svelte";

	interface Props {
		children?: Snippet;
	}

	let { children }: Props = $props();
</script>

<div class="scroll-container">
	{@render children?.()}
</div>

<style lang="scss">
	.scroll-container {
		max-width: 100%;
		overflow-x: auto;
		overflow-y: auto;
		max-height: 100vh;
	}
</style>
