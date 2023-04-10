use core::fmt;

use chrono::NaiveDateTime;
use diesel::{
    backend::Backend,
    expression::AsExpression,
    helper_types::{AsSelect, Filter, Select},
    prelude::*,
    sql_types::Text,
};

use crate::{models::DbConn, schema::input_files};

use super::{revision::Revision, revision_file::RevisionFile};

#[derive(Debug)]
pub enum Ty<'a> {
    Static(&'a str),
    Template(&'a str),
    Content(&'a str),
    Unknown,
}

pub fn ty(logical_path: &str) -> Ty<'_> {
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

    #[must_use]
    pub fn ty(&self) -> Ty<'_> {
        ty(&self.logical_path)
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

    pub fn create(&self, conn: &mut DbConn) -> QueryResult<usize> {
        let existing_count = input_files::dsl::input_files
            .count()
            .filter(with_id(&self.id))
            .get_result::<i64>(conn)?;

        if existing_count == 0 {
            tracing::debug!("Inserted input file: {}", self.id);
            diesel::insert_into(input_files::table)
                .values(self)
                .execute(conn)
        } else {
            Ok(0)
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
