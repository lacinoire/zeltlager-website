import { utils, writeFile } from "xlsx";

export const YEAR = 2023;

export interface Column {
	name?: string;
	displayName?: string;
}

export interface FormError {
	field?: string;
	message: string;
}

export function getRegion(plz: number, ort: string) {
	if (inMunich(plz, ort)) return "München";
	if (inMunichLandkreis(plz, ort)) return "Landkreis München";
	return "Außerhalb";
}

export function inMunich(plz: number, _ort: string) {
	return plz >= 80331 && plz <= 81929;
}

export function inMunichLandkreis(plz: number, ort: string) {
	// Complicated, we need to check plz and ort
	// E.g. plz 82131 can be either in Neuried (im Landkreis München) or
	// Gauting (nicht im Landkreis München).
	const places: [number, string][] = [
		[82008, "Unterhaching"],
		[82024, "Taufkirchen"],
		[82031, "Grünwald"],
		[82064, "Grünwald"],
		[82041, "Oberhaching"],
		[82064, "Oberhaching"],
		[82049, "Pullach"],
		[82054, "Sauerlach"],
		[82061, "Neuried"],
		[82064, "Straßlach-Dingharting"],
		[82065, "Baierbrunn"],
		[82067, "Schäftlarn"],
		[82069, "Schäftlarn"],
		[82152, "Planegg"],
		[82166, "Gräfelfing"],
		[85521, "Hohenbrunn"],
		[85662, "Hohenbrunn"],
		[85521, "Ottobrunn"],
		[85540, "Haar"],
		[85551, "Kirchheim"],
		[85579, "Neubiberg"],
		[85609, "Aschheim"],
		[85622, "Feldkirchen"],
		[85630, "Grasbrunn"],
		[85635, "Höhenkirchen-Siegertsbrunn"],
		[85640, "Putzbrunn"],
		[85649, "Brunnthal"],
		[85653, "Aying"],
		[85716, "Unterschleißheim"],
		[85737, "Ismaning"],
		[85748, "Garching"],
		[85764, "Oberschleißheim"],
		[85774, "Unterföhring"],
	];

	for (const p of places) {
		if (plz == p[0] && ort.toLowerCase().includes(p[1].toLowerCase())) return true;
	}
	return false;
}

// Infer Ort from PLZ
export function inferPlace(form: HTMLFormElement) {
	if (form.ort.value !== "") return;
	if (form.land.value === "Deutschland" && inMunich(parseInt(form.plz.value), ""))
		form.ort.value = "München";
}

export function createCsv(data: any[], member: boolean) {
	let res = "";
	for (const line of data) {
		let first = true;
		for (let field of line) {
			if (first) first = false;
			else res += ",";
			if (field !== null) {
				if (typeof field === "boolean") field = boolToStr(field);
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

export function createXlsx(data: any[], member: boolean) {
	for (let i = 1; i < data.length; i++) {
		const row = data[i];
		for (let j = 0; j < row.length; j++) {
			const name = data[0][j];
			if (
				name === "Geburtsdatum" ||
				name === "Führungszeugnis Ausstellung" ||
				name === "Führungszeugnis Eingesehen"
			) {
				if (row[j] !== "" && row[j]) row[j] = { t: "d", v: row[j], z: "dd.mm.yyyy" };
			}
			if (name === "Anmeldedatum") row[j] = { t: "d", v: row[j], z: "dd.mm.yy hh:mm" };
			if (typeof row[j] === "boolean") row[j] = boolToStr(row[j]);
		}
	}
	const sheet = utils.aoa_to_sheet(data);
	sheet["!cols"] = Array(18).fill({});
	for (let j = 0; j < data[0].length; j++) {
		const name = data[0][j];
		if (
			name === "Geburtsdatum" ||
			name === "Führungszeugnis Ausstellung" ||
			name === "Führungszeugnis Eingesehen"
		)
			sheet["!cols"][j] = { wch: 10 };
		if (name === "Anmeldedatum") sheet["!cols"][j] = { wch: 14 };
	}
	const workbook = utils.book_new();
	utils.book_append_sheet(workbook, sheet, member ? "Teilnehmer" : "Betreuer");
	writeFile(workbook, member ? "teilnehmer.xlsx" : "betreuer.xlsx");
}

export function createDownload(content: BlobPart, name: string, type: string) {
	const blob = new Blob([content], { type: type });
	const link = window.document.createElement("a");
	link.href = window.URL.createObjectURL(blob);
	link.download = name;
	document.body.appendChild(link);
	link.click();
	document.body.removeChild(link);
}

export function boolToStr(b: boolean): string {
	return b === true ? "ja" : "nein";
}

export function getSortByKeyFn(sortBy: string) {
	const asc = sortBy.endsWith("asc");
	const sortName = sortBy.slice(0, sortBy.lastIndexOf("-"));
	const sortKey = sortName
		.toLowerCase()
		.replaceAll(" ", "_")
		.replaceAll("-", "_")
		.replaceAll("ä", "ae")
		.replaceAll("ö", "oe")
		.replaceAll("ü", "ue");
	console.log("Sorting by key " + sortKey);
	return (aRow: any, bRow: any) => {
		const a = aRow[sortKey];
		const b = bRow[sortKey];
		let cmp;
		if ((a === undefined && b === undefined) || (a === null && b === null)) cmp = 0;
		else if (b === undefined || b === null) cmp = -1;
		else if (a === undefined || a === null) cmp = 1;
		else if (typeof a === "boolean") cmp = a === b ? 0 : a ? 1 : -1;
		else if (typeof a === "object")
			// Moment
			cmp = a === b ? 0 : a > b ? 1 : -1;
		else cmp = a.localeCompare(b);
		return asc ? cmp : -cmp;
	};
}

interface HasAddress {
	strasse: string;
	hausnummer: string;
}

export function addressSortFn(a: HasAddress, b: HasAddress) {
	const strCmp = a.strasse.localeCompare(b.strasse);
	if (strCmp === 0) return a.hausnummer.localeCompare(b.hausnummer);
	return strCmp;
}

interface HasName {
	vorname: string;
	nachname: string;
}

export function nameSortFn(a: HasName, b: HasName) {
	const vorCmp = a.vorname.localeCompare(b.vorname);
	if (vorCmp === 0) return a.nachname.localeCompare(b.nachname);
	return vorCmp;
}

interface HasRegion {
	plz: string;
	ort: string;
}

export function regionSortFn(a: HasRegion, b: HasRegion) {
	const aInMunich = inMunich(parseInt(a.plz), a.ort);
	const bInMunich = inMunich(parseInt(b.plz), b.ort);
	if (aInMunich != bInMunich) {
		return aInMunich ? -1 : 1;
	}

	const aInMunichLandkreis = inMunichLandkreis(parseInt(a.plz), a.ort);
	const bInMunichLandkreis = inMunichLandkreis(parseInt(b.plz), b.ort);
	if (aInMunichLandkreis != bInMunichLandkreis) {
		return aInMunichLandkreis ? -1 : 1;
	}

	return 0;
}

interface HasProp {
	bezahlt: boolean;
	anwesend: boolean;
}

export function payedSortFn(a: HasProp & HasName, b: HasProp & HasName) {
	if (a.bezahlt != b.bezahlt) {
		return a.bezahlt ? 1 : -1;
	}

	return nameSortFn(a, b);
}

export function presentSortFn(a: HasProp & HasName, b: HasProp & HasName) {
	if (a.anwesend != b.anwesend) {
		return a.anwesend ? 1 : -1;
	}

	return nameSortFn(a, b);
}

export function splitImageName(s: string): string {
	enum CharType {
		Letter,
		Number,
		None,
	}

	let lastType = CharType.None;
	let res = "";
	for (const c of s) {
		const newType = c >= "0" && c <= "9" ? CharType.Number : CharType.Letter;
		if (newType !== lastType && lastType !== CharType.None) res += " ";
		res += c;
		lastType = newType;
	}
	return res;
}

export const COUNTRIES: string[] = [
	"Albanien",
	"Andorra",
	"Belarus",
	"Belgien",
	"Bosnien und Herzegowina",
	"Bulgarien",
	"Dänemark",
	"Deutschland",
	"Estland",
	"Finnland",
	"Frankreich",
	"Griechenland",
	"Irland",
	"Island",
	"Italien",
	"Kasachstan",
	"Kosovo",
	"Kroatien",
	"Lettland",
	"Liechtenstein",
	"Litauen",
	"Luxemburg",
	"Malta",
	"Moldau",
	"Monaco",
	"Montenegro",
	"Niederlande",
	"Nordmazedonien",
	"Norwegen",
	"Österreich",
	"Polen",
	"Portugal",
	"Rumänien",
	"Russland",
	"San Marino",
	"Schweden",
	"Schweiz",
	"Serbien",
	"Slowakei",
	"Slowenien",
	"Spanien",
	"Tschechien",
	"Türkei",
	"Ukraine",
	"Ungarn",
	"Vatikanstadt",
	"Vereinigtes Königreich",
];
