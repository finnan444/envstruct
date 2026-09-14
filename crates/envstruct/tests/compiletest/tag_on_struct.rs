use envstruct::prelude::*;

#[derive(EnvStruct)]
#[env(tag = "mode")]
pub struct Config {
    pub dsn: String,
}

fn main() {}
