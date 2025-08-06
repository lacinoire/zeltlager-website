<script lang="ts">
	// A container to support horizontally scrolling tables.
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
