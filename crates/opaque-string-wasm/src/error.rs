//! Error types.

/// Why a call throws.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The input holds an unpaired UTF-16 surrogate, which is no Unicode
    /// text at all; converting it would silently turn it into U+FFFD.
    #[error("input contains an unpaired surrogate")]
    LoneSurrogate,
    /// The engine's normalization predates the baseline.
    #[error(transparent)]
    Engine(#[from] opaque_string::UnsupportedEngine),
}

impl Error {
    /// The message thrown to JavaScript. A static string rather than
    /// `Display` (wasm-bindgen's blanket `From` for `JsError` calls
    /// `to_string`), which would pull `core::fmt` into the module.
    pub const fn message(&self) -> &'static str {
        match self {
            Error::LoneSurrogate => "input contains an unpaired surrogate",
            Error::Engine(_) => {
                "this JavaScript engine's String.prototype.normalize predates the Unicode baseline (see nfcBaseline())"
            }
        }
    }
}
