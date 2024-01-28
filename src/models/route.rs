use diesel::{
    backend::Backend,
    expression::AsExpression,
    helper_types::{AsSelect, Filter, Select},
    prelude::*,
    sql_types::{BigInt, Text},
};

use crate::{
    models::{input_file::InputFile, revision::Revision, DbConn, DbId},
    schema::routes,
};

/// Canonical URLs to a resource.
///
/// There may be multiple routes for the same content. When publishing static content,
/// the same content could be written to multiple files or a symlink could be used.
/// Routes are treated as pointing to separate resources even if the content is the
/// same.
///
/// Page aliases are used when redirecting resources.
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
type WithRoute<T> = diesel::dsl::Eq<routes::route, T>;
type WithInputFileId<T> = diesel::dsl::Eq<routes::input_file_id, T>;

#[inline]
#[must_use]
pub fn with_revision_id<T>(id: T) -> WithRevisionId<T>
where
    T: AsExpression<BigInt>,
{
    routes::revision_id.eq(id)
}

#[inline]
#[must_use]
pub fn with_route<T>(route: T) -> WithRoute<T>
where
    T: AsExpression<Text>,
{
    routes::route.eq(route)
}

#[inline]
#[must_use]
pub fn with_input_file_id<T>(input_file_id: T) -> WithInputFileId<T>
where
    T: AsExpression<Text>,
{
    routes::input_file_id.eq(input_file_id)
}

type All<Db> = Select<routes::table, AsSelect<Route, Db>>;
type ByRevisionId<T, Db> = Filter<All<Db>, WithRevisionId<T>>;
type ByRevisionIdAndRoute<T1, T2, Db> = Filter<ByRevisionId<T1, Db>, WithRoute<T2>>;
type ByRevisionIdAndInputFileId<T1, T2, Db> = Filter<ByRevisionId<T1, Db>, WithInputFileId<T2>>;

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

    #[inline]
    #[must_use]
    pub fn by_revision_id_and_route<Db>(
        revision_id: DbId,
        route: &str,
    ) -> ByRevisionIdAndRoute<DbId, &str, Db>
    where
        Db: Backend,
    {
        Self::all()
            .filter(with_revision_id(revision_id))
            .filter(with_route(route))
    }

    #[inline]
    #[must_use]
    pub fn by_revision_id_and_input_file_id<Db>(
        revision_id: DbId,
        input_file_id: &str,
    ) -> ByRevisionIdAndInputFileId<DbId, &str, Db>
    where
        Db: Backend,
    {
        Self::all()
            .filter(with_revision_id(revision_id))
            .filter(with_input_file_id(input_file_id))
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
