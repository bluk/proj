// @generated automatically by Diesel CLI.

diesel::table! {
    use crate::sqlite_mapping::*;

    input_files (id) {
        id -> Text,
        logical_path -> Text,
        contents_hash -> Binary,
        contents -> Nullable<Binary>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    use crate::sqlite_mapping::*;

    page_aliases (input_file_id, alias) {
        input_file_id -> Text,
        alias -> Text,
    }
}

diesel::table! {
    use crate::sqlite_mapping::*;

    page_tags (input_file_id, tag) {
        input_file_id -> Text,
        tag -> Text,
    }
}

diesel::table! {
    use crate::sqlite_mapping::*;

    pages (input_file_id) {
        input_file_id -> Text,
        front_matter -> Nullable<Text>,
        offset -> Integer,
        date -> Nullable<Timestamp>,
        description -> Nullable<Text>,
        excerpt -> Nullable<Text>,
        draft -> Bool,
        expiry_date -> Nullable<Timestamp>,
        keywords -> Nullable<Text>,
        template -> Nullable<Text>,
        publish_date -> Nullable<Timestamp>,
        summary -> Nullable<Text>,
        title -> Nullable<Text>,
    }
}

diesel::table! {
    use crate::sqlite_mapping::*;

    revision_files (revision_id, input_file_id) {
        revision_id -> Integer,
        input_file_id -> Text,
    }
}

diesel::table! {
    use crate::sqlite_mapping::*;

    revisions (id) {
        id -> Integer,
        created_at -> Timestamp,
    }
}

diesel::table! {
    use crate::sqlite_mapping::*;

    routes (revision_id, route) {
        revision_id -> Integer,
        route -> Text,
        input_file_id -> Text,
    }
}

diesel::joinable!(page_aliases -> input_files (input_file_id));
diesel::joinable!(page_tags -> input_files (input_file_id));
diesel::joinable!(pages -> input_files (input_file_id));
diesel::joinable!(revision_files -> input_files (input_file_id));
diesel::joinable!(revision_files -> revisions (revision_id));
diesel::joinable!(routes -> input_files (input_file_id));
diesel::joinable!(routes -> revisions (revision_id));

diesel::allow_tables_to_appear_in_same_query!(
    input_files,
    page_aliases,
    page_tags,
    pages,
    revision_files,
    revisions,
    routes,
);
