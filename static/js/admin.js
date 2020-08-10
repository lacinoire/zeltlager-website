var allMembers;
var allSupervisors;
var sorting = ["alphabetical", "region"].includes(localStorage.adminMemberSorting) ?
	localStorage.adminMemberSorting : "alphabetical";

async function loadMembers() {
	var response;
	try {
		response = await fetch("/admin/teilnehmer.json");
		if (!response.ok) {
			alert("Fehler: Teilnehmer konnten nicht geladen werden (Server-Fehler)");
			return;
		}
	} catch(e) {
		console.error("Failed to fetch members", e);
		alert("Fehler: Teilnehmer konnten nicht geladen werden (Server nicht erreichbar)");
		return;
	}
	try {
		allMembers = await response.json();
	} catch(e) {
		console.error("Failed to parse members json", e);
		alert("Fehler: Teilnehmer konnten nicht geladen werden (unlesbar)");
		return;
	}

	showMembers();
}

async function loadSupervisors() {
	var response;
	try {
		response = await fetch("/admin/betreuer.json");
		if (!response.ok) {
			alert("Fehler: Betreuer konnten nicht geladen werden (Server-Fehler)");
			return;
		}
	} catch(e) {
		console.error("Failed to fetch supervisor", e);
		alert("Fehler: Betreuer konnten nicht geladen werden (Server nicht erreichbar)");
		return;
	}
	try {
		allSupervisors = await response.json();
	} catch(e) {
		console.error("Failed to parse supervisor json", e);
		alert("Fehler: Betreuer konnten nicht geladen werden (unlesbar)");
		return;
	}

	showSupervisors();
}

async function removeMember(id) {
	var member;
	for (var m of allMembers) {
		if (m.id === id) {
			member = m;
			break;
		}
	}
	if (!window.confirm(`${member.vorname} ${member.nachname} löschen?`))
		return;

	var response;
	try {
		const data = {
			member: id,
		};

		response = await fetch("/admin/teilnehmer/remove", {
			method: "POST",
			headers: {
				'Content-Type': 'application/json',
			},
			body: JSON.stringify(data),
		});
		if (!response.ok) {
			alert("Fehler: Teilnehmer konnte nicht gelöscht werden (Server-Fehler)");
			return;
		}
	} catch(e) {
		console.error("Failed to delete member", e);
		alert("Fehler: Teilnehmer konnte nicht gelöscht werden");
		return;
	}

	loadMembers();
}

async function editMember(data) {
	var response;
	try {
		response = await fetch("/admin/teilnehmer/edit", {
			method: "POST",
			headers: {
				'Content-Type': 'application/json',
			},
			body: JSON.stringify(data),
		});
		if (!response.ok) {
			alert("Fehler: Teilnehmer konnte nicht bearbeitet werden (Server-Fehler)");
			showMembers(); // Refresh list with unedited properties
			return;
		}
	} catch(e) {
		console.error("Failed to edit member", e);
		alert("Fehler: Teilnehmer konnte nicht bearbeitet werden");
		showMembers();
		return;
	}

	for (var m of allMembers) {
		if (m.id === data.member) {
			m.bezahlt = data.bezahlt;
			m.anwesend = data.anwesend;
			break;
		}
	}
}

async function editSupervisor(data) {
	var response;
	try {
		response = await fetch("/admin/betreuer/edit", {
			method: "POST",
			headers: {
				'Content-Type': 'application/json',
			},
			body: JSON.stringify(data),
		});
		if (!response.ok) {
			alert("Fehler: Betreuer konnte nicht bearbeitet werden (Server-Fehler)");
			showSupervisors(); // Refresh list with unedited properties
			return;
		}
	} catch(e) {
		console.error("Failed to edit supervisor", e);
		alert("Fehler: Betreuer konnte nicht bearbeitet werden");
		showSupervisors();
		return;
	}
	loadSupervisors();
}

function getRegion(plz, ort) {
	if (inMunich(plz, ort))
		return "München";
	if (inMunichLandkreis(plz, ort))
		return "Landkreis München";
	return "";
}

function inMunich(plz, _ort) {
	return plz >= 80331 && plz <= 81929;
}

function inMunichLandkreis(plz, ort) {
	// Complicated, we need to check plz and ort
	// E.g. plz 82131 can be either in Neuried (im Landkreis München) or
	// Gauting (nicht im Landkreis München).
	const places = [
		[82008, 'Unterhaching'],
		[82024, 'Taufkirchen'],
		[82031, 'Grünwald'],
		[82064, 'Grünwald'],
		[82041, 'Oberhaching'],
		[82064, 'Oberhaching'],
		[82049, 'Pullach'],
		[82054, 'Sauerlach'],
		[82061, 'Neuried'],
		[82064, 'Straßlach-Dingharting'],
		[82065, 'Baierbrunn'],
		[82067, 'Schäftlarn'],
		[82069, 'Schäftlarn'],
		[82152, 'Planegg'],
		[82166, 'Gräfelfing'],
		[85521, 'Hohenbrunn'],
		[85662, 'Hohenbrunn'],
		[85521, 'Ottobrunn'],
		[85540, 'Haar'],
		[85551, 'Kirchheim'],
		[85579, 'Neubiberg'],
		[85609, 'Aschheim'],
		[85622, 'Feldkirchen'],
		[85630, 'Grasbrunn'],
		[85635, 'Höhenkirchen-Siegertsbrunn'],
		[85640, 'Putzbrunn'],
		[85649, 'Brunnthal'],
		[85653, 'Aying'],
		[85716, 'Unterschleißheim'],
		[85737, 'Ismaning'],
		[85748, 'Garching'],
		[85764, 'Oberschleißheim'],
		[85774, 'Unterföhring'],
	];

	for (var p of places) {
		if (plz == p[0] && ort.toLowerCase().includes(p[1].toLowerCase()))
			return true;
	}
	return false;
}

function createCsv(data, member) {
	var res = "";
	for (var line of data) {
		var first = true;
		for (var field of line) {
			if (first)
				first = false;
			else
				res += ",";
			if (field !== null) {
				if (field.includes(",") || field.includes("\n") || field.includes('"')) {
					res += '"' + field.replace('"', '""') + '"';
				} else {
					res += field;
				}
			}
		}
		res += "\n";
	}

	createDownload(res, member ? "teilnehmer.csv" : "betreuer.csv", "text/csv");
}

function createXlsx(data, member) {
	if (member) {
		for (var i = 1; i < data.length; i++) {
			let row = data[i];
			if (row[5] !== "") {
				row[5] = { t: "d", v: row[5], z: "dd.mm.yyyy" };
				row[17] = { t: "d", v: row[17], z: "dd.mm.yy hh:mm" };
			}
		}
	} else {
		for (var i = 1; i < data.length; i++) {
			let row = data[i];
			if (row[3] !== "")
				row[3] = { t: "d", v: row[3], z: "dd.mm.yyyy" };
			if (row[11] !== "" && row[11])
				row[11] = { t: "d", v: row[11], z: "dd.mm.yyyy" };
			if (row[12] !== "" && row[12])
				row[12] = { t: "d", v: row[12], z: "dd.mm.yyyy" };
			if (row[14] !== "")
				row[14] = { t: "d", v: row[14], z: "dd.mm.yy hh:mm" };
		}
	}
	let sheet = XLSX.utils.aoa_to_sheet(data);
	sheet["!cols"] = Array(18).fill({});
	if (member) {
		sheet["!cols"][5] = { wch: 10 };
		sheet["!cols"][17] = { wch: 14 };
	} else {
		sheet["!cols"][3] = { wch: 10 };
		sheet["!cols"][11] = { wch: 10 };
		sheet["!cols"][12] = { wch: 10 };
		sheet["!cols"][14] = { wch: 14 };
	}
	const workbook = XLSX.utils.book_new();
	XLSX.utils.book_append_sheet(workbook, sheet, member ? "Teilnehmer" : "Betreuer");
	XLSX.writeFile(workbook, member ? "teilnehmer.xlsx" : "betreuer.xlsx");
}

function createDownload(content, name, type) {
	var blob = new Blob([content], { type: type });
	var link = window.document.createElement("a");
	link.href = window.URL.createObjectURL(blob);
	link.download = name;
	document.body.appendChild(link);
	link.click();
	document.body.removeChild(link);
}

function boolToStr(b) {
	return b === true ? "ja" : "nein";
}

function createMemberData(asDate = false) {
	let members = [];
	for (var m of allMembers)
		members.push(m);
	if (sorting === "alphabetical") {
		members.sort(nameSortFn);
	} else if (sorting === "region") {
		members.sort(regionSortFn);
	} else {
		console.error("Unknown sorting type '" + sorting + "'");
	}

	let data = [["Anwesend", "Bezahlt", "Vorname", "Nachname", "Geschlecht", "Geburtsdatum",
		"Schwimmer", "Vegetarier", "Tetanus-Impfung", "Eltern", "E-Mail", "Handynummer",
		"Straße", "Hausnummer", "Ort", "PLZ", "Besonderheiten", "Anmeldedatum"]];
	var lastRegion = undefined;
	for (var m of members) {
		const curRegion = getRegion(m.plz, m.ort);
		if (sorting === "region" && lastRegion !== undefined && curRegion !== lastRegion) {
			// Empty row between Munich/Landkreis and Landkreis/rest
			data.push(Array(data[0].length).fill(""));
		}

		lastRegion = curRegion;
		const geburtsdatum = moment.utc(m.geburtsdatum).local();
		const anmeldedatum = moment.utc(m.anmeldedatum).local();
		data.push([boolToStr(m.anwesend), boolToStr(m.bezahlt), m.vorname, m.nachname,
			m.geschlecht === "Male" ? "m" : "w", asDate ? geburtsdatum.toDate() : geburtsdatum.format("DD.MM.YYYY"),
			boolToStr(m.schwimmer), boolToStr(m.vegetarier), boolToStr(m.tetanus_impfung),
			m.eltern_name, m.eltern_mail, m.eltern_handynummer, m.strasse, m.hausnummer,
			m.ort, m.plz, m.besonderheiten, asDate ? anmeldedatum.toDate() : anmeldedatum.format("DD.MM.YY HH:mm")]);
	}

	return data;
}

function createSupervisorData(asDate = false) {
	let members = [];
	for (var m of allSupervisors)
		members.push(m);
	members.sort(nameSortFn);

	let data = [["Vorname", "Nachname", "Geschlecht", "Geburtsdatum", "JuLeiCa",
		"E-Mail", "Handynummer", "Straße", "Hausnummer", "Ort", "PLZ",
		"Führungszeugnis Ausstellung", "Führungszeugnis Eingesehen", "Besonderheiten",
		"Anmeldedatum"]];
	for (var m of members) {
		const geburtsdatum = moment.utc(m.geburtsdatum).local();
		const anmeldedatum = moment.utc(m.anmeldedatum).local();
		let fuehrungszeugnis_ausstellung = moment.utc(m.fuehrungszeugnis_auststellung).local();
		let fuehrungszeugnis_eingesehen = moment.utc(m.fuehrungszeugnis_eingesehen).local();
		fuehrungszeugnis_ausstellung = m.fuehrungszeugnis_auststellung ? (asDate ? fuehrungszeugnis_ausstellung.toDate() : fuehrungszeugnis_ausstellung.format("DD.MM.YYYY")) : m.fuehrungszeugnis_auststellung;
		fuehrungszeugnis_eingesehen = m.fuehrungszeugnis_eingesehen ? (asDate ? fuehrungszeugnis_eingesehen.toDate() : fuehrungszeugnis_eingesehen.format("DD.MM.YYYY")) : m.fuehrungszeugnis_eingesehen;
		data.push([m.vorname, m.nachname, m.geschlecht === "Male" ? "m" : "w",
			asDate ? geburtsdatum.toDate() : geburtsdatum.format("DD.MM.YYYY"),
			m.juleica_nummer, m.mail, m.handynummer, m.strasse, m.hausnummer,
			m.ort, m.plz, fuehrungszeugnis_ausstellung, fuehrungszeugnis_eingesehen,
			m.besonderheiten, asDate ? anmeldedatum.toDate() : anmeldedatum.format("DD.MM.YY HH:mm")]);
	}

	return data;
}

function nameSortFn(a, b) {
	const vorCmp = a.vorname.localeCompare(b.vorname);
	if (vorCmp === 0)
		return a.nachname.localeCompare(b.nachname);
	return vorCmp;
}

function regionSortFn(a, b) {
	const aInMunich = inMunich(a.plz, a.ort);
	const bInMunich = inMunich(b.plz, b.ort);
	if (aInMunich != bInMunich) {
		return aInMunich ? -1 : 1;
	}

	const aInMunichLandkreis = inMunichLandkreis(a.plz, a.ort);
	const bInMunichLandkreis = inMunichLandkreis(b.plz, b.ort);
	if (aInMunichLandkreis != bInMunichLandkreis) {
		return aInMunichLandkreis ? -1 : 1;
	}

	return nameSortFn(a, b);
}

/// Show the filtered members
function showMembers() {
	$("#memberTableBody, #birthdayTableBody").children().remove();
	const filter = $("#memberFilter").val().toLowerCase();

	var members = [];
	var birthdays = [];

	const start = moment.utc("1970-07-25").local();
	const end = moment.utc("1970-08-20").local();
	for (var m of allMembers) {
		if (filter.length === 0 || m.vorname.toLowerCase().includes(filter) || m.nachname.toLowerCase().includes(filter)) {
			members.push(m);
			const birthday = moment.utc(m.geburtsdatum).local();
			birthday.year(1970);
			// Potential birthays during the camp
			if (birthday > start && birthday < end) {
				birthdays.push(m);
			}
		}
	}

	if (sorting === "alphabetical") {
		members.sort(nameSortFn);
		birthdays.sort(nameSortFn);
	} else if (sorting === "region") {
		members.sort(regionSortFn);
		birthdays.sort(regionSortFn);
	} else {
		console.error("Unknown sorting type '" + sorting + "'");
	}

	var list = document.getElementById("memberTableBody");
	var lastRegion = undefined;
	for (var m of members) {
		const row = document.createElement("tr");
		var cell;
		const id = m.id;
		const bezahlt = m.bezahlt;
		const anwesend = m.anwesend;

		const curRegion = getRegion(m.plz, m.ort);
		if (sorting === "region" && lastRegion !== undefined && curRegion !== lastRegion) {
			// Empty row between Munich/Landkreis and Landkreis/rest
			const emptyRow = document.createElement("tr");
			for (var i = 0; i < 17; i++)
				emptyRow.appendChild(document.createElement("td"));
			list.appendChild(emptyRow);
		}

		lastRegion = curRegion;

		cell = document.createElement("td");
		var checkbox = document.createElement("input");
		checkbox.type = "checkbox";
		checkbox.checked = m.anwesend === true;
		checkbox.oninput = function() {
			editMember({
				member: id,
				bezahlt: bezahlt,
				anwesend: !anwesend,
			})
		};
		cell.appendChild(checkbox);
		row.appendChild(cell);

		cell = document.createElement("td");
		var checkbox = document.createElement("input");
		checkbox.type = "checkbox";
		checkbox.checked = m.bezahlt === true;
		checkbox.oninput = function() {
			editMember({
				member: id,
				bezahlt: !bezahlt,
				anwesend: anwesend,
			})
		};
		cell.appendChild(checkbox);
		row.appendChild(cell);

		cell = document.createElement("td");
		cell.innerText = m.vorname + " " + m.nachname;
		row.appendChild(cell);

		cell = document.createElement("td");
		cell.innerText = m.geschlecht === "Male" ? "m" : "w";
		row.appendChild(cell);

		cell = document.createElement("td");
		cell.innerText = moment.utc(m.geburtsdatum).local().format("DD.MM.YYYY");
		row.appendChild(cell);

		for (let c of [m.schwimmer, m.vegetarier, m.tetanus_impfung]) {
			cell = document.createElement("td");
			cell.innerHTML = '<input type="checkbox"' + (c ? " checked" : "") + ' disabled>';
			row.appendChild(cell);
		}

		for (let c of [m.eltern_name, m.eltern_mail, m.eltern_handynummer, m.strasse + " " + m.hausnummer,
			m.ort, m.plz, m.besonderheiten]) {
			cell = document.createElement("td");
			cell.innerText = c;
			row.appendChild(cell);
		}

		cell = document.createElement("td");
		cell.innerText = moment.utc(m.anmeldedatum).local().format("DD.MM.YY HH:mm");
		row.appendChild(cell);

		cell = document.createElement("td");
		var link = document.createElement("a");
		link.href = `javascript:removeMember(${m.id})`;
		link.innerText = "löschen";
		cell.style.textAlign = "right";
		cell.appendChild(link);
		row.appendChild(cell);

		list.appendChild(row);
	}

	var list = document.getElementById("birthdayTableBody");
	for (var m of birthdays) {
		const row = document.createElement("tr");
		var cell;

		cell = document.createElement("td");
		cell.innerText = m.vorname + " " + m.nachname;
		row.appendChild(cell);

		cell = document.createElement("td");
		cell.innerText = m.geschlecht === "Male" ? "m" : "w";
		row.appendChild(cell);

		cell = document.createElement("td");
		cell.innerText = moment.utc(m.geburtsdatum).local().format("DD.MM.YYYY");
		row.appendChild(cell);

		list.appendChild(row);
	}
}

/// Show the filtered supervisors
function showSupervisors() {
	$("#supervisorTableBody").children().remove();
	const filter = $("#supervisorFilter").val().toLowerCase();

	var members = [];

	for (var m of allSupervisors) {
		if (filter.length === 0 || m.vorname.toLowerCase().includes(filter) || m.nachname.toLowerCase().includes(filter)) {
			members.push(m);
		}
	}

	members.sort(nameSortFn);

	var list = document.getElementById("supervisorTableBody");
	for (var m of members) {
		const row = document.createElement("tr");
		let cell;
		const id = m.id;
		const juleica_nummer = m.juleica_nummer;
		const fuehrungszeugnis_ausstellung = m.fuehrungszeugnis_auststellung;
		const fuehrungszeugnis_eingesehen = m.fuehrungszeugnis_eingesehen;

		cell = document.createElement("td");
		cell.innerText = m.vorname + " " + m.nachname;
		row.appendChild(cell);

		cell = document.createElement("td");
		cell.innerText = m.geschlecht === "Male" ? "m" : "w";
		row.appendChild(cell);

		cell = document.createElement("td");
		cell.innerText = moment.utc(m.geburtsdatum).local().format("DD.MM.YYYY");
		row.appendChild(cell);

		let juleica_cell = document.createElement("td");
		juleica_cell.innerText = m.juleica_nummer;
		let juleica_nummer_editing = false;
		juleica_cell.onclick = (_event) => {
			if (!juleica_nummer_editing) {
				juleica_nummer_editing = true;
				juleica_cell.innerText = "";
				const form = document.createElement("form");
				const input = document.createElement("input");
				input.value = juleica_nummer;
				form.appendChild(input);
				const button = document.createElement("input");
				button.type = "submit";
				button.value = "💾";
				form.appendChild(button);
				form.onsubmit = (e) => {
					e.preventDefault();
					editSupervisor({
						supervisor: id,
						juleica_nummer: input.value,
						fuehrungszeugnis_ausstellung: fuehrungszeugnis_ausstellung,
						fuehrungszeugnis_eingesehen: fuehrungszeugnis_eingesehen,
					})
				};
				juleica_cell.appendChild(form);
			}
		};
		row.appendChild(juleica_cell);

		for (var c of [m.mail, m.handynummer, m.strasse + " " + m.hausnummer, m.ort, m.plz]) {
			cell = document.createElement("td");
			cell.innerText = c;
			row.appendChild(cell);
		}

		let fuehrungszeugnis_ausstellung_cell = document.createElement("td");
		if (m.fuehrungszeugnis_auststellung)
			fuehrungszeugnis_ausstellung_cell.innerText = moment.utc(m.fuehrungszeugnis_auststellung).format("DD.MM.YYYY");
		let fuehrungszeugnis_ausstellung_editing = false;
		fuehrungszeugnis_ausstellung_cell.onclick = (_event) => {
			if (!fuehrungszeugnis_ausstellung_editing) {
				fuehrungszeugnis_ausstellung_editing = true;
				fuehrungszeugnis_ausstellung_cell.innerText = "";
				const form = document.createElement("form");
				const input = document.createElement("input");
				if (fuehrungszeugnis_ausstellung)
					input.value = moment.utc(fuehrungszeugnis_ausstellung).format("DD.MM.YYYY");
				form.appendChild(input);
				const button = document.createElement("input");
				button.type = "submit";
				button.value = "💾";
				form.appendChild(button);
				form.onsubmit = (e) => {
					e.preventDefault();
					editSupervisor({
						supervisor: id,
						juleica_nummer: juleica_nummer,
						fuehrungszeugnis_ausstellung: input.value.length > 0 ? moment(input.value, "DD.MM.YYYY").format("YYYY-MM-DD") : null,
						fuehrungszeugnis_eingesehen: fuehrungszeugnis_eingesehen,
					})
				};
				fuehrungszeugnis_ausstellung_cell.appendChild(form);
			}
		};
		row.appendChild(fuehrungszeugnis_ausstellung_cell);

		let fuehrungszeugnis_eingesehen_cell = document.createElement("td");
		if (m.fuehrungszeugnis_eingesehen)
			fuehrungszeugnis_eingesehen_cell.innerText = moment.utc(m.fuehrungszeugnis_eingesehen).format("DD.MM.YYYY");
		let fuehrungszeugnis_eingesehen_editing = false;
		fuehrungszeugnis_eingesehen_cell.onclick = (_event) => {
			if (!fuehrungszeugnis_eingesehen_editing) {
				fuehrungszeugnis_eingesehen_editing = true;
				fuehrungszeugnis_eingesehen_cell.innerText = "";
				const form = document.createElement("form");
				const input = document.createElement("input");
				if (fuehrungszeugnis_eingesehen)
					input.value = moment.utc(fuehrungszeugnis_eingesehen).format("DD.MM.YYYY");
				form.appendChild(input);
				const button = document.createElement("input");
				button.type = "submit";
				button.value = "💾";
				form.appendChild(button);
				form.onsubmit = (e) => {
					e.preventDefault();
					editSupervisor({
						supervisor: id,
						juleica_nummer: juleica_nummer,
						fuehrungszeugnis_ausstellung: fuehrungszeugnis_ausstellung,
						fuehrungszeugnis_eingesehen: input.value.length > 0 ? moment(input.value, "DD.MM.YYYY").format("YYYY-MM-DD") : null,
					})
				};
				fuehrungszeugnis_eingesehen_cell.appendChild(form);
			}
		};
		row.appendChild(fuehrungszeugnis_eingesehen_cell);

		cell = document.createElement("td");
		cell.innerText = m.besonderheiten;
		row.appendChild(cell);

		cell = document.createElement("td");
		cell.innerText = moment.utc(m.anmeldedatum).local().format("DD.MM.YY HH:mm");
		row.appendChild(cell);

		list.appendChild(row);
	}
}

window.addEventListener('load', function() {
	var isMembers = document.getElementById("memberTableBody") !== null;
	if (isMembers) {
		$("#sortSelect :input").change(function() {
			sorting = this.dataset.sort;
			localStorage.adminMemberSorting = sorting;
			showMembers();
		});
		if (sorting !== "alphabetical") {
			$("#sortSelect label").toggleClass('active');
		}
		loadMembers();
	} else {
		loadSupervisors();
	}
});
