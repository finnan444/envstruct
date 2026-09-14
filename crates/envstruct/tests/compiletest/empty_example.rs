use envstruct::prelude::*;

#[derive(EnvStruct)]
pub struct Config {
    #[env(example = "")]
    pub dsn: String,
}

fn main() {}
