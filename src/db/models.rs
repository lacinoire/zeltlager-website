use std::collections::HashMap;
use std::fmt;
use std::io::Write;

use chrono::{self, Date, Datelike, NaiveDate, Utc};
use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::Text;

use super::schema::betreuer;
use super::schema::teilnehmer;
use Result;

macro_rules! get_str {
	($map:ident, $key:expr) => {
		$map.remove($key)
			.ok_or_else(|| format_err!("{} fehlt", $key))
	};
}

macro_rules! get_bool {
	($map:ident, $key:expr) => {
		$map.remove($key)
			.ok_or_else(|| format_err!("{} fehlt", $key))
			.and_then(|s| {
				if s == "true" {
					Ok(true)
				} else if s == "false" {
					Ok(false)
				} else {
					Err(format_err!(
						"{} ({}) ist kein Wahrheitswert",
						$key,
						s
					))
					}
				})
	};
}

macro_rules! check_empty {
	($obj:ident, $($field:ident),*) => {
		$(
		if $obj.$field.is_empty() {
			bail!("{} muss ausgefüllt werden", stringify!($field));
		}
		)*
	}
}

pub fn try_parse_date(s: &str) -> Result<NaiveDate> {
	const FORMATS: &[&str] = &["%Y-%m-%d", "%d.%m.%Y"];
	let mut res = None;
	for f in FORMATS {
		if let Ok(date) = NaiveDate::parse_from_str(s, f) {
			res = Some(date);
			break;
		}
	}

	if let Some(mut date) = res {
		if date.year() <= 1900 {
			// Only the last digits of the year where written so correct it.
			// Like 10 for 2010
			let cur_year = Utc::now().year();
			if date.year() <= cur_year % 100 {
				date = NaiveDate::from_ymd(
					date.year() + cur_year / 100 * 100,
					date.month(),
					date.day(),
				);
			} else {
				date = NaiveDate::from_ymd(
					date.year() + cur_year / 100 * 100 - 100,
					date.month(),
					date.day(),
				);
			}
		}
		Ok(date)
	} else {
		bail!(
			"Bitte geben Sie das Geburtsdatum ({}) im Format TT.MM.JJJJ an.",
			s
		);
	}
}

pub fn try_parse_gender(s: &str) -> Result<Gender> {
	const MALE: &[&str] = &[
		"m",
		"M",
		"männlich",
		"Männlich",
		"maennlich",
		"Maennlich",
		"male",
		"Male",
	];
	const FEMALE: &[&str] = &[
		"w", "W", "weiblich", "Weiblich", "female", "Female"
	];

	if MALE.contains(&s) {
		Ok(Gender::Male)
	} else if FEMALE.contains(&s) {
		Ok(Gender::Female)
	} else {
		bail!("{} ist kein bekanntes Geschlecht.", s);
	}
}

pub fn years_old(date: Date<Utc>) -> i32 {
	let now = Utc::now().date();
	let mut years = now.year() - date.year();
	if now.month() < date.month()
		|| (now.month() == date.month() && now.day() < date.day())
	{
		years -= 1;
	}
	years
}

pub fn check_only_numbers(text: &str, length: usize) -> bool {
	text.len() == length && text.chars().all(|c| c.is_numeric())
}

pub fn check_email(text: &str) -> bool {
	text.contains('@') && !text.contains(' ') && text.find('@') == text.rfind('@')
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, FromSqlRow, AsExpression)]
#[sql_type = "Text"]
pub enum Gender {
	Male,
	Female,
}

impl fmt::Display for Gender {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if *self == Gender::Male {
			write!(f, "m")
		} else {
			write!(f, "w")
		}
	}
}

impl ToSql<Text, Pg> for Gender {
	fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
		match *self {
			Gender::Male => out.write_all(b"m")?,
			Gender::Female => out.write_all(b"w")?,
		}
		Ok(IsNull::No)
	}
}

impl FromSql<Text, Pg> for Gender {
	fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
		match not_none!(bytes) {
			b"m" => Ok(Gender::Male),
			b"w" => Ok(Gender::Female),
			_ => Err("Unrecognized enum variant".into()),
		}
	}
}

#[derive(Clone, Debug, Insertable, Queryable)]
#[table_name = "teilnehmer"]
pub struct Teilnehmer {
	pub vorname: String,
	pub nachname: String,
	pub geburtsdatum: chrono::NaiveDate,
	pub geschlecht: Gender,
	pub schwimmer: bool,
	pub vegetarier: bool,
	pub tetanus_impfung: bool,
	pub eltern_name: String,
	pub eltern_mail: String,
	pub eltern_handynummer: String,
	pub strasse: String,
	pub hausnummer: String,
	pub ort: String,
	pub plz: String,
	pub besonderheiten: String,
	pub agb: bool,
}

impl Teilnehmer {
	pub fn from_hashmap(mut map: HashMap<String, String>) -> Result<Self> {
		let date = get_str!(map, "geburtsdatum")?;
		let geburtsdatum = try_parse_date(&date)?;
		let geschlecht = try_parse_gender(&get_str!(map, "geschlecht")?)?;

		let res = Self {
			vorname: get_str!(map, "vorname")?,
			nachname: get_str!(map, "nachname")?,
			geburtsdatum,
			geschlecht,

			schwimmer: get_bool!(map, "schwimmer")?,
			vegetarier: get_bool!(map, "vegetarier")?,
			tetanus_impfung: get_bool!(map, "tetanus_impfung")?,

			eltern_name: get_str!(map, "eltern_name")?,
			eltern_mail: get_str!(map, "eltern_mail")?,
			eltern_handynummer: get_str!(map, "eltern_handynummer")?,
			strasse: get_str!(map, "strasse")?,
			hausnummer: get_str!(map, "hausnummer")?,
			ort: get_str!(map, "ort")?,
			plz: get_str!(map, "plz")?,
			besonderheiten: get_str!(map, "besonderheiten")?,

			agb: get_bool!(map, "agb")?,
		};

		if !res.agb {
			bail!("Die AGB müssen akzeptiert werden");
		}
		check_empty!(
			res,
			vorname,
			nachname,
			eltern_name,
			eltern_mail,
			eltern_handynummer,
			strasse,
			hausnummer,
			ort,
			plz
		);
		// Check PLZ
		if !check_only_numbers(&res.plz, 5) {
			bail!("Ungültige Postleitzahl ({})", res.plz);
		}
		// Check mail address
		if !check_email(&res.eltern_mail) {
			bail!(
				"Ungültige E-Mail Addresse ({})",
				res.eltern_mail
			);
		}
		// Check birth date
		let birthday = Date::from_utc(res.geburtsdatum, Utc);
		let now = Utc::now().date();
		let years = years_old(birthday);
		if now <= birthday || years >= 100 {
			bail!(
				"Sind Sie sicher, dass {} das Geburtsdatum Ihres Kindes \
				 ist?\nBitte geben Sie das Geburtsdatum im Format TT.MM.JJJJ \
				 an.",
				res.geburtsdatum.format("%d.%m.%Y")
			);
		}

		if years < 5 {
			bail!(
				"Ihr Kind ist leider zu jung (Geburtsdatum {}).\nDas \
				 Zeltlager ist für Kinder und Jugendliche zwischen 7 und 15 \
				 Jahren.",
				res.geburtsdatum.format("%d.%m.%Y")
			);
		}
		if years > 15 {
			bail!(
				"Ihr Kind ist leider zu alt um als Teilnehmer beim Zeltlager \
				 mitzufahren (Geburtsdatum {}).\nWir suchen immer nach \
				 motivierten Betreuern (ab 16 Jahren), die auf das Zeltlager \
				 mitfahren.\nInfos dazu finden Sie auf der \
				 Betreuerseite.\nDas Zeltlager ist für Kinder und Jugendliche \
				 zwischen 7 und 15 Jahren.",
				res.geburtsdatum.format("%d.%m.%Y")
			);
		}

		map.remove("submit");
		if !map.is_empty() {
			warn!(
				"Teilnehmer::from_hashmap: Map is not yet empty ({:?})",
				map
			);
		}

		Ok(res)
	}
}

#[derive(Clone, Debug, Insertable, Queryable)]
#[table_name = "betreuer"]
pub struct Betreuer {
	pub vorname: String,
	pub nachname: String,
	pub geburtsdatum: chrono::NaiveDate,
	pub geschlecht: Gender,
	pub vegetarier: bool,
	pub tetanus_impfung: bool,
	pub juleica_nummer: String,
	pub mail: String,
	pub handynummer: String,
	pub strasse: String,
	pub hausnummer: String,
	pub ort: String,
	pub plz: String,
	pub besonderheiten: String,
	pub agb: bool,
	pub selbsterklaerung: bool,
	pub fuehrungszeugnis_auststellung: chrono::NaiveDate,
	pub fuehrungszeugnis_eingesehen: Option<chrono::NaiveDate>,
}

impl Betreuer {
	pub fn from_hashmap(mut map: HashMap<String, String>) -> Result<Self> {
		let date = get_str!(map, "geburtsdatum")?;
		let geburtsdatum = try_parse_date(&date)?;
		let geschlecht = try_parse_gender(&get_str!(map, "geschlecht")?)?;
		let f_date = get_str!(map, "fuehrungszeugnis_auststellung")?;
		let fuehrungszeugnis_auststellung = try_parse_date(&f_date)?;

		let res = Self {
			vorname: get_str!(map, "vorname")?,
			nachname: get_str!(map, "nachname")?,
			geburtsdatum,
			geschlecht,

			vegetarier: get_bool!(map, "vegetarier")?,
			tetanus_impfung: get_bool!(map, "tetanus_impfung")?,

			juleica_nummer: get_str!(map, "juleica_nummer")?,
			mail: get_str!(map, "mail")?,
			handynummer: get_str!(map, "handynummer")?,
			strasse: get_str!(map, "strasse")?,
			hausnummer: get_str!(map, "hausnummer")?,
			ort: get_str!(map, "ort")?,
			plz: get_str!(map, "plz")?,
			besonderheiten: get_str!(map, "besonderheiten")?,
			selbsterklaerung: get_bool!(map, "selbsterklaerung")?,
			fuehrungszeugnis_auststellung,
			fuehrungszeugnis_eingesehen: None,

			agb: get_bool!(map, "agb")?,
		};

		if !res.agb {
			bail!("Die AGB müssen akzeptiert werden");
		}
		if !res.selbsterklaerung {
			bail!("Die Selbsterklärung muss abgegeben werden");
		}
		check_empty!(
			res,
			vorname,
			nachname,
			juleica_nummer,
			mail,
			handynummer,
			strasse,
			hausnummer,
			ort,
			plz
		);
		// Check PLZ
		if !check_only_numbers(&res.plz, 5) {
			bail!("Ungültige Postleitzahl ({})", res.plz);
		}
		// Check mail address
		if !check_email(&res.mail) {
			bail!("Ungültige E-Mail Addresse ({})", res.mail);
		}
		// Check Juleica Number
		if !check_only_numbers(&res.juleica_nummer, 11) {
			bail!("Ungültige Juleicanummer ({}, Länge ≠ 11 oder Buchstaben drin)", res.juleica_nummer)
		}
		// Check birth date
		let birthday = Date::from_utc(res.geburtsdatum, Utc);
		let now = Utc::now().date();
		let years = years_old(birthday);
		if now <= birthday || years >= 100 {
			bail!(
				"Sind Sie sicher, dass {} ihr Geburtsdatum ist?\nBitte geben \
				 Sie das Geburtsdatum im Format TT.MM.JJJJ an.",
				res.geburtsdatum.format("%d.%m.%Y")
			);
		}

		if years < 15 {
			bail!(
				"Mit deinem Geburtsdatum {} bist du leider zu jung, um als \
				 Betreuer mit aufs Zeltlager zu fahren :), bitte melde dich \
				 als Teilnehmer an",
				res.geburtsdatum.format("%d.%m.%Y")
			);
		}

		map.remove("submit");
		if !map.is_empty() {
			warn!(
				"Teilnehmer::from_hashmap: Map is not yet empty ({:?})",
				map
			);
		}

		Ok(res)
	}
}
