use std::env;

use actix::prelude::*;
use diesel;
use diesel::prelude::*;
use diesel::pg::PgConnection;
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

impl DbExecutor {
    pub fn new() -> Result<Self> {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").map_err(|e|
            format_err!("DATABASE_URL is not set, are you missing a .env file? \
                ({:?})", e))?;
        let connection = PgConnection::establish(&database_url)?;
        Ok(Self {
            connection,
        })
    }
}

impl Handler<SignupMessage> for DbExecutor {
    type Result = Result<()>;

    fn handle(&mut self, msg: SignupMessage, _: &mut Self::Context) -> Self::Result {
        use self::schema::teilnehmer;

        diesel::insert_into(teilnehmer::table)
            .values(&msg.member)
            .execute(&self.connection)?;

        Ok(())
    }
}
