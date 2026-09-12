#![allow(dead_code)]

use envstruct::prelude::*;
use serial_test::*;
use std::env;

/// The mode and the configuration it selects are declared once, in the type.
#[derive(EnvStruct, Debug, PartialEq)]
#[env(tag = "mode")]
pub enum Backend {
    Local,
    Remote(RemoteConfig),
}

#[derive(EnvStruct, Debug, PartialEq)]
pub struct RemoteConfig {
    pub dsn: String,
    #[env(default = "10000")]
    pub cache_size: u32,
}

#[derive(EnvStruct, Debug, PartialEq)]
pub struct DoomConfig {
    #[env(flatten, default = "local")]
    pub backend: Backend,

    #[env(default = "60s")]
    pub reload_delay: envstruct::Duration,
}

fn clean_env() {
    std::env::vars().for_each(|(name, _)| {
        std::env::remove_var(name);
    });
}

fn find_field<'a>(items: &'a [UsageItem], name: &str) -> Option<&'a UsageField> {
    for item in items {
        match item {
            UsageItem::Field(field) if field.name == name => return Some(field),
            UsageItem::Group(group) => {
                if let Some(field) = find_field(&group.items, name) {
                    return Some(field);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_group<'a>(items: &'a [UsageItem], title: &str) -> Option<&'a UsageGroup> {
    for item in items {
        match item {
            UsageItem::Group(group) if group.title == title => return Some(group),
            UsageItem::Group(group) => {
                if let Some(found) = find_group(&group.items, title) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

/// Fields a group lists, in declaration order of the usage tree.
fn group_fields(group: &UsageGroup) -> Vec<&UsageField> {
    group
        .items
        .iter()
        .filter_map(|item| match item {
            UsageItem::Field(field) => Some(field),
            UsageItem::Group(_) => None,
        })
        .collect()
}

#[test]
#[serial]
fn the_switch_selects_the_variant_and_its_config() {
    clean_env();
    env::set_var("DOOM_MODE", "remote");
    env::set_var("DOOM_REMOTE_DSN", "https://deploy.example.com");

    let config = DoomConfig::with_prefix("DOOM").unwrap();
    assert_eq!(
        config.backend,
        Backend::Remote(RemoteConfig {
            dsn: "https://deploy.example.com".to_string(),
            // the default of the selected config still applies
            cache_size: 10000,
        })
    );

    env::set_var("DOOM_MODE", "local");
    assert_eq!(
        DoomConfig::with_prefix("DOOM").unwrap().backend,
        Backend::Local
    );
}

#[test]
#[serial]
fn the_selected_config_keeps_its_required_fields() {
    clean_env();
    env::set_var("DOOM_MODE", "remote");

    let err = DoomConfig::with_prefix("DOOM").unwrap_err();
    assert!(matches!(err, EnvStructError::MissingEnvVar(name) if name == "DOOM_REMOTE_DSN"));
}

#[test]
#[serial]
fn a_variant_that_is_not_selected_is_not_parsed() {
    clean_env();
    env::set_var("DOOM_MODE", "local");
    env::set_var("DOOM_REMOTE_CACHE_SIZE", "not-a-number");

    assert_eq!(
        DoomConfig::with_prefix("DOOM").unwrap().backend,
        Backend::Local
    );
}

#[test]
#[serial]
fn a_missing_switch_falls_back_to_the_default_of_the_field() {
    clean_env();
    assert_eq!(
        DoomConfig::with_prefix("DOOM").unwrap().backend,
        Backend::Local
    );
}

#[test]
#[serial]
fn a_missing_switch_without_a_default_is_reported_as_missing() {
    clean_env();
    let err = Backend::with_prefix("DOOM").unwrap_err();
    assert!(matches!(err, EnvStructError::MissingEnvVar(name) if name == "DOOM_MODE"));
}

#[test]
#[serial]
fn an_unknown_switch_value_names_the_values_that_work() {
    clean_env();
    env::set_var("DOOM_MODE", "s3");

    let err = DoomConfig::with_prefix("DOOM").unwrap_err();
    assert_eq!(
        err.to_string(),
        "Configuration from environment variables failed. `DOOM_MODE` unable to parse value \
         `s3`, expected one of: local, remote"
    );
}

#[test]
#[serial]
fn an_unknown_default_is_reported_as_a_default() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        #[env(flatten, default = "s3")]
        pub backend: Backend,
    }

    clean_env();
    let err = Config::with_prefix("DOOM").unwrap_err();
    assert_eq!(
        err.to_string(),
        "Configuration from environment variables failed. `DOOM_MODE` unable to parse default \
         value `s3`, expected one of: local, remote"
    );
}

#[test]
#[serial]
fn every_variant_is_documented_with_the_condition_that_selects_it() {
    let tree = DoomConfig::get_usage_tree("DOOM", None).unwrap();

    let switch = find_field(&tree.items, "DOOM_MODE").unwrap();
    assert_eq!(switch.typ, UsageType::Enum);
    assert!(!switch.required);
    assert_eq!(switch.default.as_deref(), Some("local"));
    assert_eq!(
        switch.values.as_deref(),
        Some(["local".to_string(), "remote".to_string()].as_slice())
    );

    for (title, value) in [("Local", "local"), ("Remote", "remote")] {
        let group = find_group(&tree.items, title).unwrap();
        let used_if = group.used_if.as_ref().unwrap();
        assert_eq!(used_if.env_name, "DOOM_MODE");
        assert_eq!(used_if.value, value);
        assert_eq!(used_if.switch_default.as_deref(), Some("local"));
        assert!(
            used_if.enforced,
            "the parser selects the group, it is not only application usage"
        );
    }

    let remote = find_group(&tree.items, "Remote").unwrap();
    assert!(
        find_field(&remote.items, "DOOM_REMOTE_DSN")
            .unwrap()
            .required
    );
    assert!(
        !find_field(&remote.items, "DOOM_REMOTE_CACHE_SIZE")
            .unwrap()
            .required
    );
    assert!(find_group(&tree.items, "Local").unwrap().items.is_empty());
}

#[test]
#[serial]
fn what_the_help_lists_under_a_condition_is_what_that_mode_parses() {
    let tree = DoomConfig::get_usage_tree("DOOM", None).unwrap();

    for (title, expect) in [
        ("Local", Backend::Local),
        (
            "Remote",
            Backend::Remote(RemoteConfig {
                dsn: "dsn".to_string(),
                cache_size: 10000,
            }),
        ),
    ] {
        let group = find_group(&tree.items, title).unwrap();
        let used_if = group.used_if.as_ref().unwrap();

        clean_env();
        env::set_var(&used_if.env_name, &used_if.value);
        for field in group_fields(group) {
            if field.required {
                let err = DoomConfig::with_prefix("DOOM").unwrap_err();
                assert!(
                    matches!(err, EnvStructError::MissingEnvVar(ref name) if name == &field.name),
                    "{title}: usage marks {} required, parsing must ask for it, got {err}",
                    field.name
                );
                env::set_var(&field.name, "dsn");
            }
        }
        assert_eq!(DoomConfig::with_prefix("DOOM").unwrap().backend, expect);
    }
}

#[test]
#[serial]
fn renamed_and_flattened_variants_agree_between_parser_and_help() {
    #[derive(EnvStruct, Debug, PartialEq)]
    pub struct StoreConfig {
        pub avatars: Store,
    }

    #[derive(EnvStruct, Debug, PartialEq)]
    #[env(tag = "backend")]
    pub enum Store {
        #[env(name = "gcs", flatten)]
        GoogleCloud(GcsConfig),
        Local(LocalConfig),
    }

    #[derive(EnvStruct, Debug, PartialEq)]
    pub struct GcsConfig {
        pub bucket_name: String,
    }

    #[derive(EnvStruct, Debug, PartialEq)]
    pub struct LocalConfig {
        pub data_dir: String,
    }

    let tree = StoreConfig::get_usage_tree("APP", None).unwrap();
    let switch = find_field(&tree.items, "APP_AVATARS_BACKEND").unwrap();
    assert!(switch.required);
    assert_eq!(
        switch.values.as_deref(),
        Some(["gcs".to_string(), "local".to_string()].as_slice())
    );

    // a flattened variant drops its own segment, a plain one keeps it
    let gcs = find_group(&tree.items, "Gcs").unwrap();
    assert_eq!(gcs.used_if.as_ref().unwrap().value, "gcs");
    assert!(find_field(&gcs.items, "APP_AVATARS_BUCKET_NAME").is_some());
    let local = find_group(&tree.items, "Local").unwrap();
    assert!(find_field(&local.items, "APP_AVATARS_LOCAL_DATA_DIR").is_some());

    clean_env();
    env::set_var("APP_AVATARS_BACKEND", "gcs");
    env::set_var("APP_AVATARS_BUCKET_NAME", "avatars");
    assert_eq!(
        StoreConfig::with_prefix("APP").unwrap().avatars,
        Store::GoogleCloud(GcsConfig {
            bucket_name: "avatars".to_string()
        })
    );

    clean_env();
    env::set_var("APP_AVATARS_BACKEND", "local");
    env::set_var("APP_AVATARS_LOCAL_DATA_DIR", "/tmp/avatars");
    assert_eq!(
        StoreConfig::with_prefix("APP").unwrap().avatars,
        Store::Local(LocalConfig {
            data_dir: "/tmp/avatars".to_string()
        })
    );
}

#[test]
#[serial]
fn the_help_is_built_without_a_configured_environment() {
    clean_env();
    let usage = DoomConfig::usage_with_prefix("DOOM").unwrap();
    assert!(usage.contains("[selected when DOOM_MODE=local (default)]"));
    assert!(usage.contains("[selected when DOOM_MODE=remote]"));
    assert!(!usage.contains("[used when"));
    assert!(
        usage.lines().all(|line| line == line.trim_end()),
        "usage output must not have trailing whitespace"
    );
}

#[test]
#[serial]
fn usage_snapshot_of_an_enum_with_data() {
    let usage = DoomConfig::usage_with_prefix("DOOM").unwrap();
    let expected = r#"Environment variables

REQUIRED=yes means an explicit value is needed within the group's parsing scope.
DEFAULT is used when omitted; — = no default; "" = empty string.
All groups are shown, regardless of the current environment.
[selected when NAME=value] is checked by the parser: only the selected group is parsed.

Durations accept values such as 15s, 10m, and 24h.

VARIABLE                                   TYPE      REQUIRED  DEFAULT  VALUES
DOOM_RELOAD_DELAY                          duration  no        60s      —
DOOM_MODE                                  enum      no        local    local | remote
[selected when DOOM_MODE=local (default)]            yes
[selected when DOOM_MODE=remote]                     yes
  DOOM_REMOTE_CACHE_SIZE                   integer   no        10000    —
  DOOM_REMOTE_DSN                          string    yes       —        —
"#;
    if usage != expected {
        panic!("usage snapshot mismatch\n=== actual ===\n{usage}=== expected ===\n{expected}");
    }
}
