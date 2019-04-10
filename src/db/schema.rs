table! {
    betreuer (id) {
        id -> Int4,
        vorname -> Text,
        nachname -> Text,
        geburtsdatum -> Date,
        geschlecht -> Text,
        juleica_nummer -> Nullable<Text>,
        mail -> Text,
        handynummer -> Text,
        strasse -> Text,
        hausnummer -> Text,
        ort -> Text,
        plz -> Text,
        besonderheiten -> Text,
        agb -> Bool,
        selbsterklaerung -> Bool,
        fuehrungszeugnis_auststellung -> Nullable<Date>,
        fuehrungszeugnis_eingesehen -> Nullable<Date>,
    }
}

table! {
    rate_limiting (ip_addr) {
        ip_addr -> Inet,
        counter -> Int4,
        first_count -> Timestamp,
    }
}

table! {
    roles (user_id, role) {
        user_id -> Int4,
        role -> Text,
    }
}

table! {
    teilnehmer (id) {
        id -> Int4,
        vorname -> Text,
        nachname -> Text,
        geburtsdatum -> Date,
        geschlecht -> Text,
        schwimmer -> Bool,
        vegetarier -> Bool,
        tetanus_impfung -> Bool,
        eltern_name -> Text,
        eltern_mail -> Text,
        eltern_handynummer -> Text,
        strasse -> Text,
        hausnummer -> Text,
        ort -> Text,
        plz -> Text,
        besonderheiten -> Text,
        agb -> Bool,
    }
}

table! {
    users (id) {
        id -> Int4,
        username -> Text,
        password -> Text,
    }
}

joinable!(roles -> users (user_id));

allow_tables_to_appear_in_same_query!(
    betreuer,
    rate_limiting,
    roles,
    teilnehmer,
    users,
);
