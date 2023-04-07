use std::{
    fs::File,
    io,
    ops::Deref,
    path::{Path, PathBuf},
    sync::mpsc::{self},
};

use blake3::Hash;
use diesel::Connection;
use ignore::Walk;
use memmap2::Mmap;
use rayon::prelude::*;

use crate::models::{
    input_file::NewInputFile, revision, revision_file::NewRevisionFile, DbConn, DbPool,
};

const INLINE_CONTENT: &[&str] = &["hbs", "html", "md"];

#[derive(Debug)]
pub struct Local {
    pub disk_path: PathBuf,
    pub logical_path: String,
    pub size: u64,
}

impl Local {
    pub fn is_inline(&self) -> bool {
        self.disk_path
            .extension()
            .map(|ext| INLINE_CONTENT.contains(&ext.to_string_lossy().to_lowercase().as_ref()))
            .unwrap_or_default()
    }
}

pub struct Content {
    meta: Local,
    contents: Box<dyn Deref<Target = [u8]> + Send + Sync>,
    hash: Hash,
}

#[derive(Debug)]
struct EmptyContents {}

impl Deref for EmptyContents {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &[]
    }
}

impl Content {
    pub fn new(meta: Local) -> io::Result<Self> {
        let contents: Box<dyn Deref<Target = [u8]> + Send + Sync> = if meta.size == 0 {
            Box::new(EmptyContents {})
        } else {
            let file = File::open(&meta.disk_path)?;
            let mm = unsafe { Mmap::map(&file)? };
            Box::new(mm)
        };

        let mut hasher = blake3::Hasher::new();
        hasher.update(meta.logical_path.as_bytes());
        hasher.update(b"/");
        hasher.update(&contents);
        let hash = hasher.finalize();

        Ok(Self {
            meta,
            contents,
            hash,
        })
    }
}

#[derive(Debug)]
pub struct Config<'a> {
    pub src: &'a Path,
    pub dest: &'a Path,
}

pub fn process(sink: &mut mpsc::Sender<Content>, asset: Local) -> anyhow::Result<()> {
    tracing::debug!("Processing: {}", asset.logical_path);

    let content = Content::new(asset)?;
    sink.send(content)?;

    Ok(())
}

pub fn walk_dir<P, F>(config: &Config, dir: P, mut f: F) -> anyhow::Result<()>
where
    P: AsRef<Path>,
    F: FnMut(Local) -> anyhow::Result<()>,
{
    let dir = &config.src.join(dir);
    tracing::debug!("Working on {}", dir.display());

    assert!(dir.is_dir());

    let base_path = dir
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "directory is not valid"))?;

    itertools::process_results(Walk::new(dir), |entries| {
        for disk_path in entries
            .map(ignore::DirEntry::into_path)
            .filter(|disk_path| disk_path.is_file())
        {
            let logical_path = disk_path
                .strip_prefix(base_path)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
                .to_str()
                .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "not a valid UTF-8 path"))?
                .to_owned();
            let metadata = disk_path.metadata()?;
            let size = metadata.len();

            f(Local {
                disk_path,
                logical_path,
                size,
            })?;
        }

        Ok::<_, anyhow::Error>(())
    })
    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))??;

    Ok(())
}

pub fn walk(config: &Config, pool: &DbPool) -> anyhow::Result<()> {
    let (tx, rx) = mpsc::channel();

    let mut walk_result = Ok(());
    let mut send_result = Ok(());

    rayon::scope(|s| {
        let walk_result = &mut walk_result;
        let send_result = &mut send_result;

        s.spawn(move |_| {
            *walk_result = (|| -> anyhow::Result<()> {
                for &prefix in &["content", "static", "templates"] {
                    walk_dir(config, prefix, |asset| {
                        tx.send(asset)?;
                        Ok(())
                    })?;
                }

                Ok(())
            })();
        });

        let (event_tx, event_rx) = mpsc::channel::<Content>();

        s.spawn(move |_| {
            let mut conn = pool.get().unwrap();

            let conn: &mut DbConn = &mut conn;

            conn.transaction(|conn| -> anyhow::Result<()> {
                let rev_id = revision::create(conn)?;

                while let Ok(content) = event_rx.recv() {
                    let is_inline = content.meta.is_inline();

                    let new_input_file = NewInputFile::new(
                        &content.meta.logical_path,
                        content.hash.as_bytes().as_slice(),
                        if is_inline { &content.contents } else { &[] },
                    );

                    if !is_inline {
                        // TODO: Copy file to the cache as the content hash name
                    }

                    new_input_file.create(conn).unwrap();

                    NewRevisionFile::new(rev_id, &new_input_file.id)
                        .create(conn)
                        .unwrap();
                }

                Ok(())
            })
            .unwrap();
        });

        *send_result = rx
            .into_iter()
            .par_bridge()
            .map_with(event_tx, process)
            .collect::<Result<_, _>>();
    });

    walk_result.and(send_result)
}
