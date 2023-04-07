use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};

pub mod input_file;
pub mod revision;
pub mod revision_file;

pub type DbId = i32;
pub type DbConn = SqliteConnection;
pub type DbPool = Pool<ConnectionManager<DbConn>>;
