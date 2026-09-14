#![allow(dead_code)]
use envstruct::prelude::*;
use serial_test::*;
use std::env;

#[derive(EnvStruct, Debug, PartialEq)]
pub struct Remote {
    pub dsn: String,
}

#[derive(EnvStruct, Debug, PartialEq)]
#[env(tag = "mode")]
pub enum Backend {
    Local,
    Remote(Remote),
}

#[derive(EnvStruct, Debug, PartialEq)]
pub struct Config {
    #[env(flatten, default = "local")]
    pub backend: Backend,

    #[env(default = 8080)]
    pub port: u16,
}

fn clean_env() {
    std::env::vars().for_each(|(name, _)| {
        std::env::remove_var(name);
    });
}

#[test]
#[serial]
fn test_strict_rejects_unknown_var() {
    clean_env();
    env::set_var("TEST_PROT", "9090");

    let error = Config::with_prefix_strict("TEST").unwrap_err();
    let EnvStructError::UnknownEnvVars { prefix, vars } = &error else {
        panic!("unexpected error: {error}");
    };
    assert_eq!(prefix, "TEST");
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].name, "TEST_PROT");
    assert_eq!(vars[0].suggestion.as_deref(), Some("TEST_PORT"));
    assert!(error.to_string().contains("TEST_PROT"), "{error}");
    assert!(error.to_string().contains("TEST_PORT"), "{error}");
}

#[test]
#[serial]
fn test_strict_accepts_declared_vars_of_inactive_branch() {
    clean_env();
    env::set_var("TEST_MODE", "local");
    env::set_var("TEST_REMOTE_DSN", "postgres://localhost");

    let config = Config::with_prefix_strict("TEST").unwrap();
    assert_eq!(config.backend, Backend::Local);
}

#[test]
#[serial]
fn test_strict_ignores_other_prefixes() {
    clean_env();
    env::set_var("OTHER_THING", "1");
    env::set_var("PATH", "/usr/bin");

    assert!(Config::with_prefix_strict("TEST").is_ok());
}

#[test]
#[serial]
fn test_strict_allows_listed_vars() {
    clean_env();
    env::set_var("TEST_OTEL_ENDPOINT", "http://localhost:4317");
    env::set_var("TEST_BUILD", "42");

    let error = Config::with_prefix_strict("TEST").unwrap_err();
    assert!(error.to_string().contains("TEST_OTEL_ENDPOINT"), "{error}");

    Config::with_prefix_strict_allowing("TEST", &["TEST_OTEL_*", "TEST_BUILD"]).unwrap();
}

#[test]
#[serial]
fn test_strict_needs_a_prefix() {
    clean_env();
    let error = Config::with_prefix_strict("").unwrap_err();
    assert!(
        matches!(error, EnvStructError::StrictWithoutPrefix),
        "unexpected error: {error}"
    );
}

#[test]
#[serial]
fn test_without_strict_unknown_vars_are_ignored() {
    clean_env();
    env::set_var("TEST_PROT", "9090");

    assert!(Config::with_prefix("TEST").is_ok());
}
