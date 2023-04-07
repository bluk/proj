use diesel::prelude::*;

use crate::{
    models::{
        input_file::InputFile,
        revision::{self, Revision},
        DbConn, DbId,
    },
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

#[allow(clippy::module_name_repetitions)]
#[derive(Insertable)]
#[diesel(table_name = revision_files)]
pub struct NewRevisionFile<'a> {
    pub revision_id: DbId,
    pub input_file_id: &'a str,
}

impl<'a> NewRevisionFile<'a> {
    pub fn new(revision_id: revision::Id, input_file_id: &'a str) -> Self {
        Self {
            revision_id: revision_id.0,
            input_file_id,
        }
    }

    pub fn create(&self, conn: &mut DbConn) -> QueryResult<usize> {
        diesel::insert_into(revision_files::table)
            .values(self)
            .execute(conn)
    }
}
