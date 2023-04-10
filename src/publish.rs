//! Publishes a build for distribution.

use std::{fs, path::Path};

use diesel::prelude::*;
use handlebars::Handlebars;
use itertools::Itertools;
use pulldown_cmark::{html, Options, Parser};
use serde_json::json;

use crate::models::{
    input_file::{InputFile, Ty},
    page::Page,
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

    let mut templates = Handlebars::new();

    for r in routes {
        let dest_path = dest.join(Path::new(&r.route));
        let input_file = InputFile::by_id(&r.input_file_id).get_result(conn)?;

        match input_file.ty() {
            Ty::Content(_) => {
                if let Some(contents) = input_file.contents {
                    let page = Page::by_input_file_id(&input_file.id).get_result(conn)?;

                    if let Some(template_name) = page.template {
                        if !templates.has_template(&template_name) {
                            let template = InputFile::template(rev, &template_name, conn)?;
                            let template_contents = template.contents.unwrap();
                            let template_string = core::str::from_utf8(&template_contents)?;
                            templates.register_template_string(&template_name, template_string)?;
                        }

                        let (_, contents) = contents.split_at(usize::try_from(page.offset)?);
                        let contents = core::str::from_utf8(contents)?;

                        let options = Options::empty();
                        let parser = Parser::new_ext(contents, options);

                        let mut contents = String::new();
                        html::push_html(&mut contents, parser);

                        let html_output =
                            templates.render(&template_name, &json!({ "content": contents }))?;

                        tracing::trace!("Writing content to file: {}", dest_path.display());
                        fs::write(dest_path, html_output)?;
                    } else {
                        todo!();
                    }
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
