use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub schema_version: u32,
    pub name: String,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Action {
    CreateManager {
        id: String,
    },
    CreatePlayer {
        id: String,
    },
    ObserveConfiguration {
        id: String,
    },
    Load {
        id: String,
        identifier: String,
        track: String,
        #[serde(default)]
        ordered_key: Option<String>,
        #[serde(default)]
        cancel: bool,
    },
    ObserveTrack {
        id: String,
        track: String,
    },
    SetUserData {
        id: String,
        track: String,
        value: String,
    },
    SetMarker {
        id: String,
        track: String,
        marker: String,
        position_ms: i64,
    },
    RemoveMarker {
        id: String,
        track: String,
        marker: String,
    },
    Seek {
        id: String,
        track: String,
        position_ms: i64,
    },
    Play {
        id: String,
        track: String,
        #[serde(default)]
        no_interrupt: bool,
    },
    SetPaused {
        id: String,
        paused: bool,
    },
    ProvideFrame {
        id: String,
        timeout_ms: u64,
    },
    Stop {
        id: String,
    },
    EncodeTrack {
        id: String,
        track: String,
        encoding: String,
    },
    DecodeTrack {
        id: String,
        encoding: String,
        track: String,
    },
    Shutdown {
        id: String,
    },
}

impl Action {
    pub fn id(&self) -> &str {
        match self {
            Self::CreateManager { id }
            | Self::CreatePlayer { id }
            | Self::ObserveConfiguration { id }
            | Self::Load { id, .. }
            | Self::ObserveTrack { id, .. }
            | Self::SetUserData { id, .. }
            | Self::SetMarker { id, .. }
            | Self::RemoveMarker { id, .. }
            | Self::Seek { id, .. }
            | Self::Play { id, .. }
            | Self::SetPaused { id, .. }
            | Self::ProvideFrame { id, .. }
            | Self::Stop { id }
            | Self::EncodeTrack { id, .. }
            | Self::DecodeTrack { id, .. }
            | Self::Shutdown { id } => id,
        }
    }
}

impl Scenario {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(format!(
                "unsupported scenario schema version {}; expected {SCHEMA_VERSION}",
                self.schema_version
            ));
        }
        if self.name.trim().is_empty() {
            return Err("scenario name must not be empty".into());
        }
        if self.actions.is_empty() {
            return Err("scenario must contain at least one action".into());
        }

        let mut state = ValidationState::default();
        for (index, action) in self.actions.iter().enumerate() {
            state.observe(index, action)?;
        }

        if !state.shutdown {
            return Err("scenario must end with shutdown".into());
        }
        Ok(())
    }

    pub fn protocol(&self) -> String {
        let mut output = String::new();
        for action in &self.actions {
            output.push_str(&action.protocol_line());
            output.push('\n');
        }
        output
    }
}

#[derive(Default)]
struct ValidationState<'a> {
    action_ids: HashSet<&'a str>,
    tracks: HashSet<&'a str>,
    markers: HashSet<&'a str>,
    encodings: HashSet<&'a str>,
    manager: bool,
    player: bool,
    shutdown: bool,
}

impl<'a> ValidationState<'a> {
    fn observe(&mut self, index: usize, action: &'a Action) -> Result<(), String> {
        let id = action.id();
        if !valid_id(id) {
            return Err(format!(
                "action {index} has invalid id {id:?}; use ASCII letters, digits, '.', '_' or '-'"
            ));
        }
        if !self.action_ids.insert(id) {
            return Err(format!("duplicate action id {id:?}"));
        }
        if self.shutdown {
            return Err(format!("action {id:?} appears after shutdown"));
        }

        match action {
            Action::CreateManager { .. } => {
                if self.manager {
                    return Err("manager may only be created once".into());
                }
                self.manager = true;
            }
            Action::CreatePlayer { .. } => {
                require(self.manager, id, "manager")?;
                if self.player {
                    return Err("player may only be created once".into());
                }
                self.player = true;
            }
            Action::ObserveConfiguration { .. } => require(self.manager, id, "manager")?,
            Action::Load { track, cancel, .. } => {
                require(self.manager, id, "manager")?;
                if !valid_id(track) {
                    return Err(format!("action {id:?} has invalid track id {track:?}"));
                }
                if !cancel && !self.tracks.insert(track) {
                    return Err(format!("track id {track:?} is defined more than once"));
                }
            }
            Action::ObserveTrack { track, .. }
            | Action::SetUserData { track, .. }
            | Action::Seek { track, .. }
            | Action::Play { track, .. }
            | Action::EncodeTrack { track, .. } => {
                require_track(&self.tracks, id, track)?;
                if matches!(action, Action::Play { .. }) {
                    require(self.player, id, "player")?;
                }
                if let Action::EncodeTrack { encoding, .. } = action
                    && (!valid_id(encoding) || !self.encodings.insert(encoding))
                {
                    return Err(format!("invalid or duplicate encoding id {encoding:?}"));
                }
            }
            Action::SetMarker { track, marker, .. } => {
                require_track(&self.tracks, id, track)?;
                if !valid_id(marker) || !self.markers.insert(marker) {
                    return Err(format!("invalid or duplicate marker id {marker:?}"));
                }
            }
            Action::RemoveMarker { track, marker, .. } => {
                require_track(&self.tracks, id, track)?;
                if !self.markers.contains(marker.as_str()) {
                    return Err(format!(
                        "action {id:?} references unknown marker {marker:?}"
                    ));
                }
            }
            Action::SetPaused { .. } | Action::ProvideFrame { .. } | Action::Stop { .. } => {
                require(self.player, id, "player")?;
            }
            Action::DecodeTrack {
                encoding, track, ..
            } => {
                require(self.manager, id, "manager")?;
                if !self.encodings.contains(encoding.as_str()) {
                    return Err(format!(
                        "action {id:?} references unknown encoding {encoding:?}"
                    ));
                }
                if !valid_id(track) || !self.tracks.insert(track) {
                    return Err(format!("invalid or duplicate track id {track:?}"));
                }
            }
            Action::Shutdown { .. } => {
                require(self.manager, id, "manager")?;
                self.shutdown = true;
            }
        }
        Ok(())
    }
}

impl Action {
    fn protocol_line(&self) -> String {
        let fields = match self {
            Self::CreateManager { id } => vec![id.as_str(), "create_manager"],
            Self::CreatePlayer { id } => vec![id.as_str(), "create_player"],
            Self::ObserveConfiguration { id } => vec![id.as_str(), "observe_configuration"],
            Self::Load {
                id,
                identifier,
                track,
                ordered_key,
                cancel,
            } => {
                return format!(
                    "{id}\tload\t{}\t{}\t{}\t{cancel}",
                    hex(identifier),
                    hex(track),
                    ordered_key
                        .as_ref()
                        .map_or_else(|| "-".into(), |key| hex(key))
                );
            }
            Self::ObserveTrack { id, track } => {
                return format!("{id}\tobserve_track\t{}", hex(track));
            }
            Self::SetUserData { id, track, value } => {
                return format!("{id}\tset_user_data\t{}\t{}", hex(track), hex(value));
            }
            Self::SetMarker {
                id,
                track,
                marker,
                position_ms,
            } => {
                return format!(
                    "{id}\tset_marker\t{}\t{}\t{position_ms}",
                    hex(track),
                    hex(marker)
                );
            }
            Self::RemoveMarker { id, track, marker } => {
                return format!("{id}\tremove_marker\t{}\t{}", hex(track), hex(marker));
            }
            Self::Seek {
                id,
                track,
                position_ms,
            } => return format!("{id}\tseek\t{}\t{position_ms}", hex(track)),
            Self::Play {
                id,
                track,
                no_interrupt,
            } => return format!("{id}\tplay\t{}\t{no_interrupt}", hex(track)),
            Self::SetPaused { id, paused } => {
                return format!("{id}\tset_paused\t{paused}");
            }
            Self::ProvideFrame { id, timeout_ms } => {
                return format!("{id}\tprovide_frame\t{timeout_ms}");
            }
            Self::Stop { id } => vec![id.as_str(), "stop"],
            Self::EncodeTrack {
                id,
                track,
                encoding,
            } => return format!("{id}\tencode_track\t{}\t{}", hex(track), hex(encoding)),
            Self::DecodeTrack {
                id,
                encoding,
                track,
            } => return format!("{id}\tdecode_track\t{}\t{}", hex(encoding), hex(track)),
            Self::Shutdown { id } => vec![id.as_str(), "shutdown"],
        };
        fields.join("\t")
    }
}

fn require(value: bool, action: &str, dependency: &str) -> Result<(), String> {
    if value {
        Ok(())
    } else {
        Err(format!("action {action:?} requires {dependency}"))
    }
}

fn require_track(tracks: &HashSet<&str>, action: &str, track: &str) -> Result<(), String> {
    if tracks.contains(track) {
        Ok(())
    } else {
        Err(format!(
            "action {action:?} references unknown track {track:?}"
        ))
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn hex(value: &str) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(value.len() * 2);
    for byte in value.bytes() {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{Action, SCHEMA_VERSION, Scenario};

    fn valid_scenario() -> Scenario {
        Scenario {
            schema_version: SCHEMA_VERSION,
            name: "test".into(),
            actions: vec![
                Action::CreateManager { id: "m".into() },
                Action::CreatePlayer { id: "p".into() },
                Action::Load {
                    id: "load".into(),
                    identifier: "gate:track".into(),
                    track: "track".into(),
                    ordered_key: None,
                    cancel: false,
                },
                Action::Shutdown { id: "end".into() },
            ],
        }
    }

    #[test]
    fn validates_references_and_lifecycle() {
        valid_scenario().validate().unwrap();
        let mut invalid = valid_scenario();
        invalid.actions.swap(0, 1);
        assert!(invalid.validate().unwrap_err().contains("requires manager"));
    }

    #[test]
    fn protocol_hex_encodes_untrusted_text() {
        let protocol = valid_scenario().protocol();
        assert!(protocol.contains("676174653a747261636b"));
        assert!(!protocol.contains("gate:track"));
    }
}
