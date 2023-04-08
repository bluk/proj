use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};

pub mod input_file;
pub mod revision;
pub mod revision_file;
pub mod route;

pub type DbId = i32;
pub type DbConn = SqliteConnection;
pub type DbPool = Pool<ConnectionManager<DbConn>>;

pub fn establish_connection_pool(
    url: &str,
) -> Result<Pool<ConnectionManager<SqliteConnection>>, r2d2::Error> {
    let manager = ConnectionManager::<SqliteConnection>::new(url);
    Pool::builder().test_on_check_out(true).build(manager)
}
