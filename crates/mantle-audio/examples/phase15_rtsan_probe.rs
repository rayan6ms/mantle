#![allow(
    unsafe_code,
    reason = "the Phase 15 C ABI uses unsafe export attributes but accepts no pointers and performs no unsafe memory access"
)]

use std::cell::RefCell;
use std::mem;

use mantle_audio::{
    COMPATIBLE_CHANNELS, COMPATIBLE_SAMPLE_RATE, EncodedFrameConsumer, EncodedFrameProducer,
    EncodedFrameSlot, OpusPassthrough, PcmFormat, encoded_frame_queue,
};

const PACKET: [u8; 3] = [19 << 3, 0x11, 0x22];
const QUEUE_CAPACITY: usize = 8;
const MAX_ITERATIONS: usize = 16_384;

struct ProbeState {
    router: OpusPassthrough,
    producer: EncodedFrameProducer,
    consumer: EncodedFrameConsumer,
    write_slot: EncodedFrameSlot,
    read_slot: EncodedFrameSlot,
    checksum: u64,
}

impl ProbeState {
    fn create() -> Option<Self> {
        let format = PcmFormat::new(COMPATIBLE_SAMPLE_RATE, COMPATIBLE_CHANNELS).ok()?;
        let (producer, consumer) = encoded_frame_queue(QUEUE_CAPACITY).ok()?;
        Some(Self {
            router: OpusPassthrough::new(format),
            producer,
            consumer,
            write_slot: EncodedFrameSlot::new(),
            read_slot: EncodedFrameSlot::new(),
            checksum: 0xcbf2_9ce4_8422_2325,
        })
    }

    fn process(&mut self, iterations: usize) -> bool {
        if !(1..=MAX_ITERATIONS).contains(&iterations) {
            return false;
        }
        for sequence in 0..iterations {
            let Ok(route) = self
                .router
                .route_packet(&PACKET, None, &mut self.write_slot)
            else {
                return false;
            };
            if !route.delivered() {
                return false;
            }
            if self
                .producer
                .try_push(mem::take(&mut self.write_slot))
                .is_err()
            {
                return false;
            }
            if !self.consumer.try_pop_into(&mut self.read_slot) {
                return false;
            }
            self.checksum = (self.checksum ^ sequence as u64).wrapping_mul(0x100_0000_01b3);
            for byte in self.read_slot.data() {
                self.checksum = (self.checksum ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3);
            }
            mem::swap(&mut self.write_slot, &mut self.read_slot);
        }
        true
    }
}

thread_local! {
    static PROBE: RefCell<Option<ProbeState>> = const { RefCell::new(None) };
}

#[unsafe(no_mangle)]
pub extern "C" fn mantle_phase15_rtsan_setup() -> i32 {
    PROBE.with(|probe| {
        let Ok(mut probe) = probe.try_borrow_mut() else {
            return 1;
        };
        if probe.is_some() {
            return 2;
        }
        let Some(state) = ProbeState::create() else {
            return 3;
        };
        *probe = Some(state);
        0
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn mantle_phase15_rtsan_process(iterations: usize) -> i32 {
    PROBE.with(|probe| {
        let Ok(mut probe) = probe.try_borrow_mut() else {
            return 1;
        };
        let Some(state) = probe.as_mut() else {
            return 2;
        };
        i32::from(!state.process(iterations))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn mantle_phase15_rtsan_checksum() -> u64 {
    PROBE.with(|probe| {
        probe
            .try_borrow()
            .ok()
            .and_then(|probe| probe.as_ref().map(|state| state.checksum))
            .unwrap_or(0)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn mantle_phase15_rtsan_teardown() {
    PROBE.with(|probe| {
        if let Ok(mut probe) = probe.try_borrow_mut() {
            *probe = None;
        }
    });
}
