// @generated automatically by Diesel CLI.

diesel::table! {
    previews (url) {
        url -> Text,
        added_date -> Date,
        bookmarked -> Bool,
        source -> Nullable<Text>,
        title -> Nullable<Text>,
        published_date -> Nullable<Text>,
        tags -> Nullable<Text>,
        summary -> Nullable<Text>,
    }
}
