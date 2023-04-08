use std::path::{Path, PathBuf};

use clap::{command, Parser};

use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod asset;
mod build;
mod models;
mod publish;
mod schema;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    src: Option<PathBuf>,
    #[arg(short, long)]
    dest: Option<PathBuf>,
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

    let src = args.src.as_deref().unwrap_or_else(|| Path::new("./"));
    assert!(src.is_dir());
    let dest = args.dest.as_deref().unwrap_or_else(|| Path::new("./build"));

    info!("Building {} to {}", src.display(), dest.display(),);

    let rev = asset::walk(src, |evt_tx| {
        let mut conn = pool.get()?;

        build::create_revision(&evt_tx, &mut conn)
    })?;

    info!("Created revision {}", rev.id);

    let mut conn = pool.get()?;

    publish::dist_revision(dest, &rev, &mut conn)?;

    Ok(())
}
