use std::collections::HashMap;
use std::fmt;

use chrono::{self, DateTime, Datelike, NaiveDate, Utc};
use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Text;
use heck::ToTitleCase;
use ipnetwork::IpNetwork;
use log::warn;
use serde::Serialize;

use super::schema::betreuer;
use super::schema::erwischt_member;
use super::schema::rate_limiting;
use super::schema::teilnehmer;
use super::schema::users;
use super::FormError;

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
	($obj:ident, $($field:ident),*) => {
		$(
		if $obj.$field.is_empty() {
			return Err(FormError {
				field: Some(stringify!($field).into()),
				message: format!("{} muss ausgefüllt werden", stringify!($field).to_title_case()),
			});
		}
		)*
	}
}

#[derive(Clone, Debug, Insertable, Serialize, Queryable)]
#[diesel(table_name = teilnehmer)]
pub struct Teilnehmer {
	pub vorname: String,
	pub nachname: String,
	pub geburtsdatum: chrono::NaiveDate,
	pub geschlecht: Gender,
	pub schwimmer: bool,
	pub ernaehrung: String,
	pub tetanus_impfung: bool,
	pub eltern_name: String,
	pub eltern_mail: String,
	pub eltern_handynummer: String,
	pub land: String,
	pub strasse: String,
	pub hausnummer: String,
	pub ort: String,
	pub plz: String,
	pub krankenversicherung: String,
	pub unvertraeglichkeiten: String,
	pub allergien: String,
	pub medikamente: String,
	pub krankheiten: String,
	pub kommentar: String,
	pub eigenanreise: bool,
	pub agb: bool,
}

#[derive(Clone, Debug, Serialize, Queryable)]
pub struct FullTeilnehmer {
	pub id: i32,
	pub vorname: String,
	pub nachname: String,
	pub geburtsdatum: chrono::NaiveDate,
	pub geschlecht: Gender,
	pub schwimmer: bool,
	pub ernaehrung: String,
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
	pub anmeldedatum: chrono::NaiveDateTime,
	pub bezahlt: bool,
	pub anwesend: bool,
	pub allergien: String,
	pub unvertraeglichkeiten: String,
	pub medikamente: String,
	pub krankenversicherung: String,
	pub land: String,
}

#[derive(Clone, Debug, Insertable, Serialize, Queryable)]
#[diesel(table_name = betreuer)]
pub struct Supervisor {
	pub vorname: String,
	pub nachname: String,
	pub geburtsdatum: chrono::NaiveDate,
	pub geschlecht: Gender,
	pub juleica_nummer: Option<String>,
	pub mail: String,
	pub handynummer: String,
	pub land: String,
	pub strasse: String,
	pub hausnummer: String,
	pub ort: String,
	pub plz: String,
	pub ernaehrung: String,
	pub tetanus_impfung: bool,
	pub krankenversicherung: String,
	pub allergien: String,
	pub unvertraeglichkeiten: String,
	pub medikamente: String,
	pub besonderheiten: String,
	pub agb: bool,
	pub selbsterklaerung: bool,
	pub fuehrungszeugnis_auststellung: Option<chrono::NaiveDate>,
	pub fuehrungszeugnis_eingesehen: Option<chrono::NaiveDate>,
}

#[derive(Clone, Debug, Serialize, Queryable)]
pub struct FullSupervisor {
	pub id: i32,
	pub vorname: String,
	pub nachname: String,
	pub geburtsdatum: chrono::NaiveDate,
	pub geschlecht: Gender,
	pub juleica_nummer: Option<String>,
	pub mail: String,
	pub handynummer: String,
	pub strasse: String,
	pub hausnummer: String,
	pub ort: String,
	pub plz: String,
	pub besonderheiten: String,
	pub agb: bool,
	pub selbsterklaerung: bool,
	pub fuehrungszeugnis_auststellung: Option<chrono::NaiveDate>,
	pub fuehrungszeugnis_eingesehen: Option<chrono::NaiveDate>,
	pub anmeldedatum: chrono::NaiveDateTime,
	pub allergien: String,
	pub unvertraeglichkeiten: String,
	pub medikamente: String,
	pub krankenversicherung: String,
	pub ernaehrung: String,
	pub tetanus_impfung: bool,
	pub land: String,
}

#[derive(Clone, Debug, Insertable, Queryable, Identifiable)]
#[diesel(primary_key(ip_addr))]
#[diesel(table_name = rate_limiting)]
pub struct RateLimiting {
	pub ip_addr: IpNetwork,
	pub counter: i32,
	pub first_count: chrono::NaiveDateTime,
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
	pub created: chrono::NaiveDateTime,
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
	pub last_change: Option<chrono::NaiveDateTime>,
}

pub fn try_parse_date(s: &str, field: &str) -> Result<NaiveDate, FormError> {
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
				date = NaiveDate::from_ymd_opt(
					date.year() + cur_year / 100 * 100,
					date.month(),
					date.day(),
				)
				.unwrap();
			} else {
				date = NaiveDate::from_ymd_opt(
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
			message: format!("Bitte geben Sie das Geburtsdatum ({}) im Format TT.MM.JJJJ an.", s),
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

pub fn get_birthday_date(birthday_date: &str) -> DateTime<Utc> {
	let date = NaiveDate::parse_from_str(&format!("0000-{}", birthday_date), "%Y-%m-%d")
		.expect("Date has wrong format");
	let mut date = date.and_time(Default::default()).and_utc();

	// Set the right year
	let now = Utc::now();
	date = date.with_year(now.year()).unwrap();
	if date < now {
		date = date.with_year(now.year() + 1).unwrap();
	}
	date
}

pub fn years_old(date: DateTime<Utc>, birthday_date: &DateTime<Utc>) -> i32 {
	let mut years = birthday_date.year() - date.year();
	if birthday_date.month() < date.month()
		|| (birthday_date.month() == date.month() && birthday_date.day() < date.day())
	{
		years -= 1;
	}
	years
}

pub fn check_plz(text: &str, country: &str) -> Result<(), FormError> {
	let valid = if !text.chars().all(|c| c.is_numeric()) {
		false
	} else if country == "Deutschland" {
		text.len() == 5
	} else {
		true
	};
	if !valid {
		return Err(FormError {
			field: Some("plz".into()),
			message: format!("Ungültige Postleitzahl ({})", text),
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

pub fn check_ernaehrung(text: &str) -> Result<(), FormError> {
	if text != "alles" && text != "vegetarisch" && text != "vegan" {
		return Err(FormError {
			field: Some("ernaehrung".into()),
			message: format!(
				"Ungültige Ernährung ({}), muss entweder alles, vegetarisch oder vegan sein",
				text
			),
		});
	}
	Ok(())
}

pub fn check_email(text: &str, field: &str) -> Result<(), FormError> {
	let at_pos = text.find('@');
	let valid = at_pos.is_some() && !text.contains(' ') && at_pos == text.rfind('@'); // Only one mail address
	if !valid {
		return Err(FormError {
			field: Some(field.into()),
			message: format!("Ungültige E-Mail Addresse ({})", text),
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

impl Teilnehmer {
	pub fn from_hashmap(
		mut map: HashMap<String, String>, birthday_date: &str,
	) -> Result<Self, FormError> {
		let date = get_str!(map, "geburtsdatum")?;
		let geburtsdatum = try_parse_date(&date, "geburtsdatum")?;
		let geschlecht = try_parse_gender(&get_str!(map, "geschlecht")?)?;

		let res = Self {
			vorname: get_str!(map, "vorname")?,
			nachname: get_str!(map, "nachname")?,
			geburtsdatum,
			geschlecht,

			schwimmer: get_bool!(map, "schwimmer")?,
			ernaehrung: get_str!(map, "ernaehrung")?,
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
			allergien: get_str!(map, "allergien")?,
			unvertraeglichkeiten: get_str!(map, "unvertraeglichkeiten")?,
			medikamente: get_str!(map, "medikamente")?,
			besonderheiten: get_str!(map, "besonderheiten")?,

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
			plz
		);

		check_plz(&res.plz, &res.land)?;
		check_krankenversicherung(&res.krankenversicherung)?;
		check_email(&res.eltern_mail, "eltern_mail")?;
		check_house_number(&res.hausnummer)?;

		// Check birth date
		let birthday = res.geburtsdatum.and_time(Default::default()).and_utc();
		let now = Utc::now();
		let years = years_old(birthday, &get_birthday_date(birthday_date));
		if now <= birthday || years >= 100 {
			return Err(FormError {
				field: Some("geburtsdatum".into()),
				message: format!(
					"Sind Sie sicher, dass {} das Geburtsdatum Ihres Kindes ist?\nBitte geben Sie \
					 das Geburtsdatum im Format TT.MM.JJJJ an.",
					res.geburtsdatum.format("%d.%m.%Y")
				),
			});
		}

		if years < 7 {
			return Err(FormError {
				field: Some("geburtsdatum".into()),
				message: format!(
					"Ihr Kind ist zu jung (Geburtsdatum {}).\nDas Zeltlager ist für Kinder und \
					 Jugendliche zwischen 7 und 15 Jahren.",
					res.geburtsdatum.format("%d.%m.%Y")
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
					res.geburtsdatum.format("%d.%m.%Y")
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
		self.land = self.land.trim().into();
		self.strasse = self.strasse.trim().into();
		self.hausnummer = self.hausnummer.trim().into();
		self.ort = self.ort.trim().into();
		self.plz = self.plz.trim().into();
		self.krankenversicherung = self.krankenversicherung.trim().into();
		self.allergien = self.allergien.trim().into();
		self.unvertraeglichkeiten = self.unvertraeglichkeiten.trim().into();
		self.medikamente = self.medikamente.trim().into();
		self.besonderheiten = self.besonderheiten.trim().into();
	}
}

impl Supervisor {
	pub fn from_hashmap(
		mut map: HashMap<String, String>, birthday_date: &str,
	) -> Result<Self, FormError> {
		let date = get_str!(map, "geburtsdatum")?;
		let geburtsdatum = try_parse_date(&date, "geburtsdatum")?;
		let geschlecht = try_parse_gender(&get_str!(map, "geschlecht")?)?;

		let f_date = get_str!(map, "fuehrungszeugnis_auststellung")?;
		let fuehrungszeugnis_auststellung = if !f_date.is_empty() {
			Some(try_parse_date(&f_date, "fuehrungszeugnis_auststellung")?)
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

			juleica_nummer,
			mail: get_str!(map, "mail")?,
			handynummer: get_str!(map, "handynummer")?,
			land: get_str!(map, "land")?,
			strasse: get_str!(map, "strasse")?,
			hausnummer: get_str!(map, "hausnummer")?,
			ort: get_str!(map, "ort")?,
			plz: get_str!(map, "plz")?,
			ernaehrung: get_str!(map, "ernaehrung")?,
			tetanus_impfung: get_bool!(map, "tetanus_impfung")?,
			krankenversicherung: get_str!(map, "krankenversicherung")?,
			allergien: get_str!(map, "allergien")?,
			unvertraeglichkeiten: get_str!(map, "unvertraeglichkeiten")?,
			medikamente: get_str!(map, "medikamente")?,
			besonderheiten: get_str!(map, "besonderheiten")?,
			selbsterklaerung: get_bool!(map, "selbsterklaerung")?,
			fuehrungszeugnis_auststellung,
			fuehrungszeugnis_eingesehen: None,

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
			land,
			strasse,
			hausnummer,
			ort,
			plz
		);

		check_plz(&res.plz, &res.land)?;
		check_krankenversicherung(&res.krankenversicherung)?;
		check_email(&res.mail, "mail")?;
		check_house_number(&res.hausnummer)?;

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
		let birthday = res.geburtsdatum.and_time(Default::default()).and_utc();
		let now = Utc::now();
		let years = years_old(birthday, &get_birthday_date(birthday_date));
		if now <= birthday || years >= 100 {
			return Err(FormError {
				field: Some("geburtsdatum".into()),
				message: format!(
					"Sind Sie sicher, dass {} ihr Geburtsdatum ist?\nBitte geben Sie das \
					 Geburtsdatum im Format TT.MM.JJJJ an.",
					res.geburtsdatum.format("%d.%m.%Y")
				),
			});
		}

		if years < 15 {
			return Err(FormError {
				field: Some("geburtsdatum".into()),
				message: format!(
					"Mit deinem Geburtsdatum {} bist du leider zu jung, um als Betreuer mit aufs \
					 Zeltlager zu fahren 🙂, bitte melde dich als Teilnehmer an.",
					res.geburtsdatum.format("%d.%m.%Y")
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
