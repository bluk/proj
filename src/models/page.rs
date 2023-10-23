use chrono::NaiveDateTime;
use diesel::{
    backend::Backend,
    expression::AsExpression,
    helper_types::{AsSelect, Filter, Select},
    prelude::*,
    sql_types::Text,
};

use crate::{models::DbConn, schema::pages};

#[derive(Debug, PartialEq, Queryable, Selectable, Identifiable)]
#[diesel(primary_key(input_file_id))]
pub struct Page {
    pub input_file_id: String,
    pub front_matter: Option<String>,
    pub offset: i64,
    pub date: Option<NaiveDateTime>,
    pub description: Option<String>,
    pub excerpt: Option<String>,
    pub draft: bool,
    pub expiry_date: Option<NaiveDateTime>,
    pub keywords: Option<String>,
    pub template: Option<String>,
    pub publish_date: Option<NaiveDateTime>,
    pub summary: Option<String>,
    pub title: Option<String>,
}

type WithInputFileId<T> = diesel::dsl::Eq<pages::input_file_id, T>;

#[inline]
#[must_use]
pub fn with_input_file_id<T>(id: T) -> WithInputFileId<T>
where
    T: AsExpression<Text>,
{
    pages::input_file_id.eq(id)
}

type All<Db> = Select<pages::table, AsSelect<Page, Db>>;
type ByInputFileId<T, Db> = Filter<All<Db>, WithInputFileId<T>>;

impl Page {
    #[inline]
    #[must_use]
    pub fn all<Db>() -> All<Db>
    where
        Db: Backend,
    {
        pages::table.select(Self::as_select())
    }

    #[inline]
    #[must_use]
    pub fn by_input_file_id<Db>(id: &str) -> ByInputFileId<&'_ str, Db>
    where
        Db: Backend,
    {
        Self::all().filter(with_input_file_id(id))
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Insertable)]
#[diesel(table_name = pages)]
pub struct NewPage<'a> {
    pub input_file_id: &'a str,
    pub front_matter: Option<&'a str>,
    pub offset: i64,
    pub date: Option<NaiveDateTime>,
    pub description: Option<&'a str>,
    pub excerpt: Option<&'a str>,
    pub draft: bool,
    pub expiry_date: Option<NaiveDateTime>,
    pub keywords: Option<&'a str>,
    pub template: Option<&'a str>,
    pub publish_date: Option<NaiveDateTime>,
    pub summary: Option<&'a str>,
    pub title: Option<&'a str>,
}

impl<'a> NewPage<'a> {
    pub fn create(&self, conn: &mut DbConn) -> QueryResult<usize> {
        diesel::insert_into(pages::table).values(self).execute(conn)
    }
}
