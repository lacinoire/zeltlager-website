//! All Database requests are implemented here
//! for each request you need:
//! - A Message struct with a `Message` Implementation
//! - A Handler method for the DbExecutor

use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;

use actix::prelude::*;
use chrono::Utc;
use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::Error;
use dotenv::dotenv;
use ipnetwork::IpNetwork;
use libpasta;

use Result;

macro_rules! get_str {
	($map:ident, $key:expr) => {
		$map.remove($key)
			.ok_or_else(|| format_err!("{} fehlt", $key))
	};
}

pub mod models;
// Generate with `diesel print-schema > src/db/schema.rs`
pub mod schema;

pub struct DbExecutor {
	connection: PgConnection,
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

impl AuthenticateMessage {
	pub fn from_hashmap(
		mut map: HashMap<String, String>,
	) -> Result<AuthenticateMessage> {
		Ok(AuthenticateMessage {
			username: get_str!(map, "username")?,
			password: get_str!(map, "password")?,
		})
	}
}

impl DbExecutor {
	pub fn new() -> Result<Self> {
		dotenv().ok();
		let database_url = env::var("DATABASE_URL").map_err(|e| {
			format_err!(
				"DATABASE_URL is not set, are you missing a .env file? ({:?})",
				e
			)
		})?;
		let connection = PgConnection::establish(&database_url)?;
		Ok(Self { connection })
	}
}

impl Handler<CheckRateMessage> for DbExecutor {
	type Result = Result<bool>;

	fn handle(
		&mut self,
		msg: CheckRateMessage,
		_: &mut Self::Context,
	) -> Self::Result {
		use self::schema::rate_limiting::dsl::*;
		use diesel::dsl::insert_into;
		use diesel::expression::dsl::now;

		let ip: IpNetwork = msg.ip.parse::<SocketAddr>()?.ip().into();
		let entry_res = rate_limiting
			.find(ip)
			.first::<models::RateLimiting>(&self.connection);
		// check for no entry found
		match entry_res {
			Ok(entry) => {
				if entry.first_count
					<= Utc::now().naive_utc() - ::ratelimit_duration()
				{
					// reset counter and grant request
					diesel::update(&entry)
						.set(counter.eq(1))
						.execute(&self.connection)?;
					diesel::update(&entry)
						.set(first_count.eq(now))
						.execute(&self.connection)?;
					Ok(true)
				} else if entry.counter >= ::RATELIMIT_MAX_COUNTER {
					// limit reached
					Ok(false)
				} else {
					diesel::update(&entry)
						.set(counter.eq(counter + 1))
						.execute(&self.connection)?;
					Ok(true)
				}
			}
			Err(Error::NotFound) => {
				// TODO Work in UTC?
				insert_into(rate_limiting)
					.values((
						ip_addr.eq(ip),
						counter.eq(1),
						first_count.eq(now),
					))
					.execute(&self.connection)?;
				Ok(true)
			}
			Err(e) => Err(e.into()),
		}
	}
}

impl Handler<DecreaseRateCounterMessage> for DbExecutor {
	type Result = Result<()>;

	fn handle(
		&mut self,
		msg: DecreaseRateCounterMessage,
		_: &mut Self::Context,
	) -> Self::Result {
		use self::schema::rate_limiting::dsl::*;

		let ip: IpNetwork = msg.ip.parse::<SocketAddr>()?.ip().into();
		let entry_res = rate_limiting
			.find(ip)
			.first::<models::RateLimiting>(&self.connection);
		// check for no entry found
		match entry_res {
			Ok(entry) => {
				diesel::update(&entry)
						.set(counter.eq(counter - 1))
						.execute(&self.connection)?;
				Ok(())
			}
			Err(Error::NotFound) => bail!("Ip to decrease rate counter for not found in db"),
			Err(e) => Err(e.into()),
		}
	}
}

impl Handler<SignupMessage> for DbExecutor {
	type Result = Result<()>;

	fn handle(
		&mut self,
		msg: SignupMessage,
		_: &mut Self::Context,
	) -> Self::Result {
		use self::schema::teilnehmer;

		diesel::insert_into(teilnehmer::table)
			.values(&msg.member)
			.execute(&self.connection)?;

		Ok(())
	}
}

impl Handler<SignupSupervisorMessage> for DbExecutor {
	type Result = Result<()>;

	fn handle(
		&mut self,
		msg: SignupSupervisorMessage,
		_: &mut Self::Context,
	) -> Self::Result {
		use self::schema::betreuer;

		diesel::insert_into(betreuer::table)
			.values(&msg.supervisor)
			.execute(&self.connection)?;

		Ok(())
	}
}

impl Handler<CountMemberMessage> for DbExecutor {
	type Result = Result<i64>;

	fn handle(
		&mut self,
		_: CountMemberMessage,
		_: &mut Self::Context,
	) -> Self::Result {
		use self::schema::teilnehmer;

		Ok(teilnehmer::table.count().get_result(&self.connection)?)
	}
}

impl Handler<AuthenticateMessage> for DbExecutor {
	type Result = Result<Option<i32>>;

	fn handle(
		&mut self,
		msg: AuthenticateMessage,
		_: &mut Self::Context,
	) -> Self::Result {
		use self::schema::users::dsl::*;

		// Fetch user from db
		match users
			.filter(username.eq(msg.username))
			.first::<models::UserQueryResult>(&self.connection)
		{
			Ok(user) => {
				if libpasta::verify_password(&user.password, &msg.password) {
					Ok(Some(user.id))
				} else {
					Ok(None)
				}
			}
			// TODO Maybe sleep a random amount of time to hide that the user
			// exists?
			Err(Error::NotFound) => Ok(None),
			Err(err) => Err(err.into()),
		}
	}
}
