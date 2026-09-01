#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};

use mantle_media::{EncodedPacket, MediaLimits, MediaSession, PcmFrame};

struct CountingAllocator;

thread_local! {
    static COUNT_THIS_THREAD: Cell<bool> = const { Cell::new(false) };
    static ALLOCATION_CALLS: Cell<usize> = const { Cell::new(0) };
    static ALLOCATION_BYTES: Cell<usize> = const { Cell::new(0) };
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation(layout.size());
        // SAFETY: The layout is forwarded unchanged to the process allocator, whose returned
        // pointer and ownership contract are returned unchanged to the caller.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: This is the exact pointer/layout pair originally returned by `System` through
        // this adapter, and ownership is transferred back exactly once.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record_allocation(layout.size());
        // SAFETY: The layout is forwarded unchanged to the process allocator.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record_allocation(new_size);
        // SAFETY: `pointer` and `layout` identify a live allocation owned by `System`, and the
        // requested size and resulting ownership follow the global allocator contract.
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

fn record_allocation(bytes: usize) {
    COUNT_THIS_THREAD.with(|enabled| {
        if enabled.get() {
            ALLOCATION_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
            ALLOCATION_BYTES.with(|total| total.set(total.get().saturating_add(bytes)));
        }
    });
}

fn measured<T>(operation: impl FnOnce() -> T) -> (T, usize, usize) {
    ALLOCATION_CALLS.with(|calls| calls.set(0));
    ALLOCATION_BYTES.with(|bytes| bytes.set(0));
    COUNT_THIS_THREAD.with(|enabled| enabled.set(true));
    let result = operation();
    COUNT_THIS_THREAD.with(|enabled| enabled.set(false));
    (
        result,
        ALLOCATION_CALLS.with(Cell::get),
        ALLOCATION_BYTES.with(Cell::get),
    )
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/media/fixtures")
        .join(name)
}

#[test]
fn concurrent_allocation_measurements_are_isolated() {
    let start = Arc::new(Barrier::new(2));
    let finish = Arc::new(Barrier::new(2));

    let idle_start = Arc::clone(&start);
    let idle_finish = Arc::clone(&finish);
    let idle = std::thread::spawn(move || {
        measured(|| {
            idle_start.wait();
            idle_finish.wait();
        })
    });

    let allocating = std::thread::spawn(move || {
        measured(|| {
            start.wait();
            let allocation = Box::new([0_u8; 64]);
            std::hint::black_box(&allocation);
            finish.wait();
            allocation
        })
    });

    let ((), idle_calls, idle_bytes) = idle.join().unwrap();
    let (allocation, allocating_calls, allocating_bytes) = allocating.join().unwrap();
    std::hint::black_box(allocation);

    assert_eq!(idle_calls, 0);
    assert_eq!(idle_bytes, 0);
    assert_eq!(allocating_calls, 1);
    assert_eq!(allocating_bytes, 64);
}

#[test]
fn decoded_reads_have_a_stable_bounded_allocation_envelope_after_warmup() {
    const WARMUP_FRAMES: usize = 8;
    const MEASURED_FRAMES: usize = 64;

    for (name, maximum_bytes_per_read) in [
        ("tone-pcm-s16le.wav", 8_192),
        ("tone-mp3.mp3", 1_024),
        ("tone-aac-lc.m4a", 1_024),
        ("tone-he-aac-v1.m4a", 1_024),
        ("tone-he-aac-v2.m4a", 1_024),
    ] {
        let mut session = MediaSession::open_file(fixture(name), MediaLimits::default()).unwrap();
        let mut frame = PcmFrame::with_capacity(session.limits().max_pcm_samples_per_frame);
        let frame_storage = frame.samples().as_ptr();
        for _ in 0..WARMUP_FRAMES {
            assert!(session.read_pcm(&mut frame).unwrap(), "{name}");
        }

        let mut frames = 0_usize;
        let mut maximum_calls = 0_usize;
        let mut maximum_bytes = 0_usize;
        let mut total_calls = 0_usize;
        let mut total_bytes = 0_usize;
        while frames < MEASURED_FRAMES {
            let (has_frame, calls, bytes) = measured(|| session.read_pcm(&mut frame).unwrap());
            if !has_frame {
                break;
            }
            assert_eq!(frame.samples().as_ptr(), frame_storage, "{name}");
            maximum_calls = maximum_calls.max(calls);
            maximum_bytes = maximum_bytes.max(bytes);
            total_calls = total_calls.saturating_add(calls);
            total_bytes = total_bytes.saturating_add(bytes);
            frames += 1;
        }

        eprintln!(
            "{name}: frames={frames}, max_calls={maximum_calls}, max_bytes={maximum_bytes}, total_calls={total_calls}, total_bytes={total_bytes}"
        );
        assert_eq!(frames, MEASURED_FRAMES, "{name}");
        assert_eq!(maximum_calls, 1, "{name}");
        assert_eq!(total_calls, MEASURED_FRAMES, "{name}");
        assert!(
            maximum_bytes <= maximum_bytes_per_read,
            "{name}: {maximum_bytes}"
        );
        assert!(
            total_bytes <= MEASURED_FRAMES * maximum_bytes_per_read,
            "{name}: {total_bytes}"
        );
    }
}

#[test]
fn opus_packet_extraction_has_two_bounded_backend_allocations_per_packet() {
    const WARMUP_PACKETS: usize = 8;
    const MEASURED_PACKETS: usize = 64;
    const MAXIMUM_BYTES_PER_PACKET_READ: usize = 1_024;

    let mut session =
        MediaSession::open_file(fixture("tone-opus.webm"), MediaLimits::default()).unwrap();
    let mut packet = EncodedPacket::with_capacity(session.limits().max_packet_bytes);
    let packet_storage = packet.data().as_ptr();
    for _ in 0..WARMUP_PACKETS {
        assert!(session.read_encoded(&mut packet).unwrap());
    }

    let mut maximum_calls = 0_usize;
    let mut maximum_bytes = 0_usize;
    let mut total_calls = 0_usize;
    let mut total_bytes = 0_usize;
    for _ in 0..MEASURED_PACKETS {
        let (has_packet, calls, bytes) = measured(|| session.read_encoded(&mut packet).unwrap());
        assert!(has_packet);
        assert_eq!(packet.data().as_ptr(), packet_storage);
        maximum_calls = maximum_calls.max(calls);
        maximum_bytes = maximum_bytes.max(bytes);
        total_calls = total_calls.saturating_add(calls);
        total_bytes = total_bytes.saturating_add(bytes);
    }

    eprintln!(
        "tone-opus.webm: packets={MEASURED_PACKETS}, max_calls={maximum_calls}, max_bytes={maximum_bytes}, total_calls={total_calls}, total_bytes={total_bytes}"
    );
    assert_eq!(maximum_calls, 2);
    assert_eq!(total_calls, MEASURED_PACKETS * 2);
    assert!(maximum_bytes <= MAXIMUM_BYTES_PER_PACKET_READ);
    assert!(total_bytes <= MEASURED_PACKETS * MAXIMUM_BYTES_PER_PACKET_READ);
}
