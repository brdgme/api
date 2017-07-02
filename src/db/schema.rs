table! {
    users {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        name -> Text,
        pref_colors -> Array<Text>,
        login_confirmation -> Nullable<Text>,
        login_confirmation_at -> Nullable<Timestamp>,
    }
}

table! {
    user_emails {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        user_id -> Uuid,
        email -> Text,
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
        player_counts -> Array<Integer>,
        weight -> Float,
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
        finished_at -> Nullable<Timestamp>,
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
        color -> Text,
        has_accepted -> Bool,
        is_turn -> Bool,
        is_turn_at -> Timestamp,
        last_turn_at -> Timestamp,
        is_eliminated -> Bool,
        is_read -> Bool,
        points -> Nullable<Float>,
        undo_game_state -> Nullable<Text>,
        place -> Nullable<Integer>,
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

table! {
    game_type_users {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        game_type_id -> Uuid,
        user_id -> Uuid,
        last_game_finished_at -> Nullable<Timestamp>,
        rating -> Integer,
        peak_rating -> Integer,
    }
}

table! {
    friends {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        source_user_id -> Uuid,
        target_user_id -> Uuid,
        has_accepted -> Nullable<Bool>,
    }
}
