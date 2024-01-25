use std::{
    io,
    path::{Path, PathBuf},
};

use clap::{command, Parser, Subcommand};

use models::DbPool;
use tracing::info;

mod asset;
mod build;
mod content;
mod models;
mod publish;
#[allow(clippy::wildcard_imports)]
mod schema;
mod sqlite_mapping;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "./.cache")]
    cache_dir: PathBuf,
    #[arg(long, env)]
    database_url: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create a new revision of the site.
    Create {
        #[arg(short, long, default_value = "./")]
        src: PathBuf,
    },
    /// Publish a revision of the site.
    Publish {
        #[arg(short, long, default_value = "./build")]
        dest: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let pool = models::establish_connection_pool(&args.database_url)?;

    {
        let mut conn = pool.get()?;
        models::run_migrations(&mut conn).expect("migrations could not be run");
    }

    match args.command {
        Command::Create { src } => create(&src, pool)?,
        Command::Publish { dest } => publish(&dest)?,
    }

    Ok(())
}

fn create(src: &Path, pool: DbPool) -> anyhow::Result<()> {
    assert!(src.is_dir());

    info!("Scanning {}", src.display());

    let rev = asset::walk(src, |evt_tx| {
        let mut conn = pool.get()?;
        build::create_revision(&evt_tx, &mut conn)
    })?;

    info!("Created revision {}", rev.id);

    Ok(())
}

fn publish(_dest: &Path) -> io::Result<()> {
    todo!()

    // let cache_dir = &args.cache_dir;
    // if cache_dir.exists() {
    //     assert!(cache_dir.is_dir());
    // } else {
    //     fs::create_dir_all(cache_dir)?;
    // }

    // info!("Building {} to {}", src.display(), dest.display(),);

    // let mut conn = pool.get()?;

    // publish::dist_revision(dest, &rev, cache_dir, &mut conn)?;
}
