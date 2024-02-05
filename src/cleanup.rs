use std::{fs, path::Path};

use crate::{
    models::{
        input_file::{with_no_revision_file, InputFile},
        DbConn,
    },
    schema::input_files,
};

use diesel::prelude::*;
use itertools::Itertools;

pub fn cleanup(cache_dir: &Path, conn: &mut DbConn) -> anyhow::Result<()> {
    conn.transaction(|conn| {
        let files = InputFile::with_no_revision_file().load(conn)?;

        for input_file in files {
            let is_inline = input_file.contents.is_some();
            if !is_inline {
                let contents_hash =
                    blake3::Hash::from_bytes(input_file.contents_hash.try_into().unwrap());
                let content_hash_string =
                    format!("{:x}", contents_hash.as_bytes().iter().format(""));

                let cache_path = cache_dir.join(&content_hash_string);
                let display_path = cache_path.to_string_lossy();
                if !cache_path.exists() {
                    tracing::error!(path = %display_path, id = %input_file.id, "Cache path does not exist for input file");
                } else {
                    tracing::info!(path = %display_path, "Removed file.");
                    fs::remove_file(&cache_path)?;
                }
            }
        }

        diesel::delete(input_files::dsl::input_files.filter(with_no_revision_file())).execute(conn)?;

        Ok::<_, anyhow::Error>(())
    })?;

    Ok(())
}
