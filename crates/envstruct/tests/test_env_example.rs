#![allow(dead_code)]

use envstruct::prelude::*;

#[test]
fn field_docs_preserve_markdown_and_stay_with_the_assignment() {
    #[rustfmt::skip]
    #[derive(EnvStruct)]
    struct Config {
        /// How long a session stays valid without a refresh.
        ///
        /// - Use `24h` for a day.
        ///   See [sessions](https://example.com/sessions).
        #[env(name = "refresh_ttl", default = "24h")]
        ttl: String,
        undocumented: String,
        /// Never expose this token.
        #[env(secret, default = "hidden-token")]
        token: String,
        /// This field is not configured through the environment.
        #[env(skip)]
        skipped: String,
    }

    let tree = Config::get_usage_tree("ACCOUNT_SESSION", None).unwrap();
    let UsageItem::Field(field) = &tree.items[0] else {
        panic!("expected a field")
    };
    assert_eq!(field.description.as_deref(), Some("How long a session stays valid without a refresh.\n\n- Use `24h` for a day.\n  See [sessions](https://example.com/sessions)."));
    assert_eq!(tree.to_env_example(), "# How long a session stays valid without a refresh.\n#\n# - Use `24h` for a day.\n#   See [sessions](https://example.com/sessions).\n# ACCOUNT_SESSION_REFRESH_TTL=24h\nACCOUNT_SESSION_UNDOCUMENTED=\n# Never expose this token.\n# ACCOUNT_SESSION_TOKEN=\n");
    assert!(!Config::usage().unwrap().contains("How long"));
}

#[test]
fn group_description_replaces_title_with_undocumented_title_as_fallback() {
    #[derive(EnvStruct)]
    struct Database {
        dsn: String,
    }
    #[derive(EnvStruct)]
    struct Config {
        /// Player database, read to enrich a ticket with game info.
        datadb: Database,
        /// Optional archive database.
        archive: Option<Database>,
        fallback: Database,
    }

    let example = Config::get_usage_tree("", None).unwrap().to_env_example();
    assert_eq!(example, "# Player database, read to enrich a ticket with game info.\nDATADB_DSN=\n\n# Optional archive database.\n# ARCHIVE_DSN=\n\n# Fallback\nFALLBACK_DSN=\n\n");
}

#[test]
fn inactive_groups_keep_their_own_and_their_child_descriptions() {
    #[derive(EnvStruct)]
    struct Remote {
        /// Connection string for the remote store.
        dsn: String,
    }
    #[derive(EnvStruct)]
    struct Config {
        #[env(default = "local")]
        mode: String,
        /// Configure this store when local storage is unavailable.
        #[env(used_if = "mode=remote")]
        remote: Option<Remote>,
    }

    let tree = Config::get_usage_tree("STORE", None).unwrap();
    let UsageItem::Group(group) = &tree.items[1] else {
        panic!("expected a group")
    };
    assert_eq!(
        group.description.as_deref(),
        Some("Configure this store when local storage is unavailable.")
    );
    assert_eq!(tree.to_env_example(), "# STORE_MODE=local\n\n# Configure this store when local storage is unavailable.\n# used when STORE_MODE=remote\n# Connection string for the remote store.\n# STORE_REMOTE_DSN=\n\n");
}

#[test]
fn docs_on_flattened_and_inline_fields_do_not_change_the_short_table() {
    #[derive(EnvStruct)]
    struct Inner {
        /// Public listener address.
        addr: String,
        #[env(default = "local")]
        mode: String,
        #[env(used_if = "mode=remote")]
        remote: Option<Credentials>,
    }
    #[derive(EnvStruct)]
    struct Documented {
        /// Public API configuration.
        #[env(flatten)]
        public: Inner,
        /// Administrative API configuration.
        #[env(inline)]
        admin: Inner,
    }
    #[derive(EnvStruct)]
    struct Undocumented {
        #[env(flatten)]
        public: Inner,
        #[env(inline)]
        admin: Inner,
    }

    assert_eq!(Documented::usage().unwrap(), Undocumented::usage().unwrap());
    let example = Documented::get_usage_tree("APP", None)
        .unwrap()
        .to_env_example();
    assert!(
        example.starts_with("# Public API configuration.\n# Public listener address.\nAPP_ADDR=\n")
    );
    assert!(example.contains(
        "# Administrative API configuration.\n# Public listener address.\nAPP_ADMIN_ADDR=\n"
    ));
    assert_eq!(example.matches("# Public API configuration.").count(), 1);
    assert!(!example.contains("# Public\n"));
    assert!(!example.contains("# Admin\n"));
}

#[derive(EnvStruct)]
struct Credentials {
    dsn: String,
    #[env(default = "30s")]
    timeout: String,
}

#[test]
fn placeholders_distinguish_required_optional_and_secret_values() {
    #[derive(EnvStruct)]
    struct Config {
        port: u16,
        #[env(default = "30s")]
        shutdown_timeout: String,
        label: Option<String>,
        #[env(secret, default = "do-not-publish")]
        token: String,
        #[env(secret)]
        password: String,
        #[env(secret)]
        database: Credentials,
    }

    let example = Config::get_usage_tree("APP", None)
        .unwrap()
        .to_env_example();
    assert_eq!(example, "APP_PORT=\n# APP_SHUTDOWN_TIMEOUT=30s\n# APP_LABEL=\n# APP_TOKEN=\nAPP_PASSWORD=\n\n# Database\nAPP_DATABASE_DSN=\n# APP_DATABASE_TIMEOUT=\n\n");
}

#[test]
fn only_default_branch_is_enabled_even_for_optional_used_if_groups() {
    #[derive(EnvStruct)]
    struct Config {
        #[env(default = "local")]
        mode: String,
        #[env(used_if = "mode=local")]
        local: Option<Credentials>,
        #[env(used_if = "mode=remote")]
        remote: Option<Credentials>,
    }

    let example = Config::get_usage_tree("STORE", None)
        .unwrap()
        .to_env_example();
    assert_eq!(example, "# STORE_MODE=local\n\n# used when STORE_MODE=local\nSTORE_LOCAL_DSN=\n# STORE_LOCAL_TIMEOUT=30s\n\n# used when STORE_MODE=remote\n# STORE_REMOTE_DSN=\n# STORE_REMOTE_TIMEOUT=30s\n\n");
}

#[derive(EnvStruct)]
#[env(tag = "mode")]
enum Backend {
    Local(Credentials),
    Remote(Credentials),
}

#[test]
fn missing_switch_default_does_not_choose_a_branch() {
    let example = Backend::get_usage_tree("STORE", None)
        .unwrap()
        .to_env_example();
    assert_eq!(example, "STORE_MODE=\n\n# used when STORE_MODE=local\n# STORE_LOCAL_DSN=\n# STORE_LOCAL_TIMEOUT=30s\n\n# used when STORE_MODE=remote\n# STORE_REMOTE_DSN=\n# STORE_REMOTE_TIMEOUT=30s\n\n");
}

#[test]
fn nested_default_cannot_enable_an_inactive_parent_branch() {
    #[derive(EnvStruct)]
    struct Nested {
        #[env(flatten, default = "local")]
        backend: Backend,
    }

    #[derive(EnvStruct)]
    struct Config {
        #[env(default = "false")]
        enabled: bool,
        #[env(used_if = "enabled=true")]
        nested: Nested,
    }

    let example = Config::get_usage_tree("APP", None)
        .unwrap()
        .to_env_example();
    assert!(example
        .lines()
        .all(|line| line.is_empty() || line.starts_with("# ")));
    let active = Nested::get_usage_tree("APP", None)
        .unwrap()
        .to_env_example();
    assert!(active.lines().any(|line| line == "APP_LOCAL_DSN="));
    assert!(active.lines().any(|line| line == "# APP_REMOTE_DSN="));
}

#[test]
fn optional_groups_are_not_enabled_by_required_children() {
    #[derive(EnvStruct)]
    struct Config {
        database: Option<Credentials>,
    }

    for tree in [
        Config::get_usage_tree("APP", None).unwrap(),
        <Option<Credentials>>::get_usage_tree("APP", None).unwrap(),
    ] {
        let example = tree.to_env_example();
        assert!(example.contains("DSN="));
        assert!(example
            .lines()
            .all(|line| line.is_empty() || line.starts_with("# ")));
    }
}

#[test]
fn multiline_defaults_cannot_inject_active_assignments() {
    let tree = UsageTree::leaf_field(
        "APP_TEXT",
        UsageType::String,
        false,
        Some("hello # \"world\"\nINJECTED=yes\r\n$HOME\\path".to_string()),
        None,
    );
    assert_eq!(
        tree.to_env_example(),
        "# APP_TEXT=\"hello # \\\"world\\\"\\nINJECTED=yes\\r\\n\\$HOME\\\\path\"\n"
    );
}
