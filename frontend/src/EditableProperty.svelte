<script lang="ts">
	import moment from "moment";
	import { createEventDispatcher } from "svelte";
	import Icon from "./Icon.svelte";

	export let value: any;
	export let isMoment = false;
	export let momentFormat = "DD.MM.YYYY";

	const dispatch = createEventDispatcher<{ edit: undefined }>();

	let editing = false;
	let editValue: string = "";

	function edit() {
		if (isMoment)
			editValue = value === undefined || value === null ? "" : value.format(momentFormat);
		else editValue = value;
		editing = true;
	}

	function keydown(e: KeyboardEvent) {
		if (e.key === "Escape") cancel();
	}

	function cancel() {
		editing = false;
	}

	function edited() {
		let newValue: any;
		if (isMoment) {
			if (editValue === "") newValue = undefined;
			else newValue = moment(editValue, momentFormat);
		} else if (typeof value !== "boolean") {
			newValue = editValue;
		}
		editing = false;
		if (newValue !== value) {
			value = newValue;
			dispatch("edit");
		}
	}
</script>

<form on:submit|preventDefault={edited}>
	{#if value === undefined || value === null}
		<!-- -->
	{:else if isMoment}
		{#if !editing}
			{value.format(momentFormat)}
		{/if}
	{:else if typeof value === "string"}
		{#if !editing}
			{value}
		{/if}
	{:else if typeof value === "boolean"}
		<input type="checkbox" bind:checked={value} />
	{/if}
	{#if typeof value !== "boolean"}
		{#if !editing}
			<button class="btn px-1" on:click={edit}><Icon name="pencil" /></button>
		{:else}
			<!-- svelte-ignore a11y-autofocus -->
			<input
				id="value"
				class="form-control here"
				name="value"
				type="text"
				autofocus={true}
				bind:value={editValue}
				on:keydown={keydown}
			/>
			<button class="btn px-1" type="submit"><Icon name="content-save" /></button>
		{/if}
	{/if}
</form>

<style>
	button {
		font-size: 0.7em;
	}
</style>
