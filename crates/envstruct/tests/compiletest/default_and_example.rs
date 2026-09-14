use envstruct::prelude::*;

#[derive(EnvStruct)]
pub struct Config {
    #[env(default = "lax", example = "strict")]
    pub same_site: String,
}

fn main() {}
