#![no_main]

use libfuzzer_sys::fuzz_target;
use restrictive_usernames_fuzz::{check_properties, decode};

fuzz_target!(|data: &[u8]| {
    if let Some((policy, name)) = decode(data) {
        check_properties(&policy, name);
    }
});
