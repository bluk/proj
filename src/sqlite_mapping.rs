//! SQL type mapping for Diesel.
//!
//! This mapping only exists because Sqlite requires `AUTOINCREMENT` on `INTEGER
//! PRIMARY KEY` fields. Diesel maps `INTEGER` to [i32], but depending on the
//! use case, it should be [i64].
//!
//! Using the type mapping feature of Diesel, **all** `INTEGER` fields are
//! mapped to [`BigInt`] which maps them to [i64] fields.
//!
//! See [Diesel Issue #852](https://github.com/diesel-rs/diesel/issues/852).

pub use diesel::sql_types::*;

/// Mapping `INTEGER` SQL type to `BigInt` for [i64].
#[allow(dead_code)]
pub type Integer = BigInt;
