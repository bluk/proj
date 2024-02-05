use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::{command, Parser, Subcommand};

use models::DbPool;
use tracing::info;
use url::Url;

use crate::models::revision::{self, Revision};

mod asset;
mod build;
mod cleanup;
mod content;
mod delete;
mod models;
mod publish;
#[allow(clippy::wildcard_imports)]
mod schema;
mod sqlite_mapping;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, env, default_value = "./.cache")]
    cache_dir: PathBuf,
    #[arg(short, long, env, default_value = "./site.db")]
    database_url: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create a new revision of the site.
    Create {
        #[arg(short, long, default_value = "./")]
        src_dir: PathBuf,
    },
    /// Publish a revision of the site.
    Publish {
        /// Directory to publish the build.
        #[arg(long, default_value = "https://127.0.0.1")]
        base_url: Url,
        /// Directory to publish the build.
        #[arg(short, long, default_value = "./build")]
        build_dir: PathBuf,
        /// Revision to publish.
        #[arg(short, long)]
        revision: Option<i64>,
    },
    /// Deletes a revision
    Delete {
        #[arg(short, long)]
        revision: i64,
    },
    /// Removes unreachable data in the database and cache.
    Cleanup,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    if args.cache_dir.exists() {
        assert!(args.cache_dir.is_dir());
    } else {
        fs::create_dir_all(&args.cache_dir)?;
    }

    let pool = models::establish_connection_pool(&args.database_url)?;

    {
        let mut conn = pool.get()?;
        models::run_migrations(&mut conn).expect("migrations could not be run");
    }

    match args.command {
        Command::Create { src_dir } => create(&src_dir, &args.cache_dir, pool)?,
        Command::Publish {
            base_url,
            build_dir,
            revision,
        } => publish(revision, &base_url, &build_dir, &args.cache_dir, pool)?,
        Command::Delete { revision } => delete(revision, pool)?,
        Command::Cleanup => cleanup(&args.cache_dir, pool)?,
    }

    Ok(())
}

fn create(src: &Path, cache_dir: &Path, pool: DbPool) -> anyhow::Result<()> {
    assert!(src.is_dir());

    info!("Scanning {}", src.display());

    let rev = asset::walk(src, |evt_tx| {
        let mut conn = pool.get()?;
        build::create_revision(cache_dir, &evt_tx, &mut conn)
    })?;

    info!("Created revision {}", rev.id);

    Ok(())
}

fn publish(
    revision: Option<i64>,
    base_url: &Url,
    build_dir: &Path,
    cache_dir: &Path,
    pool: DbPool,
) -> anyhow::Result<()> {
    use diesel::prelude::*;

    let mut conn = pool.get()?;

    let rev = if let Some(revision) = revision {
        Revision::by_id(revision::Id(revision)).get_result(&mut conn)?
    } else {
        Revision::order_by_created_at_desc().first(&mut conn)?
    };

    info!("Building revision {} at {}", rev.id, build_dir.display());

    publish::dist_revision(build_dir, &rev, base_url, cache_dir, &mut conn)?;

    Ok(())
}

fn delete(revision: i64, pool: DbPool) -> anyhow::Result<()> {
    use diesel::prelude::*;

    let mut conn = pool.get()?;

    let rev = Revision::by_id(revision::Id(revision)).get_result(&mut conn)?;

    info!("Deleting revision {}", rev.id);

    delete::delete(&rev, &mut conn)?;

    Ok(())
}

fn cleanup(cache_dir: &Path, pool: DbPool) -> anyhow::Result<()> {
    use diesel::prelude::*;

    let mut conn = pool.get()?;

    info!("Cleaning up");

    cleanup::cleanup(cache_dir, &mut conn)?;

    Ok(())
}
