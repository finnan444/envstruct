//! From a mode next to an optional config, to an enum that carries it.

use envstruct::prelude::*;

// Before: the mode and the config it selects are two declarations. Nothing links them, so
// the link is repeated by hand in `used_if`, and the group has to be optional to keep the
// local mode parsable.
#[allow(non_camel_case_types)]
#[derive(EnvStruct, Debug, Clone, PartialEq, Eq, strum::Display, strum::EnumString)]
pub enum Mode {
    local,
    remote,
}

#[derive(EnvStruct, Debug)]
pub struct BeforeConfig {
    #[env(default = "local")]
    pub mode: Mode,

    #[env(title = "Remote", used_if = "mode=remote")]
    pub remote: Option<RemoteConfig>,

    #[env(default = "60s")]
    pub reload_delay: envstruct::Duration,
}

// After: one declaration. `DOOM_MODE=remote` selects the variant, parses `RemoteConfig` with
// its own rules and states the condition of its group in the help. No `used_if`.
#[derive(EnvStruct, Debug)]
#[env(tag = "mode")]
pub enum Backend {
    Local,
    Remote(RemoteConfig),
}

#[derive(EnvStruct, Debug)]
pub struct AfterConfig {
    #[env(flatten, default = "local")]
    pub backend: Backend,

    #[env(default = "60s")]
    pub reload_delay: envstruct::Duration,
}

#[derive(EnvStruct, Debug)]
pub struct RemoteConfig {
    pub dsn: String,

    #[env(default = "10000")]
    pub cache_size: u32,
}

fn main() {
    println!(
        "before:\n\n{}",
        BeforeConfig::usage_with_prefix("DOOM").unwrap()
    );
    println!(
        "after:\n\n{}",
        AfterConfig::usage_with_prefix("DOOM").unwrap()
    );
}
