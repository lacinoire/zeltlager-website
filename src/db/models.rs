use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;

use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Text;
use heck::ToTitleCase;
use ipnetwork::IpNetwork;
use log::warn;
use serde::Serialize;
use serde::ser::Error;
use time::macros::format_description;
use time::{Date, OffsetDateTime, PrimitiveDateTime};

use crate::{GERMAN_DATE_FORMAT, ISO_DATE_FORMAT, LAGER_START, PRIMITIVE_DATE_TIME_FORMAT};

use super::FormError;
use super::schema::betreuer;
use super::schema::erwischt_member;
use super::schema::rate_limiting;
use super::schema::teilnehmer;
use super::schema::users;

macro_rules! get_bool {
	($map:ident, $key:expr) => {
		$map.remove($key)
			.ok_or_else(|| FormError {
				field: Some($key.into()),
				message: format!("{} fehlt", $key.to_title_case()),
			})
			.and_then(|s| {
				if s == "true" {
					Ok(true)
				} else if s == "false" {
					Ok(false)
				} else {
					Err(FormError {
						field: Some($key.into()),
						message: format!("{} ({}) ist kein Wahrheitswert", $key.to_title_case(), s),
					})
				}
			})
	};
}

macro_rules! check_empty {
	($obj:ident, $(,)?) => {};
	($obj:ident, $field:ident? $($rest:tt)*) => {
		if $obj.$field.as_ref().unwrap().is_empty() {
			return Err(FormError {
				field: Some(stringify!($field).into()),
				message: format!("{} muss ausgefüllt werden", stringify!($field).to_title_case()),
			});
		}
		check_empty!($obj $($rest)*)
	};
	($obj:ident, $field:ident $($rest:tt)*) => {
		if $obj.$field.is_empty() {
			return Err(FormError {
				field: Some(stringify!($field).into()),
				message: format!("{} muss ausgefüllt werden", stringify!($field).to_title_case()),
			});
		}
		check_empty!($obj $($rest)*)
	};
}

// Without id, bezahlt, anwesend and anmeldedatum
#[derive(Clone, Debug, Insertable, Serialize, Queryable)]
#[diesel(table_name = teilnehmer)]
pub struct Teilnehmer {
	pub vorname: String,
	pub nachname: String,
	#[serde(with = "date")]
	pub geburtsdatum: Date,
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
	pub kommentar: String,
	pub agb: bool,
	pub allergien: String,
	pub unvertraeglichkeiten: String,
	pub medikamente: String,
	pub krankenversicherung: String,
	pub land: String,
	pub krankheiten: String,
	pub eigenanreise: bool,
}

#[derive(Clone, Debug, Serialize, Queryable)]
pub struct FullTeilnehmer {
	pub id: i32,
	pub vorname: String,
	pub nachname: String,
	#[serde(with = "date")]
	pub geburtsdatum: Date,
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
	pub kommentar: String,
	pub agb: bool,
	#[serde(with = "primitive_datetime")]
	pub anmeldedatum: PrimitiveDateTime,
	pub bezahlt: bool,
	pub anwesend: bool,
	pub allergien: String,
	pub unvertraeglichkeiten: String,
	pub medikamente: String,
	pub krankenversicherung: String,
	pub land: String,
	pub krankheiten: String,
	pub eigenanreise: bool,
}

// Without id, anmeldedatum and signup_token/time
#[derive(Clone, Debug, Insertable, Serialize, Queryable)]
#[diesel(table_name = betreuer)]
pub struct Supervisor {
	pub vorname: String,
	pub nachname: String,
	#[serde(with = "date")]
	pub geburtsdatum: Date,
	pub geschlecht: Gender,
	pub juleica_nummer: Option<String>,
	pub mail: String,
	pub handynummer: String,
	pub strasse: Option<String>,
	pub hausnummer: Option<String>,
	pub ort: Option<String>,
	pub plz: Option<String>,
	pub kommentar: Option<String>,
	pub agb: bool,
	pub selbsterklaerung: bool,
	#[serde(with = "opt_date")]
	pub fuehrungszeugnis_ausstellung: Option<Date>,
	#[serde(with = "opt_date")]
	pub fuehrungszeugnis_eingesehen: Option<Date>,
	pub allergien: Option<String>,
	pub unvertraeglichkeiten: Option<String>,
	pub medikamente: Option<String>,
	pub krankenversicherung: Option<String>,
	pub vegetarier: Option<bool>,
	pub tetanus_impfung: Option<bool>,
	pub land: Option<String>,
	pub krankheiten: Option<String>,
	#[serde(with = "opt_date")]
	pub juleica_gueltig_bis: Option<Date>,
}

#[derive(Clone, Debug, Serialize, Queryable)]
pub struct FullSupervisor {
	pub id: i32,
	pub vorname: String,
	pub nachname: String,
	#[serde(with = "date")]
	pub geburtsdatum: Date,
	pub geschlecht: Gender,
	pub juleica_nummer: Option<String>,
	pub mail: String,
	pub handynummer: String,
	pub strasse: Option<String>,
	pub hausnummer: Option<String>,
	pub ort: Option<String>,
	pub plz: Option<String>,
	pub kommentar: Option<String>,
	pub agb: bool,
	pub selbsterklaerung: bool,
	#[serde(with = "opt_date")]
	pub fuehrungszeugnis_ausstellung: Option<Date>,
	#[serde(with = "opt_date")]
	pub fuehrungszeugnis_eingesehen: Option<Date>,
	#[serde(with = "primitive_datetime")]
	pub anmeldedatum: PrimitiveDateTime,
	pub allergien: Option<String>,
	pub unvertraeglichkeiten: Option<String>,
	pub medikamente: Option<String>,
	pub krankenversicherung: Option<String>,
	pub vegetarier: Option<bool>,
	pub tetanus_impfung: Option<bool>,
	pub land: Option<String>,
	pub krankheiten: Option<String>,
	#[serde(with = "opt_date")]
	pub juleica_gueltig_bis: Option<Date>,
	#[serde(skip)]
	pub signup_token: Option<String>,
	#[serde(skip)]
	pub signup_token_time: Option<PrimitiveDateTime>,
}

#[derive(Clone, Debug, Insertable, Queryable, Identifiable)]
#[diesel(primary_key(ip_addr))]
#[diesel(table_name = rate_limiting)]
pub struct RateLimiting {
	pub ip_addr: IpNetwork,
	pub counter: i32,
	pub first_count: PrimitiveDateTime,
}

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = users)]
pub struct User {
	pub username: String,
	pub password: String,
}

#[derive(Clone, Debug, Queryable)]
pub struct UserQueryResult {
	pub id: i32,
	pub username: String,
	pub password: String,
}

#[derive(Clone, Debug, Queryable)]
pub struct Role {
	pub user_id: i32,
	pub role: String,
}

#[derive(Clone, Debug, Queryable, Serialize)]
pub struct ErwischtGame {
	pub id: i32,
	#[serde(with = "primitive_datetime")]
	pub created: PrimitiveDateTime,
}

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = erwischt_member)]
pub struct NewErwischtMember {
	pub game: i32,
	pub id: i32,
	pub name: String,
	pub target: i32,
}

#[derive(Clone, Debug, Queryable, Serialize)]
pub struct ErwischtMember {
	pub id: i32,
	pub name: String,
	pub target: i32,
	pub catcher: Option<i32>,
	#[serde(with = "opt_primitive_datetime")]
	pub last_change: Option<PrimitiveDateTime>,
}

pub fn try_parse_date(s: &str, field: &str) -> Result<Date, FormError> {
	let formats = &[
		GERMAN_DATE_FORMAT,
		ISO_DATE_FORMAT,
		format_description!("[day padding:none].[month padding:none].[year padding:none]"),
		format_description!("[year padding:none]-[month padding:none]-[day padding:none]"),
	];
	let mut res = None;
	for f in formats {
		if let Ok(date) = Date::parse(s, &f) {
			res = Some(date);
			break;
		}
	}

	if let Some(mut date) = res {
		if date.year() <= 1900 {
			// Only the last digits of the year where written so correct it.
			// Like 10 for 2010
			let cur_year = OffsetDateTime::now_utc().year();
			if date.year() <= cur_year % 100 {
				date = Date::from_calendar_date(
					date.year() + cur_year / 100 * 100,
					date.month(),
					date.day(),
				)
				.unwrap();
			} else {
				date = Date::from_calendar_date(
					date.year() + cur_year / 100 * 100 - 100,
					date.month(),
					date.day(),
				)
				.unwrap();
			}
		}
		Ok(date)
	} else {
		Err(FormError {
			field: Some(field.into()),
			message: format!("Bitte geben Sie das Datum ({}) im Format TT.MM.JJJJ an.", s),
		})
	}
}

pub fn try_parse_gender(s: &str) -> Result<Gender, FormError> {
	const MALE: &[&str] =
		&["m", "M", "männlich", "Männlich", "maennlich", "Maennlich", "male", "Male"];
	const FEMALE: &[&str] = &["w", "W", "weiblich", "Weiblich", "female", "Female"];

	if MALE.contains(&s) {
		Ok(Gender::Male)
	} else if FEMALE.contains(&s) {
		Ok(Gender::Female)
	} else {
		Err(FormError {
			field: Some("geschlecht".into()),
			message: format!("{} ist kein bekanntes Geschlecht.", s),
		})
	}
}

pub fn years_old(date: Date, birthday_date: &Date) -> i32 {
	let mut years = birthday_date.year() - date.year();
	if (birthday_date.month() as u8) < (date.month() as u8)
		|| (birthday_date.month() as u8 == date.month() as u8 && birthday_date.day() < date.day())
	{
		years -= 1;
	}
	years
}

pub fn check_plz(text: &str, country: &str) -> Result<(), FormError> {
	let error = if !text.chars().all(|c| c.is_numeric()) {
		Some("darf nur Zahlen enthalten")
	} else if country == "Deutschland" {
		(text.len() != 5).then_some("muss 5 Stellen haben")
	} else {
		None
	};
	if let Some(error) = error {
		return Err(FormError {
			field: Some("plz".into()),
			message: format!("Ungültige Postleitzahl ({}), {}", text, error),
		});
	}
	Ok(())
}

pub fn check_krankenversicherung(text: &str) -> Result<(), FormError> {
	// Check krankenversicherung
	if text != "gesetzlich" && text != "privat" && text != "anderes" {
		return Err(FormError {
			field: Some("krankenversicherung".into()),
			message: format!(
				"Ungültige Krankenversicherung ({}), muss entweder gesetzlich, privat oder \
				 anderes sein",
				text
			),
		});
	}
	Ok(())
}

pub fn check_email(text: &str, field: &str) -> Result<(), FormError> {
	let at_pos = text.find('@');
	let error = if at_pos.is_none() {
		Some("muss ein @ enthalten")
	} else if at_pos != text.rfind('@') {
		Some("bitten nur eine einzelne E-Mail-Adresse angeben")
	} else if text.contains(' ') {
		Some("darf keine Leerzeichen enthalten")
	} else {
		None
	};
	if let Some(error) = error {
		return Err(FormError {
			field: Some(field.into()),
			message: format!("Ungültige E-Mail-Adresse ({}), {}", text, error),
		});
	}
	Ok(())
}

pub fn check_house_number(text: &str) -> Result<(), FormError> {
	// Check for at least one digit
	if text.find(|c: char| c.is_ascii_digit()).is_none() {
		Err(FormError {
			field: Some("hausnummer".into()),
			message: format!(
				"Ungültige Hausnummer ({}), muss mindestens eine Ziffer enthalten",
				text
			),
		})
	} else {
		Ok(())
	}
}

pub fn cleanup_freetext(text: String) -> String {
	let lower = text.trim().to_lowercase();
	if ["-", "nein", "kein", "keine", "keins", "nichts", "nb" /* nicht bekannt */]
		.contains(&lower.as_str())
	{
		return String::new();
	}
	text
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, FromSqlRow, AsExpression, Serialize)]
#[diesel(sql_type = Text)]
pub enum Gender {
	Male,
	Female,
}

impl fmt::Display for Gender {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if *self == Gender::Male { write!(f, "m") } else { write!(f, "w") }
	}
}

impl<DB> ToSql<Text, DB> for Gender
where
	DB: Backend,
	str: ToSql<Text, DB>,
{
	fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
		match *self {
			Gender::Male => "m".to_sql(out),
			Gender::Female => "w".to_sql(out),
		}
	}
}

impl<DB> FromSql<Text, DB> for Gender
where
	DB: Backend,
	String: FromSql<Text, DB>,
{
	fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
		match String::from_sql(bytes)?.as_str() {
			"m" => Ok(Gender::Male),
			"w" => Ok(Gender::Female),
			_ => Err("Unrecognized enum variant".into()),
		}
	}
}

struct TimeVisitor<T: ?Sized>(PhantomData<T>);

impl serde::de::Visitor<'_> for TimeVisitor<Date> {
	type Value = Date;

	fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str("a `Date`")
	}

	fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Date, E> {
		Date::parse(value, ISO_DATE_FORMAT).map_err(E::custom)
	}
}

impl<'a> serde::de::Visitor<'a> for TimeVisitor<Option<Date>> {
	type Value = Option<Date>;

	fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str("a `Option<Date>`")
	}

	fn visit_some<D: serde::Deserializer<'a>>(
		self, deserializer: D,
	) -> Result<Self::Value, D::Error> {
		deserializer.deserialize_any(TimeVisitor::<Date>(PhantomData)).map(Some)
	}

	fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> { Ok(None) }

	fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> { Ok(None) }
}

pub mod date {
	use super::*;
	pub fn serialize<S: serde::Serializer>(date: &Date, serializer: S) -> Result<S::Ok, S::Error> {
		date.format(ISO_DATE_FORMAT).map_err(S::Error::custom)?.serialize(serializer)
	}
}

pub mod opt_date {
	use super::*;
	pub fn serialize<S: serde::Serializer>(
		date: &Option<Date>, serializer: S,
	) -> Result<S::Ok, S::Error> {
		date.map(|d| d.format(ISO_DATE_FORMAT))
			.transpose()
			.map_err(S::Error::custom)?
			.serialize(serializer)
	}

	pub fn deserialize<'a, D: serde::Deserializer<'a>>(
		deserializer: D,
	) -> Result<Option<Date>, D::Error> {
		deserializer.deserialize_option(TimeVisitor::<Option<Date>>(PhantomData))
	}
}

pub mod primitive_datetime {
	use super::*;
	pub fn serialize<S: serde::Serializer>(
		datetime: &PrimitiveDateTime, serializer: S,
	) -> Result<S::Ok, S::Error> {
		datetime.format(PRIMITIVE_DATE_TIME_FORMAT).map_err(S::Error::custom)?.serialize(serializer)
	}
}

pub mod opt_primitive_datetime {
	use super::*;
	pub fn serialize<S: serde::Serializer>(
		datetime: &Option<PrimitiveDateTime>, serializer: S,
	) -> Result<S::Ok, S::Error> {
		datetime
			.map(|d| d.format(PRIMITIVE_DATE_TIME_FORMAT))
			.transpose()
			.map_err(S::Error::custom)?
			.serialize(serializer)
	}
}

impl Teilnehmer {
	pub fn from_hashmap(mut map: HashMap<String, String>) -> Result<Self, FormError> {
		let date = get_str!(map, "geburtsdatum")?;
		let geburtsdatum = try_parse_date(&date, "geburtsdatum")?;
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
			land: get_str!(map, "land")?,
			strasse: get_str!(map, "strasse")?,
			hausnummer: get_str!(map, "hausnummer")?,
			ort: get_str!(map, "ort")?,
			plz: get_str!(map, "plz")?,

			krankenversicherung: get_str!(map, "krankenversicherung")?,
			krankheiten: get_freetext_str!(map, "krankheiten")?,
			allergien: get_freetext_str!(map, "allergien")?,
			unvertraeglichkeiten: get_freetext_str!(map, "unvertraeglichkeiten")?,
			medikamente: get_freetext_str!(map, "medikamente")?,
			kommentar: get_freetext_str!(map, "kommentar")?,

			eigenanreise: get_bool!(map, "eigenanreise")?,
			agb: get_bool!(map, "agb")?,
		};

		if !res.agb {
			return Err(FormError {
				field: Some("agb".into()),
				message: "Die AGB müssen akzeptiert werden".into(),
			});
		}

		check_empty!(
			res,
			vorname,
			nachname,
			eltern_name,
			eltern_mail,
			eltern_handynummer,
			land,
			strasse,
			hausnummer,
			ort,
			plz,
		);

		check_plz(&res.plz, &res.land)?;
		check_krankenversicherung(&res.krankenversicherung)?;
		check_email(&res.eltern_mail, "eltern_mail")?;
		check_house_number(&res.hausnummer)?;

		// Check birth date
		let now = OffsetDateTime::now_utc().date();
		let years = years_old(res.geburtsdatum, &LAGER_START);
		if now <= res.geburtsdatum || years >= 100 {
			return Err(FormError {
				field: Some("geburtsdatum".into()),
				message: format!(
					"Sind Sie sicher, dass {} das Geburtsdatum Ihres Kindes ist?\nBitte geben Sie \
					 das Geburtsdatum im Format TT.MM.JJJJ an.",
					res.geburtsdatum.format(GERMAN_DATE_FORMAT).unwrap()
				),
			});
		}

		if years < 7 {
			return Err(FormError {
				field: Some("geburtsdatum".into()),
				message: format!(
					"Ihr Kind ist zu jung (Geburtsdatum {}).\nDas Zeltlager ist für Kinder und \
					 Jugendliche zwischen 7 und 15 Jahren.",
					res.geburtsdatum.format(GERMAN_DATE_FORMAT).unwrap()
				),
			});
		}
		if years > 15 {
			return Err(FormError {
				field: Some("geburtsdatum".into()),
				message: format!(
					"Ihr Kind ist zu alt um als Teilnehmer beim Zeltlager mitzufahren \
					 (Geburtsdatum {}).\nWir suchen immer nach motivierten Betreuern (ab 16 \
					 Jahren), die auf das Zeltlager mitfahren.\nInfos dazu finden Sie auf der \
					 Betreuerseite.\nDas Zeltlager ist für Kinder und Jugendliche zwischen 7 und \
					 15 Jahren.",
					res.geburtsdatum.format(GERMAN_DATE_FORMAT).unwrap()
				),
			});
		}

		map.remove("submit");
		if !map.is_empty() {
			warn!("Teilnehmer::from_hashmap: Map is not yet empty ({:?})", map);
		}

		Ok(res)
	}

	pub fn trim(&mut self) {
		self.vorname = self.vorname.trim().into();
		self.nachname = self.nachname.trim().into();
		self.eltern_name = self.eltern_name.trim().into();
		self.eltern_mail = self.eltern_mail.trim().into();
		self.eltern_handynummer = self.eltern_handynummer.trim().into();
		self.strasse = self.strasse.trim().into();
		self.hausnummer = self.hausnummer.trim().into();
		self.ort = self.ort.trim().into();
		self.plz = self.plz.trim().into();
		self.kommentar = self.kommentar.trim().into();
		self.allergien = self.allergien.trim().into();
		self.unvertraeglichkeiten = self.unvertraeglichkeiten.trim().into();
		self.medikamente = self.medikamente.trim().into();
		self.krankenversicherung = self.krankenversicherung.trim().into();
		self.land = self.land.trim().into();
		self.krankheiten = self.krankheiten.trim().into();
	}
}

impl Supervisor {
	pub fn from_hashmap(mut map: HashMap<String, String>) -> Result<Self, FormError> {
		let date = get_str!(map, "geburtsdatum")?;
		let geburtsdatum = try_parse_date(&date, "geburtsdatum")?;
		let geschlecht = try_parse_gender(&get_str!(map, "geschlecht")?)?;

		let f_date = get_str!(map, "fuehrungszeugnis_ausstellung")?;
		let fuehrungszeugnis_ausstellung = if !f_date.is_empty() {
			Some(try_parse_date(&f_date, "fuehrungszeugnis_ausstellung")?)
		} else {
			None
		};
		let j_date = get_str!(map, "juleica_gueltig_bis")?;
		let juleica_gueltig_bis = if !j_date.is_empty() {
			Some(try_parse_date(&j_date, "juleica_gueltig_bis")?)
		} else {
			None
		};

		let juleica_nummer_str = get_str!(map, "juleica_nummer")?;
		let juleica_nummer =
			if juleica_nummer_str.is_empty() { None } else { Some(juleica_nummer_str) };

		let res = Self {
			vorname: get_str!(map, "vorname")?,
			nachname: get_str!(map, "nachname")?,
			geburtsdatum,
			geschlecht,

			vegetarier: Some(get_bool!(map, "vegetarier")?),
			tetanus_impfung: Some(get_bool!(map, "tetanus_impfung")?),

			juleica_nummer,
			juleica_gueltig_bis,
			mail: get_str!(map, "mail")?,
			handynummer: get_str!(map, "handynummer")?,
			land: Some(get_str!(map, "land")?),
			strasse: Some(get_str!(map, "strasse")?),
			hausnummer: Some(get_str!(map, "hausnummer")?),
			ort: Some(get_str!(map, "ort")?),
			plz: Some(get_str!(map, "plz")?),

			krankenversicherung: Some(get_str!(map, "krankenversicherung")?),
			krankheiten: Some(get_freetext_str!(map, "krankheiten")?),
			allergien: Some(get_freetext_str!(map, "allergien")?),
			unvertraeglichkeiten: Some(get_freetext_str!(map, "unvertraeglichkeiten")?),
			medikamente: Some(get_freetext_str!(map, "medikamente")?),
			kommentar: Some(get_freetext_str!(map, "kommentar")?),
			fuehrungszeugnis_ausstellung,
			fuehrungszeugnis_eingesehen: None,

			selbsterklaerung: get_bool!(map, "selbsterklaerung")?,
			agb: get_bool!(map, "agb")?,
		};

		if !res.agb {
			return Err(FormError {
				field: Some("agb".into()),
				message: "Die AGB müssen akzeptiert werden".into(),
			});
		}
		if !res.selbsterklaerung {
			return Err(FormError {
				field: Some("selbsterklaerung".into()),
				message: "Die Selbsterklärung muss abgegeben werden".into(),
			});
		}

		check_empty!(
			res,
			vorname,
			nachname,
			mail,
			handynummer,
			land?,
			strasse?,
			hausnummer?,
			ort?,
			plz?,
		);

		check_plz(res.plz.as_ref().unwrap(), res.land.as_ref().unwrap())?;
		check_krankenversicherung(res.krankenversicherung.as_ref().unwrap())?;
		check_email(&res.mail, "mail")?;
		check_house_number(res.hausnummer.as_ref().unwrap())?;

		// Check Juleica Number
		if let Some(ref jn) = res.juleica_nummer {
			if !jn.chars().all(|c| c.is_numeric()) {
				return Err(FormError {
					field: Some("juleica_nummer".into()),
					message: format!(
						"Ungültige Juleicanummer ({}), darf nur Ziffern enthalten",
						jn
					),
				});
			}
		}
		// Check birth date
		let now = OffsetDateTime::now_utc().date();
		let years = years_old(res.geburtsdatum, &LAGER_START);
		if now <= res.geburtsdatum || years >= 100 {
			return Err(FormError {
				field: Some("geburtsdatum".into()),
				message: format!(
					"Sind Sie sicher, dass {} ihr Geburtsdatum ist?\nBitte geben Sie das \
					 Geburtsdatum im Format TT.MM.JJJJ an.",
					res.geburtsdatum.format(GERMAN_DATE_FORMAT).unwrap()
				),
			});
		}

		if years < 15 {
			return Err(FormError {
				field: Some("geburtsdatum".into()),
				message: format!(
					"Mit deinem Geburtsdatum {} bist du leider zu jung, um als Betreuer mit aufs \
					 Zeltlager zu fahren 🙂, bitte melde dich als Teilnehmer an.",
					res.geburtsdatum.format(GERMAN_DATE_FORMAT).unwrap()
				),
			});
		}

		map.remove("submit");
		if !map.is_empty() {
			warn!("Supervisor::from_hashmap: Map is not yet empty ({:?})", map);
		}

		Ok(res)
	}
}

#[cfg(test)]
mod tests {
	use time::{Date, Month};

	use super::try_parse_date;

	#[test]
	fn parse_date() {
		for d in &[
			("02.01.2000", Date::from_calendar_date(2000, Month::January, 2)),
			("2000-01-02", Date::from_calendar_date(2000, Month::January, 2)),
			("02.01.01", Date::from_calendar_date(2001, Month::January, 2)),
			("02.01.95", Date::from_calendar_date(1995, Month::January, 2)),
			("2.1.2000", Date::from_calendar_date(2000, Month::January, 2)),
			("2000-1-2", Date::from_calendar_date(2000, Month::January, 2)),
		] {
			println!("Parse: {}", d.0);
			let res = try_parse_date(d.0, "").unwrap();
			assert_eq!(d.1.unwrap(), res);
		}
	}
}
