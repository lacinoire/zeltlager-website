<script lang="ts">
	import { onMount } from "svelte";
	import moment from "moment";
	import type { Moment } from "moment";
	import { browser } from "$app/environment";
	import { mdiChevronDown } from "@mdi/js";
	import Icon from "$lib/Icon.svelte";

	interface Member {
		id: number;
		name: string;
		target: number;
		catcher: number | null;
		last_change: Moment | null;
		nextTarget: Member;
	}

	interface Action {
		catcher: number | null;
		lastCatcher: number;
		member: number;
	}

	interface GameEntry {
		id: number;
		created: Moment;
	}

	type Game = Member[];

	const historyLength = 5;
	let lastActions: Action[] = [];

	let games: GameEntry[] = [];
	let currentGameId: number | undefined;
	let currentGame: Game | undefined;
	let filteredLive: Member[] | undefined;
	let filteredCatched: Member[] | undefined;

	let showTarget = false;
	let insertMember = false;
	let filter = "";

	if (browser) {
		lastActions = localStorage.erwischtLastActions
			? JSON.parse(localStorage.erwischtLastActions)
			: [];
		showTarget = localStorage.erwischtShowTarget === "true";
	}

	function findMember(id: number) {
		if (id === undefined) return undefined;
		const target = currentGame[id];
		if (target === undefined) return undefined;
		if (target.id === id) return target;
		return currentGame.find((m) => m.id == id);
	}

	function findNextTarget(member: Member) {
		let target = findMember(member.target);
		while (target.catcher !== null && target !== member) {
			target = findMember(target.target);
		}
		return target;
	}

	async function insertNewMember(before: number) {
		const beforeName = findMember(before).name;
		const name = prompt(`Vor ${beforeName} einfügen. Name:`);
		if (name === null || name === "undefined") return;

		await insertNewMemberName(before, name);
	}

	async function insertNewMemberName(before: number, name: string) {
		try {
			const data = {
				game: currentGameId,
				before: before,
				name: name,
			};

			const response = await fetch("/api/erwischt/game/insert", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify(data),
			});
			if (!response.ok) {
				alert("Fehler: Spieler konnte nicht eingefügt werden (Server-Fehler)");
				return;
			}
		} catch (e) {
			console.error("Failed to insert member", e);
			alert("Fehler: Spieler konnte nicht eingefügt werden (Server nicht erreichbar)");
			return;
		}
		await loadGame(currentGameId);
	}

	async function catchMember(catcher: number | null, member: number) {
		console.log(`${catcher} catched ${member}`);

		const m = findMember(member);
		try {
			const data = {
				game: currentGameId,
				catcher: catcher,
				member: member,
			};
			lastActions = localStorage.erwischtLastActions
				? JSON.parse(localStorage.erwischtLastActions)
				: [];
			lastActions.push({ ...data, lastCatcher: m.catcher });
			if (lastActions.length > historyLength)
				lastActions = lastActions.slice(lastActions.length - historyLength);
			localStorage.erwischtLastActions = JSON.stringify(lastActions);

			const response = await fetch("/api/erwischt/game/setCatch", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify(data),
			});
			if (!response.ok) {
				alert("Fehler: Spieler konnte nicht erwischt werden (Server-Fehler)");
				return;
			}
		} catch (e) {
			console.error("Failed to catch member", e);
			alert("Fehler: Spieler konnte nicht erwischt werden (Server nicht erreichbar)");
			return;
		}

		if (catcher === null) {
			const c = findMember(m.catcher);
			m.catcher = catcher;
			c.nextTarget = findNextTarget(c);
			m.nextTarget = findNextTarget(m);
		} else {
			m.catcher = catcher;
			const c = findMember(m.catcher);
			c.nextTarget = findNextTarget(c);
		}
		m.last_change = moment();
		showMembers(filter, showTarget, currentGame);
	}

	function showTargetChanged(showTarget: boolean) {
		if (browser) localStorage.erwischtShowTarget = showTarget ? "true" : "false";
	}

	$: showTargetChanged(showTarget);

	async function loadGames() {
		let response: Response;
		try {
			response = await fetch("/api/erwischt/games");
			if (!response.ok) {
				alert("Fehler: Spiele konnten nicht geladen werden (Server-Fehler)");
				return;
			}
		} catch (e) {
			console.error("Failed to list games", e);
			alert("Fehler: Spiele konnten nicht geladen werden (Server nicht erreichbar)");
			return;
		}
		try {
			games = await response.json();
		} catch (e) {
			console.error("Failed to parse games json", e);
			alert("Fehler: Spiele konnten nicht geladen werden (unlesbar)");
			return;
		}

		for (const g of games) g.created = moment.utc(g.created).local();

		if (games.length > 0) currentGameId = games[games.length - 1].id;
		else currentGameId = undefined;
	}

	async function loadGame(currentGameId: number | undefined) {
		if (currentGameId === undefined) {
			currentGame = undefined;
			return;
		}
		let response: Response;
		try {
			response = await fetch(`/api/erwischt/game/${currentGameId}`);
			if (!response.ok) {
				alert("Fehler: Spiel konnten nicht geladen werden (Server-Fehler)");
				return;
			}
		} catch (e) {
			console.error("Failed to fetch game", e);
			alert("Fehler: Spiel konnten nicht geladen werden (Server nicht erreichbar)");
			return;
		}
		try {
			currentGame = await response.json();
		} catch (e) {
			console.error("Failed to parse game json", e);
			alert("Fehler: Spiel konnten nicht geladen werden (unlesbar)");
			return;
		}

		// Compute next targets
		for (const m of currentGame) {
			m.last_change = moment.utc(m.last_change).local();
			if (m.catcher === null) m.nextTarget = findNextTarget(m);
		}

		console.log("Loaded game", currentGameId);
	}

	$: loadGame(currentGameId);

	/// Show the filtered members
	function showMembers(filter: string, showTarget: boolean, currentGame: Game | undefined) {
		if (currentGame === undefined) {
			filteredLive = undefined;
			filteredCatched = undefined;
			return;
		}
		filter = filter.toLowerCase();

		filteredLive = [];
		for (const m of currentGame) {
			if (
				m.catcher === null &&
				(filter.length === 0 || m.name.toLowerCase().includes(filter))
			) {
				filteredLive.push(m);
			}
		}
		filteredLive.sort((a, b) => a.name.localeCompare(b.name));

		filteredCatched = [];
		for (const m of currentGame) {
			if (
				m.catcher !== null &&
				(filter.length === 0 || m.name.toLowerCase().includes(filter))
			) {
				filteredCatched.push(m);
			}
		}
		filteredCatched.sort((a, b) => a.name.localeCompare(b.name));
	}

	$: showMembers(filter, showTarget, currentGame);

	async function newGame() {
		let response: Response;
		try {
			response = await fetch("/api/erwischt/game", {
				method: "POST",
			});
			if (!response.ok) {
				alert("Fehler: Spiel konnte nicht erstellt werden (Server-Fehler)");
				return;
			}
		} catch (e) {
			console.error("Failed to create game", e);
			alert("Fehler: Spiel konnte nicht erstellt werden (Server nicht erreichbar)");
			return;
		}
		try {
			currentGameId = await response.json();
		} catch (e) {
			console.error("Failed to parse created game json", e);
			alert("Fehler: Spiel konnte nicht erstellt werden (Spiel unlesbar)");
			return;
		}
		console.log("Created game", currentGameId);

		await loadGames();
	}

	function deleteGame() {
		const game = games.find((g) => g.id === currentGameId);
		if (
			window.confirm(
				`Spiel vom ${moment.utc(game.created).local().format("DD.MM.YYYY HH:mm")} löschen?`
			)
		)
			realDeleteGame();
	}

	async function realDeleteGame() {
		try {
			const response = await fetch(`/api/erwischt/game/${currentGameId}`, {
				method: "DELETE",
			});
			if (!response.ok) {
				alert("Fehler: Spiel konnte nicht gelöscht werden (Server-Fehler)");
				return;
			}
		} catch (e) {
			console.error("Failed to delete game", e);
			alert("Fehler: Spiel konnte nicht gelöscht werden (Server nicht erreichbar)");
			return;
		}

		await loadGames();
	}

	onMount(() => {
		loadGames();
	});
</script>

<svelte:head>
	<title>Erwischt – Zeltlager – FT München Gern e.V.</title>
</svelte:head>

<div class="tabs" style="margin-bottom: 1em;">
	<ul>
		{#each games as game}
			<li class:is-active={game.id === currentGameId}>
				<!-- svelte-ignore a11y-missing-attribute -->
				<a
					role="button"
					tabindex="0"
					on:click={() => (currentGameId = game.id)}
					on:keydown={(e) => {
						if (e.key === "Enter") currentGameId = game.id;
					}}
					title={game.created.format("DD.MM.YYYY HH:mm")}>
					{game.created.format("DD.MM.YYYY")}
				</a>
			</li>
		{/each}
		<li>
			<!-- svelte-ignore a11y-missing-attribute -->
			<a
				role="button"
				tabindex="0"
				on:click={newGame}
				on:keydown={(e) => {
					if (e.key === "Enter") newGame();
				}}
				title="Neues Spiel erstellen">
				+
			</a>
		</li>
	</ul>
</div>

<div class="header-flex">
	<div class="control">
		<!-- svelte-ignore a11y-autofocus -->
		<input
			class="input"
			type="text"
			autofocus={true}
			bind:value={filter}
			placeholder="Suchen…" />
	</div>
	<div class="tabs is-toggle togglebuttons">
	  <ul>
	    <li class:is-active={!insertMember}>
				<!-- svelte-ignore a11y-invalid-attribute -->
	      <a on:click={() => insertMember = !insertMember} href="#">
	        <span>erwischen</span>
	      </a>
	    </li>
	    <li class:is-active={insertMember}>
				<!-- svelte-ignore a11y-invalid-attribute -->
	      <a on:click={() => insertMember = !insertMember} href="#">
	        <span>einfügen</span>
	      </a>
	    </li>
	  </ul>
	</div>
	<div>
		<button class="button is-danger" on:click={deleteGame}>
			Spiel löschen
		</button>
	</div>
</div>

<table class="table is-striped">
	<thead>
		<tr>
			<th>Name</th>
			<th>
				Nächstes Ziel
				<label>
					anzeigen <input type="checkbox" bind:checked={showTarget} />
				</label>
			</th>
			<th />
		</tr>
	</thead>
	<tbody>
		{#if filteredLive !== undefined}
			{#each filteredLive as m}
				<tr>
					<td>{m.name}</td>
					<td>
						{#if showTarget}
							{m.nextTarget.name}
						{/if}
					</td>
					<td class="alignRight">
						{#if insertMember}
							<!-- svelte-ignore a11y-missing-attribute -->
							<a
								role="button"
								tabindex="0"
								on:click={() => insertNewMember(m.nextTarget.id)}
								on:keydown={(e) => {
									if (e.key === "Enter") insertNewMember(m.nextTarget.id);
								}}>
								einfügen
							</a>
						{:else}
							<!-- svelte-ignore a11y-missing-attribute -->
							<a
								role="button"
								tabindex="0"
								on:click={() => catchMember(m.id, m.nextTarget.id)}
								on:keydown={(e) => {
									if (e.key === "Enter") catchMember(m.id, m.nextTarget.id);
								}}>
								erwischt
							</a>
						{/if}
					</td>
				</tr>
			{/each}
		{/if}
	</tbody>
</table>

<h3 class="title is-3">Schon erwischt</h3>

<table class="table is-striped">
	<thead>
		<tr>
			<th>Name</th>
			<th>Von</th>
			<th>Zeit</th>
			<th />
		</tr>
	</thead>
	<tbody>
		{#if filteredCatched !== undefined}
			{#each filteredCatched as m}
				<tr>
					<td>{m.name}</td>
					<td>
						{findMember(m.catcher).name}
					</td>
					<td>
						{m.last_change.format("DD.MM. HH:mm")}
					</td>
					<td>
						<!-- svelte-ignore a11y-missing-attribute -->
						<a
							role="button"
							tabindex="0"
							on:click={() => catchMember(null, m.id)}
							on:keydown={(e) => {
								if (e.key === "Enter") catchMember(null, m.id);
							}}>
							wiederbeleben
						</a>
					</td>
				</tr>
			{/each}
		{/if}
	</tbody>
</table>

<h3 class="title is-3">Letzte Änderungen</h3>
<div class="recentChanges">
	{#if currentGame !== undefined}
		{#each lastActions as a}
			{#if findMember(a.member) !== undefined}
				<div>
					{#if a.catcher !== null}
						{#if findMember(a.catcher) !== undefined}
							{findMember(a.catcher).name} → {findMember(a.member).name}
						{/if}
					{:else}
						{findMember(a.member).name} wiederbelebt
					{/if}
				</div>
				<!-- svelte-ignore a11y-missing-attribute -->
				<a
					role="button"
					tabindex="0"
					on:click={() => catchMember(a.lastCatcher, a.member)}
					on:keydown={(e) => {
						if (e.key === "Enter") catchMember(a.lastCatcher, a.member);
					}}>
					rückgängig
				</a>
			{/if}
		{/each}
	{/if}
</div>

<style lang="scss">
	.alignRight {
		text-align: right;
	}

	th label {
		font-weight: normal;
	}

	.recentChanges {
		display: grid;
		grid-template-columns: max-content max-content;
		column-gap: 1em;
	}

	.header-flex {
		display: flex;
		align-items: center;
		gap: 1em;
		margin-bottom: 1em;

		.buttons,
		.button {
			margin-bottom: 0;
		}
	}

	.togglebuttons {
		margin-bottom: 0;
	}
</style>
