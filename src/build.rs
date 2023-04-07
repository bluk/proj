//! Builds a revision.
//!
//! Collect the local file information and builds the metadata.

use std::sync::mpsc;

use diesel::Connection;

use crate::{
    asset::Asset,
    models::{input_file::NewInputFile, revision, revision_file::NewRevisionFile, DbConn},
};

pub fn create_revision(
    evt_rx: &mpsc::Receiver<Asset>,
    conn: &mut DbConn,
) -> anyhow::Result<revision::Id> {
    conn.transaction(|conn| {
        let rev_id = revision::create(conn)?;

        while let Ok(asset) = evt_rx.recv() {
            let is_inline = asset.meta.is_inline();

            let new_input_file = NewInputFile::new(
                &asset.meta.logical_path,
                asset.hash.as_bytes().as_slice(),
                is_inline.then_some(&asset.contents),
            );

            if !is_inline {
                // TODO: Copy file to the cache as the content hash name
            }

            new_input_file.create(conn)?;

            NewRevisionFile::new(rev_id, &new_input_file.id).create(conn)?;
        }

        Ok(rev_id)
    })
}
