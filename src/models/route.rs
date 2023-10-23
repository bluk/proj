use diesel::{
    backend::Backend,
    expression::AsExpression,
    helper_types::{AsSelect, Filter, Select},
    prelude::*,
    sql_types::BigInt,
};

use crate::{
    models::{input_file::InputFile, revision::Revision, DbConn, DbId},
    schema::routes,
};

#[derive(Debug, PartialEq, Queryable, Selectable, Identifiable, Associations)]
#[diesel(belongs_to(Revision))]
#[diesel(belongs_to(InputFile))]
#[diesel(table_name = routes)]
#[diesel(primary_key(revision_id, route))]
pub struct Route {
    pub revision_id: DbId,
    pub route: String,
    pub input_file_id: String,
}

type WithRevisionId<T> = diesel::dsl::Eq<routes::revision_id, T>;

#[inline]
#[must_use]
pub fn with_revision_id<T>(id: T) -> WithRevisionId<T>
where
    T: AsExpression<BigInt>,
{
    routes::revision_id.eq(id)
}

type All<Db> = Select<routes::table, AsSelect<Route, Db>>;
type ByRevisionId<T, Db> = Filter<All<Db>, WithRevisionId<T>>;

impl Route {
    #[inline]
    #[must_use]
    pub fn all<Db>() -> All<Db>
    where
        Db: Backend,
    {
        routes::table.select(Self::as_select())
    }

    #[inline]
    #[must_use]
    pub fn by_revision_id<Db>(id: DbId) -> ByRevisionId<DbId, Db>
    where
        Db: Backend,
    {
        Self::all().filter(with_revision_id(id))
    }

    #[inline]
    pub fn with_revision(rev: &Revision, conn: &mut DbConn) -> QueryResult<Vec<Self>> {
        Route::belonging_to(rev)
            .select(Self::as_select())
            .load(conn)
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Insertable)]
#[diesel(table_name = routes)]
pub struct NewRoute<'a> {
    pub revision_id: DbId,
    pub route: &'a str,
    pub input_file_id: &'a str,
}

impl<'a> NewRoute<'a> {
    pub fn new(revision_id: DbId, route: &'a str, input_file_id: &'a str) -> Self {
        Self {
            revision_id,
            route,
            input_file_id,
        }
    }

    pub fn create(&self, conn: &mut DbConn) -> QueryResult<usize> {
        diesel::insert_into(routes::table)
            .values(self)
            .execute(conn)
    }
}
