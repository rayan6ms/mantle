use std::collections::VecDeque;
use std::time::Duration;

macro_rules! identifier {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub(crate) u64);

        impl $name {
            #[must_use]
            pub const fn get(self) -> u64 {
                self.0
            }
        }
    };
}

identifier!(ManagerId);
identifier!(PlayerId);
identifier!(TrackId);
identifier!(PlaylistId);
identifier!(ListenerId);
identifier!(MarkerId);
identifier!(UserDataToken);

impl ListenerId {
    #[must_use]
    pub const fn from_opaque(value: u64) -> Self {
        Self(value)
    }
}

impl MarkerId {
    #[must_use]
    pub const fn from_opaque(value: u64) -> Self {
        Self(value)
    }
}

impl UserDataToken {
    #[must_use]
    pub const fn from_opaque(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResamplingQuality {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Configuration {
    pub frame_buffer: Duration,
    pub cleanup_threshold: Duration,
    pub stuck_threshold: Duration,
    pub seek_ghosting: bool,
    pub resampling_quality: ResamplingQuality,
    pub opus_encoding_quality: u8,
    pub filter_hot_swap: bool,
    pub channel_count: u8,
    pub sample_rate: u32,
    pub chunk_sample_count: u16,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            frame_buffer: Duration::from_secs(5),
            cleanup_threshold: Duration::from_mins(1),
            stuck_threshold: Duration::from_secs(10),
            seek_ghosting: true,
            resampling_quality: ResamplingQuality::Low,
            opus_encoding_quality: 10,
            filter_hot_swap: false,
            channel_count: 2,
            sample_rate: 48_000,
            chunk_sample_count: 960,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackInfo {
    pub title: String,
    pub author: String,
    pub duration: Duration,
    pub identifier: String,
    pub is_stream: bool,
    pub uri: Option<String>,
    pub artwork_url: Option<String>,
    pub isrc: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackState {
    Inactive,
    Loading,
    Playing,
    Seeking,
    Stopping,
    Finished,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pub timecode: Duration,
    pub volume: u16,
    pub data: Box<[u8]>,
    pub terminator: bool,
}

impl Frame {
    #[must_use]
    pub fn synthetic(timecode: Duration, data: impl Into<Box<[u8]>>) -> Self {
        Self {
            timecode,
            volume: 100,
            data: data.into(),
            terminator: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkerState {
    Reached,
    Bypassed,
    Removed,
    Overwritten,
    Late,
    Stopped,
    Ended,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MarkerSignal {
    pub marker: MarkerId,
    pub state: MarkerState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Marker {
    pub id: MarkerId,
    pub timecode: Duration,
}

#[derive(Clone, Debug)]
pub struct Track {
    pub info: TrackInfo,
    pub state: TrackState,
    pub position: Duration,
    pub user_data: Option<UserDataToken>,
    pub(crate) markers: Vec<Marker>,
    pub(crate) frames: VecDeque<Frame>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntheticPlaylist {
    pub name: String,
    pub tracks: Vec<TrackId>,
    pub selected_track: Option<TrackId>,
    pub search_result: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndReason {
    Finished,
    LoadFailed,
    Stopped,
    Replaced,
    Cleanup,
}

impl EndReason {
    #[must_use]
    pub const fn may_start_next(self) -> bool {
        matches!(self, Self::Finished | Self::LoadFailed)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    TrackStart {
        player: PlayerId,
        track: TrackId,
    },
    TrackEnd {
        player: PlayerId,
        track: TrackId,
        reason: EndReason,
    },
    PlayerPause {
        player: PlayerId,
    },
    PlayerResume {
        player: PlayerId,
    },
    TrackStuck {
        player: PlayerId,
        track: TrackId,
        threshold: Duration,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventDelivery {
    pub listener: ListenerId,
    pub event: Event,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Transition {
    pub events: Vec<Event>,
    pub deliveries: Vec<EventDelivery>,
    pub marker_signals: Vec<MarkerSignal>,
}

impl Transition {
    pub(crate) fn append(&mut self, other: Self) {
        self.events.extend(other.events);
        self.deliveries.extend(other.deliveries);
        self.marker_signals.extend(other.marker_signals);
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Player {
    pub manager: ManagerId,
    pub active_track: Option<TrackId>,
    pub paused: bool,
    pub volume: u16,
    pub listeners: Vec<ListenerId>,
    pub last_request: Duration,
    pub last_receive: Duration,
    pub stuck_event_sent: bool,
    pub destroyed: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct Manager {
    pub configuration: Configuration,
    pub players: Vec<PlayerId>,
    pub shutdown: bool,
}
