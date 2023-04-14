//! Publishes a build for distribution.

use std::{fs, path::Path};

use diesel::prelude::*;
use handlebars::Handlebars;
use itertools::Itertools;
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
use lol_html::{HtmlRewriter, Settings};
use pulldown_cmark::{html, Options, Parser};
use serde_json::json;
use url::Url;

use crate::models::{
    input_file::{InputFile, Ty},
    page::Page,
    revision::Revision,
    route::Route,
    DbConn,
};

fn rewrite_html(
    html: &[u8],
    route: &str,
    rev: &Revision,
    conn: &mut DbConn,
) -> anyhow::Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut rewriter = HtmlRewriter::new(
        Settings {
            element_content_handlers: vec![lol_html::element!("link[href]", |el| {
                let href = el.get_attribute("href").expect("href was required");
                match Url::parse(&href) {
                    Err(url::ParseError::RelativeUrlWithoutBase) => {
                        let base = Url::parse("https://localhost/").unwrap().join(route)?;
                        let url = base.join(&href)?;
                        let path = url.path();
                        let route = InputFile::asset_route(rev, path, conn)?;
                        el.set_attribute("href", &route)?;
                    }
                    Ok(_) | Err(_) => {}
                }

                Ok(())
            })],
            ..Settings::default()
        },
        |c: &[u8]| output.extend_from_slice(c),
    );
    rewriter.write(html)?;
    rewriter.end()?;
    Ok(output)
}

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
        if let Some(parent) = dest_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        let input_file = InputFile::by_id(&r.input_file_id).get_result(conn)?;

        match input_file.ty() {
            Ty::Asset(path) => {
                debug_assert!(input_file.contents.is_none());

                if Path::new(path)
                    .extension()
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("css"))
                {
                    let content_hash_string =
                        format!("{:x}", input_file.contents_hash.iter().format(""));
                    let cache_path = cache_dir.join(content_hash_string);
                    assert!(cache_path.exists());
                    tracing::trace!(
                        "Writing file {} to {}",
                        cache_path.display(),
                        dest_path.display()
                    );

                    let stylesheet_contents = fs::read_to_string(cache_path)?;

                    let parser_options = ParserOptions::default();
                    let mut stylesheet =
                        StyleSheet::parse(&stylesheet_contents, parser_options).unwrap();
                    stylesheet.minify(MinifyOptions::default())?;

                    let output = stylesheet.to_css(PrinterOptions::default())?;
                    fs::write(dest_path, output.code)?;
                }
            }
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

                        let output = rewrite_html(html_output.as_bytes(), &r.route, rev, conn)?;

                        tracing::trace!("Writing content to file: {}", dest_path.display());
                        fs::write(dest_path, output)?;
                    } else {
                        todo!();
                    }
                } else {
                    unreachable!("content was not in database");
                }
            }
            Ty::Static(path) => {
                if let Some(contents) = &input_file.contents {
                    if Path::new(path)
                        .extension()
                        .map_or(false, |ext| ext.eq_ignore_ascii_case("html"))
                    {
                        tracing::trace!("Writing file: {}", dest_path.display());
                        let contents = rewrite_html(contents, &r.route, rev, conn)?;
                        fs::write(dest_path, contents)?;
                    } else {
                        tracing::trace!("Writing file: {}", dest_path.display());
                        fs::write(dest_path, contents)?;
                    }
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
