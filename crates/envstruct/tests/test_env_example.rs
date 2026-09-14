#![allow(dead_code)]

use envstruct::prelude::*;

#[test]
fn field_docs_stay_in_the_tree_and_out_of_the_example() {
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
    // Only the variable without a default is left to fill in; the docs belong to the table.
    assert_eq!(tree.to_env_example(), "ACCOUNT_SESSION_UNDOCUMENTED=\n");
    assert!(!Config::usage().unwrap().contains("How long"));
}

#[test]
fn a_group_contributes_only_the_variables_that_need_a_value() {
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
    assert_eq!(example, "DATADB_DSN=\n# ARCHIVE_DSN=\nFALLBACK_DSN=\n");
}

#[test]
fn an_inactive_branch_is_headed_by_the_assignment_that_enables_it() {
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
    assert_eq!(
        tree.to_env_example(),
        "# STORE_MODE=remote\n# STORE_REMOTE_DSN=\n"
    );
}

#[test]
fn docs_on_flattened_and_inline_fields_change_neither_the_table_nor_the_example() {
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
    assert_eq!(
        example,
        Undocumented::get_usage_tree("APP", None)
            .unwrap()
            .to_env_example()
    );
    assert_eq!(
        example,
        "APP_ADDR=\n\n# APP_MODE=remote\n# APP_REMOTE_DSN=\n\nAPP_ADMIN_ADDR=\n\n# APP_ADMIN_MODE=remote\n# APP_ADMIN_REMOTE_DSN=\n"
    );
}

#[derive(EnvStruct)]
struct Credentials {
    dsn: String,
    #[env(default = "30s")]
    timeout: String,
}

#[test]
fn only_the_variables_without_a_default_are_written_including_secrets() {
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
    assert_eq!(example, "APP_PORT=\nAPP_PASSWORD=\nAPP_DATABASE_DSN=\n");
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
    assert_eq!(
        example,
        "# STORE_MODE=local\nSTORE_LOCAL_DSN=\n\n# STORE_MODE=remote\n# STORE_REMOTE_DSN=\n"
    );
}

#[derive(EnvStruct)]
#[env(tag = "mode")]
enum Backend {
    Local(Credentials),
    Remote(Credentials),
}

#[test]
fn empty_groups_do_not_leave_orphaned_conditions() {
    #[derive(EnvStruct)]
    struct Empty {}

    #[derive(EnvStruct)]
    #[env(tag = "mode")]
    enum Store {
        Mock,
        Remote(Credentials),
    }

    #[derive(EnvStruct)]
    struct Config {
        #[env(default = "false")]
        enabled: bool,
        /// No credentials are needed when disabled.
        #[env(used_if = "enabled=false")]
        disabled: Empty,
        /// No environment configuration.
        empty: Empty,
        #[env(flatten, default = "mock")]
        store: Store,
    }

    let example = Config::get_usage_tree("APP", None)
        .unwrap()
        .to_env_example();
    assert_eq!(example, "# APP_MODE=remote\n# APP_REMOTE_DSN=\n");
}

#[test]
fn missing_switch_default_does_not_choose_a_branch() {
    let example = Backend::get_usage_tree("STORE", None)
        .unwrap()
        .to_env_example();
    assert_eq!(
        example,
        "STORE_MODE=\n\n# STORE_MODE=local\n# STORE_LOCAL_DSN=\n\n# STORE_MODE=remote\n# STORE_REMOTE_DSN=\n"
    );
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
fn multiline_examples_cannot_inject_active_assignments() {
    #[derive(EnvStruct)]
    struct Config {
        #[env(example = "hello # \"world\"\nINJECTED=yes\r\n$HOME\\path")]
        text: String,
    }

    let example = Config::get_usage_tree("APP", None)
        .unwrap()
        .to_env_example();
    assert_eq!(
        example,
        "APP_TEXT=\"hello # \\\"world\\\"\\nINJECTED=yes\\r\\n\\$HOME\\\\path\"\n"
    );
}

#[test]
fn examples_fill_variables_that_have_no_default_including_secrets() {
    #[derive(EnvStruct)]
    struct Nested {
        dsn: String,
    }
    #[derive(EnvStruct)]
    struct Config {
        /// Where the players are stored.
        #[env(example = "postgres://user:pass@localhost/app")]
        dsn: String,
        /// Rotating it logs everyone out.
        #[env(secret, example = "0123456789abcdef")]
        gcm_secret: String,
        #[env(example = "https://example.com/hook")]
        webhook: Option<String>,
        /// An example belongs to one variable, so a struct field cannot carry it.
        #[env(example = "ignored")]
        nested: Nested,
    }

    let tree = Config::get_usage_tree("APP", None).unwrap();
    assert_eq!(
        tree.to_env_example(),
        "APP_DSN=postgres://user:pass@localhost/app\nAPP_GCM_SECRET=0123456789abcdef\nAPP_NESTED_DSN=\n"
    );
    // The same examples fill the EXAMPLE column of the usage table.
    assert_eq!(
        Config::usage().unwrap(),
        r#"Environment variables

VARIABLE            | TYPE   | DEFAULT    | EXAMPLE                            | FIELD
--------------------+--------+------------+------------------------------------+------------------
DSN                 | string | <required> | postgres://user:pass@localhost/app | Config.dsn
GCM_SECRET (secret) | string | <required> | 0123456789abcdef                   | Config.gcm_secret
NESTED_DSN          | string | <required> |                                    | Nested.dsn
WEBHOOK             | string | none       | https://example.com/hook           | Config.webhook
"#
    );
}
