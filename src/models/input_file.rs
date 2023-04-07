use core::fmt;

use chrono::NaiveDateTime;
use diesel::{
    backend::Backend,
    expression::AsExpression,
    helper_types::{AsSelect, Filter, Select},
    prelude::*,
    sql_types::Text,
};
use itertools::Itertools;

use crate::{models::DbConn, schema::input_files};

#[derive(Debug, PartialEq, Queryable, Selectable, Identifiable)]
pub struct InputFile {
    pub id: String,
    pub logical_path: String,
    pub content_hash: Vec<u8>,
    pub content: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

type WithId<T> = diesel::dsl::Eq<input_files::id, T>;

#[inline]
#[must_use]
pub fn with_id<T>(id: T) -> WithId<T>
where
    T: AsExpression<Text>,
{
    input_files::id.eq(id)
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
    pub id: String,
    pub logical_path: &'a str,
    pub content_hash: &'a [u8],
    pub content: &'a [u8],
}

impl<'a> NewInputFile<'a> {
    pub fn new(logical_path: &'a str, content_hash: &'a [u8], content: &'a [u8]) -> Self {
        let content_hash_string = format!("{:x}", content_hash.iter().format(""));
        let id = format!("{content_hash_string},{logical_path}");

        Self {
            id,
            logical_path,
            content_hash,
            content,
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
