use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    COMPATIBLE_CHANNELS, COMPATIBLE_PCM_SAMPLES, COMPATIBLE_SAMPLE_RATE, EncodedFrameSlot,
    FilterChainBuilder, FilterPipeline, OpusEncodingQuality, OpusPassthrough, PcmFilter,
    PcmFilterFactory, PcmFormat, PcmFrame, PcmOpusDecoder, PcmOpusEncoder, StreamingPcmPoll,
    StreamingPcmProcessor, StreamingPcmProgress, VolumeLevel, encoded_frame_queue,
};

struct IdentityFilter;

impl PcmFilter for IdentityFilter {
    fn process(&mut self, _frame: &mut PcmFrame) -> Result<(), crate::AudioFrameError> {
        Ok(())
    }

    fn reset(&mut self) {}
}

struct IdentityFactory;

impl PcmFilterFactory for IdentityFactory {
    fn build(
        &self,
        _format: PcmFormat,
        builder: &mut FilterChainBuilder,
    ) -> Result<(), crate::AudioFrameError> {
        builder.push(IdentityFilter)
    }
}

struct StreamingIdentity;

impl StreamingPcmProcessor for StreamingIdentity {
    fn process(
        &mut self,
        input: &[f32],
        output: &mut [f32],
    ) -> Result<StreamingPcmProgress, crate::AudioFrameError> {
        let copied = input.len().min(output.len());
        output[..copied].copy_from_slice(&input[..copied]);
        Ok(StreamingPcmProgress::new(copied, copied))
    }

    fn finish(&mut self, _output: &mut [f32]) -> Result<usize, crate::AudioFrameError> {
        Ok(0)
    }

    fn reset(&mut self) {}
}

struct StreamingIdentityFactory;

impl PcmFilterFactory for StreamingIdentityFactory {
    fn build(
        &self,
        _format: PcmFormat,
        builder: &mut FilterChainBuilder,
    ) -> Result<(), crate::AudioFrameError> {
        builder.push_streaming(StreamingIdentity)
    }
}

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

#[test]
fn pcm_opus_encoding_and_spsc_delivery_allocate_zero_times_after_construction() {
    const ITERATIONS: usize = 5_000;

    let format = PcmFormat::new(COMPATIBLE_SAMPLE_RATE, COMPATIBLE_CHANNELS).unwrap();
    let mut pcm = PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES);
    pcm.copy_from_interleaved(&[0.125; COMPATIBLE_PCM_SAMPLES], format, None)
        .unwrap();
    let mut encoder = PcmOpusEncoder::new(OpusEncodingQuality::MAXIMUM).unwrap();
    let (mut sender, mut receiver) = encoded_frame_queue(8).unwrap();
    let mut write_slot = EncodedFrameSlot::new();
    let mut read_slot = EncodedFrameSlot::new();

    encoder
        .encode(&pcm, &mut write_slot, VolumeLevel::NORMAL)
        .unwrap();
    sender.try_push(mem::take(&mut write_slot)).unwrap();
    assert!(receiver.try_pop_into(&mut read_slot));

    let allocations = count_allocations(|| {
        for _ in 0..ITERATIONS {
            encoder
                .encode(&pcm, &mut write_slot, VolumeLevel::NORMAL)
                .unwrap();
            sender.try_push(mem::take(&mut write_slot)).unwrap();
            assert!(receiver.try_pop_into(&mut read_slot));
            mem::swap(&mut write_slot, &mut read_slot);
        }
    });

    assert_eq!(allocations, 0);
}

#[test]
fn opus_decode_filter_encode_allocates_zero_times_after_construction() {
    const ITERATIONS: usize = 5_000;

    let format = PcmFormat::new(COMPATIBLE_SAMPLE_RATE, COMPATIBLE_CHANNELS).unwrap();
    let mut source = PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES);
    source
        .copy_from_interleaved(&[0.125; COMPATIBLE_PCM_SAMPLES], format, None)
        .unwrap();
    let mut source_encoder = PcmOpusEncoder::new(OpusEncodingQuality::MAXIMUM).unwrap();
    let mut packet = EncodedFrameSlot::new();
    source_encoder
        .encode(&source, &mut packet, VolumeLevel::NORMAL)
        .unwrap();

    let mut decoder = PcmOpusDecoder::new(format, COMPATIBLE_PCM_SAMPLES / 2).unwrap();
    let mut pcm_output = PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES);
    let mut filters = FilterPipeline::new(format, 1).unwrap();
    filters.install_factory(Some(&IdentityFactory)).unwrap();
    let mut encoder = PcmOpusEncoder::new(OpusEncodingQuality::MAXIMUM).unwrap();
    let mut output = EncodedFrameSlot::new();

    decoder
        .decode(packet.data(), None, &mut pcm_output)
        .unwrap();
    filters.process(&mut pcm_output).unwrap();
    encoder
        .encode(&pcm_output, &mut output, VolumeLevel::NORMAL)
        .unwrap();

    let allocations = count_allocations(|| {
        for _ in 0..ITERATIONS {
            decoder
                .decode(packet.data(), None, &mut pcm_output)
                .unwrap();
            filters.process(&mut pcm_output).unwrap();
            encoder
                .encode(&pcm_output, &mut output, VolumeLevel::NORMAL)
                .unwrap();
        }
    });

    assert_eq!(allocations, 0);
}

#[test]
fn streaming_pcm_assembly_allocates_zero_times_after_construction() {
    const ITERATIONS: usize = 5_000;

    let format = PcmFormat::new(COMPATIBLE_SAMPLE_RATE, COMPATIBLE_CHANNELS).unwrap();
    let mut input = PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES);
    input
        .copy_from_interleaved(&[0.125; COMPATIBLE_PCM_SAMPLES], format, None)
        .unwrap();
    let mut output = PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES);
    let mut pipeline = FilterPipeline::new(format, 1).unwrap();
    pipeline
        .install_factory(Some(&StreamingIdentityFactory))
        .unwrap();

    let allocations = count_allocations(|| {
        for _ in 0..ITERATIONS {
            pipeline.submit_input(&input).unwrap();
            assert_eq!(
                pipeline.read_output(&mut output).unwrap(),
                StreamingPcmPoll::Frame
            );
            assert_eq!(
                pipeline.read_output(&mut output).unwrap(),
                StreamingPcmPoll::NeedInput
            );
        }
    });

    assert_eq!(allocations, 0);
}
