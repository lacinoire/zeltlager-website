<script lang="ts">
	import moment from "moment";
	import { createEventDispatcher } from "svelte";
	import { mdiPencil, mdiContentSave } from "@mdi/js";
	import Icon from "./Icon.svelte";

	interface Props {
		value: any;
		isMoment?: boolean;
		momentFormat?: string;
	}

	let { value = $bindable(), isMoment = false, momentFormat = "DD.MM.YYYY" }: Props = $props();

	interface EditEvent {
		setEnabled: (enabled: boolean) => void;
	}

	const dispatch = createEventDispatcher<{ edit: EditEvent }>();

	let editing = $state(false);
	let editValue: string = $state("");
	let enabled = $state(true);

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

	function onsubmit(e) {
		e.preventDefault();
		edited();
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

<form {onsubmit}>
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
		<input type="checkbox" bind:checked={value} onchange={edited} disabled={!enabled} />
		{#if !enabled}
			<button class="button is-loading"></button>
		{/if}
	{/if}
	{#if typeof value !== "boolean"}
		{#if !editing}
			<button class="button" onclick={edit} class:is-loading={!enabled}>
				<Icon name={mdiPencil} />
			</button>
		{:else}
			<!-- svelte-ignore a11y_autofocus -->
			<input
				id="value"
				class="form-control here"
				name="value"
				type="text"
				autofocus={true}
				bind:value={editValue}
				onkeydown={keydown} />
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
