//! Builds a revision.
//!
//! Collect the local file information and builds the metadata.

use std::{fs, path::Path, sync::mpsc};

use diesel::Connection;
use itertools::Itertools;

use crate::{
    asset::Asset,
    models::{
        input_file::{self, NewInputFile, Ty},
        revision::Revision,
        revision_file::NewRevisionFile,
        route::NewRoute,
        DbConn,
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

            match input_file::ty(&asset.meta.logical_path) {
                Ty::Content(path) => {
                    if let Some(path) = path.strip_suffix(".md") {
                        let path = format!("{path}.html");
                        tracing::trace!("Adding content route: {}", path);
                        NewRoute::new(rev.id, &path, new_input_file.id).create(conn)?;
                    }
                }
                Ty::Static(path) => {
                    tracing::trace!("Adding static route: {}", path);
                    NewRoute::new(rev.id, path, new_input_file.id).create(conn)?;
                }
                Ty::Template(_) => {}
                Ty::Unknown => {
                    todo!()
                }
            }
        }

        Ok(rev)
    })
}
