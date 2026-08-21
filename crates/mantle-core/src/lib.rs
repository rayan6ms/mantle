//! JVM-independent player state and synthetic test media for Mantle.

mod clock;
mod engine;
mod load;
mod model;
mod serialization;
mod source;
mod source_load;

pub use clock::{Clock, ManualClock, SystemClock};
pub use engine::{Engine, EngineError, ResourceLimits};
pub use load::{
    CancelledLoad, LoadError, LoadId, LoadKey, LoadScheduler, LoadState, OpaqueLoadKey,
    ScheduledLoad,
};
pub use model::{
    Configuration, EndReason, Event, EventDelivery, Frame, ListenerId, ManagerId, MarkerId,
    MarkerSignal, MarkerState, PlayerId, PlaylistId, ResamplingQuality, SyntheticPlaylist, Track,
    TrackId, TrackInfo, TrackState, Transition, UserDataToken,
};
pub use serialization::{
    DecodedSourceTrack, DecodedTrack, SerializationError, SerializationLimits,
    SourceTrackCodecError, decode_source_details, decode_source_track, decode_synthetic_track,
    decode_synthetic_track_details, encode_source_details, encode_source_track,
    encode_synthetic_track, encode_synthetic_track_details,
};
pub use source::{
    LoadedSourceItem, SourceCancellation, SourceDetails, SourceLoad, SourceManager,
    SourceReference, SourceRegistrationId, SourceRegistry, SourceRegistryError,
    SourceRegistryLimits,
};
pub use source_load::{
    LoadExecutorBuildError, LoadExecutorLimits, LoadHandleState, LoadTerminalHook,
    SourceLoadExecutor, SourceLoadFailure, SourceLoadHandle, SourceLoadResult,
    SourceLoadResultHandler, dispatch_source_load,
};
