use core::fmt;
use std::{
    fs::File,
    io,
    ops::Deref,
    path::{Path, PathBuf},
    sync::mpsc::{self},
};

use blake3::Hash;
use ignore::Walk;
use memmap2::Mmap;
use rayon::prelude::*;

const INLINE_CONTENT: &[&str] = &["hbs", "html", "md"];

#[derive(Debug)]
struct EmptyContents {}

impl Deref for EmptyContents {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &[]
    }
}

type Contents = Box<dyn Deref<Target = [u8]> + Send + Sync>;

#[derive(Debug)]
pub struct Metadata {
    pub disk_path: PathBuf,
    pub logical_path: String,
    pub size: u64,
}

impl Metadata {
    pub fn is_inline(&self) -> bool {
        self.disk_path
            .extension()
            .map(|ext| INLINE_CONTENT.contains(&ext.to_string_lossy().to_lowercase().as_ref()))
            .unwrap_or_default()
    }

    fn contents(&self) -> io::Result<Contents> {
        if self.size == 0 {
            Ok(Box::new(EmptyContents {}))
        } else {
            let file = File::open(&self.disk_path)?;
            let mm = unsafe { Mmap::map(&file)? };
            Ok(Box::new(mm))
        }
    }
}

pub struct Asset {
    pub meta: Metadata,
    pub contents: Contents,
    pub hash: Hash,
}

impl fmt::Debug for Asset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Content")
            .field("meta", &self.meta)
            // .field("contents", &self.contents)
            .field("hash", &self.hash)
            .finish()
    }
}

fn walk_dir<F>(dir: &Path, mut f: F) -> anyhow::Result<()>
where
    F: FnMut(Metadata) -> anyhow::Result<()>,
{
    tracing::debug!("Working on {}", dir.display());
    assert!(dir.is_dir());
    let base_path = dir.parent().expect("src directory does not exist");

    itertools::process_results(Walk::new(dir), |entries| {
        for disk_path in entries
            .map(ignore::DirEntry::into_path)
            .filter(|disk_path| disk_path.is_file())
        {
            let logical_path = disk_path
                .strip_prefix(base_path)?
                .to_str()
                .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "not a valid UTF-8 path"))?
                .to_string();
            let metadata = disk_path.metadata()?;
            let size = metadata.len();

            f(Metadata {
                disk_path,
                logical_path,
                size,
            })?;
        }

        Ok::<_, anyhow::Error>(())
    })??;

    Ok(())
}

const SRC_SUB_DIRS: &[&str] = &["content", "static", "templates"];

fn walk_src_dirs<F>(src: &Path, mut f: F) -> anyhow::Result<()>
where
    F: FnMut(Metadata) -> anyhow::Result<()>,
{
    for &prefix in SRC_SUB_DIRS {
        let dir = &src.join(prefix);
        walk_dir(dir, &mut f)?;
    }

    Ok(())
}

pub fn process(sink: &mut mpsc::Sender<Asset>, meta: Metadata) -> anyhow::Result<()> {
    tracing::debug!("Processing: {}", meta.logical_path);

    let contents = meta.contents()?;

    let mut hasher = blake3::Hasher::new();
    hasher.update(meta.logical_path.as_bytes());
    hasher.update(b"/");
    hasher.update(&contents);
    let hash = hasher.finalize();

    sink.send(Asset {
        meta,
        contents,
        hash,
    })?;

    Ok(())
}

pub fn walk<F>(src: &Path, f: F) -> anyhow::Result<()>
where
    F: FnOnce(mpsc::Receiver<Asset>) -> anyhow::Result<()> + Sync + Send,
{
    let (tx, rx) = mpsc::channel();

    let mut walk_result = Ok(());
    let mut send_result = Ok(());
    let mut process_result = Ok(());

    rayon::scope(|s| {
        let walk_result = &mut walk_result;
        let send_result = &mut send_result;
        let process_result = &mut process_result;

        s.spawn(move |_| {
            *walk_result = walk_src_dirs(src, |metadata| {
                tx.send(metadata)?;
                Ok(())
            });
        });

        let (event_tx, event_rx) = mpsc::channel::<Asset>();

        s.spawn(move |_| {
            *process_result = f(event_rx);
        });

        *send_result = rx
            .into_iter()
            .par_bridge()
            .map_with(event_tx, process)
            .collect::<Result<_, _>>();
    });

    walk_result.and(send_result).and(process_result)
}
