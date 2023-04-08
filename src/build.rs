//! Builds a revision.
//!
//! Collect the local file information and builds the metadata.

use std::sync::mpsc;

use diesel::Connection;

use crate::{
    asset::Asset,
    models::{
        input_file::NewInputFile, revision::Revision, revision_file::NewRevisionFile,
        route::NewRoute, DbConn,
    },
};

pub fn create_revision(
    evt_rx: &mpsc::Receiver<Asset>,
    conn: &mut DbConn,
) -> anyhow::Result<Revision> {
    conn.transaction(|conn| {
        let rev = Revision::create(conn)?;

        // TODO: Should receive a "Done" event to commit the transaction
        while let Ok(asset) = evt_rx.recv() {
            let is_inline = asset.meta.is_inline();

            let new_input_file = NewInputFile::new(
                &asset.meta.logical_path,
                asset.hash.as_bytes().as_slice(),
                is_inline.then_some(&asset.contents),
            );
            new_input_file.create(conn)?;

            if !is_inline {
                // TODO: Copy file to the cache as the content hash name
            }

            NewRevisionFile::new(rev.id, &new_input_file.id).create(conn)?;

            if let Some(static_path) = new_input_file.logical_path.strip_prefix("static/") {
                tracing::trace!("Adding static route: {}", static_path);
                NewRoute::new(rev.id, static_path, &new_input_file.id).create(conn)?;
            }
        }

        Ok(rev)
    })
}
