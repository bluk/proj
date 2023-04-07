use std::{
    env, fs,
    path::{Path, PathBuf},
};

use clap::{command, Parser};
use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::asset::Config;

mod asset;
mod models;
mod schema;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    src: Option<PathBuf>,
    #[arg(short, long)]
    dest: Option<PathBuf>,
}

#[must_use]
fn establish_connection_pool(url: &str) -> Pool<ConnectionManager<SqliteConnection>> {
    let manager = ConnectionManager::<SqliteConnection>::new(url);
    Pool::builder()
        .test_on_check_out(true)
        .build(manager)
        .expect("could not build connection pool")
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL was not set");
    let pool = establish_connection_pool(&database_url);

    let args = Args::parse();

    let src = args.src.as_deref().unwrap_or_else(|| Path::new("./"));
    assert!(src.is_dir());

    let dest = args.dest.as_deref().unwrap_or_else(|| Path::new("./build"));
    if dest.exists() {
        assert!(dest.is_dir());
    } else {
        fs::create_dir_all(dest)?;
    }

    info!("Building {} to {}", src.display(), dest.display(),);

    asset::walk(&Config { src, dest }, &pool)?;

    Ok(())
}
