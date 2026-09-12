# EnvStruct

EnvStruct simplifies the process of handling environment variables in Rust applications. It provides tools to parse common types from environment variables and map them into nested structures effortlessly. With derive macros, the crate ensures clean and readable code. EnvStruct offers built-in support for various types, making it easy to get started quickly.

## Installation

Add the following to your Cargo.toml:

```toml
[dependencies]
envstruct = "1.0"
```

```rust
use envstruct::prelude::*;

#[derive(EnvStruct, Debug)]
pub struct Config {
    pub db: DB,

    #[env(default = "https://example.com")]
    pub url: url::Url,

    #[env(default = "/var/log/app.log")]
    pub file_path: std::path::PathBuf,

    #[env(default = "1h")]
    pub duration: envstruct::Duration,

    #[env(default = "10Mib")]
    pub bytesize: envstruct::ByteSize,
}

#[derive(EnvStruct, Debug)]
pub struct DB {
    #[env(default = "localhost")]
    pub host: String,

    #[env(default = 8080)]
    pub port: u16,

    #[env(default = false)]
    pub debug: bool,
}

fn main() -> Result<(), envstruct::EnvStructError> {
    let config = Config::with_prefix("MY_APP")?;
    println!("{:#?}", config);
    println!("{}", Config::usage_with_prefix("MY_APP")?);
    Ok(())
}


```

## Features

- Nested Structures: Parse environment variables into nested Rust structures.
- Custom Parsing: Create custom parsers for special types.
- Prefix Support: Handle environment variables with a common prefix.
- Default Values: Set default values for environment variables.
- Error Handling: Get detailed error messages for troubleshooting.
- Testing: Well-tested library with many test cases.
- Derive Macros: Clean and readable code with derive macros.

## Complex Types

- Dates and Times: Parse `chrono::DateTime` and `chrono::NaiveDateTime` types.
- Durations: Parse durations like "1h", "30m", or "15s".
- URLs: Parse `url::Url` to handle and validate URLs.
- Regex Patterns: Parse `regex::Regex` for dynamic regular expressions.
- File Paths: Parse `std::path::PathBuf` for file and directory paths.
- Byte Sizes: Parse sizes like "10KB", "5MB", or "1GB" into bytes.
- JSON Values: Parse `serde_json::Value` for arbitrary JSON data.
- Collections: Parse `HashMap`, `BTreeMap`, and `HashSet` from environment variables.
- Vectors: Parse lists of items separated by commas.
- EnvMap: Parse variables with a common prefix into a `HashMap`.

## Macro Attributes

- `name`: Name of the environment variable for a field.
- `default`: Default value if the environment variable doesn't exist.
- `flatten`: Ignore the field name when collecting the full name of an environment variable.
- `with`: Custom parser for a field.
- `title`: Optional group heading in usage output. If omitted, the field name is used (`client_registry` → `Client Registry`).
- `used_if`: Application-usage condition as `field=value`. Shown in usage; not enforced by the parser.
- `inline`: Merge a nested struct's fields into the parent usage group.
- `skip`: Do not parse or document the field.
- `tag`: On an enum, the variable that selects the variant, as in `#[env(tag = "mode")]`. On a variant of such an enum, `name` renames the value that selects it and `flatten` drops its segment from the names of its payload.

## Enums with data

An enum without `tag` stays a single value parsed by its own `FromStr`. With `tag`, the variants carry their own configuration: the tag variable selects one variant, only its payload is parsed, and the help states the condition of every group by itself.

Before, the mode and the configuration it selects are two declarations. Nothing links them, so the link is repeated by hand in `used_if`, and the group has to be optional to keep the other mode parsable:

```rust
#[derive(EnvStruct, Debug)]
pub struct DeployConfig {
    #[env(default = "local")]
    pub mode: Mode,

    #[env(title = "Remote", used_if = "mode=remote")]
    pub remote: Option<RemoteConfig>,
}
```

After, the link lives in the type:

```rust
#[derive(EnvStruct, Debug)]
#[env(tag = "mode")]
pub enum Backend {
    Local,
    Remote(RemoteConfig),
}

#[derive(EnvStruct, Debug)]
pub struct DeployConfig {
    #[env(flatten, default = "local")]
    pub backend: Backend,
}
```

With the prefix `DEPLOY`, `DEPLOY_MODE` selects the variant:

```text
VARIABLE                                    | TYPE   | REQUIRED | DEFAULT | VALUES
--------------------------------------------+--------+----------+---------+---------------------
DEPLOY_MODE                                 | enum   | no       | "local" | "local" | "remote"
[selected when DEPLOY_MODE=local (default)] |        | yes
[selected when DEPLOY_MODE=remote]          |        | yes
  DEPLOY_REMOTE_DSN                         | string | yes      | —       | —
```

- A variant is a unit variant or a newtype variant holding one configuration; other shapes are rejected at compile time.
- The value selecting a variant is its name in snake case (`Remote` becomes `remote`), or `#[env(name = "...")]`.
- The payload of `Remote` parses from `DEPLOY_REMOTE_`; `#[env(flatten)]` on the variant parses it from `DEPLOY_` instead. Both the parser and the help use the same names.
- The payload keeps its own required fields and defaults; variables of the variants that are not selected are never parsed.
- The default variant is the default of the field holding the enum, as for any other value. Without one, a missing tag variable is a missing required variable.
- An unknown value names the values that would work.

## License

This project is licensed under the MPL-2 License. See the LICENSE file for details.
