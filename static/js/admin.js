var allMembers;
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

function createCsv() {
	var res = "";
	const data = createData();
	for (var line of data) {
		var first = true;
		for (var field of line) {
			if (first)
				first = false;
			else
				res += ",";
			if (field.includes(",") || field.includes("\n") || field.includes('"')) {
				res += '"' + field.replace('"', '""') + '"';
			} else {
				res += field;
			}
		}
		res += "\n";
	}

	createDownload(res, "teilnehmer.csv", "text/csv");
}

function createXlsx() {
	let data = createData(true);
	for (var i = 1; i < data.length; i++) {
		let row = data[i];
		if (row[5] !== "") {
			row[5] = { t: "d", v: row[5], z: "dd.mm.yyyy" };
			row[17] = { t: "d", v: row[17], z: "dd.mm.yy hh:mm" };
		}
	}
	let sheet = XLSX.utils.aoa_to_sheet(data);
	sheet["!cols"] = Array(18).fill({});
	sheet["!cols"][5] = { wch: 10};
	sheet["!cols"][17] = { wch: 14};
	const workbook = XLSX.utils.book_new();
	XLSX.utils.book_append_sheet(workbook, sheet, "Teilnehmer");
	XLSX.writeFile(workbook, 'teilnehmer.xlsx');
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

function createData(asDate = false) {
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

		cell = document.createElement("td");
		cell.innerHTML = '<input type="checkbox"' + (m.schwimmer === true ? ' checked' : '') + ' disabled>';
		row.appendChild(cell);

		cell = document.createElement("td");
		cell.innerHTML = '<input type="checkbox"' + (m.vegetarier === true ? ' checked' : '') + ' disabled>';
		row.appendChild(cell);

		cell = document.createElement("td");
		cell.innerHTML = '<input type="checkbox"' + (m.tetanus_impfung === true ? ' checked' : '') + ' disabled>';
		row.appendChild(cell);

		cell = document.createElement("td");
		cell.innerText = m.eltern_name;
		row.appendChild(cell);

		cell = document.createElement("td");
		cell.innerText = m.eltern_mail;
		row.appendChild(cell);

		cell = document.createElement("td");
		cell.innerText = m.eltern_handynummer;
		row.appendChild(cell);

		cell = document.createElement("td");
		cell.innerText = m.strasse + " " + m.hausnummer;
		row.appendChild(cell);

		cell = document.createElement("td");
		cell.innerText = m.ort;
		row.appendChild(cell);

		cell = document.createElement("td");
		cell.innerText = m.plz;
		row.appendChild(cell);

		cell = document.createElement("td");
		cell.innerText = m.besonderheiten;
		row.appendChild(cell);

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

window.addEventListener('load', function() {
	$("#sortSelect :input").change(function() {
		sorting = this.dataset.sort;
		localStorage.adminMemberSorting = sorting;
		showMembers();
	});
	if (sorting !== "alphabetical") {
		$("#sortSelect label").toggleClass('active');
	}
	loadMembers();
});
