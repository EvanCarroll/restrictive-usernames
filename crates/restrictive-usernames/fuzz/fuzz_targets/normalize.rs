#![no_main]

use libfuzzer_sys::fuzz_target;
use restrictive_usernames_fuzz::{decode, normalize_properties};

fuzz_target!(|data: &[u8]| {
    if let Some((policy, name)) = decode(data) {
        normalize_properties(&policy, name);
    }
});
