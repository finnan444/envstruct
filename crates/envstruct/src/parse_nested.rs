use crate::*;
use pastey::paste;

/// Trait for parsing nested environment variables.
pub trait EnvParseNested {
    /// Creates a new instance by parsing environment variables.
    ///
    /// # Errors
    ///
    /// Returns an `EnvStructError` if parsing fails.
    fn new() -> Result<Self, EnvStructError>
    where
        Self: Sized,
    {
        Self::with_prefix("")
    }

    /// Creates a new instance with a specified prefix by parsing environment variables.
    ///
    /// # Arguments
    ///
    /// * `prefix` - A prefix for the environment variables.
    ///
    /// # Errors
    ///
    /// Returns an `EnvStructError` if parsing fails, or if two fields claim one variable.
    fn with_prefix(prefix: impl AsRef<str>) -> Result<Self, EnvStructError>
    where
        Self: Sized,
    {
        let prefix = prefix.as_ref();
        Self::get_usage_tree(prefix, None)?.check_duplicates()?;
        Self::parse_from_env_var(prefix, None)
    }

    /// Parses with a prefix, and additionally fails when the environment holds a variable
    /// with that prefix that no field of this config declares — a typo, or a name left
    /// behind by a rename, which would otherwise be ignored until the value is missed.
    ///
    /// # Arguments
    ///
    /// * `prefix` - A prefix for the environment variables, which must not be empty.
    ///
    /// # Errors
    ///
    /// Returns `UnknownEnvVars` listing the variables nothing reads, or
    /// `StrictWithoutPrefix` when the prefix is empty, besides the errors of `with_prefix`.
    fn with_prefix_strict(prefix: impl AsRef<str>) -> Result<Self, EnvStructError>
    where
        Self: Sized,
    {
        Self::with_prefix_strict_allowing(prefix, &[])
    }

    /// Parses in strict mode, passing over the variables the platform or another library
    /// puts under the same prefix.
    ///
    /// # Arguments
    ///
    /// * `prefix` - A prefix for the environment variables, which must not be empty.
    /// * `allowed` - Names to accept as unknown, where a trailing `*` matches any suffix.
    ///
    /// # Errors
    ///
    /// Returns the errors of `with_prefix_strict` for the variables left unmatched.
    fn with_prefix_strict_allowing(
        prefix: impl AsRef<str>,
        allowed: &[&str],
    ) -> Result<Self, EnvStructError>
    where
        Self: Sized,
    {
        let prefix = prefix.as_ref();
        let tree = Self::get_usage_tree(prefix, None)?;
        tree.check_duplicates()?;
        crate::strict::check_unknown_vars(prefix, &tree.flatten_entries(), allowed)?;
        Self::parse_from_env_var(prefix, None)
    }

    /// Parses the environment variable with an optional default value.
    ///
    /// # Arguments
    ///
    /// * `var_name` - The name of the environment variable.
    /// * `default` - An optional default value.
    ///
    /// # Errors
    ///
    /// Returns an `EnvStructError` if parsing fails.
    fn parse_from_env_var(
        var_name: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<Self, EnvStructError>
    where
        Self: Sized;

    /// Builds the usage tree with a specified prefix and optional default value.
    ///
    /// # Arguments
    ///
    /// * `prefix` - A prefix for the environment variables.
    /// * `default` - An optional default value.
    ///
    /// # Errors
    ///
    /// Returns an `EnvStructError` if retrieval fails.
    fn get_usage_tree(
        prefix: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<UsageTree, EnvStructError>;

    /// Retrieves the environment entries with a specified prefix and optional default value.
    ///
    /// # Arguments
    ///
    /// * `prefix` - A prefix for the environment variables.
    /// * `default` - An optional default value.
    ///
    /// # Errors
    ///
    /// Returns an `EnvStructError` if retrieval fails.
    fn get_env_entries(
        prefix: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<Vec<EnvEntry>, EnvStructError> {
        Ok(Self::get_usage_tree(prefix, default)?.flatten_entries())
    }
}

impl<T: EnvParseNested> EnvParseNested for Option<T> {
    fn parse_from_env_var(
        var_name: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<Self, EnvStructError>
    where
        Self: Sized,
    {
        let var_name = var_name.as_ref();

        // Defining any environment variable of optional type makes the field required
        // otherwise it is None.
        if !T::get_env_entries(var_name, default)?
            .iter()
            .any(|entry| std::env::var_os(&entry.name).is_some())
        {
            return Ok(None);
        }

        Ok(Some(T::parse_from_env_var(var_name, default)?))
    }

    fn get_usage_tree(
        prefix: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<UsageTree, EnvStructError> {
        let mut tree = T::get_usage_tree(prefix, default)?;
        tree.kind = UsageTreeKind::OptionalStruct;
        Ok(tree)
    }
}

/// Value of the variable that selects the variant of an enum with data.
///
/// The derive macro reads the tag once, matches it against the declared variants, and asks
/// for this error when no variant matches.
pub struct EnumTag {
    /// Name of the variable the value came from.
    pub var_name: String,
    /// Trimmed value selecting the variant.
    pub value: String,
    from_default: bool,
}

impl EnumTag {
    /// Reads the tag from the environment, falling back to the default of the field.
    ///
    /// # Errors
    ///
    /// Returns `MissingEnvVar` when the variable is absent and the field has no default.
    pub fn read(var_name: String, default: Option<&str>) -> Result<Self, EnvStructError> {
        let value = match std::env::var(&var_name) {
            Ok(value) => value,
            Err(std::env::VarError::NotUnicode(_)) => {
                return Err(EnvStructError::InvalidVarFormat(var_name))
            }
            Err(std::env::VarError::NotPresent) => match default {
                Some(default) => {
                    return Ok(Self {
                        var_name,
                        value: default.trim().to_string(),
                        from_default: true,
                    })
                }
                None => return Err(EnvStructError::MissingEnvVar(var_name)),
            },
        };
        Ok(Self {
            var_name,
            value: value.trim().to_string(),
            from_default: false,
        })
    }

    /// Error for a value that matches no variant, naming the values that do.
    pub fn unknown_value_error(&self, values: &[&str]) -> EnvStructError {
        let source: BoxError = format!("expected one of: {}", values.join(", ")).into();
        let (var_name, var_value) = (self.var_name.clone(), self.value.clone());
        if self.from_default {
            EnvStructError::ParseDefaultError {
                var_name,
                var_value,
                source,
            }
        } else {
            EnvStructError::ParseEnvError {
                var_name,
                var_value,
                source,
            }
        }
    }
}

/// Concatenates two environment variable names with an underscore.
///
/// # Arguments
///
/// * `lhs` - The left-hand side of the environment variable name.
/// * `rhs` - The right-hand side of the environment variable name.
///
/// # Returns
///
/// A concatenated string of the two environment variable names.
pub fn concat_env_name(lhs: impl AsRef<str>, rhs: impl AsRef<str>) -> String {
    let (lhs, rhs) = (lhs.as_ref().to_uppercase(), rhs.as_ref().to_uppercase());
    #[cfg(feature = "env_uppercase")]
    let (lhs, rhs) = (lhs.to_uppercase(), rhs.to_uppercase());
    match (lhs.is_empty(), rhs.is_empty()) {
        (false, true) => lhs.to_string(),
        (true, false) => rhs.to_string(),
        _ => format!("{lhs}_{rhs}"),
    }
}

macro_rules! implement_nested_t {
    ($x:ty) => {
        paste! {
            impl<T: EnvParseNested> EnvParseNested for $x::<T> {
                fn parse_from_env_var(var_name: impl AsRef<str>, default: Option<&str>) -> Result<Self, EnvStructError> {
                    Ok(T::parse_from_env_var(var_name, default)?.into())
                }

                fn get_usage_tree(
                    prefix: impl AsRef<str>,
                    default: Option<&str>,
                ) -> Result<UsageTree, EnvStructError> {
                    T::get_usage_tree(prefix, default)
                }
            }
        }
    };
}

implement_nested_t!(std::cell::Cell);
implement_nested_t!(std::cell::RefCell);
implement_nested_t!(std::rc::Rc);
implement_nested_t!(std::sync::Arc);
