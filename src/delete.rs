use crate::models::{revision::Revision, DbConn};

use diesel::prelude::*;

pub fn delete(revision: &Revision, conn: &mut DbConn) -> anyhow::Result<()> {
    conn.transaction(|conn| diesel::delete(revision).execute(conn))?;

    Ok(())
}
