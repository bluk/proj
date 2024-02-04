use diesel::{
    backend::Backend,
    expression::AsExpression,
    helper_types::{AsSelect, Filter, Select},
    prelude::*,
    sql_types::BigInt,
};

use crate::{
    models::{input_file::InputFile, revision::Revision, DbConn, DbId},
    schema::revision_files,
};

#[derive(Debug, PartialEq, Queryable, Selectable, Identifiable, Associations)]
#[diesel(belongs_to(Revision))]
#[diesel(belongs_to(InputFile))]
#[diesel(table_name = revision_files)]
#[diesel(primary_key(revision_id, input_file_id))]
pub struct RevisionFile {
    pub revision_id: DbId,
    pub input_file_id: String,
}

type WithRevisionId<T> = diesel::dsl::Eq<revision_files::revision_id, T>;

#[inline]
#[must_use]
pub fn with_revision_id<T>(id: T) -> WithRevisionId<T>
where
    T: AsExpression<BigInt>,
{
    revision_files::revision_id.eq(id)
}

type All<Db> = Select<revision_files::table, AsSelect<RevisionFile, Db>>;
type ByRevisionId<T, Db> = Filter<All<Db>, WithRevisionId<T>>;

impl RevisionFile {
    #[inline]
    #[must_use]
    pub fn all<Db>() -> All<Db>
    where
        Db: Backend,
    {
        revision_files::table.select(Self::as_select())
    }

    #[inline]
    #[must_use]
    pub fn by_revision_id<Db>(id: DbId) -> ByRevisionId<DbId, Db>
    where
        Db: Backend,
    {
        Self::all().filter(with_revision_id(id))
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Insertable)]
#[diesel(table_name = revision_files)]
pub struct NewRevisionFile<'a> {
    pub revision_id: DbId,
    pub input_file_id: &'a str,
}

impl<'a> NewRevisionFile<'a> {
    pub fn new(revision_id: DbId, input_file_id: &'a str) -> Self {
        Self {
            revision_id,
            input_file_id,
        }
    }

    pub fn create(&self, conn: &mut DbConn) -> QueryResult<usize> {
        diesel::insert_into(revision_files::table)
            .values(self)
            .execute(conn)
    }
}
