use std::error::Error;

use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub mod input_file;
pub mod page;
pub mod revision;
pub mod revision_file;
pub mod route;

pub type DbId = i64;
pub type DbConn = SqliteConnection;
pub type DbPool = Pool<ConnectionManager<DbConn>>;

pub fn establish_connection_pool(
    url: &str,
) -> Result<Pool<ConnectionManager<SqliteConnection>>, r2d2::Error> {
    let manager = ConnectionManager::<SqliteConnection>::new(url);
    Pool::builder().test_on_check_out(true).build(manager)
}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn run_migrations(conn: &mut DbConn) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    conn.run_pending_migrations(MIGRATIONS)?;

    Ok(())
}
