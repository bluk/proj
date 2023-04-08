//! Publishes a build for distribution.

use std::{fs, path::Path};

use diesel::prelude::*;
use itertools::Itertools;
use pulldown_cmark::{html, Options, Parser};

use crate::models::{
    input_file::{InputFile, Ty},
    revision::Revision,
    route::Route,
    DbConn,
};

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
        let dest_path = dest.join(Path::new(&r.route));
        let input_file = InputFile::by_id(&r.input_file_id).get_result(conn)?;

        // TODO: Determine if from the static files and do the write/copy then

        match input_file.ty() {
            Ty::Content(_) => {
                if let Some(contents) = input_file.contents {
                    let contents = core::str::from_utf8(&contents)?;
                    let options = Options::empty();
                    let parser = Parser::new_ext(contents, options);

                    let mut html_output = String::new();
                    html::push_html(&mut html_output, parser);

                    tracing::trace!("Writing content to file: {}", dest_path.display());
                    fs::write(dest_path, html_output)?;
                } else {
                    unreachable!("content was not in database");
                }
            }
            Ty::Static(_) => {
                if let Some(contents) = input_file.contents {
                    tracing::trace!("Writing file: {}", dest_path.display());
                    fs::write(dest_path, contents)?;
                } else {
                    let content_hash_string =
                        format!("{:x}", input_file.contents_hash.iter().format(""));
                    let cache_path = cache_dir.join(content_hash_string);
                    assert!(cache_path.exists());
                    tracing::trace!(
                        "Copying file {} to {}",
                        cache_path.display(),
                        dest_path.display()
                    );
                    fs::copy(cache_path, dest_path)?;
                }
            }
            Ty::Template(_) => {}
            Ty::Unknown => {
                todo!()
            }
        }
    }

    Ok(())
}
