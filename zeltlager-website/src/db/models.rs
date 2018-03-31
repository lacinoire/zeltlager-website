use std::collections::HashMap;

use chrono::{Datelike, NaiveDate, Utc};

use {chrono, Result};
use super::schema::teilnehmer;

macro_rules! get_str {
    ($map:ident, $key:expr) => {
        $map.remove($key).ok_or_else(|| format_err!("{} fehlt", $key))
    };
}

macro_rules! get_bool {
    ($map:ident, $key:expr) => {
        $map.remove($key).ok_or_else(|| format_err!("{} fehlt", $key))
            .and_then(|s|
            if s == "true" {
                Ok(true)
            } else if s == "false" {
                Ok(false)
            } else {
                Err(format_err!("{} ({}) ist kein Wahrheitswert", $key, s))
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
                date = NaiveDate::from_ymd(date.year()
                    + cur_year / 100 * 100,
                    date.month(), date.day());
            } else {
                date = NaiveDate::from_ymd(date.year()
                    + cur_year / 100 * 100 - 100,
                    date.month(), date.day());
            }
        }
        Ok(date)
    } else {
        bail!("Bitte geben Sie das Geburtsdatum ({}) im Format dd.mm.yyyy an.",
            s);
    }
}

#[derive(Clone, Debug, Insertable, Queryable)]
#[table_name = "teilnehmer"]
pub struct Teilnehmer {
    vorname: String,
    nachname: String,
    geburtsdatum: chrono::NaiveDate,
    schwimmer: bool,
    vegetarier: bool,
    tetanus_impfung: bool,
    eltern_name: String,
    eltern_mail: String,
    eltern_handynummer: String,
    strasse: String,
    hausnummer: String,
    ort: String,
    plz: String,
    besonderheiten: String,
    agb: bool,
}

impl Teilnehmer {
    pub fn from_hashmap(mut map: HashMap<String, String>) -> Result<Self> {
        let date = get_str!(map, "geburtsdatum")?;
        let geburtsdatum = try_parse_date(&date)?;

        let res = Self {
            vorname: get_str!(map, "vorname")?,
            nachname: get_str!(map, "nachname")?,
            geburtsdatum,

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
        check_empty!(res, vorname, nachname, eltern_name, eltern_mail,
            eltern_handynummer, strasse, hausnummer, ort, plz);
        // Check PLZ
        if res.plz.len() != 5 || res.plz.chars().any(|c| !c.is_numeric()) {
            bail!("Ungültige Postleitzahl ({})", res.plz);
        }
        // Check mail address
        if !res.eltern_mail.contains('@') {
            bail!("Ungültige E-Mail Addresse ({})", res.eltern_mail);
        }

        map.remove("submit");
        if !map.is_empty() {
            println!("Warning: Map is not yet empty ({:?})", map);
        }

        Ok(res)
    }
}
