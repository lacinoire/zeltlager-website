export function nameSortFn(a, b) {
	const vorCmp = a.vorname.localeCompare(b.vorname);
	if (vorCmp === 0)
		return a.nachname.localeCompare(b.nachname);
	return vorCmp;
}
