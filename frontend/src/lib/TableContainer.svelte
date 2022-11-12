<script lang="ts">
    // This is a horizontally scrollable container that adds an extra scrollbar that is fixed at
    // the bottom of the screen.
    import { onMount } from "svelte";

    let container: HTMLDivElement;
    let scrollbar: HTMLDivElement;

    let showScrollbar = false;
    let scrollbarWidth = 0;
    let setupTimeout: NodeJS.Timeout | undefined;

    function onScroll() {
        container.scrollTo({ left: scrollbar.scrollLeft });
    }

    function onContainerScroll() {
        scrollbar.scrollTo({ left: container.scrollLeft });
    }

    function setup() {
        if (setupTimeout !== undefined)
            clearTimeout(setupTimeout);
        setupTimeout = setTimeout(setupImpl, 50);
    }

    function setupImpl() {
        scrollbarWidth = container.scrollWidth;
        scrollbar.scrollTo({ left: container.scrollLeft });
        showScrollbar = container.offsetWidth < container.scrollWidth;
    }

    onMount(() => {
        const resizeObserver = new ResizeObserver(entries => {
            setup();
        });
        resizeObserver.observe(container);
        setup();
        return () => resizeObserver.unobserve(container);
    });
</script>

<svelte:window on:resize={setup} />

<div class="scroll-container">
    <div class="table-container" bind:this={container} on:scroll={onContainerScroll}>
        <slot />
    </div>
    <div class="fixed-scrollbar" bind:this={scrollbar} on:scroll={onScroll} class:is-hidden={!showScrollbar}><div style={"width: " + scrollbarWidth + "px;"}></div></div>
</div>

<style lang="scss">
    .table-container {
        scrollbar-width: none;
        margin-bottom: 0;
    }

    .scroll-container {
        margin-bottom: 1.5rem;
    }

    .fixed-scrollbar {
        overflow-x: scroll;
        position: sticky;
        width: 100%;
        bottom: 0;

        div {
            height: 20px;
        }
    }
</style>
