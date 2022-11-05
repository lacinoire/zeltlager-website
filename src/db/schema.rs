// @generated automatically by Diesel CLI.

diesel::table! {
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
        anmeldedatum -> Timestamptz,
        allergien -> Text,
        unvertraeglichkeiten -> Text,
        medikamente -> Text,
        krankenversicherung -> Text,
        vegetarier -> Bool,
        tetanus_impfung -> Bool,
    }
}

diesel::table! {
    erwischt_game (id) {
        id -> Int4,
        created -> Timestamptz,
    }
}

diesel::table! {
    erwischt_member (game, id) {
        game -> Int4,
        id -> Int4,
        name -> Text,
        target -> Int4,
        catcher -> Nullable<Int4>,
        last_change -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    rate_limiting (ip_addr) {
        ip_addr -> Inet,
        counter -> Int4,
        first_count -> Timestamp,
    }
}

diesel::table! {
    roles (user_id, role) {
        user_id -> Int4,
        role -> Text,
    }
}

diesel::table! {
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
        anmeldedatum -> Timestamptz,
        bezahlt -> Bool,
        anwesend -> Bool,
        allergien -> Text,
        unvertraeglichkeiten -> Text,
        medikamente -> Text,
        krankenversicherung -> Text,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        username -> Text,
        password -> Text,
    }
}

diesel::joinable!(erwischt_member -> erwischt_game (game));
diesel::joinable!(roles -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    betreuer,
    erwischt_game,
    erwischt_member,
    rate_limiting,
    roles,
    teilnehmer,
    users,
);
