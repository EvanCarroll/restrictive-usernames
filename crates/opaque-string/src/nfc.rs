//! The NFC backends: compiled-in tables, or the JavaScript engine.

use alloc::string::String;

use crate::space::map_space;

/// The prepared form: spaces mapped, then NFC.
#[cfg(feature = "nfc-tables")]
pub(crate) fn mapped_nfc(input: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    // NFC grows UTF-8 by at most a factor of three (UAX #15), so this
    // buffer never reallocates and leaves no unzeroed copy behind.
    let mut out = String::with_capacity(input.len().saturating_mul(3));
    out.extend(input.chars().map(map_space).nfc());
    out
}

/// The prepared form: spaces mapped, then the engine's
/// `String.prototype.normalize("NFC")`.
#[cfg(all(not(feature = "nfc-tables"), feature = "js", target_arch = "wasm32"))]
pub(crate) fn mapped_nfc(input: &str) -> String {
    use zeroize::Zeroize;
    let mut mapped: String = input.chars().map(map_space).collect();
    let out = engine_nfc(&mapped);
    mapped.zeroize();
    out
}

#[cfg(all(not(feature = "nfc-tables"), feature = "js", target_arch = "wasm32"))]
fn engine_nfc(s: &str) -> String {
    String::from(js_sys::JsString::from(s).normalize("NFC"))
}

/// Whether NFC here knows Unicode [`crate::NFC_BASELINE`]. Always `true`
/// with compiled-in tables.
#[cfg(feature = "nfc-tables")]
pub fn engine_supported() -> bool {
    true
}

/// Whether the JavaScript engine's NFC knows Unicode
/// [`crate::NFC_BASELINE`]: `"a"`, a combining mark of exactly that age,
/// U+0301 composes to `"á"` and the mark only on an engine that knows the
/// mark. Probed once, then cached.
#[cfg(all(not(feature = "nfc-tables"), feature = "js", target_arch = "wasm32"))]
pub fn engine_supported() -> bool {
    use core::sync::atomic::{AtomicU8, Ordering};
    const UNKNOWN: u8 = 0;
    const YES: u8 = 1;
    const NO: u8 = 2;
    static STATE: AtomicU8 = AtomicU8::new(UNKNOWN);
    match STATE.load(Ordering::Relaxed) {
        YES => true,
        NO => false,
        _ => {
            let c = crate::tables::CANARY;
            let mut probe = String::from("a");
            probe.push(c);
            probe.push('\u{0301}');
            let mut expected = String::from("\u{E1}");
            expected.push(c);
            let ok = engine_nfc(&probe) == expected;
            STATE.store(if ok { YES } else { NO }, Ordering::Relaxed);
            ok
        }
    }
}
