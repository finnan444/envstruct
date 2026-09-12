#![allow(dead_code)]

use envstruct::prelude::*;

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
