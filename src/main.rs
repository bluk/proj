use std::{fs, path::PathBuf};

use clap::{command, Parser};

use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod asset;
mod build;
mod content;
mod models;
mod publish;
mod schema;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "./")]
    src: PathBuf,
    #[arg(short, long, default_value = "./build")]
    dest: PathBuf,
    #[arg(short, long, default_value = "./.cache")]
    cache_dir: PathBuf,
    #[arg(long, env)]
    database_url: String,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    let pool = models::establish_connection_pool(&args.database_url)?;

    {
        let mut conn = pool.get()?;
        models::run_migrations(&mut conn).expect("migrations could not be run");
    }

    let src = &args.src;
    assert!(src.is_dir());
    let dest = &args.dest;

    let cache_dir = &args.cache_dir;
    if cache_dir.exists() {
        assert!(cache_dir.is_dir());
    } else {
        fs::create_dir_all(cache_dir)?;
    }

    info!("Building {} to {}", src.display(), dest.display(),);

    let rev = asset::walk(src, |evt_tx| {
        let mut conn = pool.get()?;

        build::create_revision(&evt_tx, cache_dir, &mut conn)
    })?;

    info!("Created revision {}", rev.id);

    let mut conn = pool.get()?;

    publish::dist_revision(dest, &rev, cache_dir, &mut conn)?;

    Ok(())
}
