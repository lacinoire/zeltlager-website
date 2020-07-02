var games;
var currentGame;
var currentGameId;
var showTarget = localStorage.erwischtShowTarget === "true";

function findMember(id) {
	var target = currentGame[id];
	if (target.id === id)
		return target;
	return currentGame.find(m => m.id == id);
}

function findNextTarget(member) {
	var target = findMember(member.target);
	while (target.catcher !== null && target !== member) {
		target = findMember(target.target);
	}
	return target;
}

async function catchMember(catcher, member) {
	console.log(`${catcher} catched ${member}`);

	try {
		const data = {
			game: currentGameId,
			catcher: catcher,
			member: member,
		};
		response = await fetch("/erwischt/game/setCatch", {
			method: "POST",
			headers: {
				'Content-Type': 'application/json',
			},
			body: JSON.stringify(data),
		});
		if (!response.ok) {
			alert("Fehler: Spieler konnte nicht erwischt werden (Server-Fehler)");
			return;
		}
	} catch(e) {
		console.error("Failed to create game", e);
		alert("Fehler: Spieler konnte nicht erwischt werden (Server nicht erreichbar)");
		return;
	}

	await loadGame(currentGameId);
}

function changeShowTarget(show) {
	showTarget = show;
	localStorage.erwischtShowTarget = showTarget ? "true" : "false";
	showMembers();
}

async function loadGames() {
	var response;
	try {
		response = await fetch("/erwischt/games");
		if (!response.ok) {
			alert("Fehler: Spiele konnten nicht geladen werden (Server-Fehler)");
			return;
		}
	} catch(e) {
		console.error("Failed to list games", e);
		alert("Fehler: Spiele konnten nicht geladen werden (Server nicht erreichbar)");
		return;
	}
	try {
		games = await response.json();
	} catch(e) {
		console.error("Failed to parse games json", e);
		alert("Fehler: Spiele konnten nicht geladen werden (unlesbar)");
		return;
	}
	console.log("Loaded games");

	// Update tabs
	$(".gameTab").remove();

	const tabs = document.getElementById("gameTabs");
	for (var game of games) {
		const elem = document.createElement("li");
		elem.classList.add("nav-item");
		elem.classList.add("gameTab");
		const link = document.createElement("a");
		link.classList.add("nav-link");
		const created = moment.utc(game.created).local();
		link.innerText = created.format("DD.MM.YYYY");
		link.title = created.format("DD.MM.YYYY HH:mm");
		link.href = `javascript:loadGame(${game.id})`;
		elem.appendChild(link);
		tabs.prepend(elem);
	}

	if (games.length > 0) {
		await loadGame(games[games.length - 1].id);
	}
}

async function loadGame(id) {
	currentGameId = id;

	// Select tab
	$(".gameTab .active").removeClass("active");
	const tabI = games.findIndex(g => g.id === currentGameId);
	$(".gameTab a").eq(games.length - 1 - tabI).addClass("active");

	var response;
	try {
		response = await fetch(`/erwischt/game/${currentGameId}`);
		if (!response.ok) {
			alert("Fehler: Spiel konnten nicht geladen werden (Server-Fehler)");
			return;
		}
	} catch(e) {
		console.error("Failed to fetch game", e);
		alert("Fehler: Spiel konnten nicht geladen werden (Server nicht erreichbar)");
		return;
	}
	try {
		currentGame = await response.json();
	} catch(e) {
		console.error("Failed to parse game json", e);
		alert("Fehler: Spiel konnten nicht geladen werden (unlesbar)");
		return;
	}

	// Compute next targets
	for (var m of currentGame) {
		if (m.catcher === null)
			m.nextTarget = findNextTarget(m);
	}

	showMembers();

	console.log("Loaded game", currentGameId);
}

/// Show the filtered members
function showMembers() {
	console.log("Refresh list");
	$("#memberTableBody, #catchedTableBody").children().remove();
	const filter = $("#memberFilter").val().toLowerCase();

	var members = [];
	for (var m of currentGame) {
		if (m.catcher === null && (filter.length === 0 || m.name.toLowerCase().includes(filter))) {
			members.push(m);
		}
	}
	members.sort((a, b) => a.name.localeCompare(b.name));

	var list = document.getElementById("memberTableBody");
	for (var m of members) {
		const row = document.createElement("tr");

		var cell = document.createElement("td");
		cell.innerText = m.name;
		row.appendChild(cell);

		var cell = document.createElement("td");
		if (showTarget)
			cell.innerText = m.nextTarget.name;
		row.appendChild(cell);

		var cell = document.createElement("td");
		var link = document.createElement("a");
		link.href = `javascript:catchMember(${m.id}, ${m.nextTarget.id})`;
		link.innerText = "erwischt";
		cell.appendChild(link);
		row.appendChild(cell);

		list.appendChild(row);
	}

	members = [];
	for (var m of currentGame) {
		if (m.catcher !== null && (filter.length === 0 || m.name.toLowerCase().includes(filter))) {
			members.push(m);
		}
	}
	members.sort((a, b) => a.name.localeCompare(b.name));

	var list = document.getElementById("catchedTableBody");
	for (var m of members) {
		const row = document.createElement("tr");

		var cell = document.createElement("td");
		cell.innerText = m.name;
		row.appendChild(cell);

		var cell = document.createElement("td");
		cell.innerText = findMember(m.catcher).name;
		row.appendChild(cell);

		var cell = document.createElement("td");
		cell.innerText = moment.utc(m.last_change).local().format("DD.MM. HH:mm");
		row.appendChild(cell);

		var cell = document.createElement("td");
		var link = document.createElement("a");
		link.href = `javascript:catchMember(null, ${m.id})`;
		link.innerText = "wiederbeleben";
		cell.appendChild(link);
		row.appendChild(cell);

		list.appendChild(row);
	}
}

async function newGame() {
	var response;
	try {
		response = await fetch("/erwischt/game", {
			method: "POST",
		});
		if (!response.ok) {
			alert("Fehler: Spiel konnte nicht erstellt werden (Server-Fehler)");
			return;
		}
	} catch(e) {
		console.error("Failed to create game", e);
		alert("Fehler: Spiel konnte nicht erstellt werden (Server nicht erreichbar)");
		return;
	}
	try {
		currentGameId = await response.json();
	} catch(e) {
		console.error("Failed to parse created game json", e);
		alert("Fehler: Spiel konnte nicht erstellt werden (Spiel unlesbar)");
		return;
	}
	console.log("Created game", currentGameId);

	loadGames();
}

function deleteGame() {
	const game = games.find(g => g.id === currentGameId);
	if (window.confirm(`Spiel vom ${moment.utc(game.created).local().format("DD.MM.YYYY HH:mm")} löschen?`))
		realDeleteGame();
}

async function realDeleteGame() {
	var response;
	try {
		response = await fetch(`/erwischt/game/${currentGameId}`, {
			method: "DELETE",
		});
		if (!response.ok) {
			alert("Fehler: Spiel konnte nicht gelöscht werden (Server-Fehler)");
			return;
		}
	} catch(e) {
		console.error("Failed to delete game", e);
		alert("Fehler: Spiel konnte nicht gelöscht werden (Server nicht erreichbar)");
		return;
	}

	await loadGames();
}

function openPdf(name) {
	const win = window.open(`/erwischt/game/${currentGameId}/${name}`, '_blank');
	win.focus();
}

window.addEventListener('load', function() {
	$("#goalToggle :input").change(function() {
		changeShowTarget(this.dataset.show === "true");
	});
	loadGames();
});
