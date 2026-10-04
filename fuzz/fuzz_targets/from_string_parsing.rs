#![no_main]
use libfuzzer_sys::fuzz_target;


fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let options = fugo_sass::Options::default();

        let _ = fugo_sass::from_string(
            s.to_owned(),
            &options
        );
    }
});
