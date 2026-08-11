//! Bounded, decoder-only ownership of Mantle's fixed libxaac FFI boundary.
//!
//! This crate is the only Rust code allowed to call libxaac. Every native allocation size,
//! access-unit length, reported consumption count, output length, channel count, and sample rate is
//! checked before the corresponding pointer is exposed or copied. Unsafe operations are kept next
//! to their invariants so the higher-level media crate remains entirely safe Rust.

use std::alloc::{Layout, alloc_zeroed, dealloc};
use std::ffi::c_void;
use std::fmt;
use std::ptr::{self, NonNull};

const IA_NO_ERROR: i32 = 0;
const IA_FATAL_ERROR: u32 = 0x8000_0000;
const IA_API_CMD_GET_API_SIZE: i32 = 2;
const IA_API_CMD_INIT: i32 = 3;
const IA_API_CMD_SET_CONFIG_PARAM: i32 = 4;
const IA_API_CMD_GET_CONFIG_PARAM: i32 = 5;
const IA_API_CMD_GET_MEMTABS_SIZE: i32 = 6;
const IA_API_CMD_SET_MEMTABS_PTR: i32 = 7;
const IA_API_CMD_GET_N_MEMTABS: i32 = 8;
const IA_API_CMD_EXECUTE: i32 = 9;
const IA_API_CMD_GET_CURIDX_INPUT_BUF: i32 = 11;
const IA_API_CMD_SET_INPUT_BYTES: i32 = 12;
const IA_API_CMD_GET_OUTPUT_BYTES: i32 = 13;
const IA_API_CMD_GET_MEM_INFO_SIZE: i32 = 17;
const IA_API_CMD_GET_MEM_INFO_ALIGNMENT: i32 = 18;
const IA_API_CMD_GET_MEM_INFO_TYPE: i32 = 19;
const IA_API_CMD_SET_MEM_PTR: i32 = 22;
const IA_CMD_TYPE_INIT_API_PRE_CONFIG_PARAMS: i32 = 0x0100;
const IA_CMD_TYPE_INIT_API_POST_CONFIG_PARAMS: i32 = 0x0200;
const IA_CMD_TYPE_INIT_PROCESS: i32 = 0x0300;
const IA_CMD_TYPE_INIT_DONE_QUERY: i32 = 0x0400;
const IA_CMD_TYPE_DO_EXECUTE: i32 = 0x0100;
const IA_MEMTYPE_INPUT: u32 = 2;
const IA_MEMTYPE_OUTPUT: u32 = 3;
const IA_XHEAAC_DEC_INIT_NONFATAL_INSUFFICIENT_INPUT_BYTES: i32 = 0x1003;
const IA_XHEAAC_DEC_EXE_NONFATAL_INSUFFICIENT_INPUT_BYTES: i32 = 0x1804;

const CONFIG_PCM_WORD_SIZE: i32 = 0;
const CONFIG_SAMPLE_RATE: i32 = 1;
const CONFIG_CHANNELS: i32 = 2;
const CONFIG_DOWNMIX: i32 = 9;
const CONFIG_TO_STEREO: i32 = 10;
const CONFIG_DOWNSAMPLE_SBR: i32 = 11;
const CONFIG_IS_MP4: i32 = 12;
const CONFIG_MAX_CHANNELS: i32 = 13;
const CONFIG_COUPLING_CHANNELS: i32 = 14;
const CONFIG_DOWNMIX_STEREO: i32 = 15;
const CONFIG_DISABLE_SYNC: i32 = 16;
const CONFIG_AUTO_SBR_UPSAMPLE: i32 = 17;
const CONFIG_HQ_ESBR: i32 = 24;
const CONFIG_PS_ENABLE: i32 = 25;
const CONFIG_PEAK_LIMITER: i32 = 27;
const CONFIG_ERROR_CONCEALMENT: i32 = 29;
const CONFIG_ESBR: i32 = 40;
const MAX_NATIVE_BLOCKS: usize = 16;

unsafe extern "C" {
    fn ixheaacd_dec_api(object: *mut c_void, command: i32, index: i32, value: *mut c_void) -> i32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XaacProfile {
    AacLc,
    HeAacV1,
    HeAacV2,
}

#[derive(Clone, Debug)]
pub struct XaacConfig {
    pub audio_specific_config: Box<[u8]>,
    pub core_sample_rate: u32,
    pub profile: XaacProfile,
    pub max_access_unit_bytes: usize,
    pub max_pcm_bytes_per_frame: usize,
    pub max_native_memory_bytes: usize,
}

impl XaacConfig {
    fn validate(&self) -> Result<(), XaacError> {
        if self.audio_specific_config.is_empty() || self.audio_specific_config.len() > 4_096 {
            return Err(XaacError::InvalidConfig(
                "AudioSpecificConfig must contain 1..=4096 bytes",
            ));
        }
        if !(7_350..=192_000).contains(&self.core_sample_rate) {
            return Err(XaacError::InvalidConfig("unsupported core sample rate"));
        }
        if self.max_access_unit_bytes == 0
            || self.max_pcm_bytes_per_frame == 0
            || self.max_native_memory_bytes == 0
        {
            return Err(XaacError::InvalidConfig("decoder limits must be non-zero"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XaacError {
    InvalidConfig(&'static str),
    AllocationLimitExceeded {
        requested: usize,
        limit: usize,
    },
    AllocationFailed {
        size: usize,
        alignment: usize,
    },
    AccessUnitTooLarge {
        actual: usize,
        limit: usize,
    },
    NativeStatus {
        operation: &'static str,
        status: i32,
    },
    InputNotFullyConsumed {
        stage: &'static str,
        consumed: usize,
        supplied: usize,
    },
    NativeContract(&'static str),
    OutputTooLarge {
        actual: usize,
        limit: usize,
    },
}

impl fmt::Display for XaacError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(message) => write!(formatter, "invalid libxaac config: {message}"),
            Self::AllocationLimitExceeded { requested, limit } => write!(
                formatter,
                "libxaac requested {requested} native bytes; limit is {limit}"
            ),
            Self::AllocationFailed { size, alignment } => write!(
                formatter,
                "failed to allocate {size} native bytes aligned to {alignment}"
            ),
            Self::AccessUnitTooLarge { actual, limit } => write!(
                formatter,
                "AAC access unit has {actual} bytes; limit is {limit}"
            ),
            Self::NativeStatus { operation, status } => {
                write!(
                    formatter,
                    "libxaac {operation} failed with status {status:#010x}"
                )
            }
            Self::InputNotFullyConsumed {
                stage,
                consumed,
                supplied,
            } => write!(
                formatter,
                "libxaac consumed {consumed} of {supplied} bytes during {stage}"
            ),
            Self::NativeContract(message) => {
                write!(formatter, "libxaac contract failed: {message}")
            }
            Self::OutputTooLarge { actual, limit } => write!(
                formatter,
                "libxaac produced {actual} PCM bytes; limit is {limit}"
            ),
        }
    }
}

impl std::error::Error for XaacError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XaacDecodeStatus<'a> {
    NeedMoreInput,
    Frame(XaacPcm<'a>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct XaacPcm<'a> {
    bytes: &'a [u8],
    sample_rate: u32,
    channels: u16,
}

impl XaacPcm<'_> {
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.bytes
    }

    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[must_use]
    pub fn channels(&self) -> u16 {
        self.channels
    }
}

#[derive(Debug)]
pub struct XaacDecoder {
    config: XaacConfig,
    api: AlignedBuffer,
    memtabs: AlignedBuffer,
    blocks: Vec<AlignedBuffer>,
    input_index: usize,
    output_index: usize,
    input_capacity: usize,
    output_capacity: usize,
    initialized: bool,
    pending_input: Vec<u8>,
}

impl XaacDecoder {
    /// Creates an MP4-raw decoder and initializes it with a complete `AudioSpecificConfig`.
    ///
    /// # Errors
    ///
    /// Returns an error when a configured bound is invalid, libxaac requests more memory than the
    /// caller permits, allocation fails, or the native decoder rejects its initialization data.
    pub fn new(config: XaacConfig) -> Result<Self, XaacError> {
        config.validate()?;
        let mut api_size = 0_u32;
        native_call(
            ptr::null_mut(),
            IA_API_CMD_GET_API_SIZE,
            0,
            ptr_from_mut(&mut api_size),
            "API-size query",
        )?;
        let api_size = usize::try_from(api_size)
            .map_err(|_| XaacError::NativeContract("API size does not fit usize"))?;
        ensure_allocation_bound(api_size, config.max_native_memory_bytes)?;
        let api = AlignedBuffer::new(api_size, 4)?;
        native_call(
            api.as_void(),
            IA_API_CMD_INIT,
            IA_CMD_TYPE_INIT_API_PRE_CONFIG_PARAMS,
            ptr::null_mut(),
            "pre-configuration initialization",
        )?;
        let mut decoder = Self {
            config,
            api,
            memtabs: AlignedBuffer::new(1, 1)?,
            blocks: Vec::new(),
            input_index: usize::MAX,
            output_index: usize::MAX,
            input_capacity: 0,
            output_capacity: 0,
            initialized: false,
            pending_input: Vec::new(),
        };
        decoder.apply_config()?;
        decoder.allocate_native_memory()?;
        decoder.prime_audio_specific_config()?;
        Ok(decoder)
    }

    /// Decodes exactly one complete MP4 AAC access unit.
    ///
    /// # Errors
    ///
    /// Returns an error when the unit exceeds its configured or native input bound, the decoder
    /// violates its consumption/output contract, or native decoding fails.
    pub fn decode_access_unit(
        &mut self,
        access_unit: &[u8],
    ) -> Result<XaacDecodeStatus<'_>, XaacError> {
        self.queue_access_unit(access_unit)?;
        if !self.initialized {
            if !self.drive_initialization_step()? {
                return Ok(XaacDecodeStatus::NeedMoreInput);
            }
            if self.pending_input.is_empty() {
                return Ok(XaacDecodeStatus::NeedMoreInput);
            }
        }
        let supplied = self.pending_input.len();
        self.copy_input(&self.pending_input);
        self.set_input_bytes(supplied)?;
        let status = self.call(IA_API_CMD_EXECUTE, IA_CMD_TYPE_DO_EXECUTE, ptr::null_mut());
        if is_fatal(status) {
            return Err(XaacError::NativeStatus {
                operation: "decode",
                status,
            });
        }
        let consumed = self.query_i32(IA_API_CMD_GET_CURIDX_INPUT_BUF, 0, "input consumption")?;
        let consumed = usize::try_from(consumed)
            .map_err(|_| XaacError::NativeContract("negative input consumption"))?;
        if consumed != supplied {
            return Err(XaacError::InputNotFullyConsumed {
                stage: "access-unit decode",
                consumed,
                supplied,
            });
        }
        self.pending_input.clear();
        if status == IA_XHEAAC_DEC_EXE_NONFATAL_INSUFFICIENT_INPUT_BYTES {
            return Ok(XaacDecodeStatus::NeedMoreInput);
        }
        if status != IA_NO_ERROR {
            return Err(XaacError::NativeStatus {
                operation: "decode",
                status,
            });
        }
        let output_bytes =
            usize::try_from(self.query_i32(IA_API_CMD_GET_OUTPUT_BYTES, 0, "output-size query")?)
                .map_err(|_| XaacError::NativeContract("negative PCM output size"))?;
        if output_bytes == 0 {
            return Ok(XaacDecodeStatus::NeedMoreInput);
        }
        self.output_frame(output_bytes).map(XaacDecodeStatus::Frame)
    }

    fn queue_access_unit(&mut self, access_unit: &[u8]) -> Result<(), XaacError> {
        if access_unit.is_empty() {
            return Err(XaacError::NativeContract("empty AAC access unit"));
        }
        if access_unit.len() > self.config.max_access_unit_bytes
            || access_unit.len() > self.input_capacity
        {
            return Err(XaacError::AccessUnitTooLarge {
                actual: access_unit.len(),
                limit: self.config.max_access_unit_bytes.min(self.input_capacity),
            });
        }
        let pending_limit = self
            .config
            .max_access_unit_bytes
            .checked_add(self.config.audio_specific_config.len())
            .ok_or(XaacError::InvalidConfig("pending-input limit overflow"))?;
        let pending_len = self
            .pending_input
            .len()
            .checked_add(access_unit.len())
            .ok_or(XaacError::AccessUnitTooLarge {
                actual: usize::MAX,
                limit: pending_limit,
            })?;
        if pending_len > pending_limit || pending_len > self.input_capacity {
            return Err(XaacError::AccessUnitTooLarge {
                actual: pending_len,
                limit: pending_limit.min(self.input_capacity),
            });
        }
        self.pending_input.extend_from_slice(access_unit);
        Ok(())
    }

    fn output_frame(&self, output_bytes: usize) -> Result<XaacPcm<'_>, XaacError> {
        if output_bytes > self.output_capacity {
            return Err(XaacError::NativeContract(
                "reported PCM output exceeds native output block",
            ));
        }
        if output_bytes > self.config.max_pcm_bytes_per_frame {
            return Err(XaacError::OutputTooLarge {
                actual: output_bytes,
                limit: self.config.max_pcm_bytes_per_frame,
            });
        }
        if !output_bytes.is_multiple_of(2) {
            return Err(XaacError::NativeContract(
                "16-bit PCM output has an odd byte count",
            ));
        }
        let sample_rate = self.query_i32(
            IA_API_CMD_GET_CONFIG_PARAM,
            CONFIG_SAMPLE_RATE,
            "sample-rate query",
        )?;
        let channels = self.query_i32(
            IA_API_CMD_GET_CONFIG_PARAM,
            CONFIG_CHANNELS,
            "channel-count query",
        )?;
        let sample_rate = u32::try_from(sample_rate)
            .ok()
            .filter(|rate| (7_350..=192_000).contains(rate))
            .ok_or(XaacError::NativeContract("invalid output sample rate"))?;
        let channels = u16::try_from(channels)
            .ok()
            .filter(|count| (1..=2).contains(count))
            .ok_or(XaacError::NativeContract("invalid output channel count"))?;
        let output = self
            .blocks
            .get(self.output_index)
            .ok_or(XaacError::NativeContract("native output block is absent"))?;
        // SAFETY: libxaac owns no allocation. `output` remains alive for the returned borrow,
        // `output_bytes` was checked against its allocated capacity, and decode completed before
        // the immutable slice is created.
        let bytes = unsafe { std::slice::from_raw_parts(output.as_ptr(), output_bytes) };
        Ok(XaacPcm {
            bytes,
            sample_rate,
            channels,
        })
    }

    /// Recreates all decoder state from the bounded, owned configuration.
    ///
    /// # Errors
    ///
    /// Returns the same initialization errors as [`Self::new`].
    pub fn reset(&mut self) -> Result<(), XaacError> {
        let replacement = Self::new(self.config.clone())?;
        *self = replacement;
        Ok(())
    }

    fn apply_config(&mut self) -> Result<(), XaacError> {
        for (index, value) in [
            (CONFIG_PCM_WORD_SIZE, 16),
            (CONFIG_DOWNMIX, 0),
            (CONFIG_TO_STEREO, 1),
            (CONFIG_DOWNSAMPLE_SBR, 0),
            (CONFIG_IS_MP4, 1),
            (CONFIG_MAX_CHANNELS, 2),
            (CONFIG_COUPLING_CHANNELS, 0),
            (CONFIG_DOWNMIX_STEREO, 0),
            (CONFIG_DISABLE_SYNC, 1),
            (CONFIG_AUTO_SBR_UPSAMPLE, 1),
            (CONFIG_HQ_ESBR, 0),
            (
                CONFIG_PS_ENABLE,
                i32::from(self.config.profile == XaacProfile::HeAacV2),
            ),
            (CONFIG_PEAK_LIMITER, 0),
            (CONFIG_ERROR_CONCEALMENT, 1),
            (CONFIG_ESBR, 1),
            (
                CONFIG_SAMPLE_RATE,
                i32::try_from(self.config.core_sample_rate)
                    .map_err(|_| XaacError::InvalidConfig("core sample rate exceeds i32"))?,
            ),
        ] {
            self.set_i32(index, value, "configuration")?;
        }
        Ok(())
    }

    fn allocate_native_memory(&mut self) -> Result<(), XaacError> {
        let memtabs_size = self.query_u32(IA_API_CMD_GET_MEMTABS_SIZE, 0, "memory-table size")?;
        let memtabs_size = usize::try_from(memtabs_size)
            .map_err(|_| XaacError::NativeContract("memory-table size does not fit usize"))?;
        ensure_allocation_bound(memtabs_size, self.config.max_native_memory_bytes)?;
        self.memtabs = AlignedBuffer::new(memtabs_size, 4)?;
        native_call(
            self.api.as_void(),
            IA_API_CMD_SET_MEMTABS_PTR,
            0,
            self.memtabs.as_void(),
            "memory-table registration",
        )?;
        native_call(
            self.api.as_void(),
            IA_API_CMD_INIT,
            IA_CMD_TYPE_INIT_API_POST_CONFIG_PARAMS,
            ptr::null_mut(),
            "post-configuration initialization",
        )?;
        let count = self.query_i32(IA_API_CMD_GET_N_MEMTABS, 0, "native block count")?;
        let count = usize::try_from(count)
            .ok()
            .filter(|count| (1..=MAX_NATIVE_BLOCKS).contains(count))
            .ok_or(XaacError::NativeContract("invalid native block count"))?;
        let mut total = self.api.len().checked_add(self.memtabs.len()).ok_or(
            XaacError::AllocationLimitExceeded {
                requested: usize::MAX,
                limit: self.config.max_native_memory_bytes,
            },
        )?;
        for index in 0..count {
            let native_index = i32::try_from(index)
                .map_err(|_| XaacError::NativeContract("native block index exceeds i32"))?;
            let size = usize::try_from(self.query_u32(
                IA_API_CMD_GET_MEM_INFO_SIZE,
                native_index,
                "native block size",
            )?)
            .map_err(|_| XaacError::NativeContract("native block size does not fit usize"))?;
            let alignment = usize::try_from(self.query_u32(
                IA_API_CMD_GET_MEM_INFO_ALIGNMENT,
                native_index,
                "native block alignment",
            )?)
            .map_err(|_| XaacError::NativeContract("native alignment does not fit usize"))?;
            let memory_type = self.query_u32(
                IA_API_CMD_GET_MEM_INFO_TYPE,
                native_index,
                "native block type",
            )?;
            total = total
                .checked_add(size)
                .ok_or(XaacError::AllocationLimitExceeded {
                    requested: usize::MAX,
                    limit: self.config.max_native_memory_bytes,
                })?;
            ensure_allocation_bound(total, self.config.max_native_memory_bytes)?;
            let block = AlignedBuffer::new(size, alignment.max(1))?;
            native_call(
                self.api.as_void(),
                IA_API_CMD_SET_MEM_PTR,
                native_index,
                block.as_void(),
                "native block registration",
            )?;
            if memory_type == IA_MEMTYPE_INPUT {
                if self.input_index != usize::MAX {
                    return Err(XaacError::NativeContract("multiple native input blocks"));
                }
                self.input_index = self.blocks.len();
                self.input_capacity = size;
            } else if memory_type == IA_MEMTYPE_OUTPUT {
                if self.output_index != usize::MAX {
                    return Err(XaacError::NativeContract("multiple native output blocks"));
                }
                self.output_index = self.blocks.len();
                self.output_capacity = size;
            }
            self.blocks.push(block);
        }
        if self.input_index == usize::MAX || self.output_index == usize::MAX {
            return Err(XaacError::NativeContract(
                "native input or output block is absent",
            ));
        }
        Ok(())
    }

    fn prime_audio_specific_config(&mut self) -> Result<(), XaacError> {
        let config = self.config.audio_specific_config.clone();
        if config.len() > self.input_capacity {
            return Err(XaacError::AccessUnitTooLarge {
                actual: config.len(),
                limit: self.input_capacity,
            });
        }
        self.pending_input.extend_from_slice(&config);
        self.drive_initialization_step()?;
        Ok(())
    }

    fn drive_initialization_step(&mut self) -> Result<bool, XaacError> {
        if self.pending_input.is_empty() {
            return Ok(false);
        }
        let supplied = self.pending_input.len();
        self.copy_input(&self.pending_input);
        self.set_input_bytes(supplied)?;
        let status = self.call(IA_API_CMD_INIT, IA_CMD_TYPE_INIT_PROCESS, ptr::null_mut());
        if is_fatal(status) {
            return Err(XaacError::NativeStatus {
                operation: "AudioSpecificConfig initialization",
                status,
            });
        }
        let consumed = self.query_i32(IA_API_CMD_GET_CURIDX_INPUT_BUF, 0, "config consumption")?;
        let consumed = usize::try_from(consumed)
            .map_err(|_| XaacError::NativeContract("negative config consumption"))?;
        if consumed > supplied {
            return Err(XaacError::NativeContract(
                "config consumption exceeds supplied bytes",
            ));
        }
        self.pending_input.drain(..consumed);
        if status == IA_XHEAAC_DEC_INIT_NONFATAL_INSUFFICIENT_INPUT_BYTES {
            return Ok(false);
        }
        self.initialized = self.query_i32(
            IA_API_CMD_INIT,
            IA_CMD_TYPE_INIT_DONE_QUERY,
            "initialization completion",
        )? != 0;
        Ok(self.initialized)
    }

    fn copy_input(&self, bytes: &[u8]) {
        let input = &self.blocks[self.input_index];
        // SAFETY: the input block was allocated with `input_capacity` bytes and stays alive in
        // `self.blocks`. Callers check `bytes.len() <= input_capacity`; the regions cannot overlap.
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), input.as_ptr(), bytes.len());
            ptr::write_bytes(
                input.as_ptr().add(bytes.len()),
                0,
                self.input_capacity - bytes.len(),
            );
        }
    }

    fn set_input_bytes(&self, length: usize) -> Result<(), XaacError> {
        let mut length = i32::try_from(length)
            .map_err(|_| XaacError::NativeContract("input length exceeds i32"))?;
        native_call(
            self.api.as_void(),
            IA_API_CMD_SET_INPUT_BYTES,
            0,
            ptr_from_mut(&mut length),
            "input-size registration",
        )
    }

    fn set_i32(
        &self,
        index: i32,
        mut value: i32,
        operation: &'static str,
    ) -> Result<(), XaacError> {
        native_call(
            self.api.as_void(),
            IA_API_CMD_SET_CONFIG_PARAM,
            index,
            ptr_from_mut(&mut value),
            operation,
        )
    }

    fn query_i32(
        &self,
        command: i32,
        index: i32,
        operation: &'static str,
    ) -> Result<i32, XaacError> {
        let mut value = 0_i32;
        native_call(
            self.api.as_void(),
            command,
            index,
            ptr_from_mut(&mut value),
            operation,
        )?;
        Ok(value)
    }

    fn query_u32(
        &self,
        command: i32,
        index: i32,
        operation: &'static str,
    ) -> Result<u32, XaacError> {
        let mut value = 0_u32;
        native_call(
            self.api.as_void(),
            command,
            index,
            ptr_from_mut(&mut value),
            operation,
        )?;
        Ok(value)
    }

    fn call(&self, command: i32, index: i32, value: *mut c_void) -> i32 {
        // SAFETY: `api` is the live, correctly aligned object storage whose size libxaac reported;
        // command-specific value pointers are constructed by the checked wrappers above.
        unsafe { ixheaacd_dec_api(self.api.as_void(), command, index, value) }
    }
}

#[derive(Debug)]
struct AlignedBuffer {
    pointer: NonNull<u8>,
    layout: Layout,
    len: usize,
}

impl AlignedBuffer {
    fn new(size: usize, alignment: usize) -> Result<Self, XaacError> {
        let size = size.max(1);
        let alignment = alignment
            .max(1)
            .checked_next_power_of_two()
            .ok_or(XaacError::AllocationFailed { size, alignment })?;
        let layout = Layout::from_size_align(size, alignment)
            .map_err(|_| XaacError::AllocationFailed { size, alignment })?;
        // SAFETY: `layout` is validated above. Ownership transfers to `Self` and is released once
        // with the identical layout in `Drop`.
        let pointer = NonNull::new(unsafe { alloc_zeroed(layout) })
            .ok_or(XaacError::AllocationFailed { size, alignment })?;
        Ok(Self {
            pointer,
            layout,
            len: size,
        })
    }

    fn as_ptr(&self) -> *mut u8 {
        self.pointer.as_ptr()
    }

    fn as_void(&self) -> *mut c_void {
        self.as_ptr().cast()
    }

    fn len(&self) -> usize {
        self.len
    }
}

impl Drop for AlignedBuffer {
    fn drop(&mut self) {
        // SAFETY: this pointer was allocated with this exact layout in `AlignedBuffer::new` and
        // has not been deallocated or transferred.
        unsafe { dealloc(self.pointer.as_ptr(), self.layout) };
    }
}

fn ptr_from_mut<T>(value: &mut T) -> *mut c_void {
    ptr::from_mut(value).cast()
}

fn native_call(
    object: *mut c_void,
    command: i32,
    index: i32,
    value: *mut c_void,
    operation: &'static str,
) -> Result<(), XaacError> {
    // SAFETY: the optional object pointer and command-specific value pointer are supplied by
    // wrappers that keep their backing allocations alive for the duration of this call.
    let status = unsafe { ixheaacd_dec_api(object, command, index, value) };
    if status == IA_NO_ERROR {
        Ok(())
    } else {
        Err(XaacError::NativeStatus { operation, status })
    }
}

fn ensure_allocation_bound(requested: usize, limit: usize) -> Result<(), XaacError> {
    if requested > limit {
        Err(XaacError::AllocationLimitExceeded { requested, limit })
    } else {
        Ok(())
    }
}

fn is_fatal(status: i32) -> bool {
    status.cast_unsigned() & IA_FATAL_ERROR != 0
}
