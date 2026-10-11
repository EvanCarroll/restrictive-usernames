//! Error types.

/// A `kind` argument that names no [`restrictive_usernames::Kind`].
///
/// Crosses into JavaScript as a thrown `Error` carrying this message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown kind \"{0}\": expected \"username\", \"nickname\" or \"label\"")]
pub struct UnknownKind(pub String);

impl From<&str> for UnknownKind {
    fn from(kind: &str) -> Self {
        Self(kind.to_owned())
    }
}
