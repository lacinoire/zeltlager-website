//! All Database requests are implemented here
//! for each request you need:
//! - A Message struct with a `Message` Implementation
//! - A Handler method for the DbExecutor

// TODO Use more RunOnDbMsg

use std::collections::HashMap;
use std::env;
use std::net::IpAddr;
use std::net::SocketAddr;

use actix::prelude::*;
use anyhow::{bail, format_err, Result};
use chrono::Utc;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::Error;
use diesel_migrations::MigrationHarness;
use dotenv::dotenv;
use heck::ToTitleCase;
use ipnetwork::IpNetwork;
use log::info;
use scrypt::password_hash::{
	McfHasher, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use scrypt::Scrypt;
use serde::Serialize;

use crate::auth;

macro_rules! get_str {
	($map:ident, $key:expr) => {
		$map.remove($key).ok_or_else(|| FormError {
			field: Some($key.into()),
			message: format!("{} fehlt", $key.to_title_case()),
		})
	};
}

macro_rules! get_freetext_str {
	($map:ident, $key:expr) => {
		$map.remove($key).map(cleanup_freetext).ok_or_else(|| FormError {
			field: Some($key.into()),
			message: format!("{} fehlt", $key.to_title_case()),
		})
	};
}

pub mod models;
// Generate with `diesel print-schema > src/db/schema.rs`
pub mod schema;

pub const MIGRATIONS: diesel_migrations::EmbeddedMigrations =
	diesel_migrations::embed_migrations!();

#[derive(Clone, Debug, Serialize)]
pub struct FormError {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub field: Option<String>,
	pub message: String,
}

pub struct DbExecutor {
	pub(crate) connection: PgConnection,
}

impl Actor for DbExecutor {
	type Context = SyncContext<Self>;
}

pub struct CheckRateMessage {
	pub ip: String,
}
impl Message for CheckRateMessage {
	type Result = Result<bool>;
}

pub struct SignupMessage {
	pub member: models::Teilnehmer,
}
impl Message for SignupMessage {
	type Result = Result<()>;
}

pub struct DownloadMembersMessage;
impl Message for DownloadMembersMessage {
	type Result = Result<Vec<models::Teilnehmer>>;
}

pub struct DownloadFullMembersMessage;
impl Message for DownloadFullMembersMessage {
	type Result = Result<Vec<models::FullTeilnehmer>>;
}

pub struct DownloadFullSupervisorsMessage;
impl Message for DownloadFullSupervisorsMessage {
	type Result = Result<Vec<models::FullSupervisor>>;
}

pub struct DownloadBetreuerMessage;
impl Message for DownloadBetreuerMessage {
	type Result = Result<Vec<models::Supervisor>>;
}

pub struct SignupSupervisorMessage {
	pub supervisor: models::Supervisor,
}
impl Message for SignupSupervisorMessage {
	type Result = Result<()>;
}

pub struct CountMemberMessage;
impl Message for CountMemberMessage {
	type Result = Result<i64>;
}

pub struct AuthenticateMessage {
	pub username: String,
	pub password: String,
}
impl Message for AuthenticateMessage {
	type Result = Result<Option<i32>>;
}

pub struct DecreaseRateCounterMessage {
	pub ip: String,
}
impl Message for DecreaseRateCounterMessage {
	type Result = Result<()>;
}

pub struct RunOnDbMsg<I: 'static, F: FnOnce(&mut DbExecutor) -> Result<I>>(pub F);
impl<I: 'static, F: FnOnce(&mut DbExecutor) -> Result<I>> Message for RunOnDbMsg<I, F> {
	type Result = Result<I>;
}

impl AuthenticateMessage {
	pub fn from_hashmap(mut map: HashMap<String, String>) -> Result<AuthenticateMessage> {
		Ok(AuthenticateMessage {
			username: get_str!(map, "username").map_err(|e| format_err!(e.message))?,
			password: get_str!(map, "password").map_err(|e| format_err!(e.message))?,
		})
	}
}

/// If the user is member of this role.
pub struct GetRolesMessage {
	pub user: i32,
}

impl Message for GetRolesMessage {
	type Result = Result<Vec<auth::Roles>>;
}

pub struct RunMigrationsMessage;
impl Message for RunMigrationsMessage {
	type Result = Result<()>;
}

impl From<String> for FormError {
	fn from(s: String) -> Self { Self { field: None, message: s } }
}

impl<'a> From<&'a str> for FormError {
	fn from(s: &'a str) -> Self { Self { field: None, message: s.into() } }
}

impl DbExecutor {
	pub fn new() -> Result<Self> {
		dotenv().ok();
		let database_url = env::var("DATABASE_URL").map_err(|e| {
			format_err!("DATABASE_URL is not set, are you missing a .env file? ({:?})", e)
		})?;
		let connection = PgConnection::establish(&database_url)?;

		Ok(Self { connection })
	}
}

impl Handler<RunMigrationsMessage> for DbExecutor {
	type Result = Result<()>;

	fn handle(&mut self, _: RunMigrationsMessage, _: &mut Self::Context) -> Self::Result {
		let migrated = self
			.connection
			.run_pending_migrations(MIGRATIONS)
			.map_err(|e| format_err!("Failed to run migrations: {}", e))?;
		if !migrated.is_empty() {
			info!("Run database migrations: {:?}", migrated);
		}

		Ok(())
	}
}

impl Handler<CheckRateMessage> for DbExecutor {
	type Result = Result<bool>;

	fn handle(&mut self, msg: CheckRateMessage, _: &mut Self::Context) -> Self::Result {
		use self::schema::rate_limiting::dsl::*;
		use diesel::dsl::insert_into;
		use diesel::dsl::now;

		let parse_result = msg.ip.parse::<SocketAddr>();
		let ip: IpNetwork = match parse_result {
			Ok(result) => result.ip().into(),
			Err(_) => msg.ip.parse::<IpAddr>()?.into(),
		};
		let entry_res = rate_limiting.find(ip).first::<models::RateLimiting>(&mut self.connection);
		// check for no entry found
		match entry_res {
			Ok(entry) => {
				if entry.first_count <= Utc::now().naive_utc() - crate::ratelimit_duration() {
					// reset counter and grant request
					diesel::update(&entry).set(counter.eq(1)).execute(&mut self.connection)?;
					diesel::update(&entry)
						.set(first_count.eq(now.at_time_zone("utc")))
						.execute(&mut self.connection)?;
					Ok(true)
				} else if entry.counter >= crate::RATELIMIT_MAX_COUNTER {
					// limit reached
					Ok(false)
				} else {
					diesel::update(&entry)
						.set(counter.eq(counter + 1))
						.execute(&mut self.connection)?;
					Ok(true)
				}
			}
			Err(Error::NotFound) => {
				insert_into(rate_limiting)
					.values((
						ip_addr.eq(ip),
						counter.eq(1),
						first_count.eq(now.at_time_zone("utc")),
					))
					.execute(&mut self.connection)?;
				Ok(true)
			}
			Err(e) => Err(e.into()),
		}
	}
}

impl Handler<DecreaseRateCounterMessage> for DbExecutor {
	type Result = Result<()>;

	fn handle(&mut self, msg: DecreaseRateCounterMessage, _: &mut Self::Context) -> Self::Result {
		use self::schema::rate_limiting::dsl::*;

		let ip: IpNetwork = msg.ip.parse::<SocketAddr>()?.ip().into();
		let entry_res = rate_limiting.find(ip).first::<models::RateLimiting>(&mut self.connection);
		// check for no entry found
		match entry_res {
			Ok(entry) => {
				diesel::update(&entry)
					.set(counter.eq(counter - 1))
					.execute(&mut self.connection)?;
				Ok(())
			}
			Err(Error::NotFound) => {
				bail!("Ip to decrease rate counter for not found in db")
			}
			Err(e) => Err(e.into()),
		}
	}
}

impl<I: 'static, F: FnOnce(&mut DbExecutor) -> Result<I>> Handler<RunOnDbMsg<I, F>> for DbExecutor {
	type Result = Result<I>;
	fn handle(&mut self, msg: RunOnDbMsg<I, F>, _: &mut Self::Context) -> Self::Result {
		msg.0(self)
	}
}

impl Handler<SignupMessage> for DbExecutor {
	type Result = Result<()>;

	fn handle(&mut self, msg: SignupMessage, _: &mut Self::Context) -> Self::Result {
		use self::schema::teilnehmer;

		diesel::insert_into(teilnehmer::table).values(&msg.member).execute(&mut self.connection)?;

		Ok(())
	}
}

impl Handler<DownloadMembersMessage> for DbExecutor {
	type Result = Result<Vec<models::Teilnehmer>>;

	fn handle(&mut self, _: DownloadMembersMessage, _: &mut Self::Context) -> Self::Result {
		use self::schema::teilnehmer;
		use self::schema::teilnehmer::*;
		let all_columns_but_id = (
			vorname,
			nachname,
			geburtsdatum,
			geschlecht,
			schwimmer,
			tetanus_impfung,
			eltern_name,
			eltern_mail,
			eltern_handynummer,
			strasse,
			hausnummer,
			ort,
			plz,
			kommentar,
			agb,
			allergien,
			unvertraeglichkeiten,
			medikamente,
			krankenversicherung,
			land,
			krankheiten,
			ernaehrung,
			eigenanreise,
		);

		Ok(teilnehmer::table
			.select(all_columns_but_id)
			.load::<models::Teilnehmer>(&mut self.connection)?)
	}
}

impl Handler<DownloadFullMembersMessage> for DbExecutor {
	type Result = Result<Vec<models::FullTeilnehmer>>;

	fn handle(&mut self, _: DownloadFullMembersMessage, _: &mut Self::Context) -> Self::Result {
		use self::schema::teilnehmer;

		Ok(teilnehmer::table.load::<models::FullTeilnehmer>(&mut self.connection)?)
	}
}

impl Handler<DownloadFullSupervisorsMessage> for DbExecutor {
	type Result = Result<Vec<models::FullSupervisor>>;

	fn handle(&mut self, _: DownloadFullSupervisorsMessage, _: &mut Self::Context) -> Self::Result {
		use self::schema::betreuer;

		Ok(betreuer::table.load::<models::FullSupervisor>(&mut self.connection)?)
	}
}

impl Handler<DownloadBetreuerMessage> for DbExecutor {
	type Result = Result<Vec<models::Supervisor>>;

	fn handle(&mut self, _: DownloadBetreuerMessage, _: &mut Self::Context) -> Self::Result {
		use self::schema::betreuer;
		use self::schema::betreuer::*;
		let all_columns_but_id = (
			vorname,
			nachname,
			geburtsdatum,
			geschlecht,
			juleica_nummer,
			mail,
			handynummer,
			strasse,
			hausnummer,
			ort,
			plz,
			kommentar,
			agb,
			selbsterklaerung,
			fuehrungszeugnis_auststellung,
			fuehrungszeugnis_eingesehen,
			allergien,
			unvertraeglichkeiten,
			medikamente,
			krankenversicherung,
			tetanus_impfung,
			land,
			krankheiten,
			ernaehrung,
			juleica_gueltig_bis,
		);

		Ok(betreuer::table
			.select(all_columns_but_id)
			.load::<models::Supervisor>(&mut self.connection)?)
	}
}

impl Handler<SignupSupervisorMessage> for DbExecutor {
	type Result = Result<()>;

	fn handle(&mut self, msg: SignupSupervisorMessage, _: &mut Self::Context) -> Self::Result {
		use self::schema::betreuer;

		diesel::insert_into(betreuer::table)
			.values(&msg.supervisor)
			.execute(&mut self.connection)?;

		Ok(())
	}
}

impl Handler<CountMemberMessage> for DbExecutor {
	type Result = Result<i64>;

	fn handle(&mut self, _: CountMemberMessage, _: &mut Self::Context) -> Self::Result {
		use self::schema::teilnehmer;

		Ok(teilnehmer::table.count().get_result(&mut self.connection)?)
	}
}

impl Handler<AuthenticateMessage> for DbExecutor {
	type Result = Result<Option<i32>>;

	fn handle(&mut self, msg: AuthenticateMessage, _: &mut Self::Context) -> Self::Result {
		use self::schema::users::dsl::*;

		// Fetch user from db
		match users
			.filter(username.eq(&msg.username))
			.first::<models::UserQueryResult>(&mut self.connection)
		{
			Ok(user) => {
				if PasswordHash::new(&user.password)
					.and_then(|hash| scrypt::Scrypt.verify_password(msg.password.as_bytes(), &hash))
					.is_ok()
				{
					Ok(Some(user.id))
				} else {
					// If parsing in the new format does not work, try the old hash format
					if scrypt::Scrypt
						.verify_mcf_hash(msg.password.as_bytes(), &user.password)
						.is_ok()
					{
						// Update password to new format
						let salt = SaltString::generate(&mut rand::thread_rng());
						let pw = Scrypt
							.hash_password_simple(msg.password.as_bytes(), salt.as_ref())?
							.to_string();
						diesel::update(users.filter(username.eq(&msg.username)))
							.set(password.eq(pw))
							.execute(&mut self.connection)?;

						Ok(Some(user.id))
					} else {
						Ok(None)
					}
				}
			}
			Err(Error::NotFound) => {
				// Hash a random password so we don’t leak much timing information if a user exists
				// or not.
				let salt = SaltString::generate(&mut rand::thread_rng());
				let pw = Scrypt
					.hash_password_simple(msg.username.as_bytes(), salt.as_ref())?
					.to_string();
				let hash = PasswordHash::new(&pw)?;
				let _ = Scrypt.verify_password(msg.password.as_bytes(), &hash);
				Ok(None)
			}
			Err(err) => Err(err.into()),
		}
	}
}

impl Handler<GetRolesMessage> for DbExecutor {
	type Result = Result<Vec<auth::Roles>>;

	fn handle(&mut self, msg: GetRolesMessage, _: &mut Self::Context) -> Self::Result {
		use self::schema::roles::dsl::*;

		// Fetch user from db
		match roles.filter(user_id.eq(msg.user)).get_results::<models::Role>(&mut self.connection) {
			Ok(mut res) => {
				// Convert to enum
				res.drain(..).map(|r| r.role.parse()).collect::<Result<_>>()
			}
			Err(err) => Err(err.into()),
		}
	}
}
