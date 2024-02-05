use std::error::Error;

use diesel::{
    backend::Backend,
    migration::MigrationVersion,
    prelude::*,
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

#[derive(Debug)]
pub struct DbPool {
    inner: Pool<ConnectionManager<SqliteConnection>>,
}

pub fn establish_connection_pool(url: &str) -> Result<DbPool, r2d2::Error> {
    let manager = ConnectionManager::new(url);
    Ok(DbPool {
        inner: Pool::builder().test_on_check_out(true).build(manager)?,
    })
}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn run_migrations<T, DB>(
    conn: &mut T,
) -> Result<Vec<MigrationVersion>, Box<dyn Error + Send + Sync + 'static>>
where
    T: MigrationHarness<DB>,
    DB: Backend,
{
    conn.run_pending_migrations(MIGRATIONS)
}

impl DbPool {
    pub fn get(
        &self,
    ) -> Result<r2d2::PooledConnection<ConnectionManager<SqliteConnection>>, r2d2::Error> {
        let mut conn = self.inner.get()?;
        diesel::sql_query("PRAGMA foreign_keys = ON")
            .execute(&mut conn)
            .expect("foreign keys should be enabled");
        Ok(conn)
    }
}
