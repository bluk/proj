// @generated automatically by Diesel CLI.

diesel::table! {
    input_files (id) {
        id -> Text,
        logical_path -> Text,
        content_hash -> Binary,
        content -> Nullable<Binary>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    revision_files (revision_id, input_file_id) {
        revision_id -> Integer,
        input_file_id -> Text,
    }
}

diesel::table! {
    revisions (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    routes (revision_id, route) {
        revision_id -> Integer,
        route -> Text,
        input_file_id -> Text,
    }
}

diesel::joinable!(revision_files -> input_files (input_file_id));
diesel::joinable!(revision_files -> revisions (revision_id));
diesel::joinable!(routes -> input_files (input_file_id));
diesel::joinable!(routes -> revisions (revision_id));

diesel::allow_tables_to_appear_in_same_query!(
    input_files,
    revision_files,
    revisions,
    routes,
);
