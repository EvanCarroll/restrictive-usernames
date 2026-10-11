//! Policy parameters.

use crate::error::ParseRuleError;
use crate::kind::Kind;
use crate::rule::Rule;

/// The parameters a name is checked against.
///
/// The constructors carry the defaults; the builder methods adjust them in
/// `const` context. Non-exhaustive so a later parameter is not a breaking
/// change: start from [`Policy::for_kind`] or a kind's constructor, then
/// use the `with_*` methods or assign the public fields.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Policy {
    /// The profile.
    pub kind: Kind,
    /// Minimum length in code points (inclusive). RFC 8264 §5.2.6 leaves
    /// length to the application.
    pub min_len: u32,
    /// Maximum length in code points (inclusive).
    pub max_len: u32,
    /// Most consecutive marks (General Category M*, counted on the NFD form)
    /// allowed; exactly this many distinct marks pass. UTS #39 §5.4
    /// "Optional Detection" recommends limiting nonspacing-mark sequences;
    /// the same mark twice in a row is refused regardless.
    pub max_mark_run: u32,
}

impl Policy {
    /// [`Kind::Username`], 3 to 32 code points, at most 4 consecutive marks.
    pub const fn username() -> Self {
        Policy {
            kind: Kind::Username,
            min_len: 3,
            max_len: 32,
            max_mark_run: 4,
        }
    }

    /// [`Kind::Nickname`], 1 to 32 code points, at most 4 consecutive marks.
    pub const fn nickname() -> Self {
        Policy {
            kind: Kind::Nickname,
            min_len: 1,
            max_len: 32,
            max_mark_run: 4,
        }
    }

    /// [`Kind::Label`], 1 to 50 code points, at most 4 consecutive marks.
    pub const fn label() -> Self {
        Policy {
            kind: Kind::Label,
            min_len: 1,
            max_len: 50,
            max_mark_run: 4,
        }
    }

    /// Replace the minimum length.
    pub const fn with_min_len(self, n: u32) -> Self {
        Policy { min_len: n, ..self }
    }

    /// Replace the maximum length.
    pub const fn with_max_len(self, n: u32) -> Self {
        Policy { max_len: n, ..self }
    }

    /// Replace the most consecutive marks allowed.
    pub const fn with_max_mark_run(self, n: u32) -> Self {
        Policy {
            max_mark_run: n,
            ..self
        }
    }

    /// The default policy of `kind`.
    pub const fn for_kind(kind: Kind) -> Policy {
        match kind {
            Kind::Username => Policy::username(),
            Kind::Nickname => Policy::nickname(),
            Kind::Label => Policy::label(),
        }
    }

    /// Parse a [`Rule::as_str`] name and fill the parameters of
    /// [`Rule::Length`] and [`Rule::MarkRun`] from this policy.
    ///
    /// This is the way back from the SQL `text[]`: the names carry no
    /// parameters, the policy that produced them does. [`str::parse`]
    /// returns those two variants with zeroed parameters instead.
    pub fn parse_rule(&self, name: &str) -> Result<Rule, ParseRuleError> {
        Ok(match name.parse::<Rule>()? {
            Rule::Length { .. } => Rule::Length {
                min: self.min_len,
                max: self.max_len,
            },
            Rule::MarkRun { .. } => Rule::MarkRun {
                max: self.max_mark_run,
            },
            other => other,
        })
    }
}
