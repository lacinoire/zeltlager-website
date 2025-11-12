//! All Database requests are implemented here
//! for each request you need:
//! - A Message struct with a `Message` Implementation
//! - A Handler method for the DbExecutor

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};

use anyhow::{Result, bail, format_err};
use diesel::prelude::*;
use diesel::result::Error;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::{AsyncMigrationHarness, AsyncPgConnection, RunQueryDsl};
use diesel_migrations::MigrationHarness;
use ipnetwork::IpNetwork;
use scrypt::Scrypt;
use scrypt::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use serde::Serialize;
use time::{OffsetDateTime, PrimitiveDateTime};
use tracing::{info, warn};

use crate::auth;

#[macro_export]
macro_rules! get_str {
	($map:ident, $key:expr) => {
		$map.remove($key).ok_or_else(|| crate::db::FormError {
			field: Some($key.into()),
			message: format!("{} fehlt", heck::ToTitleCase::to_title_case($key)),
		})
	};
}

#[macro_export]
macro_rules! get_freetext_str {
	($map:ident, $key:expr) => {
		$map.remove($key).map(crate::db::models::cleanup_freetext).ok_or_else(|| {
			crate::db::FormError {
				field: Some($key.into()),
				message: format!("{} fehlt", heck::ToTitleCase::to_title_case($key)),
			}
		})
	};
}

pub use get_freetext_str;
pub use get_str;

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

#[derive(Clone)]
pub struct Database {
	pool: Pool<AsyncPgConnection>,
}

pub struct Authentication {
	pub username: String,
	pub password: String,
}

impl Authentication {
	pub fn from_hashmap(mut map: HashMap<String, String>) -> Result<Self, FormError> {
		let res =
			Self { username: get_str!(map, "username")?, password: get_str!(map, "password")? };

		map.remove("submit");
		if !map.is_empty() {
			warn!(?map, "Authentication::from_hashmap: Map is not yet empty");
		}

		Ok(res)
	}
}

impl From<String> for FormError {
	fn from(s: String) -> Self { Self { field: None, message: s } }
}

impl<'a> From<&'a str> for FormError {
	fn from(s: &'a str) -> Self { Self { field: None, message: s.into() } }
}

impl Database {
	pub fn new(config: &crate::Config) -> Result<Self> {
		let config = diesel_async::pooled_connection::AsyncDieselConnectionManager::<
			AsyncPgConnection,
		>::new(&config.database);
		let pool: Pool<AsyncPgConnection> = Pool::builder(config).build()?;
		Ok(Self { pool })
	}

	/// Get a connection from the pool.
	pub async fn get(
		&self,
	) -> Result<diesel_async::pooled_connection::deadpool::Object<AsyncPgConnection>> {
		Ok(self.pool.get().await?)
	}

	pub async fn run_migrations(&self) -> Result<()> {
		let mut migrations = AsyncMigrationHarness::new(self.get().await?);
		let migrated = migrations
			.run_pending_migrations(MIGRATIONS)
			.map_err(|e| format_err!("Failed to run migrations: {}", e))?;
		if !migrated.is_empty() {
			info!(?migrated, "Run database migrations");
		}

		Ok(())
	}

	pub async fn check_rate(&self, ip: &str) -> Result<bool> {
		use self::schema::rate_limiting::dsl::*;
		use diesel::dsl::insert_into;

		let mut connection = self.get().await?;

		let parse_result = ip.parse::<SocketAddr>();
		let ip: IpNetwork = match parse_result {
			Ok(result) => result.ip().into(),
			Err(_) => ip.parse::<IpAddr>()?.into(),
		};
		let entry_res = rate_limiting.find(ip).first::<models::RateLimiting>(&mut connection).await;
		// check for no entry found
		match entry_res {
			Ok(entry) => {
				let now = OffsetDateTime::now_utc();
				let now = PrimitiveDateTime::new(now.date(), now.time());
				if entry.first_count <= now - crate::ratelimit_duration() {
					// reset counter and grant request
					diesel::update(&entry).set(counter.eq(1)).execute(&mut connection).await?;
					diesel::update(&entry)
						.set(first_count.eq(diesel::dsl::now.at_time_zone("utc")))
						.execute(&mut connection)
						.await?;
					Ok(true)
				} else if entry.counter >= crate::RATELIMIT_MAX_COUNTER {
					// limit reached
					Ok(false)
				} else {
					diesel::update(&entry)
						.set(counter.eq(counter + 1))
						.execute(&mut connection)
						.await?;
					Ok(true)
				}
			}
			Err(Error::NotFound) => {
				insert_into(rate_limiting)
					.values((
						ip_addr.eq(ip),
						counter.eq(1),
						first_count.eq(diesel::dsl::now.at_time_zone("utc")),
					))
					.execute(&mut connection)
					.await?;
				Ok(true)
			}
			Err(e) => Err(e.into()),
		}
	}

	pub async fn decrease_rate_counter(&self, ip: &str) -> Result<()> {
		use self::schema::rate_limiting::dsl::*;

		let mut connection = self.get().await?;

		let parse_result = ip.parse::<SocketAddr>();
		let ip: IpNetwork = match parse_result {
			Ok(result) => result.ip().into(),
			Err(_) => ip.parse::<IpAddr>()?.into(),
		};
		let entry_res = rate_limiting.find(ip).first::<models::RateLimiting>(&mut connection).await;
		// check for no entry found
		match entry_res {
			Ok(entry) => {
				diesel::update(&entry)
					.set(counter.eq(counter - 1))
					.execute(&mut connection)
					.await?;
				Ok(())
			}
			Err(Error::NotFound) => {
				bail!("Ip to decrease rate counter for not found in db")
			}
			Err(e) => Err(e.into()),
		}
	}

	pub async fn count_members(&self) -> Result<i64> {
		use self::schema::teilnehmer;

		Ok(teilnehmer::table.count().get_result(&mut self.get().await?).await?)
	}

	pub async fn signup_supervisor(
		&self, supervisor: &models::Supervisor, is_pre_signup: bool,
	) -> Result<()> {
		use self::schema::betreuer;
		use self::schema::betreuer::columns::*;

		let mut connection = self.get().await?;

		// Check if the e-mail already exists
		let supervisor_id = match betreuer::table
			.filter(mail.eq(&supervisor.mail))
			.select(id)
			.first::<i32>(&mut connection)
			.await
		{
			Err(diesel::result::Error::NotFound) => None,
			Err(e) => return Err(e.into()),
			Ok(supervisor) => Some(supervisor),
		};

		if is_pre_signup && supervisor_id.is_some() {
			// Disable updating for pre-signups
			bail!("E-mail address is already registered");
		}

		if let Some(supervisor_id) = supervisor_id {
			// Update
			diesel::update(betreuer::table)
				.filter(id.eq(&supervisor_id))
				.set((
					supervisor,
					anmeldedatum.eq(diesel::dsl::now),
					signup_token.eq(None::<String>),
					signup_token_time.eq(None::<PrimitiveDateTime>),
				))
				.execute(&mut connection)
				.await?;
		} else {
			// Insert new
			diesel::insert_into(betreuer::table)
				.values(supervisor)
				.execute(&mut connection)
				.await?;
		}

		Ok(())
	}

	/// Returns `None` when the user was not found or the password is incorrect
	/// or `Some(<id>)` if authentication succeeded.
	pub async fn authenticate(&self, msg: &Authentication) -> Result<Option<i32>> {
		use self::schema::users::dsl::*;

		let mut connection = self.get().await?;

		// Fetch user from db
		match users
			.filter(username.eq(&msg.username))
			.first::<models::UserQueryResult>(&mut connection)
			.await
		{
			Ok(user) => {
				if PasswordHash::new(&user.password)
					.and_then(|hash| scrypt::Scrypt.verify_password(msg.password.as_bytes(), &hash))
					.is_ok()
				{
					Ok(Some(user.id))
				} else {
					Ok(None)
				}
			}
			Err(Error::NotFound) => {
				// Hash a random password so we don’t leak much timing information if a user exists
				// or not.
				let salt = SaltString::generate(&mut rand::thread_rng());
				let pw = Scrypt.hash_password(msg.username.as_bytes(), &salt)?.to_string();
				let hash = PasswordHash::new(&pw)?;
				let _ = Scrypt.verify_password(msg.password.as_bytes(), &hash);
				Ok(None)
			}
			Err(err) => Err(err.into()),
		}
	}

	pub async fn get_roles(&self, user: i32) -> Result<Vec<auth::Roles>> {
		use self::schema::roles::dsl::*;

		let mut connection = self.get().await?;

		// Fetch user from db
		match roles.filter(user_id.eq(user)).get_results::<models::Role>(&mut connection).await {
			Ok(mut res) => {
				// Convert to enum
				res.drain(..).map(|r| r.role.parse()).collect::<Result<_>>()
			}
			Err(err) => Err(err.into()),
		}
	}
}
