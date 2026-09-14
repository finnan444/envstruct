#![allow(dead_code)]
use envstruct::prelude::*;
use serial_test::*;
use std::env;

#[derive(EnvStruct, Debug)]
pub struct Db {
    pub host: String,
}

/// `flatten` without a name of its own puts `host` next to the field below it.
#[derive(EnvStruct, Debug)]
pub struct Collide {
    #[env(flatten)]
    pub db: Db,

    pub host: u16,
}

#[derive(EnvStruct, Debug)]
pub struct Nested {
    pub host: String,
}

#[derive(EnvStruct, Debug)]
pub struct Distinct {
    pub db: Nested,

    pub host: u16,
}

#[derive(EnvStruct, Debug, PartialEq)]
pub struct Local {
    pub path: String,
}

#[derive(EnvStruct, Debug, PartialEq)]
pub struct Remote {
    pub path: String,
}

/// Both variants are flattened onto the prefix, so they declare the same variable.
#[derive(EnvStruct, Debug, PartialEq)]
#[env(tag = "mode")]
pub enum Backend {
    #[env(flatten)]
    Local(Local),

    #[env(flatten)]
    Remote(Remote),
}

#[derive(EnvStruct, Debug, PartialEq)]
pub struct Branchy {
    #[env(flatten, default = "local")]
    pub backend: Backend,
}

/// Two flattened structs that read one set of variables on purpose.
#[derive(EnvStruct, Debug)]
pub struct Aliased {
    #[env(flatten)]
    pub primary: Db,

    #[env(flatten)]
    pub replica: Option<Db>,
}

fn clean_env() {
    std::env::vars().for_each(|(name, _)| {
        std::env::remove_var(name);
    });
}

#[test]
#[serial]
fn test_duplicate_var_is_rejected() {
    clean_env();
    env::set_var("TEST_HOST", "8080");

    let error = Collide::with_prefix("TEST").unwrap_err();
    assert!(
        matches!(&error, EnvStructError::DuplicateEnvVar { name, .. } if name == "TEST_HOST"),
        "unexpected error: {error}"
    );
    assert!(error.to_string().contains("TEST_HOST"), "{error}");
}

#[test]
#[serial]
fn test_duplicate_var_does_not_break_usage() {
    clean_env();
    let usage = Collide::usage_with_prefix("TEST").unwrap();
    assert!(usage.contains("TEST_HOST"));
}

#[test]
#[serial]
fn test_distinct_vars_are_accepted() {
    clean_env();
    env::set_var("TEST_DB_HOST", "localhost");
    env::set_var("TEST_HOST", "8080");

    let config = Distinct::with_prefix("TEST").unwrap();
    assert_eq!(config.db.host, "localhost");
    assert_eq!(config.host, 8080);
}

#[test]
#[serial]
fn test_exclusive_branches_may_share_a_var() {
    clean_env();
    env::set_var("TEST_MODE", "local");
    env::set_var("TEST_PATH", "/tmp");

    let config = Branchy::with_prefix("TEST").unwrap();
    assert_eq!(
        config.backend,
        Backend::Local(Local {
            path: "/tmp".to_string()
        })
    );
}

#[test]
#[serial]
fn test_alias_of_the_same_type_is_accepted() {
    clean_env();
    env::set_var("TEST_HOST", "localhost");

    let config = Aliased::with_prefix("TEST").unwrap();
    assert_eq!(config.primary.host, "localhost");
    assert_eq!(config.replica.unwrap().host, "localhost");
}

#[derive(EnvStruct, Debug, PartialEq)]
#[env(tag = "mode")]
pub enum Colors {
    Red,
    Blue,
}

#[derive(EnvStruct, Debug, PartialEq)]
#[env(tag = "mode")]
pub enum Speeds {
    Red,
    Fast,
}

/// Two enums claim one tag variable while accepting different values.
#[derive(EnvStruct, Debug, PartialEq)]
pub struct TwoTags {
    #[env(flatten, default = "red")]
    pub colors: Colors,

    #[env(flatten, default = "red")]
    pub speeds: Speeds,
}

#[test]
#[serial]
fn test_two_tags_on_one_var_are_rejected() {
    clean_env();
    env::set_var("TEST_MODE", "red");

    let error = TwoTags::with_prefix("TEST").unwrap_err();
    assert!(
        matches!(&error, EnvStructError::DuplicateEnvVar { name, .. } if name == "TEST_MODE"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial]
fn test_duplicate_error_names_the_flattened_struct() {
    clean_env();
    let error = Collide::with_prefix("TEST").unwrap_err();
    assert_eq!(
        error.to_string(),
        "Configuration from environment variables failed. Environment variable `TEST_HOST` \
         is declared twice, in `Db` and in the top level"
    );
}
