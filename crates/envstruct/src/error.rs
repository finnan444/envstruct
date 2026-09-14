use thiserror::Error;

/// A boxed error type that is `Send`, `Sync`, and `'static`.
pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

const CAPTION: &str = "Configuration from environment variables failed";

/// Represents errors that can occur while processing environment variables.
#[derive(Debug, Error)]
pub enum EnvStructError {
    /// Error that occurs when an environment variable cannot be parsed.
    ///
    /// `var_name` is the name of the environment variable.
    /// `var_value` is the value of the environment variable.
    /// `source` is the underlying error that caused this error.
    #[error("{CAPTION}. `{var_name}` unable to parse value `{var_value}`, {source}")]
    ParseEnvError {
        var_name: String,
        var_value: String,
        #[source]
        source: BoxError,
    },

    /// Error that occurs when a default value cannot be parsed.
    ///
    /// `var_name` is the name of the environment variable.
    /// `var_value` is the default value of the environment variable.
    /// `source` is the underlying error that caused this error.
    #[error("{CAPTION}. `{var_name}` unable to parse default value `{var_value}`, {source}")]
    ParseDefaultError {
        var_name: String,
        var_value: String,
        #[source]
        source: BoxError,
    },

    /// Error that occurs when an expected environment variable is missing.
    ///
    /// The string is the name of the missing environment variable.
    #[error("{CAPTION}. Environment variable `{0}` is not present")]
    MissingEnvVar(String),

    /// Error that occurs when an environment variable key has an invalid format.
    ///
    /// The string is the invalid key.
    #[error("{CAPTION}. Invalid key format `{0}`")]
    InvalidKeyFormat(String),

    /// Error that occurs when an environment variable value has an invalid format.
    ///
    /// The string is the invalid value.
    #[error("{CAPTION}. Invalid environment value format `{0}`")]
    InvalidVarFormat(String),

    /// Error that occurs when two fields that can be read at the same time claim one variable.
    ///
    /// `name` is the variable both fields read.
    /// `first` and `second` locate the declarations in the usage tree.
    #[error(
        "{CAPTION}. Environment variable `{name}` is declared twice, in {first} and in {second}"
    )]
    DuplicateEnvVar {
        name: String,
        first: String,
        second: String,
    },

    /// Error that occurs in strict mode when the environment holds variables with the prefix
    /// of the config that no field declares.
    ///
    /// `prefix` is the prefix the config was parsed with.
    /// `vars` are the variables nothing reads, sorted by name.
    #[error("{CAPTION}. Unknown environment variables with prefix `{prefix}`: {}", join_unknown(.vars))]
    UnknownEnvVars {
        prefix: String,
        vars: Vec<UnknownEnvVar>,
    },

    /// Error that occurs when strict mode is used without a prefix, which would make every
    /// variable of the process unknown.
    #[error("{CAPTION}. Strict mode needs a prefix to tell the variables of this config apart")]
    StrictWithoutPrefix,
}

/// A variable with the prefix of the config that no field declares.
#[derive(Debug)]
pub struct UnknownEnvVar {
    /// Name of the variable found in the environment.
    pub name: String,
    /// Declared variable it is close enough to be a typo of.
    pub suggestion: Option<String>,
}

fn join_unknown(vars: &[UnknownEnvVar]) -> String {
    vars.iter()
        .map(|var| match &var.suggestion {
            Some(suggestion) => format!("`{}` (did you mean `{suggestion}`?)", var.name),
            None => format!("`{}`", var.name),
        })
        .collect::<Vec<_>>()
        .join(", ")
}
