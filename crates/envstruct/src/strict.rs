use crate::*;
use std::collections::HashSet;

/// Fails when the environment holds a variable with this prefix that no field declares.
///
/// The environment is shared with the platform, so an unknown variable is not always a
/// mistake of the config: `allowed` lists the names to pass over, with a trailing `*`
/// matching any suffix.
pub(crate) fn check_unknown_vars(
    prefix: &str,
    entries: &[EnvEntry],
    allowed: &[&str],
) -> Result<(), EnvStructError> {
    if prefix.is_empty() {
        return Err(EnvStructError::StrictWithoutPrefix);
    }

    let declared: HashSet<&str> = entries.iter().map(|entry| entry.name.as_str()).collect();
    let scope = format!("{}_", concat_env_name(prefix, ""));

    let mut unknown: Vec<UnknownEnvVar> = std::env::vars_os()
        .filter_map(|(name, _)| name.into_string().ok())
        .filter(|name| name.starts_with(&scope))
        .filter(|name| !declared.contains(name.as_str()) && !is_allowed(name, allowed))
        .map(|name| UnknownEnvVar {
            suggestion: nearest_name(&name, &declared),
            name,
        })
        .collect();

    if unknown.is_empty() {
        return Ok(());
    }

    unknown.sort_by(|left, right| left.name.cmp(&right.name));
    Err(EnvStructError::UnknownEnvVars {
        prefix: prefix.to_string(),
        vars: unknown,
    })
}

fn is_allowed(name: &str, allowed: &[&str]) -> bool {
    allowed
        .iter()
        .any(|pattern| match pattern.strip_suffix('*') {
            Some(head) => name.starts_with(head),
            None => *pattern == name,
        })
}

/// The declared name an unknown variable is close enough to be a typo of.
fn nearest_name(name: &str, declared: &HashSet<&str>) -> Option<String> {
    // A suggestion only helps while it stays a plausible misspelling, so allow one edit per
    // three characters and never more than a third of the name.
    let limit = (name.chars().count() / 3).min(3);
    if limit == 0 {
        return None;
    }
    declared
        .iter()
        .map(|declared| (edit_distance(name, declared), *declared))
        .filter(|(distance, _)| *distance <= limit)
        .min_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(right.1)))
        .map(|(_, declared)| declared.to_string())
}

/// Levenshtein distance, kept to one row of the matrix.
fn edit_distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut row: Vec<usize> = (0..=right.len()).collect();

    for (left_index, left_char) in left.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = left_index + 1;
        for (right_index, right_char) in right.iter().enumerate() {
            let replace = diagonal + usize::from(left_char != *right_char);
            diagonal = row[right_index + 1];
            row[right_index + 1] = replace.min(row[right_index] + 1).min(diagonal + 1);
        }
    }
    row[right.len()]
}
