#![no_main]

use libfuzzer_sys::fuzz_target;
use mantle_media_fuzz::{LocalBoundary, exercise_local_boundary};

fuzz_target!(|data: &[u8]| exercise_local_boundary(data, LocalBoundary::Ogg));
