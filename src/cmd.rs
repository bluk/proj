use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};

use clap::Subcommand;
use diesel::prelude::*;
use tracing::info;
use url::Url;

use crate::{
    asset, build, cleanup, delete,
    models::{
        revision::{self, Revision},
        DbPool,
    },
    publish,
};

#[derive(Debug, Subcommand)]
pub enum Command {
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

pub fn create(src: &Path, cache_dir: &Path, pool: DbPool) -> anyhow::Result<()> {
    assert!(src.is_dir());

    info!("Scanning {}", src.display());

    let rev = asset::walk(src, |evt_tx| {
        let mut conn = pool.get()?;
        build::create_revision(cache_dir, &evt_tx, &mut conn)
    })?;

    info!("Created revision {}", rev.id);

    Ok(())
}

pub fn publish(
    revision: Option<i64>,
    base_url: &Url,
    build_dir: &Path,
    cache_dir: &Path,
    pool: DbPool,
) -> anyhow::Result<()> {
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

pub fn delete(revision: i64, pool: DbPool) -> anyhow::Result<()> {
    let mut conn = pool.get()?;

    let rev = Revision::by_id(revision::Id(revision)).get_result(&mut conn)?;

    info!("Deleting revision {}", rev.id);

    delete::delete(&rev, &mut conn)?;

    Ok(())
}

pub fn cleanup(cache_dir: &Path, pool: DbPool) -> anyhow::Result<()> {
    let mut conn = pool.get()?;

    info!("Cleaning up");

    cleanup::cleanup(cache_dir, &mut conn)?;

    Ok(())
}
