<svelte:options immutable />

<script lang="ts">
	interface Props {
		raw?: string | undefined;
		size?: number | string;
		name?: string | undefined;
		isLeft?: boolean;
		isRight?: boolean;
		style?: string;
		title?: string | undefined;
	}

	let {
		raw = undefined,
		size = "1.2em",
		name = undefined,
		isLeft = false,
		isRight = false,
		style = "",
		title = undefined
	}: Props = $props();
</script>

<span class="icon{isLeft ? ' is-left' : ''}{isRight ? ' is-right' : ''}" {title}>
	{#if raw}
		{raw}
	{:else}
		<svg
			viewBox="0 0 24 24"
			style="width: {typeof size === 'number' ? size + 'px' : size}; height: {typeof size ===
			'number'
				? size + 'px'
				: size}; {style}">
			<path d={name} />
		</svg>
	{/if}
</span>

<style>
	svg {
		vertical-align: middle;
	}
</style>
