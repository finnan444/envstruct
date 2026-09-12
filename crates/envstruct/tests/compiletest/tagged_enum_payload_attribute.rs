use envstruct::prelude::*;

#[derive(EnvStruct)]
pub struct RemoteConfig {
    pub dsn: String,
}

#[derive(EnvStruct)]
#[env(tag = "mode")]
pub enum Backend {
    Local,
    Remote(#[env(default = "x")] RemoteConfig),
}

fn main() {}
