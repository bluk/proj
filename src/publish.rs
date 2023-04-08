//! Publishes a build for distribution.

use std::{fs, path::Path};

use diesel::prelude::*;
use itertools::Itertools;

use crate::models::{input_file::InputFile, revision::Revision, route::Route, DbConn};

pub fn dist_revision(
    dest: &Path,
    rev: &Revision,
    cache_dir: &Path,
    conn: &mut DbConn,
) -> anyhow::Result<()> {
    if dest.exists() {
        assert!(dest.is_dir());
    } else {
        fs::create_dir_all(dest)?;
    }

    let routes = Route::with_revision(rev, conn)?;

    for r in routes {
        let path = dest.join(Path::new(&r.route));
        let input_file = InputFile::by_id(&r.input_file_id).get_result(conn)?;

        if let Some(contents) = input_file.contents {
            tracing::trace!("Writing file: {}", path.display());
            fs::write(path, contents)?;
        } else {
            let content_hash_string = format!("{:x}", input_file.contents_hash.iter().format(""));
            let cache_path = cache_dir.join(content_hash_string);
            assert!(cache_path.exists());
            tracing::trace!(
                "Copying file {} to {}",
                cache_path.display(),
                path.display()
            );
            fs::copy(cache_path, path)?;
        }
    }

    Ok(())
}
