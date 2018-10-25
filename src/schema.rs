table! {
    beer (id) {
        id -> Int4,
        name -> Varchar,
        brewery_id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    brewery (id) {
        id -> Int4,
        name -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    drink (id) {
        id -> Int4,
        drank_on -> Date,
        beer_id -> Int4,
        rating -> Int2,
        comment -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    identity (identifier) {
        identifier -> Varchar,
        person_id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    login_session (id) {
        id -> Bpchar,
        person_id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        expires_at -> Timestamptz,
    }
}

table! {
    person (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

joinable!(beer -> brewery (brewery_id));
joinable!(drink -> beer (beer_id));
joinable!(identity -> person (person_id));
joinable!(login_session -> person (person_id));

allow_tables_to_appear_in_same_query!(
    beer,
    brewery,
    drink,
    identity,
    login_session,
    person,
);
