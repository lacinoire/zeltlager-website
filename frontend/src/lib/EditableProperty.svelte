<script lang="ts">
	import moment from "moment";
	import { mdiPencil, mdiContentSave } from "@mdi/js";
	import Icon from "./Icon.svelte";
	import type { EnumValue } from "./utils.ts";

	interface Props {
		value: any;
		isMoment: boolean;
		momentFormat: string;
		enumValues?: EnumValue[];
		onedit?: (setEnabled: (enabled: boolean) => void) => void;
	}

	let { value = $bindable(), isMoment = false, momentFormat = "DD.MM.YYYY", enumValues, onedit }: Props = $props();

	let editing = $state(false);
	let editValue: string = $state("");
	let enabled = $state(true);
	let select;

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
		} else if (select !== undefined) {
			newValue = select.value;
		} else if (typeof value !== "boolean") {
			newValue = editValue;
		}
		editing = false;
		if (newValue !== value) {
			if (typeof value !== "boolean") value = newValue;
			onedit?.(setEnabled);
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
	{:else if enumValues !== undefined}
		<select bind:this={select} name="value" id="value" onchange={edited}>
			{#each enumValues as val}
				{@const name = typeof val === "string" ? val : val.name}
				{@const displayName = typeof val === "string" ? val : (val.displayName ?? val.name)}
			  <option value={name} selected={name === value}>{displayName}</option>
			{/each}
		</select>
		{#if !enabled}
			<button class="button is-loading"></button>
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
	{#if typeof value !== "boolean" && enumValues === undefined}
		{#if !editing}
			<button class="button" onclick={edit} class:is-loading={!enabled}>
				<Icon name={mdiPencil} />
			</button>
		{:else}
			<!-- svelte-ignore a11y_autofocus -->
			<input
				id="value"
				class="input"
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
	form {
		display: flex;
		gap: 0.2em;
	}
</style>
