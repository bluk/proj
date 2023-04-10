//! Builds a revision.
//!
//! Collect the local file information and builds the metadata.

use std::{fs, path::Path, sync::mpsc};

use diesel::Connection;
use itertools::Itertools;
use toml_edit::Document;

use crate::{
    asset::Asset,
    content,
    models::{
        input_file::{self, NewInputFile, Ty},
        page::NewPage,
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
            let created_input_file = new_input_file.create(conn)? != 0;

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

                        let contents = core::str::from_utf8(&asset.contents)?;
                        let (front_matter, content_offset, _) = content::parse(contents)?;

                        if created_input_file {
                            let mut page = NewPage {
                                input_file_id: &id,
                                front_matter,
                                offset: i32::try_from(content_offset)?,
                                date: None,
                                description: None,
                                excerpt: None,
                                draft: false,
                                expiry_date: None,
                                keywords: None,
                                template: None,
                                publish_date: None,
                                summary: None,
                                title: None,
                            };

                            if let Some(front_matter) = front_matter {
                                let doc = front_matter.parse::<Document>()?;
                                page.date = doc.get("date").and_then(|value| {
                                    value.as_datetime().map(content::convert_datetime)
                                });
                                page.description =
                                    doc.get("description").and_then(toml_edit::Item::as_str);
                                page.excerpt = doc.get("excerpt").and_then(toml_edit::Item::as_str);
                                page.draft = doc
                                    .get("draft")
                                    .and_then(toml_edit::Item::as_bool)
                                    .unwrap_or_default();
                                page.expiry_date = doc.get("expiry_date").and_then(|value| {
                                    value.as_datetime().map(content::convert_datetime)
                                });
                                page.keywords =
                                    doc.get("keywords").and_then(toml_edit::Item::as_str);
                                page.publish_date = doc.get("publish_date").and_then(|value| {
                                    value.as_datetime().map(content::convert_datetime)
                                });
                                page.summary = doc.get("summary").and_then(toml_edit::Item::as_str);
                                page.template =
                                    doc.get("template").and_then(toml_edit::Item::as_str);
                                page.title = doc.get("title").and_then(toml_edit::Item::as_str);

                                page.create(conn)?;
                            } else {
                                page.create(conn)?;
                            }
                        }
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
