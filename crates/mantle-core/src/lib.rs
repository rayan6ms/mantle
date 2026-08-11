//! JVM-independent player state and synthetic test media for Mantle.

mod clock;
mod engine;
mod load;
mod model;
mod serialization;

pub use clock::{Clock, ManualClock, SystemClock};
pub use engine::{Engine, EngineError, ResourceLimits};
pub use load::{LoadError, LoadId, LoadKey, LoadScheduler, LoadState, ScheduledLoad};
pub use model::{
    Configuration, EndReason, Event, EventDelivery, Frame, ListenerId, ManagerId, MarkerId,
    MarkerSignal, MarkerState, PlayerId, PlaylistId, ResamplingQuality, SyntheticPlaylist, Track,
    TrackId, TrackInfo, TrackState, Transition, UserDataToken,
};
pub use serialization::{
    DecodedTrack, SerializationError, SerializationLimits, decode_synthetic_track,
    decode_synthetic_track_details, encode_synthetic_track, encode_synthetic_track_details,
};
