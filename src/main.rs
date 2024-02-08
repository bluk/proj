use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::{command, Parser};

use cmd::Command;
use models::DbPool;
use tracing::info;

mod asset;
mod build;
mod cleanup;
mod cmd;
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
        Command::Create { src_dir } => cmd::create(&src_dir, &args.cache_dir, pool),
        Command::Publish {
            base_url,
            build_dir,
            revision,
        } => cmd::publish(revision, &base_url, &build_dir, &args.cache_dir, pool),
        Command::Delete { revision } => cmd::delete(revision, pool),
        Command::Cleanup => cmd::cleanup(&args.cache_dir, pool),
    }
}
