use envstruct::prelude::*;

pub struct RemoteConfig {
    pub dsn: String,
}

#[derive(EnvStruct)]
pub enum Backend {
    Local,
    Remote(RemoteConfig),
}

fn main() {}
