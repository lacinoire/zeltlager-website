use std::env;

use actix::prelude::*;
use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;

use Result;

pub mod models;
// Generate with `diesel print-schema > src/db/schema.rs`
pub mod schema;

pub struct DbExecutor {
	connection: PgConnection,
}

impl Actor for DbExecutor {
	type Context = SyncContext<Self>;
}

pub struct SignupMessage {
	pub member: models::Teilnehmer,
}

impl Message for SignupMessage {
	type Result = Result<()>;
}

pub struct SignupBetreuerMessage {
	pub betreuer: models::Betreuer,
}

impl Message for SignupBetreuerMessage {
	type Result = Result<()>;
}

pub struct CountMemberMessage;

impl Message for CountMemberMessage {
	type Result = Result<i64>;
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

impl Handler<SignupBetreuerMessage> for DbExecutor {
	type Result = Result<()>;

	fn handle(
		&mut self,
		msg: SignupBetreuerMessage,
		_: &mut Self::Context,
	) -> Self::Result {
		use self::schema::betreuer;

		diesel::insert_into(betreuer::table)
			.values(&msg.betreuer)
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

		Ok(teilnehmer::table
			.count()
			.get_result(&self.connection)?)
	}
}
