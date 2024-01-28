//! Publishes a build for distribution.

use std::{fs, path::Path};

use diesel::prelude::*;
use handlebars::Handlebars;
use itertools::Itertools;
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

fn base_relative_href(
    base_url: &Url,
    route_abs_url: &Url,
    href: &str,
) -> Result<Option<String>, url::ParseError> {
    if !matches!(
        Url::parse(href),
        Err(url::ParseError::RelativeUrlWithoutBase)
    ) {
        return Ok(None);
    }

    let href_url = route_abs_url.join(href)?;
    let Some(rel_url) = base_url.make_relative(&href_url) else {
        return Ok(None);
    };

    if rel_url.starts_with("../") {
        return Ok(None);
    }

    Ok(Some(
        rel_url
            .strip_prefix('/')
            .map(|p| p.to_owned())
            .unwrap_or(rel_url),
    ))
}

fn route_relative_href(
    base_url: &Url,
    route_abs_url: &Url,
    path: &str,
) -> Result<Option<String>, url::ParseError> {
    Ok(route_abs_url.make_relative(&base_url.join(path)?))
}

fn rewrite_html(
    html: &[u8],
    base_url: &Url,
    route_rel_url: &str,
    rev: &Revision,
    cache_dir: &Path,
    conn: &mut DbConn,
) -> anyhow::Result<Vec<u8>> {
    let route_abs_url = base_url.join(route_rel_url)?;

    let html = rewrite_a_hrefs(html, base_url, &route_abs_url, rev, conn)?;
    let html = rewrite_link_hrefs(&html, base_url, &route_abs_url, cache_dir, rev, conn)?;
    Ok(html)
}

fn rewrite_a_hrefs(
    html: &[u8],
    base_url: &Url,
    route_abs_url: &Url,
    rev: &Revision,
    conn: &mut DbConn,
) -> anyhow::Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut rewriter = HtmlRewriter::new(
        Settings {
            element_content_handlers: vec![lol_html::element!("a[href]", |el| {
                let Some(href) = el.get_attribute("href") else {
                    unreachable!();
                };

                if let Ok(Some(path)) = base_relative_href(base_url, route_abs_url, &href) {
                    if let Some(_route) = Route::by_revision_id_and_route(rev.id, &path)
                        .first(conn)
                        .optional()?
                    {
                        // Path is a valid route
                    } else {
                        // TODO: See if an alias exists

                        if let Some(asset_input_file) =
                            InputFile::asset(rev, &path, conn).optional()?
                        {
                            if let Some(route) = Route::by_revision_id_and_input_file_id(
                                rev.id,
                                &asset_input_file.id,
                            )
                            .first(conn)
                            .optional()?
                            {
                                if let Some(href_value) =
                                    route_relative_href(base_url, route_abs_url, &route.route)?
                                {
                                    el.set_attribute("href", &href_value)?;
                                }
                            }
                        } else {
                            tracing::warn!(
                                "In revision {} route: {} a href: {} points to non-existent resource {}",
                                rev.id,
                                route_abs_url,
                                href,
                                path
                            )
                        }
                    }
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

fn rewrite_link_hrefs(
    html: &[u8],
    base_url: &Url,
    route_abs_url: &Url,
    cache_dir: &Path,
    rev: &Revision,
    conn: &mut DbConn,
) -> anyhow::Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut rewriter = HtmlRewriter::new(
        Settings {
            element_content_handlers: vec![lol_html::element!("link[href]", |el| {
                let Some(href) = el.get_attribute("href") else {
                    unreachable!();
                };

                if let Ok(Some(path)) = base_relative_href(base_url, route_abs_url, &href) {
                    if let Some(route) = Route::by_revision_id_and_route(rev.id, &path)
                        .first(conn)
                        .optional()?
                    {
                        // Path is a valid route
                        let input_file = InputFile::by_id(&route.input_file_id).first(conn)?;
                        let sri_hash = input_file.sri_hash(cache_dir)?;
                        el.set_attribute("integrity", &sri_hash)?;
                    } else {
                        // TODO: See if an alias exists

                        if let Some(asset_input_file) =
                            InputFile::asset(rev, &path, conn).optional()?
                        {
                            if let Some(route) = Route::by_revision_id_and_input_file_id(
                                rev.id,
                                &asset_input_file.id,
                            )
                            .first(conn)
                            .optional()?
                            {
                                if let Some(href_value) =
                                    route_relative_href(base_url, route_abs_url, &route.route)?
                                {
                                    el.set_attribute("href", &href_value)?;
                                }

                                let sri_hash = asset_input_file.sri_hash(cache_dir)?;
                                el.set_attribute("integrity", &sri_hash)?;
                            }
                        } else {
                            tracing::warn!(
                                "In revision {} route: {} link href: {} points to non-existent resource",
                                rev.id,
                                route_abs_url,
                                href
                            )
                        }
                    }
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
    base_url: &Url,
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

        let ty = input_file.ty();
        match ty {
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

                        let output = rewrite_html(
                            html_output.as_bytes(),
                            base_url,
                            &r.route,
                            rev,
                            cache_dir,
                            conn,
                        )?;

                        tracing::trace!("Writing content to file: {}", dest_path.display());
                        fs::write(dest_path, output)?;
                    } else {
                        todo!();
                    }
                } else {
                    unreachable!("content was not in database");
                }
            }
            Ty::Asset(_) | Ty::Static(_) => {
                if let Some(contents) = &input_file.contents {
                    if ty.is_html() {
                        tracing::trace!("Writing file: {}", dest_path.display());
                        let contents =
                            rewrite_html(contents, base_url, &r.route, rev, cache_dir, conn)?;

                        fs::write(dest_path, contents)?;
                    } else {
                        tracing::trace!(
                            "Writing file from database contents: {}",
                            dest_path.display()
                        );
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
