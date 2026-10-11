//! Error types.

use crate::rule::Rule;

/// A name failed one or more rules.
///
/// Returned by [`crate::normalize`]. The vector lists every failed rule in
/// [`Rule::ALL`] order, without duplicates, and is never empty.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}", join_rules(.0))]
pub struct Rejected(pub Vec<Rule>);

impl From<Vec<Rule>> for Rejected {
    fn from(rules: Vec<Rule>) -> Self {
        Self(rules)
    }
}

fn join_rules(rules: &[Rule]) -> String {
    rules
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// A string that is not the [`Rule::as_str`] name of any rule.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown rule name {0:?}")]
pub struct ParseRuleError(pub String);
