//! Error types.

use std::path::PathBuf;

/// Why the table could not be generated.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A UCD file could not be read.
    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    /// A UCD line did not parse.
    #[error("{file}:{line}: {message}")]
    Parse {
        file: &'static str,
        line: usize,
        message: String,
    },
    /// The UCD files are not the version the table is pinned to.
    #[error("{file} is Unicode {found}, expected {expected}")]
    Version {
        file: &'static str,
        found: String,
        expected: String,
    },
    /// A derivation invariant does not hold for this UCD.
    #[error("invariant violated: {0}")]
    Invariant(String),
    /// The output could not be written.
    #[error("cannot write {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
}
