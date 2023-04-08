//! Builds a revision.
//!
//! Collect the local file information and builds the metadata.

use std::{fs, path::Path, sync::mpsc};

use diesel::Connection;
use itertools::Itertools;

use crate::{
    asset::Asset,
    models::{
        input_file::NewInputFile, revision::Revision, revision_file::NewRevisionFile,
        route::NewRoute, DbConn,
    },
};

pub fn create_revision(
    evt_rx: &mpsc::Receiver<Asset>,
    cache_dir: &Path,
    conn: &mut DbConn,
) -> anyhow::Result<Revision> {
    conn.transaction(|conn| {
        let rev = Revision::create(conn)?;

        // TODO: Should receive a "Done" event to commit the transaction
        while let Ok(asset) = evt_rx.recv() {
            let is_inline = asset.meta.is_inline();

            let content_hash_string = format!("{:x}", asset.hash.as_bytes().iter().format(""));
            let id = format!("{content_hash_string},{}", asset.meta.logical_path);

            let new_input_file = NewInputFile::new(
                &id,
                &asset.meta.logical_path,
                asset.hash.as_bytes().as_slice(),
                is_inline.then_some(&asset.contents),
            );
            new_input_file.create(conn)?;

            if !is_inline {
                let cache_path = cache_dir.join(&content_hash_string);
                if !cache_path.exists() {
                    tracing::trace!(
                        "Copying file {} to {}",
                        asset.meta.disk_path.display(),
                        cache_path.display()
                    );
                    fs::write(&cache_path, &**asset.contents)?;
                }
                debug_assert_eq!(
                    asset.contents.len() as u64,
                    cache_path.metadata().unwrap().len()
                );
            }

            NewRevisionFile::new(rev.id, new_input_file.id).create(conn)?;

            if let Some(static_path) = new_input_file.logical_path.strip_prefix("static/") {
                tracing::trace!("Adding static route: {}", static_path);
                NewRoute::new(rev.id, static_path, new_input_file.id).create(conn)?;
            }
        }

        Ok(rev)
    })
}
