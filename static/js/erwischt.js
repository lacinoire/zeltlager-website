var games;
var currentGame;
var currentGameId;
var showTarget = localStorage.erwischtShowTarget === "true";
var insertMember = false;

var lastActions = localStorage.erwischtLastActions ?
	JSON.parse(localStorage.erwischtLastActions) : [];
const historyLength = 5;

function findMember(id) {
	if (id === undefined)
		return undefined;
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

async function insertNewMember(before) {
	const beforeName = findMember(before).name;
	const name = prompt(`Vor ${beforeName} einfügen. Name:`);
	if (name === null || name === "undefined")
		return;

	await insertNewMemberName(before, name);
}

async function insertNewMemberName(before, name) {
	try {
		const data = {
			game: currentGameId,
			before: before,
			name: name,
		};

		response = await fetch("/erwischt/game/insert", {
			method: "POST",
			headers: {
				'Content-Type': 'application/json',
			},
			body: JSON.stringify(data),
		});
		if (!response.ok) {
			alert("Fehler: Spieler konnte nicht eingefügt werden (Server-Fehler)");
			return;
		}
	} catch(e) {
		console.error("Failed to insert member", e);
		alert("Fehler: Spieler konnte nicht eingefügt werden (Server nicht erreichbar)");
		return;
	}
	await loadGame(currentGameId);
}

async function catchMember(catcher, member) {
	console.log(`${catcher} catched ${member}`);

	const m = findMember(member);
	try {
		const data = {
			game: currentGameId,
			catcher: catcher,
			member: member,
		};
		lastActions = localStorage.erwischtLastActions ?
			JSON.parse(localStorage.erwischtLastActions) : [];
		lastActions.push({ ...data, lastCatcher: m.catcher });
		if (lastActions.length > historyLength)
			lastActions = lastActions.slice(lastActions.length - historyLength);
		localStorage.erwischtLastActions = JSON.stringify(lastActions);
		showHistory();

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
	showMembers();
}

function changeShowTarget(show) {
	showTarget = show;
	localStorage.erwischtShowTarget = showTarget ? "true" : "false";
	showMembers();
}

function changeInsertMember(insert) {
	insertMember = insert;
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
	showHistory();

	console.log("Loaded game", currentGameId);
}

/// Show the filtered members
function showMembers() {
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
		if (insertMember) {
			link.href = `javascript:insertNewMember(${m.nextTarget.id})`;
			link.innerText = "einfügen";
		} else {
			link.href = `javascript:catchMember(${m.id}, ${m.nextTarget.id})`;
			link.innerText = "erwischt";
		}
		cell.style.textAlign = "right";
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
		cell.style.textAlign = "right";
		cell.appendChild(link);
		row.appendChild(cell);

		list.appendChild(row);
	}
}

function showHistory() {
	$("#undoList").children().remove();
	var list = document.getElementById("undoList");
	for (var a of lastActions) {
		if (a.game === currentGameId) {
			var item = document.createElement("li");
			var text = document.createElement("span");
			if (a.catcher !== null) {
				text.innerText = `${findMember(a.catcher).name} → ${findMember(a.member).name} `;
			} else {
				text.innerText = `${findMember(a.member).name} wiederbelebt `;
			}
			item.appendChild(text);
			var link = document.createElement("a");
			link.href = `javascript:catchMember(${a.lastCatcher}, ${a.member})`;
			link.innerText = "rückgängig";
			link.style.textAlign = "right";
			item.appendChild(link);
			list.appendChild(item);
		}
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
	$("#addToggle :input").change(function() {
		changeInsertMember(this.dataset.show === "true");
	});
	if (showTarget) {
		$("#goalToggle label").toggleClass('active');
	}
	loadGames();
});
