#![no_main]

use libfuzzer_sys::fuzz_target;
use restrictive_usernames_fuzz::key_properties;

fuzz_target!(|data: &[u8]| {
    if let Ok(name) = std::str::from_utf8(data) {
        key_properties(name);
    }
});
