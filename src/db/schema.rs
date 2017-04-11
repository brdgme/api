table! {
    users {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        name -> VarChar,
        pref_colors -> Array<VarChar>,
        login_confirmation -> Nullable<VarChar>,
        login_confirmation_at -> Nullable<Timestamp>,
    }
}

table! {
    user_emails {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        user_id -> Uuid,
        email -> VarChar,
        is_primary -> Bool,
    }
}

table! {
    user_auth_tokens {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        user_id -> Uuid,
    }
}

table! {
    game_types {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        name -> VarChar,
    }
}

table! {
    game_versions {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        game_type_id -> Uuid,
        name -> VarChar,
        uri -> VarChar,
        is_public -> Bool,
        is_deprecated -> Bool,
    }
}

table! {
    games {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        game_version_id -> Uuid,
        is_finished -> Bool,
        game_state -> Text,
    }
}

table! {
    game_players {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        game_id -> Uuid,
        user_id -> Uuid,
        position -> Integer,
        color -> VarChar,
        has_accepted -> Bool,
        is_turn -> Bool,
        is_eliminated -> Bool,
        is_winner -> Bool,
        is_read -> Bool,
    }
}

table! {
    game_logs {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        game_id -> Uuid,
        body -> Text,
        is_public -> Bool,
        logged_at -> Timestamp,
    }
}

table! {
    game_log_targets {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        game_log_id -> Uuid,
        game_player_id -> Uuid,
    }
}
