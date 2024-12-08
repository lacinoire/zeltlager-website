<svelte:options accessors={true} />

<script lang="ts">
	import { onMount, tick, createEventDispatcher } from "svelte";
	import { browser } from "$app/environment";
	import type { FormError } from "$lib/utils";
	import Icon from "$lib/Icon.svelte";
	import { mdiDelete, mdiHelpCircle } from "@mdi/js";

	let error: FormError | undefined;
	let isLoading = false;
	let errorMsg: HTMLElement | undefined;
	let curCategory: number = 0;
	let formSaved = false;
	// Value to trigger updates for the category state
	let somethingChanged = false;

	const dispatch = createEventDispatcher<{ submit: undefined }>();

	export interface Variant {
		// Defaults to name.toLowerCase()
		id?: string;
		name: string;
	}

	export interface Field {
		name: string;
		// Defaults to name.toLowerCase()
		id?: string;
		defaultValue?: string;
		// Defaults to name
		placeholder?: string;
		autocomplete?: string;
		inputmode?: string;
		// Defaults to true
		required?: boolean;
		// Defaults to text
		type?: string;
		help?: string;
		help_tooltip?: string;
		// For type=radio, defaults to DEFAULT_VARIANTS
		variants?: Variant[];
	}

	export interface Category {
		name: string;
		// Defaults to name.toLowerCase()
		id?: string;
		fields: Field[];
	}

	export const DEFAULT_VARIANTS: Variant[] = [
		{ id: "true", name: "Ja" },
		{ id: "false", name: "Nein" },
	];

	export let form: HTMLFormElement | undefined;

	export let name: string;
	export let categories: Category[];
	export let submitText: string;

	export let nojs_submit_url: string;

	export async function setError(msg: string) {
		error = { message: msg };
		await tick();
		errorMsg?.scrollIntoView({ behavior: "smooth" });
	}

	export async function setErrorMsg(msg: FormError) {
		error = msg;
		await tick();
		errorMsg?.scrollIntoView({ behavior: "smooth" });
	}

	function isCategoryFinished(cat: number) {
		if (form === undefined) return false;
		for (const f of categories[cat].fields) {
			const id = f.id ?? f.name.toLowerCase();
			if ((f.required ?? true) && (!form[id].value || form[id].value === "")) return false;
		}
		return true;
	}

	function isCategoryFinishedUpTo(cat: number) {
		for (let i = 0; i < cat; i++) {
			if (!isCategoryFinished(i)) return false;
		}
		return true;
	}

	function setCategory(cat: number) {
		curCategory = cat;
		const id = categories[cat].id ?? categories[cat].name.toLowerCase();
		location.hash = `#${id}`;
		form?.scrollIntoView(true);
	}

	function saveEntries() {
		somethingChanged = !somethingChanged;
		const formObj = {};
		for (const c of categories) {
			for (const f of c.fields) {
				const id = f.id ?? f.name.toLowerCase();
				if (f.type !== "checkbox" && form[id].value !== "") formObj[id] = form[id].value;
			}
		}
		if (Object.keys(formObj).length !== 0) {
			localStorage[name] = JSON.stringify(formObj);
			formSaved = true;
		} else {
			localStorage.removeItem(name);
			formSaved = false;
		}
	}

	function loadEntries() {
		if (localStorage[name] === undefined) return;
		formSaved = true;
		const formObj = JSON.parse(localStorage[name]);
		for (const c of categories) {
			for (const f of c.fields) {
				const id = f.id ?? f.name.toLowerCase();
				if (id in formObj) form[id].value = formObj[id];
			}
		}
	}

	export function clearEntries() {
		localStorage.removeItem(name);
		formSaved = false;
		form.reset();
		somethingChanged = !somethingChanged;
	}

	async function handleSubmit() {
		// Skip if there is a submit in progress
		if (isLoading && error === undefined) return;
		error = undefined;
		isLoading = true;

		await dispatch("submit");

		isLoading = false;
	}

	onMount(() => {
		loadEntries();

		if (browser) {
			// Set category by location hash
			const loc = location.hash;
			if (loc && loc !== "" && loc !== "#") {
				const id = loc.substring(1);
				for (let i = 0; i < categories.length; i++) {
					const catId = categories[i].id ?? categories[i].name.toLowerCase();
					if (catId === id) {
						curCategory = i;
						break;
					}
				}
			}

			// Remove required classes for firefox on android, it doesn't show any popup there
			const userAgent = navigator.userAgent.toLowerCase();
			if (userAgent.includes("android") && userAgent.includes("firefox") && form) {
				form.querySelectorAll("input").forEach((element) => (element.required = false));
			}
		}
	});
</script>

{#if error !== undefined && error.field === undefined}
	<div bind:this={errorMsg} class="error-msg">
		<article class="message is-danger">
			<div class="message-body">
				{error.message}
			</div>
		</article>
	</div>
{/if}

<div class="progress-indicator-container">
	<div class="progress-indicator">
		{#each categories as category, i}
			<div
				class="category"
				class:active={i == curCategory}
				class:finished={i <= curCategory && isCategoryFinishedUpTo(i, somethingChanged)}>
				{#if i > 0}
					<div class="bar" />
				{/if}
				<div
					class="knob label-container"
					class:finished={i <= curCategory && isCategoryFinished(i, somethingChanged)}>
					<div class="progress-label">
						<a
							href={`#${category.id ?? category.name.toLowerCase()}`}
							on:click={() => (curCategory = i)}>{category.name}</a>
					</div>
				</div>
			</div>
		{/each}
	</div>
</div>

<form
	class="form"
	method="post"
	action={nojs_submit_url}
	on:submit|preventDefault={handleSubmit}
	bind:this={form}>
	{#each categories as category, i}
		<div
			class:is-hidden={(i != curCategory &&
				browser &&
				curCategory != categories.length - 1) ||
				category.fields.length == 0}>
			<h2 class="title is-4" id={category.id ?? category.name.toLowerCase()}>
				{category.name}
			</h2>
			{#each category.fields as field}
				{#if error !== undefined && error.field === (field.id ?? field.name.toLowerCase())}
					<div bind:this={errorMsg} class="error-msg">
						<article class="message is-danger">
							<div class="message-body">
								{error.message}
							</div>
						</article>
					</div>
				{/if}

				<div class="field is-horizontal" class:required={field.required ?? true}>
					<div class="field-label">
						{#if field.type !== "checkbox"}
							<label for={field.id ?? field.name.toLowerCase()} class="label">
								{@html field.name}{#if field.help_tooltip !== undefined}
									<span class="helpTooltip">
										<div class="helpTooltipContent">
											{@html field.help_tooltip ?? ""}
										</div>
										<span class="helpTooltipIcon">
											<Icon name={mdiHelpCircle} />
										</span>
									</span>
								{/if}</label>
						{/if}
					</div>
					<div class="field-body">
						<div class="field">
							<div class="control">
								{#if field.type === undefined || field.type === "text" || field.type === "email"}
									<input
										id={field.id ?? field.name.toLowerCase()}
										name={field.id ?? field.name.toLowerCase()}
										placeholder={field.placeholder ?? field.name}
										required={field.required ?? true}
										class="input"
										autocomplete={field.autocomplete ?? false}
										value={field.defaultValue ?? ""}
										inputmode={field.inputmode ?? ""}
										on:keydown={field.keydown}
										on:blur={saveEntries}
										on:focusout={field.focusout}
										type={field.type ?? "text"} />
								{:else if field.type === "radio"}
									{#each field.variants ?? DEFAULT_VARIANTS as variant}
										<label class="radio">
											<input
												name={field.id ?? field.name.toLowerCase()}
												value={variant.id ?? variant.name.toLowerCase()}
												required
												type="radio"
												on:change={saveEntries} />
											{variant.name}
										</label>
									{/each}
								{:else if field.type === "textarea"}
									<textarea
										id={field.id ?? field.name.toLowerCase()}
										name={field.id ?? field.name.toLowerCase()}
										on:blur={saveEntries}
										cols="40"
										rows="1"
										class="textarea"
										aria-describedby={field.help !== undefined
											? (field.id ?? field.name.toLowerCase()) + "HelpBlock"
											: undefined} />
								{:else if field.type === "checkbox"}
									<label class="checkbox">
										<input
											name={field.id ?? field.name.toLowerCase()}
											value="true"
											required
											type="checkbox"
											on:change={saveEntries} />
											<span class="label" style="display: inline;">
												{@html field.name}
											</span>
									</label>
								{/if}
							</div>
							{#if field.help !== undefined || (field.required === false && field.type === "textarea")}
								<p
									id={(field.id ?? field.name.toLowerCase()) + "HelpBlock"}
									class="help">
									{@html field.help ?? ""}
									{#if field.required === false}
										<p class="optional">Optional</p>
									{/if}
								</p>
							{/if}
						</div>
					</div>
				</div>
			{/each}
		</div>
	{/each}

	<br />
	<div class="field is-horizontal required">
		<div class="field-label" />
		<div class="field-body">
			<div class="required"><span class="label" style="display: inline;" />Pflichtfeld</div>
		</div>
	</div>

	<div class="field is-horizontal">
		<div class="field-label" />
		<div class="field-body">
			<div class="field">
				<div class="control">
					<span class="form-buttons">
						{#if curCategory != 0 && browser}
							<button
								class="button"
								on:click|preventDefault={() => setCategory(curCategory - 1)}>
								Zurück
							</button>
						{/if}
						{#if curCategory == categories.length - 1 || !browser}
							<button
								type="submit"
								class="button is-primary"
								class:is-loading={isLoading && error === undefined}>
								{submitText}
							</button>
						{:else}
							<button
								class="button is-info"
								on:click|preventDefault={() => setCategory(curCategory + 1)}>
								Weiter
							</button>
						{/if}
					</span>
					{#if formSaved && curCategory == categories.length - 1}
						<button
							class="button reset-button"
							on:click|preventDefault={clearEntries}
							title="Formular zurücksetzen">
							<Icon name={mdiDelete} />
						</button>
					{/if}
				</div>
			</div>
		</div>
	</div>
</form>

<style lang="scss">
	:target,
	.error-msg,
	.form,
	input {
		scroll-margin-top: 5em;
	}

	.error-msg {
		margin-bottom: 1em;
	}

	h2.title.is-4 {
		margin-top: 3em;
		margin-bottom: 1.2em;
	}

	form > .field:not(:last-child) {
		margin-bottom: 1.5em;
	}

	.button {
		margin-top: 2em;
	}

	.button.is-primary {
		font-weight: bold;
	}

	.optional {
		float: right;
		font-style: italic;
	}
	.reset-button {
		float: right;
	}

	.radio {
		margin-right: 0.5em;
	}

	.progress-indicator-container {
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.helpTooltip {
		position: relative;

		.helpTooltipContent {
			display: none;
			position: absolute;
			left: 0;
			bottom: 1.5em;
			background-color: #000c;
			color: #fff;
			padding: 0.4em;
			border-radius: 0.4em;
			font-size: 0.9em;
			bottom: 1.2em;
			min-width: max-content;
			z-index: 1;
		}

		&:hover {
			.helpTooltipContent {
				display: block;
			}
		}
	}

	$knob-size: 1em;
	$knob-margin: 0.7em;
	$bar-thickness: 0.2em;
	$bar-margin: calc($knob-margin + $knob-size / 2 - $bar-thickness / 2);
	.progress-indicator {
		display: flex;
		flex-direction: row;

		// Space for the label
		margin-top: 3em;

		.category {
			display: flex;
			align-items: center;
			flex-direction: row;

			.knob {
				background-color: #ddd;
				border-radius: 100%;
				border: 0.15em solid white;
				padding: 0.15em;
				margin: $knob-margin;
				width: $knob-size;
				height: $knob-size;
				box-sizing: border-box;
				background-clip: content-box;
			}

			.bar {
				background-color: #eec73d;
				height: $bar-thickness;
				width: 13em;
				padding: 0;
			}

			.label-container {
				position: relative;
			}

			.progress-label {
				position: absolute;
				transform: translate(-50%, -2em);
				text-align: center;
				width: 15em;
				font-size: 1.2em;

				a {
					color: inherit;
					&:hover {
						color: hsl(229, 53%, 53%);
					}
				}
			}

			&.active {
				.knob {
					border-color: #eec73d;
				}

				.progress-label {
					font-weight: bold;
				}
			}

			.knob.finished {
				border-color: #0eb100;
			}

			&.finished {
				.bar {
					background-color: #0eb100;
				}
			}
		}
	}

	@media screen and (max-width: 1230px) {
		.form-buttons {
			display: flex;
			flex-direction: column-reverse;
		}

		.button {
			width: 100%;
		}

		.progress-indicator-container {
			display: inherit;
		}

		.progress-indicator {
			flex-direction: column;

			.category {
				flex-direction: column;
				align-items: start;

				.bar {
					width: $bar-thickness;
					height: 2em;
					margin-left: $bar-margin;
				}

				.progress-label {
					transform: translate(2em, -0.5em);
					text-align: left;
				}
			}
		}
	}
</style>
