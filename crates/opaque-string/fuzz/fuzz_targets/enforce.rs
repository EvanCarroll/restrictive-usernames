#![no_main]

use libfuzzer_sys::fuzz_target;
use opaque_string_fuzz::enforce_properties;

fuzz_target!(|data: &[u8]| {
    if let Ok(password) = std::str::from_utf8(data) {
        enforce_properties(password);
    }
});
