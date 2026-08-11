use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    COMPATIBLE_CHANNELS, COMPATIBLE_SAMPLE_RATE, EncodedFrameSlot, OpusPassthrough, PcmFormat,
    encoded_frame_queue,
};

struct CountingAllocator;

thread_local! {
    static COUNT_THIS_THREAD: Cell<bool> = const { Cell::new(false) };
}

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[allow(unsafe_code)]
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNT_THIS_THREAD.with(|enabled| {
            if enabled.get() {
                ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            }
        });
        // SAFETY: This adapter forwards the caller-provided allocation layout unchanged to the
        // process allocator and returns its result without changing ownership.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: `pointer` and `layout` are the exact pair previously returned by the process
        // allocator through this adapter, and this call transfers that allocation back once.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        COUNT_THIS_THREAD.with(|enabled| {
            if enabled.get() {
                ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            }
        });
        // SAFETY: The validated allocator contract is forwarded unchanged to `System`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        COUNT_THIS_THREAD.with(|enabled| {
            if enabled.get() {
                ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            }
        });
        // SAFETY: `pointer` and `layout` identify an allocation owned by `System`; `new_size` is
        // passed through unchanged and ownership follows the global allocator contract.
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

fn count_allocations(operation: impl FnOnce()) -> usize {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    COUNT_THIS_THREAD.with(|enabled| enabled.set(true));
    operation();
    COUNT_THIS_THREAD.with(|enabled| enabled.set(false));
    ALLOCATIONS.load(Ordering::Relaxed)
}

#[test]
fn passthrough_and_spsc_delivery_allocate_zero_times_after_construction() {
    const PACKET: [u8; 3] = [19 << 3, 0x11, 0x22];
    const ITERATIONS: usize = 20_000;

    let format = PcmFormat::new(COMPATIBLE_SAMPLE_RATE, COMPATIBLE_CHANNELS).unwrap();
    let mut router = OpusPassthrough::new(format);
    let (mut sender, mut receiver) = encoded_frame_queue(8).unwrap();
    let mut write_slot = EncodedFrameSlot::new();
    let mut read_slot = EncodedFrameSlot::new();

    router.route_packet(&PACKET, None, &mut write_slot).unwrap();
    sender.try_push(mem::take(&mut write_slot)).unwrap();
    assert!(receiver.try_pop_into(&mut read_slot));

    let allocations = count_allocations(|| {
        for _ in 0..ITERATIONS {
            router.route_packet(&PACKET, None, &mut write_slot).unwrap();
            sender.try_push(mem::take(&mut write_slot)).unwrap();
            assert!(receiver.try_pop_into(&mut read_slot));
            mem::swap(&mut write_slot, &mut read_slot);
        }
    });

    assert_eq!(allocations, 0);
}
