use core::fmt;
use std::{
    fs,
    path::{Path, PathBuf},
};

use base64ct::{Base64, Encoding};
use chrono::NaiveDateTime;
use diesel::{
    backend::Backend,
    dsl::{exists, not},
    expression::AsExpression,
    helper_types::{AsSelect, Filter, Select},
    prelude::*,
    sql_types::Text,
};
use itertools::Itertools;

use crate::{
    models::DbConn,
    schema::{input_files, revision_files},
};

use super::{revision::Revision, revision_file::RevisionFile};

#[derive(Debug)]
pub enum Ty<'a> {
    Asset(&'a str),
    Static(&'a str),
    Template(&'a str),
    Content(&'a str),
    Unknown,
}

impl<'a> Ty<'a> {
    pub fn is_stylesheet(&self) -> bool {
        match self {
            Ty::Asset(path) => Path::new(path)
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("css"))
                .unwrap_or_default(),
            Ty::Static(_) | Ty::Template(_) | Ty::Content(_) | Ty::Unknown => false,
        }
    }

    pub fn is_html(&self) -> bool {
        match self {
            Ty::Asset(path) | Ty::Static(path) => Path::new(path)
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("html"))
                .unwrap_or_default(),
            Ty::Template(_) | Ty::Content(_) | Ty::Unknown => false,
        }
    }
}

pub fn ty(logical_path: &str) -> Ty<'_> {
    if let Some(path) = logical_path.strip_prefix("assets/") {
        return Ty::Asset(path);
    }

    if let Some(path) = logical_path.strip_prefix("content/") {
        return Ty::Content(path);
    }

    if let Some(path) = logical_path.strip_prefix("static/") {
        return Ty::Static(path);
    }

    if let Some(path) = logical_path.strip_prefix("templates/") {
        return Ty::Template(path);
    }

    Ty::Unknown
}

#[derive(Debug, PartialEq, Queryable, Selectable, Identifiable)]
pub struct InputFile {
    pub id: String,
    pub logical_path: String,
    pub contents_hash: Vec<u8>,
    pub contents: Option<Vec<u8>>,
    pub created_at: NaiveDateTime,
}

type WithId<T> = diesel::dsl::Eq<input_files::id, T>;
type WithLogicalPath<T> = diesel::dsl::Eq<input_files::logical_path, T>;
type WithNoRevisionFile<Db> = diesel::dsl::not<
    diesel::dsl::exists<
        diesel::dsl::Filter<
            Select<revision_files::table, AsSelect<RevisionFile, Db>>,
            diesel::dsl::Eq<input_files::id, revision_files::input_file_id>,
        >,
    >,
>;

#[inline]
#[must_use]
pub fn with_id<T>(id: T) -> WithId<T>
where
    T: AsExpression<Text>,
{
    input_files::id.eq(id)
}

#[inline]
#[must_use]
pub fn with_no_revision_file<Db>() -> WithNoRevisionFile<Db>
where
    Db: Backend,
{
    not(exists(
        revision_files::table
            .select(RevisionFile::as_select())
            .filter(input_files::id.eq(revision_files::input_file_id)),
    ))
}

#[inline]
#[must_use]
pub fn with_logical_path<T>(logical_path: T) -> WithLogicalPath<T>
where
    T: AsExpression<Text>,
{
    input_files::logical_path.eq(logical_path)
}

type All<Db> = Select<input_files::table, AsSelect<InputFile, Db>>;
type ById<T, Db> = Filter<All<Db>, WithId<T>>;

impl InputFile {
    #[inline]
    #[must_use]
    pub fn all<Db>() -> All<Db>
    where
        Db: Backend,
    {
        input_files::table.select(Self::as_select())
    }

    #[inline]
    #[must_use]
    pub fn by_id<Db>(id: &str) -> ById<&'_ str, Db>
    where
        Db: Backend,
    {
        Self::all().filter(with_id(id))
    }

    #[inline]
    #[must_use]
    pub fn with_no_revision_file<Db>() -> Filter<All<Db>, WithNoRevisionFile<Db>>
    where
        Db: Backend,
    {
        Self::all().filter(with_no_revision_file::<Db>())
    }

    #[inline]
    pub fn with_revision(rev: &Revision, conn: &mut DbConn) -> QueryResult<Vec<Self>> {
        RevisionFile::belonging_to(rev)
            .inner_join(input_files::table)
            .select(Self::as_select())
            .load(conn)
    }

    #[inline]
    pub fn template(rev: &Revision, name: &str, conn: &mut DbConn) -> QueryResult<Self> {
        RevisionFile::belonging_to(rev)
            .inner_join(input_files::table)
            .filter(with_logical_path(format!("templates/{name}")))
            .select(Self::as_select())
            .get_result(conn)
    }

    #[inline]
    pub fn asset(rev: &Revision, name: &str, conn: &mut DbConn) -> QueryResult<Self> {
        RevisionFile::belonging_to(rev)
            .inner_join(input_files::table)
            .filter(with_logical_path(format!("assets/{name}")))
            .select(Self::as_select())
            .get_result(conn)
    }

    #[must_use]
    pub fn ty(&self) -> Ty<'_> {
        ty(&self.logical_path)
    }

    pub fn sri_hash(&self, cache_dir: &Path) -> anyhow::Result<String> {
        use sha2::{Digest, Sha384};

        let mut hasher = Sha384::new();
        if let Some(contents) = &self.contents {
            hasher.update(contents);
        } else {
            let content_hash_string = format!("{:x}", self.contents_hash.iter().format(""));
            let cache_path = cache_dir.join(content_hash_string);
            let contents = fs::read(cache_path)?;
            hasher.update(contents);
        }

        let result = hasher.finalize();
        let hash = Base64::encode_string(&result[..]);

        Ok(format!("sha384-{hash}"))
    }

    /// Returns the path to be used in the cache directory.
    #[must_use]
    pub fn cache_file_name(&self) -> Option<PathBuf> {
        if self.contents.is_some() {
            return None;
        }

        let contents_hash =
            blake3::Hash::from_bytes(self.contents_hash.as_slice().try_into().unwrap());
        let content_hash_string = format!("{:x}", contents_hash.as_bytes().iter().format(""));
        Some(PathBuf::from(content_hash_string))
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Id(pub String);

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Id").field(&self.0).finish()
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Insertable)]
#[diesel(table_name = input_files)]
pub struct NewInputFile<'a> {
    pub id: &'a str,
    pub logical_path: &'a str,
    pub contents_hash: &'a [u8],
    pub contents: Option<&'a [u8]>,
}

impl<'a> NewInputFile<'a> {
    pub fn new(
        id: &'a str,
        logical_path: &'a str,
        contents_hash: &'a [u8],
        contents: Option<&'a [u8]>,
    ) -> Self {
        Self {
            id,
            logical_path,
            contents_hash,
            contents,
        }
    }

    pub fn create(&self, conn: &mut DbConn) -> QueryResult<bool> {
        let existing_count = input_files::dsl::input_files
            .count()
            .filter(with_id(&self.id))
            .get_result::<i64>(conn)?;

        if existing_count == 0 {
            tracing::debug!("Inserted input file: {}", self.id);
            let result = diesel::insert_into(input_files::table)
                .values(self)
                .execute(conn)?;
            assert_eq!(1, result);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

type MetaAll<Db> = Select<input_files::table, AsSelect<InputFileMeta, Db>>;
type MetaById<T, Db> = Filter<MetaAll<Db>, WithId<T>>;

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, PartialEq, Queryable, Selectable, Identifiable)]
#[diesel(table_name = input_files)]
pub struct InputFileMeta {
    pub id: String,
    pub logical_path: String,
    pub contents_hash: Vec<u8>,
}

impl InputFileMeta {
    #[inline]
    #[must_use]
    pub fn all<Db>() -> MetaAll<Db>
    where
        Db: Backend,
    {
        input_files::table.select(Self::as_select())
    }

    #[inline]
    #[must_use]
    pub fn by_id<Db>(id: &str) -> MetaById<&'_ str, Db>
    where
        Db: Backend,
    {
        Self::all().filter(with_id(id))
    }

    #[inline]
    pub fn with_revision(rev: &Revision, conn: &mut DbConn) -> QueryResult<Vec<Self>> {
        RevisionFile::belonging_to(rev)
            .inner_join(input_files::table)
            .select(Self::as_select())
            .load(conn)
    }
}
