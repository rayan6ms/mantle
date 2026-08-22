use crate::clock::Clock;
use crate::load::LoadScheduler;
use crate::model::{
    Configuration, EndReason, Event, EventDelivery, Frame, ListenerId, Manager, ManagerId, Marker,
    MarkerId, MarkerSignal, MarkerState, Player, PlayerId, PlaylistId, SyntheticPlaylist, Track,
    TrackId, TrackInfo, TrackState, Transition, UserDataToken,
};
use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceLimits {
    pub managers: usize,
    pub players_per_manager: usize,
    pub tracks: usize,
    pub playlists: usize,
    pub tracks_per_playlist: usize,
    pub listeners_per_player: usize,
    pub markers_per_track: usize,
    pub frames_per_track: usize,
    pub frame_bytes: usize,
    pub metadata_bytes: usize,
    pub pending_loads: usize,
    pub ordered_load_channels: usize,
    pub pending_loads_per_channel: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            managers: 16,
            players_per_manager: 1_024,
            tracks: 8_192,
            playlists: 1_024,
            tracks_per_playlist: 10_000,
            listeners_per_player: 1_024,
            markers_per_track: 1_024,
            frames_per_track: 256,
            frame_bytes: 1 << 20,
            metadata_bytes: 64 << 10,
            pending_loads: 5_000,
            ordered_load_channels: 1_024,
            pending_loads_per_channel: 1_024,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineError {
    ResourceLimit(&'static str),
    InvalidMetadata,
    InvalidFrame,
    InvalidPlaylist,
    UnknownManager,
    UnknownPlayer,
    UnknownTrack,
    ManagerShutdown,
    PlayerDestroyed,
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResourceLimit(resource) => {
                write!(formatter, "resource limit reached: {resource}")
            }
            Self::InvalidMetadata => {
                formatter.write_str("track metadata exceeds configured limits")
            }
            Self::InvalidFrame => formatter.write_str("synthetic frame exceeds configured limits"),
            Self::InvalidPlaylist => formatter.write_str("synthetic playlist is invalid"),
            Self::UnknownManager => formatter.write_str("unknown manager"),
            Self::UnknownPlayer => formatter.write_str("unknown player"),
            Self::UnknownTrack => formatter.write_str("unknown track"),
            Self::ManagerShutdown => formatter.write_str("manager is shut down"),
            Self::PlayerDestroyed => formatter.write_str("player is destroyed"),
        }
    }
}

impl std::error::Error for EngineError {}

/// Deterministic owner of player state. Operations return callbacks to dispatch after mutation.
#[derive(Clone, Debug)]
pub struct Engine<C: Clock> {
    clock: C,
    limits: ResourceLimits,
    next_id: u64,
    managers: BTreeMap<ManagerId, Manager>,
    players: BTreeMap<PlayerId, Player>,
    tracks: BTreeMap<TrackId, Track>,
    playlists: BTreeMap<PlaylistId, SyntheticPlaylist>,
    loads: LoadScheduler<String>,
}

impl<C: Clock> Engine<C> {
    #[must_use]
    pub fn new(clock: C, limits: ResourceLimits) -> Self {
        Self {
            clock,
            limits,
            next_id: 1,
            managers: BTreeMap::new(),
            players: BTreeMap::new(),
            tracks: BTreeMap::new(),
            playlists: BTreeMap::new(),
            loads: LoadScheduler::new(
                limits.pending_loads,
                limits.ordered_load_channels,
                limits.pending_loads_per_channel,
            ),
        }
    }

    /// Creates a manager with reference-compatible defaults.
    ///
    /// # Errors
    ///
    /// Returns a resource error when the manager bound is reached.
    pub fn create_manager(&mut self) -> Result<ManagerId, EngineError> {
        if self.managers.len() >= self.limits.managers {
            return Err(EngineError::ResourceLimit("managers"));
        }
        let id = ManagerId(self.allocate_id());
        self.managers.insert(
            id,
            Manager {
                configuration: Configuration::default(),
                players: Vec::new(),
                shutdown: false,
            },
        );
        Ok(id)
    }

    /// Creates a player owned by a live manager.
    ///
    /// # Errors
    ///
    /// Returns an ownership, lifecycle, or resource-limit error.
    pub fn create_player(&mut self, manager: ManagerId) -> Result<PlayerId, EngineError> {
        let now = self.clock.now();
        let state = self
            .managers
            .get(&manager)
            .ok_or(EngineError::UnknownManager)?;
        if state.shutdown {
            return Err(EngineError::ManagerShutdown);
        }
        if state.players.len() >= self.limits.players_per_manager {
            return Err(EngineError::ResourceLimit("players per manager"));
        }
        let id = PlayerId(self.allocate_id());
        self.players.insert(
            id,
            Player {
                manager,
                active_track: None,
                paused: false,
                volume: 100,
                listeners: Vec::new(),
                last_request: now,
                last_receive: now,
                stuck_event_sent: false,
                destroyed: false,
            },
        );
        self.managers
            .get_mut(&manager)
            .ok_or(EngineError::UnknownManager)?
            .players
            .push(id);
        Ok(id)
    }

    /// Registers a bounded synthetic track and its prebuilt frames.
    ///
    /// # Errors
    ///
    /// Returns a resource or validation error for oversized metadata or frame data.
    pub fn create_track(
        &mut self,
        info: TrackInfo,
        frames: impl IntoIterator<Item = Frame>,
    ) -> Result<TrackId, EngineError> {
        if self.tracks.len() >= self.limits.tracks {
            return Err(EngineError::ResourceLimit("tracks"));
        }
        if !self.valid_info(&info) {
            return Err(EngineError::InvalidMetadata);
        }
        let frames = frames.into_iter().collect::<VecDeque<_>>();
        if frames.len() > self.limits.frames_per_track
            || frames
                .iter()
                .any(|frame| frame.data.len() > self.limits.frame_bytes)
        {
            return Err(EngineError::InvalidFrame);
        }
        let id = TrackId(self.allocate_id());
        self.tracks.insert(
            id,
            Track {
                info,
                state: TrackState::Inactive,
                position: Duration::ZERO,
                user_data: None,
                markers: Vec::new(),
                frames,
            },
        );
        Ok(id)
    }

    /// Registers a playlist containing existing tracks.
    ///
    /// # Errors
    ///
    /// Returns a resource or validation error for invalid or oversized playlists.
    pub fn create_playlist(
        &mut self,
        playlist: SyntheticPlaylist,
    ) -> Result<PlaylistId, EngineError> {
        if self.playlists.len() >= self.limits.playlists {
            return Err(EngineError::ResourceLimit("playlists"));
        }
        if playlist.name.len() > self.limits.metadata_bytes
            || playlist.tracks.len() > self.limits.tracks_per_playlist
            || playlist
                .tracks
                .iter()
                .any(|track| !self.tracks.contains_key(track))
            || playlist
                .selected_track
                .is_some_and(|track| !playlist.tracks.contains(&track))
        {
            return Err(EngineError::InvalidPlaylist);
        }
        let id = PlaylistId(self.allocate_id());
        self.playlists.insert(id, playlist);
        Ok(id)
    }

    /// Returns a manager's configuration.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::UnknownManager`] for an unknown identifier.
    pub fn configuration(&self, manager: ManagerId) -> Result<Configuration, EngineError> {
        self.managers
            .get(&manager)
            .map(|manager| manager.configuration)
            .ok_or(EngineError::UnknownManager)
    }

    /// Returns a track snapshot by identifier.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::UnknownTrack`] for an unknown identifier.
    pub fn track(&self, track: TrackId) -> Result<&Track, EngineError> {
        self.tracks.get(&track).ok_or(EngineError::UnknownTrack)
    }

    /// Replaces metadata on an inactive synthetic track after compatibility decoding.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown track, invalid metadata, or a non-inactive track.
    pub fn replace_track_info(
        &mut self,
        track: TrackId,
        info: TrackInfo,
    ) -> Result<(), EngineError> {
        if !self.valid_info(&info) {
            return Err(EngineError::InvalidMetadata);
        }
        let state = self.track_mut(track)?;
        if state.state != TrackState::Inactive {
            return Err(EngineError::ResourceLimit(
                "metadata of active track cannot be replaced",
            ));
        }
        state.info = info;
        Ok(())
    }

    /// Returns the player's active track.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown or destroyed player.
    pub fn player_active_track(&self, player: PlayerId) -> Result<Option<TrackId>, EngineError> {
        self.player(player).map(|player| player.active_track)
    }

    /// Returns the player's pause state.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown or destroyed player.
    pub fn player_paused(&self, player: PlayerId) -> Result<bool, EngineError> {
        self.player(player).map(|player| player.paused)
    }

    /// Returns the player's volume.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown or destroyed player.
    pub fn player_volume(&self, player: PlayerId) -> Result<u16, EngineError> {
        self.player(player).map(|player| player.volume)
    }

    /// Sets and clamps volume to the reference range of 0 through 1000.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown or destroyed player.
    pub fn set_volume(&mut self, player: PlayerId, volume: i32) -> Result<(), EngineError> {
        self.player_mut(player)?.volume = u16::try_from(volume.clamp(0, 1_000))
            .map_err(|_| EngineError::ResourceLimit("player volume"))?;
        Ok(())
    }

    /// Adds a listener token while preserving registration order and duplicates.
    ///
    /// # Errors
    ///
    /// Returns a player or listener-limit error.
    pub fn add_listener(
        &mut self,
        player: PlayerId,
        listener: ListenerId,
    ) -> Result<(), EngineError> {
        if self.player(player)?.listeners.len() >= self.limits.listeners_per_player {
            return Err(EngineError::ResourceLimit("listeners per player"));
        }
        self.player_mut(player)?.listeners.push(listener);
        Ok(())
    }

    /// Removes every registration of a listener token.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown or destroyed player.
    pub fn remove_listener(
        &mut self,
        player: PlayerId,
        listener: ListenerId,
    ) -> Result<(), EngineError> {
        self.player_mut(player)?
            .listeners
            .retain(|candidate| *candidate != listener);
        Ok(())
    }

    /// Starts, replaces, or conditionally rejects a synthetic track.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown entity or destroyed player.
    pub fn start_track(
        &mut self,
        player: PlayerId,
        track: TrackId,
        no_interrupt: bool,
    ) -> Result<(bool, Transition), EngineError> {
        self.track(track)?;
        let previous = self.player(player)?.active_track;
        if no_interrupt && previous.is_some() {
            return Ok((false, Transition::default()));
        }

        let now = self.clock.now();
        {
            let player_state = self.player_mut(player)?;
            player_state.active_track = Some(track);
            player_state.last_request = now;
            player_state.last_receive = now;
            player_state.stuck_event_sent = false;
        }
        let mut transition = Transition::default();
        if let Some(previous) = previous {
            self.tracks
                .get_mut(&previous)
                .ok_or(EngineError::UnknownTrack)?
                .state = TrackState::Finished;
            self.emit(
                player,
                Event::TrackEnd {
                    player,
                    track: previous,
                    reason: EndReason::Replaced,
                },
                &mut transition,
            )?;
        }
        self.tracks
            .get_mut(&track)
            .ok_or(EngineError::UnknownTrack)?
            .state = TrackState::Playing;
        self.emit(player, Event::TrackStart { player, track }, &mut transition)?;
        Ok((true, transition))
    }

    /// Stops the active track with an explicit reason.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown entity or destroyed player.
    pub fn stop_track(
        &mut self,
        player: PlayerId,
        reason: EndReason,
    ) -> Result<Transition, EngineError> {
        let previous = self.player_mut(player)?.active_track.take();
        let mut transition = Transition::default();
        if let Some(track) = previous {
            let state = match reason {
                EndReason::Finished => MarkerState::Ended,
                _ => MarkerState::Stopped,
            };
            let track_state = self
                .tracks
                .get_mut(&track)
                .ok_or(EngineError::UnknownTrack)?;
            track_state.state = TrackState::Finished;
            transition
                .marker_signals
                .extend(drain_markers(track_state, state));
            self.emit(
                player,
                Event::TrackEnd {
                    player,
                    track,
                    reason,
                },
                &mut transition,
            )?;
        }
        Ok(transition)
    }

    /// Changes pause state and emits an event only on a real transition.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown or destroyed player.
    pub fn set_paused(
        &mut self,
        player: PlayerId,
        paused: bool,
    ) -> Result<Transition, EngineError> {
        if self.player(player)?.paused == paused {
            return Ok(Transition::default());
        }
        let now = self.clock.now();
        let state = self.player_mut(player)?;
        state.paused = paused;
        if !paused {
            state.last_receive = now;
        }
        let event = if paused {
            Event::PlayerPause { player }
        } else {
            Event::PlayerResume { player }
        };
        let mut transition = Transition::default();
        self.emit(player, event, &mut transition)?;
        Ok(transition)
    }

    /// Provides one synthetic frame and performs deterministic stuck detection.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown player, manager, or active track.
    pub fn provide(
        &mut self,
        player: PlayerId,
        timeout: Duration,
    ) -> Result<(Option<Frame>, Transition), EngineError> {
        let now = self.clock.now();
        let (track, paused) = {
            let state = self.player_mut(player)?;
            state.last_request = now;
            (state.active_track, state.paused)
        };
        if paused && timeout.is_zero() {
            return Ok((None, Transition::default()));
        }
        let Some(track) = track else {
            return Ok((None, Transition::default()));
        };
        let frame = self
            .tracks
            .get_mut(&track)
            .ok_or(EngineError::UnknownTrack)?
            .frames
            .pop_front();
        if let Some(frame) = frame {
            self.player_mut(player)?.last_receive = now;
            if frame.terminator {
                let transition = self.stop_track(player, EndReason::Finished)?;
                return Ok((None, transition));
            }
            return Ok((Some(frame), Transition::default()));
        }
        if !timeout.is_zero() {
            return Ok((None, Transition::default()));
        }
        let threshold = self
            .manager_configuration_for_player(player)?
            .stuck_threshold;
        let stuck = {
            let state = self.player(player)?;
            !state.stuck_event_sent && now.saturating_sub(state.last_receive) > threshold
        };
        let mut transition = Transition::default();
        if stuck {
            self.player_mut(player)?.stuck_event_sent = true;
            self.emit(
                player,
                Event::TrackStuck {
                    player,
                    track,
                    threshold,
                },
                &mut transition,
            )?;
        }
        Ok((None, transition))
    }

    /// Provides one queued frame directly from a track.
    ///
    /// This is used by compatibility adapters where the reference player delegates frame
    /// delivery to its active `InternalAudioTrack`.
    ///
    /// # Errors
    ///
    /// Returns an error when the track is unknown.
    pub fn provide_track(&mut self, track: TrackId) -> Result<Option<Frame>, EngineError> {
        Ok(self
            .tracks
            .get_mut(&track)
            .ok_or(EngineError::UnknownTrack)?
            .frames
            .pop_front()
            .filter(|frame| !frame.terminator))
    }

    /// Stops an idle active player once its manager cleanup threshold is reached.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown player or manager.
    pub fn check_cleanup(&mut self, player: PlayerId) -> Result<Transition, EngineError> {
        let threshold = self
            .manager_configuration_for_player(player)?
            .cleanup_threshold;
        let state = self.player(player)?;
        if state.active_track.is_some()
            && self.clock.now().saturating_sub(state.last_request) >= threshold
        {
            self.stop_track(player, EndReason::Cleanup)
        } else {
            Ok(Transition::default())
        }
    }

    /// Stops and permanently destroys a player.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown or already destroyed player.
    pub fn destroy_player(&mut self, player: PlayerId) -> Result<Transition, EngineError> {
        let transition = self.stop_track(player, EndReason::Stopped)?;
        self.player_mut(player)?.destroyed = true;
        Ok(transition)
    }

    /// Idempotently shuts down a manager, its players, and pending loads.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::UnknownManager`] for an unknown identifier.
    pub fn shutdown_manager(&mut self, manager: ManagerId) -> Result<Transition, EngineError> {
        let players = {
            let state = self
                .managers
                .get_mut(&manager)
                .ok_or(EngineError::UnknownManager)?;
            if state.shutdown {
                return Ok(Transition::default());
            }
            state.shutdown = true;
            state.players.clone()
        };
        let mut transition = Transition::default();
        for player in players {
            transition.append(self.destroy_player(player)?);
        }
        self.loads.shutdown();
        Ok(transition)
    }

    /// Releases a destroyed player from the live resource set.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown player or a missing owner manager.
    pub fn release_player(&mut self, player: PlayerId) -> Result<Transition, EngineError> {
        let destroyed = self
            .players
            .get(&player)
            .ok_or(EngineError::UnknownPlayer)?
            .destroyed;
        let transition = if destroyed {
            Transition::default()
        } else {
            self.destroy_player(player)?
        };
        let manager = self
            .players
            .get(&player)
            .ok_or(EngineError::UnknownPlayer)?
            .manager;
        self.players.remove(&player);
        self.managers
            .get_mut(&manager)
            .ok_or(EngineError::UnknownManager)?
            .players
            .retain(|candidate| *candidate != player);
        Ok(transition)
    }

    /// Releases an inactive track from the live resource set.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown or currently active track.
    pub fn release_track(&mut self, track: TrackId) -> Result<(), EngineError> {
        if self
            .players
            .values()
            .any(|player| player.active_track == Some(track))
        {
            return Err(EngineError::ResourceLimit(
                "active track cannot be released",
            ));
        }
        self.tracks
            .remove(&track)
            .map(|_| ())
            .ok_or(EngineError::UnknownTrack)
    }

    /// Shuts down and releases a manager and every player it owns.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::UnknownManager`] for an unknown identifier.
    pub fn release_manager(&mut self, manager: ManagerId) -> Result<Transition, EngineError> {
        let transition = self.shutdown_manager(manager)?;
        let players = self
            .managers
            .remove(&manager)
            .ok_or(EngineError::UnknownManager)?
            .players;
        for player in players {
            self.players.remove(&player);
        }
        Ok(transition)
    }

    /// Replaces the core's opaque user-data token.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::UnknownTrack`] for an unknown identifier.
    pub fn set_user_data(
        &mut self,
        track: TrackId,
        user_data: Option<UserDataToken>,
    ) -> Result<(), EngineError> {
        self.track_mut(track)?.user_data = user_data;
        Ok(())
    }

    /// Replaces all markers and returns overwrite or late callback signals.
    ///
    /// # Errors
    ///
    /// Returns a track or marker-limit error.
    pub fn set_marker(
        &mut self,
        track: TrackId,
        marker: Option<(MarkerId, Duration)>,
    ) -> Result<Vec<MarkerSignal>, EngineError> {
        let limits = self.limits;
        let track = self.track_mut(track)?;
        let mut signals = drain_markers(
            track,
            if marker.is_some() {
                MarkerState::Overwritten
            } else {
                MarkerState::Removed
            },
        );
        if let Some((id, timecode)) = marker {
            signals.extend(add_marker(track, id, timecode, limits.markers_per_track)?);
        }
        Ok(signals)
    }

    /// Adds a marker and returns an immediate late signal when appropriate.
    ///
    /// # Errors
    ///
    /// Returns a track or marker-limit error.
    pub fn add_marker(
        &mut self,
        track: TrackId,
        marker: MarkerId,
        timecode: Duration,
    ) -> Result<Vec<MarkerSignal>, EngineError> {
        let maximum = self.limits.markers_per_track;
        add_marker(self.track_mut(track)?, marker, timecode, maximum)
    }

    /// Removes a marker by identity and returns its callback signal.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::UnknownTrack`] for an unknown identifier.
    pub fn remove_marker(
        &mut self,
        track: TrackId,
        marker: MarkerId,
    ) -> Result<Vec<MarkerSignal>, EngineError> {
        let track = self.track_mut(track)?;
        if let Some(index) = track.markers.iter().position(|item| item.id == marker) {
            track.markers.remove(index);
            Ok(vec![MarkerSignal {
                marker,
                state: MarkerState::Removed,
            }])
        } else {
            Ok(Vec::new())
        }
    }

    /// Seeks a track and bypasses every marker at or before the target.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::UnknownTrack`] for an unknown identifier.
    pub fn seek(
        &mut self,
        track: TrackId,
        position: Duration,
    ) -> Result<Vec<MarkerSignal>, EngineError> {
        let track = self.track_mut(track)?;
        track.position = position;
        Ok(trigger_markers(track, position, MarkerState::Bypassed))
    }

    /// Advances normal playback and reaches every marker at or before the position.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::UnknownTrack`] for an unknown identifier.
    pub fn advance_playback(
        &mut self,
        track: TrackId,
        position: Duration,
    ) -> Result<Vec<MarkerSignal>, EngineError> {
        let track = self.track_mut(track)?;
        track.position = position;
        Ok(trigger_markers(track, position, MarkerState::Reached))
    }

    #[must_use]
    pub fn loads(&self) -> &LoadScheduler<String> {
        &self.loads
    }

    pub fn loads_mut(&mut self) -> &mut LoadScheduler<String> {
        &mut self.loads
    }

    fn allocate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).unwrap_or(1);
        id
    }

    fn valid_info(&self, info: &TrackInfo) -> bool {
        !info.identifier.is_empty()
            && [
                Some(info.title.as_str()),
                Some(info.author.as_str()),
                Some(info.identifier.as_str()),
                info.uri.as_deref(),
                info.artwork_url.as_deref(),
                info.isrc.as_deref(),
            ]
            .into_iter()
            .flatten()
            .all(|value| value.len() <= self.limits.metadata_bytes)
    }

    fn player(&self, player: PlayerId) -> Result<&Player, EngineError> {
        let player = self
            .players
            .get(&player)
            .ok_or(EngineError::UnknownPlayer)?;
        if player.destroyed {
            return Err(EngineError::PlayerDestroyed);
        }
        Ok(player)
    }

    fn player_mut(&mut self, player: PlayerId) -> Result<&mut Player, EngineError> {
        let player = self
            .players
            .get_mut(&player)
            .ok_or(EngineError::UnknownPlayer)?;
        if player.destroyed {
            return Err(EngineError::PlayerDestroyed);
        }
        Ok(player)
    }

    fn track_mut(&mut self, track: TrackId) -> Result<&mut Track, EngineError> {
        self.tracks.get_mut(&track).ok_or(EngineError::UnknownTrack)
    }

    fn manager_configuration_for_player(
        &self,
        player: PlayerId,
    ) -> Result<Configuration, EngineError> {
        let manager = self.player(player)?.manager;
        self.configuration(manager)
    }

    fn emit(
        &self,
        player: PlayerId,
        event: Event,
        transition: &mut Transition,
    ) -> Result<(), EngineError> {
        transition.events.push(event);
        transition.deliveries.extend(
            self.player(player)?
                .listeners
                .iter()
                .copied()
                .map(|listener| EventDelivery { listener, event }),
        );
        Ok(())
    }
}

fn add_marker(
    track: &mut Track,
    id: MarkerId,
    timecode: Duration,
    maximum: usize,
) -> Result<Vec<MarkerSignal>, EngineError> {
    if track.position >= timecode {
        return Ok(vec![MarkerSignal {
            marker: id,
            state: MarkerState::Late,
        }]);
    }
    if track.markers.len() >= maximum {
        return Err(EngineError::ResourceLimit("markers per track"));
    }
    track.markers.push(Marker { id, timecode });
    Ok(Vec::new())
}

fn trigger_markers(track: &mut Track, timecode: Duration, state: MarkerState) -> Vec<MarkerSignal> {
    let mut signals = Vec::new();
    track.markers.retain(|marker| {
        if timecode >= marker.timecode {
            signals.push(MarkerSignal {
                marker: marker.id,
                state,
            });
            false
        } else {
            true
        }
    });
    signals
}

fn drain_markers(track: &mut Track, state: MarkerState) -> Vec<MarkerSignal> {
    track
        .markers
        .drain(..)
        .map(|marker| MarkerSignal {
            marker: marker.id,
            state,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::ManualClock;

    fn info(identifier: &str) -> TrackInfo {
        TrackInfo {
            title: "Synthetic title".into(),
            author: "Synthetic author".into(),
            duration: Duration::from_secs(1),
            identifier: identifier.into(),
            is_stream: false,
            uri: Some(format!("oracle://{identifier}")),
            artwork_url: Some("oracle://artwork".into()),
            isrc: Some("ORACLE000001".into()),
        }
    }

    fn fixture() -> (
        ManualClock,
        Engine<ManualClock>,
        ManagerId,
        PlayerId,
        TrackId,
    ) {
        let clock = ManualClock::new();
        let mut engine = Engine::new(clock.clone(), ResourceLimits::default());
        let manager = engine.create_manager().unwrap();
        let player = engine.create_player(manager).unwrap();
        let track = engine
            .create_track(
                info("one"),
                [Frame::synthetic(Duration::ZERO, [1, 2, 3, 4])],
            )
            .unwrap();
        (clock, engine, manager, player, track)
    }

    #[test]
    fn defaults_match_reference_and_resources_are_bounded() {
        let (_clock, mut engine, manager, _player, _track) = fixture();
        assert_eq!(
            engine.configuration(manager).unwrap(),
            Configuration::default()
        );

        let limits = ResourceLimits {
            tracks: 0,
            ..ResourceLimits::default()
        };
        let manager = engine.create_manager().unwrap();
        assert!(engine.create_player(manager).is_ok());
        let mut bounded = Engine::new(ManualClock::new(), limits);
        assert_eq!(
            bounded.create_track(info("too-many"), []),
            Err(EngineError::ResourceLimit("tracks"))
        );
    }

    #[test]
    fn play_no_interrupt_replace_stop_and_listener_order_match_reference() {
        let (_clock, mut engine, _manager, player, first) = fixture();
        let second = engine.create_track(info("two"), []).unwrap();
        engine.add_listener(player, ListenerId(8)).unwrap();
        engine.add_listener(player, ListenerId(3)).unwrap();

        let (started, start) = engine.start_track(player, first, false).unwrap();
        assert!(started);
        assert_eq!(
            start.events,
            [Event::TrackStart {
                player,
                track: first
            }]
        );
        assert_eq!(
            start
                .deliveries
                .iter()
                .map(|item| item.listener)
                .collect::<Vec<_>>(),
            [ListenerId(8), ListenerId(3)]
        );

        let (started, ignored) = engine.start_track(player, second, true).unwrap();
        assert!(!started);
        assert_eq!(ignored, Transition::default());
        assert_eq!(engine.player_active_track(player).unwrap(), Some(first));

        let (started, replaced) = engine.start_track(player, second, false).unwrap();
        assert!(started);
        assert_eq!(
            replaced.events,
            [
                Event::TrackEnd {
                    player,
                    track: first,
                    reason: EndReason::Replaced,
                },
                Event::TrackStart {
                    player,
                    track: second,
                },
            ]
        );
        assert_eq!(
            engine
                .stop_track(player, EndReason::Stopped)
                .unwrap()
                .events,
            [Event::TrackEnd {
                player,
                track: second,
                reason: EndReason::Stopped,
            }]
        );
        assert_eq!(engine.player_active_track(player).unwrap(), None);
    }

    #[test]
    fn pause_resume_volume_user_data_and_frame_delivery_are_deterministic() {
        let (_clock, mut engine, _manager, player, track) = fixture();
        engine.set_volume(player, -10).unwrap();
        assert_eq!(engine.player_volume(player).unwrap(), 0);
        engine.set_volume(player, 2_000).unwrap();
        assert_eq!(engine.player_volume(player).unwrap(), 1_000);
        engine
            .set_user_data(track, Some(UserDataToken(44)))
            .unwrap();
        assert_eq!(
            engine.track(track).unwrap().user_data,
            Some(UserDataToken(44))
        );
        engine.start_track(player, track, false).unwrap();

        assert_eq!(
            engine.set_paused(player, true).unwrap().events,
            [Event::PlayerPause { player }]
        );
        assert!(engine.set_paused(player, true).unwrap().events.is_empty());
        assert!(engine.provide(player, Duration::ZERO).unwrap().0.is_none());
        assert_eq!(
            engine.set_paused(player, false).unwrap().events,
            [Event::PlayerResume { player }]
        );
        assert_eq!(
            engine
                .provide(player, Duration::ZERO)
                .unwrap()
                .0
                .unwrap()
                .data
                .as_ref(),
            [1, 2, 3, 4]
        );
    }

    #[test]
    fn internal_track_adapter_can_consume_its_queued_frame() {
        let (_clock, mut engine, _manager, _player, track) = fixture();
        let frame = engine.provide_track(track).unwrap().unwrap();
        assert_eq!(frame.data.as_ref(), [1, 2, 3, 4]);
        assert!(engine.provide_track(track).unwrap().is_none());
    }

    #[test]
    fn marker_states_cover_late_overwrite_remove_seek_playback_and_stop() {
        let (_clock, mut engine, _manager, player, track) = fixture();
        assert!(
            engine
                .set_marker(track, Some((MarkerId(1), Duration::from_millis(10))))
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            engine.seek(track, Duration::from_millis(10)).unwrap(),
            [MarkerSignal {
                marker: MarkerId(1),
                state: MarkerState::Bypassed,
            }]
        );
        assert_eq!(
            engine
                .add_marker(track, MarkerId(2), Duration::from_millis(5))
                .unwrap(),
            [MarkerSignal {
                marker: MarkerId(2),
                state: MarkerState::Late,
            }]
        );
        engine.seek(track, Duration::ZERO).unwrap();
        engine
            .set_marker(track, Some((MarkerId(3), Duration::from_millis(5))))
            .unwrap();
        assert_eq!(
            engine
                .set_marker(track, Some((MarkerId(4), Duration::from_millis(8))))
                .unwrap(),
            [MarkerSignal {
                marker: MarkerId(3),
                state: MarkerState::Overwritten,
            }]
        );
        assert_eq!(
            engine.remove_marker(track, MarkerId(4)).unwrap(),
            [MarkerSignal {
                marker: MarkerId(4),
                state: MarkerState::Removed,
            }]
        );
        engine
            .set_marker(track, Some((MarkerId(7), Duration::from_millis(9))))
            .unwrap();
        assert_eq!(
            engine.set_marker(track, None).unwrap(),
            [MarkerSignal {
                marker: MarkerId(7),
                state: MarkerState::Removed,
            }]
        );
        engine
            .add_marker(track, MarkerId(5), Duration::from_millis(8))
            .unwrap();
        assert_eq!(
            engine
                .advance_playback(track, Duration::from_millis(8))
                .unwrap(),
            [MarkerSignal {
                marker: MarkerId(5),
                state: MarkerState::Reached,
            }]
        );
        engine
            .add_marker(track, MarkerId(6), Duration::from_millis(9))
            .unwrap();
        engine.start_track(player, track, false).unwrap();
        assert_eq!(
            engine
                .stop_track(player, EndReason::Stopped)
                .unwrap()
                .marker_signals,
            [MarkerSignal {
                marker: MarkerId(6),
                state: MarkerState::Stopped,
            }]
        );
    }

    #[test]
    fn manual_time_proves_one_shot_stuck_cleanup_and_shutdown() {
        let (clock, mut engine, manager, player, track) = fixture();
        engine.start_track(player, track, false).unwrap();
        let _ = engine.provide(player, Duration::ZERO).unwrap();
        clock.advance(Duration::from_secs(11));
        assert!(matches!(
            engine
                .provide(player, Duration::ZERO)
                .unwrap()
                .1
                .events
                .as_slice(),
            [Event::TrackStuck { .. }]
        ));
        assert!(
            engine
                .provide(player, Duration::ZERO)
                .unwrap()
                .1
                .events
                .is_empty()
        );
        clock.advance(Duration::from_mins(1));
        assert!(matches!(
            engine.check_cleanup(player).unwrap().events.as_slice(),
            [Event::TrackEnd {
                reason: EndReason::Cleanup,
                ..
            }]
        ));
        assert_eq!(
            engine.shutdown_manager(manager).unwrap(),
            Transition::default()
        );
        assert_eq!(
            engine.shutdown_manager(manager).unwrap(),
            Transition::default()
        );
        assert_eq!(
            engine.create_player(manager),
            Err(EngineError::ManagerShutdown)
        );
    }

    #[test]
    fn operation_sequence_invariants_hold_exhaustively() {
        #[derive(Clone, Copy)]
        enum Operation {
            Play,
            Pause,
            Resume,
            Stop,
        }
        let operations = [
            Operation::Play,
            Operation::Pause,
            Operation::Resume,
            Operation::Stop,
        ];
        for encoded in 0..operations.len().pow(5) {
            let (_clock, mut engine, _manager, player, track) = fixture();
            let mut value = encoded;
            for _ in 0..5 {
                match operations[value % operations.len()] {
                    Operation::Play => {
                        let _ = engine.start_track(player, track, false);
                    }
                    Operation::Pause => {
                        let _ = engine.set_paused(player, true);
                    }
                    Operation::Resume => {
                        let _ = engine.set_paused(player, false);
                    }
                    Operation::Stop => {
                        let _ = engine.stop_track(player, EndReason::Stopped);
                    }
                }
                value /= operations.len();
                let active = engine.player_active_track(player).unwrap();
                if let Some(active) = active {
                    assert_eq!(active, track);
                    assert_eq!(engine.track(active).unwrap().state, TrackState::Playing);
                }
                assert!(engine.player_volume(player).unwrap() <= 1_000);
            }
        }
    }
}
