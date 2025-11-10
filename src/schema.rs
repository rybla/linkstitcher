// @generated automatically by Diesel CLI.

diesel::table! {
    previews (url) {
        url -> Text,
        added_date -> Date,
        source -> Nullable<Text>,
        title -> Nullable<Text>,
        published_date -> Nullable<Text>,
        tags -> Nullable<Text>,
        summary -> Nullable<Text>,
        bookmarked -> Bool,
    }
}
