use core::fmt;

use chrono::NaiveDateTime;
use diesel::{
    backend::Backend,
    expression::AsExpression,
    helper_types::{AsSelect, Filter, Select},
    prelude::*,
    sql_types::Integer,
};

use crate::{
    models::{DbConn, DbId},
    schema::revisions,
};

#[derive(Debug, PartialEq, Queryable, Selectable, Identifiable)]
pub struct Revision {
    pub id: DbId,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

type WithId<T> = diesel::dsl::Eq<revisions::id, T>;

#[inline]
#[must_use]
pub fn with_id<T>(id: T) -> WithId<T>
where
    T: AsExpression<Integer>,
{
    revisions::id.eq(id)
}

type All<Db> = Select<revisions::table, AsSelect<Revision, Db>>;
type ById<T, Db> = Filter<All<Db>, WithId<T>>;

impl Revision {
    #[inline]
    #[must_use]
    pub fn all<Db>() -> All<Db>
    where
        Db: Backend,
    {
        revisions::table.select(Self::as_select())
    }

    #[inline]
    #[must_use]
    pub fn by_id<Db>(id: Id) -> ById<DbId, Db>
    where
        Db: Backend,
    {
        Self::all().filter(with_id(id.0))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Id(pub DbId);

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Id").field(&self.0).finish()
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

pub fn create(conn: &mut DbConn) -> QueryResult<Id> {
    conn.transaction(move |conn| {
        let id = diesel::insert_into(revisions::table)
            .default_values()
            .returning(revisions::dsl::id)
            .get_result(conn)?;

        Ok(Id(id))
    })
}
