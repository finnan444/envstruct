use envstruct::prelude::*;

#[derive(EnvStruct)]
#[env(tag = "mode")]
pub enum Backend {
    Local,
    Remote { dsn: String },
}

fn main() {}
