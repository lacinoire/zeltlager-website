<script lang="ts">
	import { onMount } from "svelte";
	import moment from "moment";
	import type { Moment } from "moment";
	import GlobalCss from "./GlobalCss.svelte";

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
	let lastActions: Action[] = localStorage.erwischtLastActions
		? JSON.parse(localStorage.erwischtLastActions)
		: [];

	let games: GameEntry[] = [];
	let currentGameId: number | undefined;
	let currentGame: Game | undefined;
	let filteredLive: Member[] | undefined;
	let filteredCatched: Member[] | undefined;

	let showTarget = localStorage.erwischtShowTarget === "true";
	let insertMember = false;
	let filter = "";

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

			const response = await fetch("/erwischt/game/insert", {
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

	async function catchMember(catcher: number, member: number) {
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

			const response = await fetch("/erwischt/game/setCatch", {
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
		localStorage.erwischtShowTarget = showTarget ? "true" : "false";
	}

	$: showTargetChanged(showTarget);

	async function loadGames() {
		let response: Response;
		try {
			response = await fetch("/erwischt/games");
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
			response = await fetch(`/erwischt/game/${currentGameId}`);
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
				(filter.length === 0 ||
					m.name.toLowerCase().includes(filter) ||
					(showTarget && m.nextTarget.name.toLowerCase().includes(filter)))
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
			response = await fetch("/erwischt/game", {
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
			const response = await fetch(`/erwischt/game/${currentGameId}`, {
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

	onMount(loadGames);
</script>

<GlobalCss />
<ul class="nav nav-tabs" style="margin-bottom: 1em;">
	{#each games as game}
		<li class="nav-item gameTab">
			<!-- svelte-ignore a11y-invalid-attribute -->
			<a
				class="nav-link"
				class:active={game.id === currentGameId}
				href="#"
				on:click={() => (currentGameId = game.id)}
				title={game.created.format("DD.MM.YYYY HH:mm")}
			>
				{game.created.format("DD.MM.YYYY")}
			</a>
		</li>
	{/each}
	<li class="nav-item">
		<!-- svelte-ignore a11y-invalid-attribute -->
		<a class="nav-link" href="#" on:click={newGame} title="Neues Spiel erstellen">+</a>
	</li>
</ul>

<div class="form-inline">
	<div class="form-group mx-sm-3 mb-2">
		<!-- svelte-ignore a11y-autofocus -->
		<input
			class="form-control"
			type="text"
			autofocus={true}
			bind:value={filter}
			placeholder="Suchen…"
		/>
	</div>
	<div class="btn-group btn-group-toggle mb-2">
		<label class="btn btn-secondary" class:active={!insertMember}>
			<input type="radio" autocomplete="off" bind:group={insertMember} value={false} /> erwischen
		</label>
		<label class="btn btn-secondary" class:active={insertMember}>
			<input type="radio" autocomplete="off" bind:group={insertMember} value={true} /> einfügen
		</label>
	</div>
	<div class="navbar-light" style="margin: 0.5em;">
		<button
			class="navbar-toggler"
			type="button"
			data-toggle="collapse"
			data-target="#gameOptions"
			aria-controls="gameOptions"
			aria-expanded="false"
			aria-label="Toggle navigation"
		>
			<span class="navbar-toggler-icon" />
		</button>
		<div
			class="collapse navbar-collapse"
			id="gameOptions"
			style="background-color: white; border: 1px solid rgba(0,0,0,.1); border-radius: 0.5em 0 0.5em 0.5em; width: 30em; max-width: 100vw"
		>
			<ul class="navbar-nav">
				<li class="nav-item">
					<a
						href={`/erwischt/game/${currentGameId}/game.pdf`}
						target="_blank"
						class="mx-sm-3"
					>
						Spiel herunterladen
					</a>
				</li>
				<li class="nav-item">
					<a
						href={`/erwischt/game/${currentGameId}/members.pdf`}
						target="_blank"
						class="mx-sm-3"
					>
						Teilnehmer herunterladen
					</a>
				</li>
				<li class="nav-item">
					<!-- svelte-ignore a11y-invalid-attribute -->
					<a href="#" on:click={deleteGame} class="mx-sm-3">Spiel löschen</a>
				</li>
			</ul>
		</div>
	</div>
</div>

<table class="table">
	<thead class="thead-light">
		<tr>
			<th scope="col">Name</th>
			<th scope="col">
				Nächstes Ziel
				<label>
					anzeigen <input type="checkbox" bind:checked={showTarget} />
				</label>
			</th>
			<th scope="col" />
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
							<!-- svelte-ignore a11y-invalid-attribute -->
							<a href="#" on:click={() => insertNewMember(m.nextTarget.id)}>
								einfügen
							</a>
						{:else}
							<!-- svelte-ignore a11y-invalid-attribute -->
							<a href="#" on:click={() => catchMember(m.id, m.nextTarget.id)}>
								erwischt
							</a>
						{/if}
					</td>
				</tr>
			{/each}
		{/if}
	</tbody>
</table>

<h3>Schon erwischt</h3>

<table class="table">
	<thead class="thead-light">
		<tr>
			<th scope="col">Name</th>
			<th scope="col">Von</th>
			<th scope="col">Zeit</th>
			<th scope="col" />
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
						<!-- svelte-ignore a11y-invalid-attribute -->
						<a href="#" on:click={() => catchMember(null, m.id)}>wiederbeleben</a>
					</td>
				</tr>
			{/each}
		{/if}
	</tbody>
</table>

<h3>Letzte Änderungen</h3>
<ul style="list-style-type: none; padding-left: 0;">
	{#if currentGame !== undefined}
		{#each lastActions as a}
			{#if findMember(a.member) !== undefined}
				<li>
					<span>
						{#if a.catcher !== null}
							{#if findMember(a.catcher) !== undefined}
								{findMember(a.catcher).name} → {findMember(a.member).name}
							{/if}
						{:else}
							{findMember(a.member).name} wiederbelebt
						{/if}
					</span>
					<!-- svelte-ignore a11y-invalid-attribute -->
					<a
						class="alignRight"
						href="#"
						on:click={() => catchMember(a.lastCatcher, a.member)}
					>
						rückgängig
					</a>
				</li>
			{/if}
		{/each}
	{/if}
</ul>

<style>
	.alignRight {
		text-align: right;
	}

	th label {
		font-weight: normal;
	}
</style>
