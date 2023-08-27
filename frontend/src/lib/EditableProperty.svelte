<script lang="ts">
	import moment from "moment";
	import { createEventDispatcher } from "svelte";
	import { mdiPencil, mdiContentSave } from "@mdi/js";
	import Icon from "./Icon.svelte";

	export let value: any;
	export let isMoment = false;
	export let momentFormat = "DD.MM.YYYY";

	interface EditEvent {
		setEnabled: (enabled: boolean) => void;
	}

	const dispatch = createEventDispatcher<{ edit: EditEvent }>();

	let editing = false;
	let editValue: string = "";
	let enabled = true;

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

	export function setEnabled(enable: boolean) {
		enabled = enable;
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
			if (typeof value !== "boolean") value = newValue;
			dispatch("edit", { setEnabled });
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
		<input type="checkbox" bind:checked={value} on:change={edited} disabled={!enabled} />
		{#if !enabled}
			<button class="button is-loading">
			</button>
		{/if}
	{/if}
	{#if typeof value !== "boolean"}
		{#if !editing}
			<button class="button" on:click={edit} class:is-loading={!enabled}>
				<Icon name={mdiPencil} />
			</button>
		{:else}
			<!-- svelte-ignore a11y-autofocus -->
			<input
				id="value"
				class="form-control here"
				name="value"
				type="text"
				autofocus={true}
				bind:value={editValue}
				on:keydown={keydown} />
			<button class="button" type="submit">
				<Icon name={mdiContentSave} />
			</button>
		{/if}
	{/if}
</form>

<style>
	button {
		font-size: 0.7em;
	}
</style>
