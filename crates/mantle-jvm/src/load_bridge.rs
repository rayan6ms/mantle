use crate::{JvmOrderingKeyReleaseQueue, JvmOrderingKeyTable, proxy};
use jni::objects::{JByteArray, JClass, JObject, JString, JValue};
use jni::refs::{Global, Weak};
use jni::{Env, JavaVM, jni_sig, jni_str};
use mantle_core::{
    LoadExecutorLimits, LoadTerminalHook, LoadedSourceItem, OpaqueLoadKey, SerializationLimits,
    SourceCancellation, SourceLoad, SourceLoadExecutor, SourceLoadHandle, SourceLoadResult,
    SourceLoadResultHandler, SourceManager, SourceReference, SourceRegistrationId, SourceRegistry,
    SourceRegistryError, SourceRegistryLimits, TrackInfo, decode_source_details,
    encode_source_details,
};
use mantle_media::{
    NicoNicoSourceManager, NicoNicoSourceOptions, NicoNicoSourceTrack, TwitchAuthentication,
    TwitchSourceManager, TwitchSourceOptions, TwitchSourceTrack, YandexMusicPlaylistKind,
    YandexMusicSourceItem, YandexMusicSourcePlaylist, YandexMusicSourceTrack, YoutubeSourceItem,
    YoutubeSourcePlaylist, YoutubeSourceTrack, route_twitch_identifier,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

const FUTURE_CLASS: &jni::strings::JNIStr = jni_str!("dev/mantle/internal/NativeLoadFuture");
const CALLBACK_CLASS: &jni::strings::JNIStr = jni_str!("dev/mantle/internal/NativeLoadCallback");
const MAXIMUM_LOADS: usize = 256;
const MAXIMUM_TRACKED_SOURCE_ITEMS: usize = 10_000;

struct GateSourceManager;

enum BridgeItem {
    Synthetic(String),
    Yandex(YandexMusicSourceItem),
    Youtube(YoutubeSourceItem),
    Track(Global<JObject<'static>>),
    Playlist(Global<JObject<'static>>),
}

struct GateYoutubeSourceManager;

struct GateYandexSourceManager;

impl SourceManager<BridgeItem> for GateYandexSourceManager {
    fn source_name(&self) -> &'static str {
        "yandex-music"
    }

    fn load(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<SourceLoad<BridgeItem>>, SourceRegistryError> {
        let item = match reference.identifier() {
            Some("gate:yandex-track") => YandexMusicSourceItem::Track(yandex_fixture_track(
                "71663565",
                "Animals",
                "Architects",
                Duration::from_millis(244_321),
            )),
            Some("gate:yandex-playlist" | "gate:yandex-search") => {
                let is_search_result = reference.identifier() == Some("gate:yandex-search");
                YandexMusicSourceItem::Playlist(YandexMusicSourcePlaylist {
                    name: if is_search_result {
                        "Search results for: architects".to_owned()
                    } else {
                        "Yandex fixture playlist".to_owned()
                    },
                    tracks: vec![
                        yandex_fixture_track(
                            "71663565",
                            "Animals",
                            "Architects",
                            Duration::from_millis(244_321),
                        ),
                        yandex_fixture_track(
                            "71663566",
                            "Second fixture",
                            "Fixture artist",
                            Duration::from_secs(20),
                        ),
                    ],
                    selected_track: None,
                    is_search_result,
                    kind: if is_search_result {
                        YandexMusicPlaylistKind::Search
                    } else {
                        YandexMusicPlaylistKind::Playlist
                    },
                    uri: (!is_search_result)
                        .then(|| "https://music.yandex.ru/users/fixture/playlists/1".to_owned()),
                    artwork_url: None,
                    author: (!is_search_result).then(|| "Fixture owner".to_owned()),
                })
            }
            _ => return Ok(None),
        };
        Ok(Some(SourceLoad::Item(BridgeItem::Yandex(item))))
    }

    fn encode(&self, item: &BridgeItem) -> Result<Vec<u8>, SourceRegistryError> {
        if matches!(item, BridgeItem::Yandex(YandexMusicSourceItem::Track(_))) {
            Ok(Vec::new())
        } else {
            Err(SourceRegistryError::NotEncodable)
        }
    }

    fn decode(&self, _payload: &[u8]) -> Result<BridgeItem, SourceRegistryError> {
        Err(SourceRegistryError::SourceFailure)
    }

    fn decode_with_info(
        &self,
        info: &TrackInfo,
        payload: &[u8],
    ) -> Result<BridgeItem, SourceRegistryError> {
        if !payload.is_empty()
            || info.identifier.is_empty()
            || info.identifier.len() > 32
            || !info.identifier.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(SourceRegistryError::SourceFailure);
        }
        Ok(BridgeItem::Yandex(YandexMusicSourceItem::Track(
            YandexMusicSourceTrack { info: info.clone() },
        )))
    }

    fn shutdown(&self) {}
}

fn yandex_fixture_track(
    identifier: &str,
    title: &str,
    author: &str,
    duration: Duration,
) -> YandexMusicSourceTrack {
    YandexMusicSourceTrack {
        info: TrackInfo {
            title: title.to_owned(),
            author: author.to_owned(),
            duration,
            identifier: identifier.to_owned(),
            is_stream: false,
            uri: Some(format!(
                "https://music.yandex.ru/album/1/track/{identifier}"
            )),
            artwork_url: (identifier == "71663565")
                .then(|| "https://avatars.yandex.net/get-music-content/fixture/400x400".to_owned()),
            isrc: None,
        },
    }
}

impl SourceManager<BridgeItem> for GateYoutubeSourceManager {
    fn source_name(&self) -> &'static str {
        "youtube"
    }

    fn load(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<SourceLoad<BridgeItem>>, SourceRegistryError> {
        let item = match reference.identifier() {
            Some("gate:youtube-track") => YoutubeSourceItem::Track(youtube_fixture_track(
                "dQw4w9WgXcQ",
                "YouTube fixture",
                "Fixture channel",
                Duration::from_secs(213),
            )),
            Some("gate:youtube-playlist" | "gate:youtube-search") => {
                let is_search_result = reference.identifier() == Some("gate:youtube-search");
                YoutubeSourceItem::Playlist(YoutubeSourcePlaylist {
                    name: if is_search_result {
                        "Search results for: fixture".to_owned()
                    } else {
                        "YouTube fixture playlist".to_owned()
                    },
                    tracks: vec![
                        youtube_fixture_track(
                            "aaaaabbbbbb",
                            "First fixture",
                            "First channel",
                            Duration::from_secs(10),
                        ),
                        youtube_fixture_track(
                            "ccccccddddd",
                            "Second fixture",
                            "Second channel",
                            Duration::from_secs(20),
                        ),
                    ],
                    selected_track: (!is_search_result).then_some(1),
                    is_search_result,
                })
            }
            _ => return Ok(None),
        };
        Ok(Some(SourceLoad::Item(BridgeItem::Youtube(item))))
    }

    fn encode(&self, item: &BridgeItem) -> Result<Vec<u8>, SourceRegistryError> {
        if matches!(item, BridgeItem::Youtube(YoutubeSourceItem::Track(_))) {
            Ok(Vec::new())
        } else {
            Err(SourceRegistryError::NotEncodable)
        }
    }

    fn decode(&self, _payload: &[u8]) -> Result<BridgeItem, SourceRegistryError> {
        Err(SourceRegistryError::SourceFailure)
    }

    fn decode_with_info(
        &self,
        info: &TrackInfo,
        payload: &[u8],
    ) -> Result<BridgeItem, SourceRegistryError> {
        if !payload.is_empty() || info.identifier.len() != 11 {
            return Err(SourceRegistryError::SourceFailure);
        }
        Ok(BridgeItem::Youtube(YoutubeSourceItem::Track(
            YoutubeSourceTrack { info: info.clone() },
        )))
    }

    fn shutdown(&self) {}
}

fn youtube_fixture_track(
    identifier: &str,
    title: &str,
    author: &str,
    duration: Duration,
) -> YoutubeSourceTrack {
    YoutubeSourceTrack {
        info: TrackInfo {
            title: title.to_owned(),
            author: author.to_owned(),
            duration,
            identifier: identifier.to_owned(),
            is_stream: false,
            uri: Some(format!("https://www.youtube.com/watch?v={identifier}")),
            artwork_url: (identifier == "dQw4w9WgXcQ")
                .then(|| "https://i.ytimg.com/fixture.jpg".to_owned()),
            isrc: None,
        },
    }
}

impl SourceManager<BridgeItem> for GateSourceManager {
    fn source_name(&self) -> &'static str {
        "mantle-oracle"
    }

    fn load(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<SourceLoad<BridgeItem>>, SourceRegistryError> {
        if reference.identifier() == Some("gate:track") {
            Ok(Some(SourceLoad::Item(BridgeItem::Synthetic(
                "gate:track".to_owned(),
            ))))
        } else {
            Ok(None)
        }
    }

    fn load_with_cancellation(
        &self,
        reference: &SourceReference,
        cancellation: &SourceCancellation,
    ) -> Result<Option<SourceLoad<BridgeItem>>, SourceRegistryError> {
        if reference.identifier() != Some("gate:pending") {
            return self.load(reference);
        }
        while !cancellation.is_cancelled() {
            thread::park_timeout(Duration::from_millis(1));
        }
        Ok(None)
    }

    fn encode(&self, item: &BridgeItem) -> Result<Vec<u8>, SourceRegistryError> {
        match item {
            BridgeItem::Synthetic(identifier) if identifier == "gate:track" => {
                Ok(b"\0\toracle-v1".to_vec())
            }
            BridgeItem::Synthetic(_)
            | BridgeItem::Yandex(_)
            | BridgeItem::Youtube(_)
            | BridgeItem::Track(_)
            | BridgeItem::Playlist(_) => Err(SourceRegistryError::NotEncodable),
        }
    }

    fn decode(&self, payload: &[u8]) -> Result<BridgeItem, SourceRegistryError> {
        if payload == b"\0\toracle-v1" {
            Ok(BridgeItem::Synthetic("gate:track".to_owned()))
        } else {
            Err(SourceRegistryError::SourceFailure)
        }
    }

    fn shutdown(&self) {}
}

struct JvmSourceManager {
    source_name: String,
    source: Global<JObject<'static>>,
    player_manager: Global<JObject<'static>>,
    reference_class: Global<JClass<'static>>,
    track_class: Global<JClass<'static>>,
    playlist_class: Global<JClass<'static>>,
    track_info_class: Global<JClass<'static>>,
    shutdown: AtomicBool,
}

impl JvmSourceManager {
    fn new(
        env: &mut Env<'_>,
        player_manager: &JObject<'_>,
        source: &JObject<'_>,
    ) -> jni::errors::Result<Self> {
        let source_name = env
            .call_method(
                source,
                jni_str!("getSourceName"),
                jni_sig!("()Ljava/lang/String;"),
                &[],
            )?
            .l()?;
        let source_name = JString::cast_local(env, source_name)?.try_to_string(env)?;
        if source_name.is_empty() {
            return Err(jni::errors::Error::NullPtr(
                "JVM source name must not be empty",
            ));
        }
        let reference_class = env.find_class(jni_str!(
            "com/sedmelluq/discord/lavaplayer/track/AudioReference"
        ))?;
        let track_class = env.find_class(jni_str!(
            "com/sedmelluq/discord/lavaplayer/track/AudioTrack"
        ))?;
        let playlist_class = env.find_class(jni_str!(
            "com/sedmelluq/discord/lavaplayer/track/AudioPlaylist"
        ))?;
        let track_info_class = env.find_class(jni_str!(
            "com/sedmelluq/discord/lavaplayer/track/AudioTrackInfo"
        ))?;
        Ok(Self {
            source_name,
            source: env.new_global_ref(source)?,
            player_manager: env.new_global_ref(player_manager)?,
            reference_class: env.new_global_ref(&reference_class)?,
            track_class: env.new_global_ref(&track_class)?,
            playlist_class: env.new_global_ref(&playlist_class)?,
            track_info_class: env.new_global_ref(&track_info_class)?,
            shutdown: AtomicBool::new(false),
        })
    }
}

impl SourceManager<BridgeItem> for JvmSourceManager {
    fn source_name(&self) -> &str {
        &self.source_name
    }

    fn load(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<SourceLoad<BridgeItem>>, SourceRegistryError> {
        self.load_with_cancellation(reference, &SourceCancellation::new())
    }

    fn load_with_cancellation(
        &self,
        reference: &SourceReference,
        cancellation: &SourceCancellation,
    ) -> Result<Option<SourceLoad<BridgeItem>>, SourceRegistryError> {
        if cancellation.is_cancelled() {
            return Ok(None);
        }
        let identifier = reference
            .identifier()
            .ok_or(SourceRegistryError::InvalidReference)?;
        let vm = JavaVM::singleton().map_err(|_| SourceRegistryError::SourceFailure)?;
        vm.attach_current_thread(|env| {
            let identifier = JObject::from(env.new_string(identifier)?);
            let java_reference = env.new_object(
                &self.reference_class,
                jni_sig!("(Ljava/lang/String;Ljava/lang/String;)V"),
                &[JValue::Object(&identifier), JValue::Object(&JObject::null())],
            )?;
            let result = env.call_method(
                &self.source,
                jni_str!("loadItem"),
                jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayerManager;Lcom/sedmelluq/discord/lavaplayer/track/AudioReference;)Lcom/sedmelluq/discord/lavaplayer/track/AudioItem;"),
                &[
                    JValue::Object(self.player_manager.as_ref()),
                    JValue::Object(&java_reference),
                ],
            );
            let item = clear_worker_exception(env, result)?.l()?;
            if item.is_null() || cancellation.is_cancelled() {
                return Ok(None);
            }
            if env.is_instance_of(&item, &self.reference_class)? {
                let identifier = env
                    .get_field(&item, jni_str!("identifier"), jni_sig!("Ljava/lang/String;"))?
                    .l()?;
                let identifier = if identifier.is_null() {
                    None
                } else {
                    Some(JString::cast_local(env, identifier)?.try_to_string(env)?)
                };
                let container = env
                    .get_field(
                        &item,
                        jni_str!("containerDescriptor"),
                        jni_sig!("Lcom/sedmelluq/discord/lavaplayer/container/MediaContainerDescriptor;"),
                    )?
                    .l()?;
                return Ok(Some(SourceLoad::Referral(SourceReference::new(
                    identifier,
                    !container.is_null(),
                ))));
            }
            if env.is_instance_of(&item, &self.track_class)? {
                return Ok(Some(SourceLoad::Item(BridgeItem::Track(
                    env.new_global_ref(&item)?,
                ))));
            }
            if env.is_instance_of(&item, &self.playlist_class)? {
                return Ok(Some(SourceLoad::Item(BridgeItem::Playlist(
                    env.new_global_ref(&item)?,
                ))));
            }
            Err(jni::errors::Error::NullPtr(
                "JVM source returned an unknown AudioItem implementation",
            ))
        })
        .map_err(|_| SourceRegistryError::SourceFailure)
    }

    fn is_encodable(&self, item: &BridgeItem) -> bool {
        let BridgeItem::Track(track) = item else {
            return false;
        };
        JavaVM::singleton().is_ok_and(|vm| {
            vm.attach_current_thread(|env| {
                let result = env.call_method(
                    &self.source,
                    jni_str!("isTrackEncodable"),
                    jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;)Z"),
                    &[JValue::Object(track.as_ref())],
                );
                clear_worker_exception(env, result)?.z()
            })
            .unwrap_or(false)
        })
    }

    fn encode(&self, item: &BridgeItem) -> Result<Vec<u8>, SourceRegistryError> {
        let BridgeItem::Track(track) = item else {
            return Err(SourceRegistryError::NotEncodable);
        };
        let vm = JavaVM::singleton().map_err(|_| SourceRegistryError::SourceFailure)?;
        vm.attach_current_thread(|env| {
            let bytes = env.new_object(
                jni_str!("java/io/ByteArrayOutputStream"),
                jni_sig!("()V"),
                &[],
            )?;
            let output = env.new_object(
                jni_str!("java/io/DataOutputStream"),
                jni_sig!("(Ljava/io/OutputStream;)V"),
                &[JValue::Object(&bytes)],
            )?;
            let result = env.call_method(
                &self.source,
                jni_str!("encodeTrack"),
                jni_sig!(
                    "(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;Ljava/io/DataOutput;)V"
                ),
                &[JValue::Object(track.as_ref()), JValue::Object(&output)],
            );
            let _ = clear_worker_exception(env, result)?;
            let encoded = env
                .call_method(&bytes, jni_str!("toByteArray"), jni_sig!("()[B"), &[])?
                .l()?;
            let encoded = JByteArray::cast_local(env, encoded)?;
            env.convert_byte_array(&encoded)
        })
        .map_err(|_| SourceRegistryError::SourceFailure)
    }

    fn decode(&self, _payload: &[u8]) -> Result<BridgeItem, SourceRegistryError> {
        Err(SourceRegistryError::SourceFailure)
    }

    fn decode_with_info(
        &self,
        info: &TrackInfo,
        payload: &[u8],
    ) -> Result<BridgeItem, SourceRegistryError> {
        let duration = i64::try_from(info.duration.as_millis())
            .map_err(|_| SourceRegistryError::SourceFailure)?;
        let vm = JavaVM::singleton().map_err(|_| SourceRegistryError::SourceFailure)?;
        vm.attach_current_thread(|env| {
            let title = JObject::from(env.new_string(&info.title)?);
            let author = JObject::from(env.new_string(&info.author)?);
            let identifier = JObject::from(env.new_string(&info.identifier)?);
            let uri = optional_java_string(env, info.uri.as_deref())?;
            let artwork = optional_java_string(env, info.artwork_url.as_deref())?;
            let isrc = optional_java_string(env, info.isrc.as_deref())?;
            let java_info = env.new_object(
                &self.track_info_class,
                jni_sig!("(Ljava/lang/String;Ljava/lang/String;JLjava/lang/String;ZLjava/lang/String;Ljava/lang/String;Ljava/lang/String;)V"),
                &[
                    JValue::Object(&title),
                    JValue::Object(&author),
                    JValue::Long(duration),
                    JValue::Object(&identifier),
                    JValue::Bool(info.is_stream),
                    JValue::Object(&uri),
                    JValue::Object(&artwork),
                    JValue::Object(&isrc),
                ],
            )?;
            let bytes = env.byte_array_from_slice(payload)?;
            let input_bytes: JObject<'_> = bytes.into();
            let byte_input = env.new_object(
                jni_str!("java/io/ByteArrayInputStream"),
                jni_sig!("([B)V"),
                &[JValue::Object(&input_bytes)],
            )?;
            let input = env.new_object(
                jni_str!("java/io/DataInputStream"),
                jni_sig!("(Ljava/io/InputStream;)V"),
                &[JValue::Object(&byte_input)],
            )?;
            let result = env.call_method(
                &self.source,
                jni_str!("decodeTrack"),
                jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackInfo;Ljava/io/DataInput;)Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;"),
                &[JValue::Object(&java_info), JValue::Object(&input)],
            );
            let track = clear_worker_exception(env, result)?.l()?;
            if track.is_null() {
                return Err(jni::errors::Error::NullPtr(
                    "JVM source returned null while decoding track details",
                ));
            }
            Ok(BridgeItem::Track(env.new_global_ref(&track)?))
        })
        .map_err(|_| SourceRegistryError::SourceFailure)
    }

    fn shutdown(&self) {
        if self.shutdown.swap(true, Ordering::AcqRel) {
            return;
        }
        if let Ok(vm) = JavaVM::singleton() {
            let result: jni::errors::Result<()> = vm.attach_current_thread(|env| {
                let result =
                    env.call_method(&self.source, jni_str!("shutdown"), jni_sig!("()V"), &[]);
                let _ = clear_worker_exception(env, result)?;
                Ok(())
            });
            if result.is_err() {
                crate::record_callback_exception();
            }
        }
    }
}

fn clear_worker_exception<T>(
    env: &mut Env<'_>,
    result: jni::errors::Result<T>,
) -> jni::errors::Result<T> {
    if result.is_err() && env.exception_check() {
        env.exception_clear();
    }
    result
}

fn optional_java_string<'local>(
    env: &mut Env<'local>,
    value: Option<&str>,
) -> jni::errors::Result<JObject<'local>> {
    value.map_or_else(
        || Ok(JObject::null()),
        |value| env.new_string(value).map(JObject::from),
    )
}

struct ResultSlot {
    state: Mutex<ResultState>,
    delivered: Condvar,
}

struct ResultState {
    result: Option<SourceLoadResult<BridgeItem>>,
    delivered: bool,
}

impl ResultSlot {
    fn new() -> Self {
        Self {
            state: Mutex::new(ResultState {
                result: None,
                delivered: false,
            }),
            delivered: Condvar::new(),
        }
    }

    fn store(&self, result: SourceLoadResult<BridgeItem>) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.result = Some(result);
        state.delivered = false;
    }

    fn take(&self) -> Option<SourceLoadResult<BridgeItem>> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .result
            .take()
    }

    fn wait_until_delivered(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while !state.delivered {
            state = self
                .delivered
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }

    fn mark_delivered(&self) {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .delivered = true;
        self.delivered.notify_all();
    }
}

struct ResultHandler {
    load_id: u64,
    slot: Arc<ResultSlot>,
}

impl SourceLoadResultHandler<BridgeItem> for ResultHandler {
    fn finished(self: Box<Self>, result: SourceLoadResult<BridgeItem>) {
        self.slot.store(result);
        if schedule_callback(self.load_id) {
            self.slot.wait_until_delivered();
        } else {
            let _ = self.slot.take();
            self.slot.mark_delivered();
        }
    }
}

struct CompletionHook {
    load_id: u64,
    ordering_hook: Option<Box<dyn LoadTerminalHook>>,
}

impl LoadTerminalHook for CompletionHook {
    fn on_terminal(mut self: Box<Self>) {
        if let Some(hook) = self.ordering_hook.take() {
            hook.on_terminal();
        }
        let _ = schedule_callback(self.load_id);
    }
}

struct LoadEntry {
    handle: Option<Arc<SourceLoadHandle>>,
    result: Arc<ResultSlot>,
    handler: Option<Global<JObject<'static>>>,
    future: Option<Global<JObject<'static>>>,
    callback: Global<JObject<'static>>,
}

struct TrackedSourceItem {
    reference: Weak<JObject<'static>>,
    registration: SourceRegistrationId,
    item: TrackedItem,
}

enum TrackedItem {
    Synthetic(String),
    Yandex(YandexMusicSourceItem),
    Youtube(YoutubeSourceItem),
    JvmTrack,
}

impl TrackedItem {
    fn from_bridge(item: BridgeItem) -> jni::errors::Result<Self> {
        match item {
            BridgeItem::Synthetic(identifier) => Ok(Self::Synthetic(identifier)),
            BridgeItem::Yandex(YandexMusicSourceItem::Track(track)) => {
                Ok(Self::Yandex(YandexMusicSourceItem::Track(track)))
            }
            BridgeItem::Youtube(YoutubeSourceItem::Track(track)) => {
                Ok(Self::Youtube(YoutubeSourceItem::Track(track)))
            }
            BridgeItem::Track(_) => Ok(Self::JvmTrack),
            BridgeItem::Yandex(YandexMusicSourceItem::Playlist(_))
            | BridgeItem::Youtube(YoutubeSourceItem::Playlist(_))
            | BridgeItem::Playlist(_) => Err(jni::errors::Error::NullPtr(
                "only source tracks can be retained for encoding",
            )),
        }
    }

    fn to_bridge(
        &self,
        env: &mut Env<'_>,
        reference: &JObject<'_>,
    ) -> jni::errors::Result<BridgeItem> {
        match self {
            Self::Synthetic(identifier) => Ok(BridgeItem::Synthetic(identifier.clone())),
            Self::Yandex(item) => Ok(BridgeItem::Yandex(item.clone())),
            Self::Youtube(item) => Ok(BridgeItem::Youtube(item.clone())),
            Self::JvmTrack => Ok(BridgeItem::Track(env.new_global_ref(reference)?)),
        }
    }
}

struct LoadRuntime {
    next_id: u64,
    registry: Option<SourceRegistry<BridgeItem>>,
    executor: Option<Arc<SourceLoadExecutor<BridgeItem, OpaqueLoadKey>>>,
    callback_executor: Global<JObject<'static>>,
    ordering_keys: JvmOrderingKeyTable,
    ordering_releases: JvmOrderingKeyReleaseQueue,
    entries: HashMap<u64, LoadEntry>,
    tracked_items: Vec<TrackedSourceItem>,
}

impl LoadRuntime {
    fn new(env: &mut Env<'_>) -> jni::errors::Result<Self> {
        let mut registry = SourceRegistry::new(SourceRegistryLimits::default());
        registry
            .register(Box::new(GateSourceManager))
            .map_err(|_| jni::errors::Error::NullPtr("could not register JVM gate source"))?;
        registry
            .register(Box::new(GateYoutubeSourceManager))
            .map_err(|_| jni::errors::Error::NullPtr("could not register YouTube gate source"))?;
        registry
            .register(Box::new(GateYandexSourceManager))
            .map_err(|_| jni::errors::Error::NullPtr("could not register Yandex gate source"))?;
        let callback_executor = env
            .call_static_method(
                jni_str!("java/util/concurrent/ForkJoinPool"),
                jni_str!("commonPool"),
                jni_sig!("()Ljava/util/concurrent/ForkJoinPool;"),
                &[],
            )?
            .l()?;
        Ok(Self {
            next_id: 1,
            registry: Some(registry),
            executor: None,
            callback_executor: env.new_global_ref(&callback_executor)?,
            ordering_keys: JvmOrderingKeyTable::new(MAXIMUM_LOADS),
            ordering_releases: JvmOrderingKeyReleaseQueue::new(MAXIMUM_LOADS),
            entries: HashMap::with_capacity(MAXIMUM_LOADS),
            tracked_items: Vec::with_capacity(MAXIMUM_TRACKED_SOURCE_ITEMS),
        })
    }

    fn allocate_id(&mut self) -> u64 {
        loop {
            let id = self.next_id.max(1);
            self.next_id = id.checked_add(1).unwrap_or(1);
            if !self.entries.contains_key(&id) {
                return id;
            }
        }
    }

    fn executor(
        &mut self,
    ) -> jni::errors::Result<Arc<SourceLoadExecutor<BridgeItem, OpaqueLoadKey>>> {
        if self.executor.is_none() {
            let registry = self
                .registry
                .take()
                .ok_or(jni::errors::Error::NullPtr("source registration is closed"))?;
            let executor = SourceLoadExecutor::new(
                registry,
                LoadExecutorLimits {
                    workers: 2,
                    maximum_pending: MAXIMUM_LOADS,
                    maximum_channels: MAXIMUM_LOADS,
                    maximum_per_channel: MAXIMUM_LOADS,
                },
            )
            .map_err(|_| jni::errors::Error::NullPtr("could not start source-load executor"))?;
            self.executor = Some(Arc::new(executor));
        }
        self.executor
            .as_ref()
            .map(Arc::clone)
            .ok_or(jni::errors::Error::NullPtr(
                "source-load executor is shut down",
            ))
    }
}

fn runtime() -> &'static Mutex<Option<LoadRuntime>> {
    static RUNTIME: OnceLock<Mutex<Option<LoadRuntime>>> = OnceLock::new();
    RUNTIME.get_or_init(|| Mutex::new(None))
}

fn shutdown_in_progress() -> &'static AtomicBool {
    static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);
    &SHUTTING_DOWN
}

pub(crate) fn register(
    env: &mut Env<'_>,
    player_manager: &JObject<'_>,
    source: &JObject<'_>,
) -> jni::errors::Result<()> {
    if shutdown_in_progress().load(Ordering::Acquire) {
        return Err(jni::errors::Error::NullPtr(
            "source-load runtime is shutting down",
        ));
    }
    let manager = JvmSourceManager::new(env, player_manager, source)?;
    let mut state = runtime()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if state.is_none() {
        *state = Some(LoadRuntime::new(env)?);
    }
    let runtime = state.as_mut().ok_or(jni::errors::Error::NullPtr(
        "source-load runtime is unavailable",
    ))?;
    let registration = if let Some(registry) = runtime.registry.as_mut() {
        registry.register(Box::new(manager))
    } else if let Some(executor) = runtime.executor.as_ref() {
        executor.register_source(Box::new(manager))
    } else {
        return Err(jni::errors::Error::NullPtr(
            "source-load executor is shut down",
        ));
    };
    registration
        .map(|_| ())
        .map_err(|_| jni::errors::Error::NullPtr("could not register JVM source manager"))
}

pub(crate) fn submit<'local>(
    env: &mut Env<'local>,
    identifier: &JString<'local>,
    result_handler: &JObject<'local>,
    ordering_key: Option<&JObject<'local>>,
) -> jni::errors::Result<JObject<'local>> {
    let identifier = identifier.try_to_string(env)?;
    submit_reference(
        env,
        SourceReference::new(Some(identifier), false),
        result_handler,
        ordering_key,
    )
}

pub(crate) fn submit_java_reference<'local>(
    env: &mut Env<'local>,
    reference: &JObject<'local>,
    result_handler: &JObject<'local>,
    ordering_key: Option<&JObject<'local>>,
) -> jni::errors::Result<JObject<'local>> {
    let reference = source_reference_from_java(env, reference)?;
    submit_reference(env, reference, result_handler, ordering_key)
}

fn submit_reference<'local>(
    env: &mut Env<'local>,
    reference: SourceReference,
    result_handler: &JObject<'local>,
    ordering_key: Option<&JObject<'local>>,
) -> jni::errors::Result<JObject<'local>> {
    if shutdown_in_progress().load(Ordering::Acquire) {
        return Err(jni::errors::Error::NullPtr(
            "source-load runtime is shutting down",
        ));
    }
    let mut state = runtime()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if state.is_none() {
        *state = Some(LoadRuntime::new(env)?);
    }
    let runtime = state.as_mut().ok_or(jni::errors::Error::NullPtr(
        "source-load runtime is unavailable",
    ))?;
    if runtime.entries.len() >= MAXIMUM_LOADS {
        return Err(jni::errors::Error::NullPtr(
            "JVM source-load capacity reached",
        ));
    }

    let load_id = runtime.allocate_id();
    let java_load_id = i64::try_from(load_id)
        .map_err(|_| jni::errors::Error::NullPtr("source-load handle overflow"))?;
    let future = env.new_object(
        FUTURE_CLASS,
        jni_sig!("(J)V"),
        &[JValue::Long(java_load_id)],
    )?;
    let callback = env.new_object(
        CALLBACK_CLASS,
        jni_sig!("(J)V"),
        &[JValue::Long(java_load_id)],
    )?;
    let result = Arc::new(ResultSlot::new());
    let ordering = ordering_key
        .map(|key| {
            runtime
                .ordering_keys
                .acquire_for_load(env, key, &runtime.ordering_releases)
        })
        .transpose()?;
    let executor = runtime.executor()?;
    runtime.entries.insert(
        load_id,
        LoadEntry {
            handle: None,
            result: Arc::clone(&result),
            handler: Some(env.new_global_ref(result_handler)?),
            future: Some(env.new_global_ref(&future)?),
            callback: env.new_global_ref(&callback)?,
        },
    );
    let handler: Box<dyn SourceLoadResultHandler<BridgeItem>> = Box::new(ResultHandler {
        load_id,
        slot: result,
    });
    let handle = if let Some((key, hook)) = ordering {
        executor.submit_ordered_with_hook(
            key,
            reference,
            handler,
            Box::new(CompletionHook {
                load_id,
                ordering_hook: Some(hook),
            }),
        )
    } else {
        executor.submit_ordered_with_hook(
            OpaqueLoadKey::from_opaque(load_id).expect("positive load id"),
            reference,
            handler,
            Box::new(CompletionHook {
                load_id,
                ordering_hook: None,
            }),
        )
    };
    if let Some(entry) = runtime.entries.get_mut(&load_id) {
        entry.handle = Some(Arc::new(handle));
    }
    Ok(future)
}

pub(crate) fn load_sync<'local>(
    env: &mut Env<'local>,
    reference: &JObject<'local>,
) -> jni::errors::Result<JObject<'local>> {
    let reference = source_reference_from_java(env, reference)?;
    let executor = current_executor(env)?;
    let item = executor
        .load_sync(&reference)
        .map_err(|_| jni::errors::Error::NullPtr("synchronous source load failed"))?;
    item.map_or_else(|| Ok(JObject::null()), |item| materialize_item(env, item))
}

pub(crate) fn load_sync_handled(
    env: &mut Env<'_>,
    reference: &JObject<'_>,
    result_handler: &JObject<'_>,
) -> jni::errors::Result<()> {
    let reference = source_reference_from_java(env, reference)?;
    let executor = current_executor(env)?;
    let result = match executor.load_sync(&reference) {
        Ok(Some(item)) => SourceLoadResult::Item(item),
        Ok(None) => SourceLoadResult::NoMatches,
        Err(_) => SourceLoadResult::Failed(mantle_core::SourceLoadFailure::Source(
            SourceRegistryError::SourceFailure,
        )),
    };
    let handler = env.new_global_ref(result_handler)?;
    invoke_load_callback(env, &handler, result)
}

pub(crate) fn load_nico_item<'local>(
    env: &mut Env<'local>,
    source: &JObject<'local>,
    reference: &JObject<'local>,
) -> jni::errors::Result<JObject<'local>> {
    let reference = source_reference_from_java(env, reference)?;
    let manager = NicoNicoSourceManager::new(NicoNicoSourceOptions::default())
        .map_err(|_| jni::errors::Error::NullPtr("could not create current NicoNico source"))?;
    let item = manager
        .load(&reference)
        .map_err(|_| jni::errors::Error::NullPtr("current NicoNico metadata load failed"))?;
    match item {
        Some(SourceLoad::Item(track)) => create_nico_track(env, &track, source),
        Some(SourceLoad::Referral(_)) | None => Ok(JObject::null()),
    }
}

pub(crate) fn load_twitch_item<'local>(
    env: &mut Env<'local>,
    source: &JObject<'local>,
    reference: &JObject<'local>,
) -> jni::errors::Result<JObject<'local>> {
    let reference = source_reference_from_java(env, reference)?;
    let options = TwitchSourceOptions::default();
    let Some(identifier) = reference.identifier() else {
        return Ok(JObject::null());
    };
    if route_twitch_identifier(identifier, &options).is_none() {
        return Ok(JObject::null());
    }

    let client_id =
        system_property(env, "dev.mantle.twitch.clientId")?.ok_or(jni::errors::Error::NullPtr(
            "Twitch metadata requires the dev.mantle.twitch.clientId system property",
        ))?;
    let access_token = system_property(env, "dev.mantle.twitch.accessToken")?.ok_or(
        jni::errors::Error::NullPtr(
            "Twitch metadata requires the dev.mantle.twitch.accessToken system property",
        ),
    )?;
    let device_id = system_property(env, "dev.mantle.twitch.deviceId")?;
    let authentication =
        TwitchAuthentication::with_device_id(client_id, access_token, device_id)
            .map_err(|_| jni::errors::Error::NullPtr("invalid Twitch JVM credentials"))?;
    let manager = TwitchSourceManager::new(options, authentication)
        .map_err(|_| jni::errors::Error::NullPtr("could not create current Twitch source"))?;
    let item = manager
        .load(&reference)
        .map_err(|_| jni::errors::Error::NullPtr("current Twitch metadata load failed"))?;
    match item {
        Some(SourceLoad::Item(track)) => create_twitch_track(env, &track, source),
        Some(SourceLoad::Referral(_)) | None => Ok(JObject::null()),
    }
}

fn system_property(env: &mut Env<'_>, key: &str) -> jni::errors::Result<Option<String>> {
    let key = JObject::from(env.new_string(key)?);
    let value = env
        .call_static_method(
            jni_str!("java/lang/System"),
            jni_str!("getProperty"),
            jni_sig!("(Ljava/lang/String;)Ljava/lang/String;"),
            &[JValue::Object(&key)],
        )?
        .l()?;
    if value.is_null() {
        Ok(None)
    } else {
        Ok(Some(JString::cast_local(env, value)?.try_to_string(env)?))
    }
}

fn current_executor(
    env: &mut Env<'_>,
) -> jni::errors::Result<Arc<SourceLoadExecutor<BridgeItem, OpaqueLoadKey>>> {
    if shutdown_in_progress().load(Ordering::Acquire) {
        return Err(jni::errors::Error::NullPtr(
            "source-load runtime is shutting down",
        ));
    }
    let mut state = runtime()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if state.is_none() {
        *state = Some(LoadRuntime::new(env)?);
    }
    state
        .as_mut()
        .ok_or(jni::errors::Error::NullPtr(
            "source-load runtime is unavailable",
        ))?
        .executor()
}

fn source_reference_from_java(
    env: &mut Env<'_>,
    reference: &JObject<'_>,
) -> jni::errors::Result<SourceReference> {
    let identifier = env
        .get_field(
            reference,
            jni_str!("identifier"),
            jni_sig!("Ljava/lang/String;"),
        )?
        .l()?;
    let identifier = if identifier.is_null() {
        None
    } else {
        Some(JString::cast_local(env, identifier)?.try_to_string(env)?)
    };
    let container = env
        .get_field(
            reference,
            jni_str!("containerDescriptor"),
            jni_sig!("Lcom/sedmelluq/discord/lavaplayer/container/MediaContainerDescriptor;"),
        )?
        .l()?;
    Ok(SourceReference::new(identifier, !container.is_null()))
}

pub(crate) fn source_manager<'local>(
    env: &mut Env<'local>,
    manager: &JObject<'local>,
    requested_class: &JClass<'local>,
) -> jni::errors::Result<JObject<'local>> {
    let sources = env
        .get_field(
            manager,
            jni_str!("mantleSources"),
            jni_sig!("Ljava/util/ArrayList;"),
        )?
        .l()?;
    let size = env
        .call_method(&sources, jni_str!("size"), jni_sig!("()I"), &[])?
        .i()?;
    for index in 0..size {
        let source = env
            .call_method(
                &sources,
                jni_str!("get"),
                jni_sig!("(I)Ljava/lang/Object;"),
                &[JValue::Int(index)],
            )?
            .l()?;
        if env.is_instance_of(&source, requested_class)? {
            return Ok(source);
        }
    }
    Ok(JObject::null())
}

fn schedule_callback(load_id: u64) -> bool {
    let state = runtime()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(runtime) = state.as_ref() else {
        return false;
    };
    let Some(entry) = runtime.entries.get(&load_id) else {
        return false;
    };
    let Ok(vm) = JavaVM::singleton() else {
        return false;
    };
    let result: jni::errors::Result<()> = vm.attach_current_thread(|env| {
        let _ = env.call_method(
            &runtime.callback_executor,
            jni_str!("execute"),
            jni_sig!("(Ljava/lang/Runnable;)V"),
            &[JValue::Object(entry.callback.as_ref())],
        )?;
        Ok(())
    });
    if result.is_err() {
        crate::record_callback_exception();
    }
    result.is_ok()
}

pub(crate) fn cancel(load_id: i64) -> bool {
    let Ok(load_id) = u64::try_from(load_id) else {
        return false;
    };
    let handle = runtime()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
        .and_then(|runtime| runtime.entries.get(&load_id))
        .and_then(|entry| entry.handle.clone());
    handle.is_some_and(|handle| handle.cancel())
}

pub(crate) fn ordering_key_count() -> usize {
    runtime()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
        .map_or(0, |runtime| runtime.ordering_keys.len())
}

fn remember_source_item(
    env: &mut Env<'_>,
    reference: &JObject<'_>,
    registration: SourceRegistrationId,
    item: BridgeItem,
) -> jni::errors::Result<()> {
    let item = TrackedItem::from_bridge(item)?;
    let mut state = runtime()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let runtime = state.as_mut().ok_or(jni::errors::Error::NullPtr(
        "source-load runtime is unavailable",
    ))?;
    prune_tracked_source_items(env, &mut runtime.tracked_items)?;
    for tracked in &mut runtime.tracked_items {
        if env.is_same_object(reference, &tracked.reference)? {
            tracked.registration = registration;
            tracked.item = item;
            return Ok(());
        }
    }
    if runtime.tracked_items.len() >= MAXIMUM_TRACKED_SOURCE_ITEMS {
        return Err(jni::errors::Error::NullPtr(
            "tracked source-item capacity reached",
        ));
    }
    runtime.tracked_items.push(TrackedSourceItem {
        reference: env.new_weak_ref(reference)?,
        registration,
        item,
    });
    Ok(())
}

fn prune_tracked_source_items(
    env: &mut Env<'_>,
    items: &mut Vec<TrackedSourceItem>,
) -> jni::errors::Result<()> {
    let mut index = 0;
    while index < items.len() {
        if items[index].reference.is_garbage_collected(env)? {
            items.swap_remove(index);
        } else {
            index += 1;
        }
    }
    Ok(())
}

pub(crate) fn encode_track_details(
    env: &mut Env<'_>,
    track: &JObject<'_>,
) -> jni::errors::Result<Option<Vec<u8>>> {
    let tracked = {
        let state = runtime()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(runtime) = state.as_ref() else {
            return Ok(None);
        };
        let Some(executor) = runtime.executor.as_ref() else {
            return Ok(None);
        };
        let mut found = None;
        for item in &runtime.tracked_items {
            if env.is_same_object(track, &item.reference)? {
                found = Some((
                    Arc::clone(executor),
                    LoadedSourceItem {
                        registration: item.registration,
                        item: item.item.to_bridge(env, track)?,
                    },
                ));
                break;
            }
        }
        found
    };
    let Some((executor, item)) = tracked else {
        return Ok(None);
    };
    let details = executor
        .encode_details(&item)
        .map_err(|_| jni::errors::Error::NullPtr("source manager could not encode track"))?;
    encode_source_details(&details, SerializationLimits::default())
        .map(Some)
        .map_err(|_| jni::errors::Error::NullPtr("source track details exceed their byte limit"))
}

pub(crate) fn decode_track_details<'local>(
    env: &mut Env<'local>,
    info: &TrackInfo,
    bytes: &[u8],
) -> jni::errors::Result<Option<JObject<'local>>> {
    let details = decode_source_details(bytes, SerializationLimits::default())
        .map_err(|_| jni::errors::Error::NullPtr("could not decode source track details"))?;
    let executor = {
        let state = runtime()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(runtime) = state.as_ref() else {
            return Ok(None);
        };
        let Some(executor) = runtime.executor.as_ref() else {
            return Ok(None);
        };
        Arc::clone(executor)
    };
    let Some(item) = executor
        .decode_details_with_info(info, &details)
        .map_err(|_| jni::errors::Error::NullPtr("source manager could not decode track"))?
    else {
        return Ok(None);
    };
    let registration = item.registration;
    let (track, tracked_item) = match item.item {
        BridgeItem::Synthetic(identifier) => {
            let identifier_object = JObject::from(env.new_string(&identifier)?);
            let track = proxy::create(env, 3, &identifier_object)?;
            (track, BridgeItem::Synthetic(identifier))
        }
        BridgeItem::Track(track) => {
            let local = env.new_local_ref(track.as_ref())?;
            (local, BridgeItem::Track(track))
        }
        BridgeItem::Youtube(YoutubeSourceItem::Track(track)) => {
            let local = create_native_track(env, &track.info)?;
            (local, BridgeItem::Youtube(YoutubeSourceItem::Track(track)))
        }
        BridgeItem::Yandex(YandexMusicSourceItem::Track(track)) => {
            let local = create_native_track(env, &track.info)?;
            (
                local,
                BridgeItem::Yandex(YandexMusicSourceItem::Track(track)),
            )
        }
        BridgeItem::Yandex(YandexMusicSourceItem::Playlist(_))
        | BridgeItem::Youtube(YoutubeSourceItem::Playlist(_))
        | BridgeItem::Playlist(_) => {
            return Err(jni::errors::Error::NullPtr(
                "source manager decoded a playlist as track details",
            ));
        }
    };
    remember_source_item(env, &track, registration, tracked_item)?;
    Ok(Some(track))
}

pub(crate) fn dispatch(env: &mut Env<'_>, load_id: i64) -> jni::errors::Result<()> {
    let load_id = u64::try_from(load_id)
        .map_err(|_| jni::errors::Error::NullPtr("invalid source-load handle"))?;
    let payload = {
        let mut state = runtime()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(runtime) = state.as_mut() else {
            return Ok(());
        };
        let _ = runtime
            .ordering_keys
            .drain_releases(env, &runtime.ordering_releases);
        let Some(entry) = runtime.entries.get_mut(&load_id) else {
            return Ok(());
        };
        if let Some(result) = entry.result.take() {
            let result_slot = Arc::clone(&entry.result);
            if let (Some(handler), Some(future)) = (entry.handler.take(), entry.future.take()) {
                Some((result, result_slot, handler, future))
            } else {
                result_slot.mark_delivered();
                runtime.entries.remove(&load_id);
                return Err(jni::errors::Error::NullPtr(
                    "source-load callback state is incomplete",
                ));
            }
        } else {
            runtime.entries.remove(&load_id);
            None
        }
    };
    let Some((result, result_slot, handler, future)) = payload else {
        return Ok(());
    };

    let callback_result = invoke_load_callback(env, &handler, result);
    if callback_result.is_err() {
        crate::record_callback_exception();
        if env.exception_check() {
            env.exception_clear();
        }
    }
    let future_result = env.call_method(
        &future,
        jni_str!("complete"),
        jni_sig!("(Ljava/lang/Object;)Z"),
        &[JValue::Object(&JObject::null())],
    );
    result_slot.mark_delivered();
    future_result.map(|_| ())
}

fn invoke_load_callback(
    env: &mut Env<'_>,
    handler: &Global<JObject<'static>>,
    result: SourceLoadResult<BridgeItem>,
) -> jni::errors::Result<()> {
    let _ = match result {
        SourceLoadResult::Item(item) => match item.item {
            BridgeItem::Synthetic(identifier) => {
                let identifier_object = JObject::from(env.new_string(&identifier)?);
                let track = proxy::create(env, 3, &identifier_object)?;
                remember_source_item(
                    env,
                    &track,
                    item.registration,
                    BridgeItem::Synthetic(identifier),
                )?;
                env.call_method(
                    handler,
                    jni_str!("trackLoaded"),
                    jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;)V"),
                    &[JValue::Object(&track)],
                )
            }
            BridgeItem::Track(track) => {
                let retained = env.new_global_ref(&track)?;
                remember_source_item(
                    env,
                    track.as_ref(),
                    item.registration,
                    BridgeItem::Track(retained),
                )?;
                env.call_method(
                    handler,
                    jni_str!("trackLoaded"),
                    jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;)V"),
                    &[JValue::Object(track.as_ref())],
                )
            }
            BridgeItem::Youtube(YoutubeSourceItem::Track(track)) => {
                let java_track = create_native_track(env, &track.info)?;
                remember_source_item(
                    env,
                    &java_track,
                    item.registration,
                    BridgeItem::Youtube(YoutubeSourceItem::Track(track)),
                )?;
                env.call_method(
                    handler,
                    jni_str!("trackLoaded"),
                    jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;)V"),
                    &[JValue::Object(&java_track)],
                )
            }
            BridgeItem::Yandex(YandexMusicSourceItem::Track(track)) => {
                let java_track = create_native_track(env, &track.info)?;
                remember_source_item(
                    env,
                    &java_track,
                    item.registration,
                    BridgeItem::Yandex(YandexMusicSourceItem::Track(track)),
                )?;
                env.call_method(
                    handler,
                    jni_str!("trackLoaded"),
                    jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;)V"),
                    &[JValue::Object(&java_track)],
                )
            }
            BridgeItem::Youtube(YoutubeSourceItem::Playlist(playlist)) => {
                let java_playlist = create_youtube_playlist(env, item.registration, playlist)?;
                env.call_method(
                    handler,
                    jni_str!("playlistLoaded"),
                    jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/track/AudioPlaylist;)V"),
                    &[JValue::Object(&java_playlist)],
                )
            }
            BridgeItem::Yandex(YandexMusicSourceItem::Playlist(playlist)) => {
                let java_playlist = create_yandex_playlist(env, item.registration, playlist)?;
                env.call_method(
                    handler,
                    jni_str!("playlistLoaded"),
                    jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/track/AudioPlaylist;)V"),
                    &[JValue::Object(&java_playlist)],
                )
            }
            BridgeItem::Playlist(playlist) => env.call_method(
                handler,
                jni_str!("playlistLoaded"),
                jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/track/AudioPlaylist;)V"),
                &[JValue::Object(playlist.as_ref())],
            ),
        },
        SourceLoadResult::NoMatches => {
            env.call_method(handler, jni_str!("noMatches"), jni_sig!("()V"), &[])
        }
        SourceLoadResult::Failed(_) => env.call_method(
            handler,
            jni_str!("loadFailed"),
            jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/tools/FriendlyException;)V"),
            &[JValue::Object(&JObject::null())],
        ),
    }?;
    Ok(())
}

fn materialize_item<'local>(
    env: &mut Env<'local>,
    item: LoadedSourceItem<BridgeItem>,
) -> jni::errors::Result<JObject<'local>> {
    match item.item {
        BridgeItem::Synthetic(identifier) => {
            let identifier_object = JObject::from(env.new_string(&identifier)?);
            let track = proxy::create(env, 3, &identifier_object)?;
            remember_source_item(
                env,
                &track,
                item.registration,
                BridgeItem::Synthetic(identifier),
            )?;
            Ok(track)
        }
        BridgeItem::Track(track) => {
            let local = env.new_local_ref(track.as_ref())?;
            let retained = env.new_global_ref(&track)?;
            remember_source_item(env, &local, item.registration, BridgeItem::Track(retained))?;
            Ok(local)
        }
        BridgeItem::Youtube(YoutubeSourceItem::Track(track)) => {
            let local = create_native_track(env, &track.info)?;
            remember_source_item(
                env,
                &local,
                item.registration,
                BridgeItem::Youtube(YoutubeSourceItem::Track(track)),
            )?;
            Ok(local)
        }
        BridgeItem::Yandex(YandexMusicSourceItem::Track(track)) => {
            let local = create_native_track(env, &track.info)?;
            remember_source_item(
                env,
                &local,
                item.registration,
                BridgeItem::Yandex(YandexMusicSourceItem::Track(track)),
            )?;
            Ok(local)
        }
        BridgeItem::Youtube(YoutubeSourceItem::Playlist(playlist)) => {
            create_youtube_playlist(env, item.registration, playlist)
        }
        BridgeItem::Yandex(YandexMusicSourceItem::Playlist(playlist)) => {
            create_yandex_playlist(env, item.registration, playlist)
        }
        BridgeItem::Playlist(playlist) => env.new_local_ref(playlist.as_ref()),
    }
}

fn create_native_track<'local>(
    env: &mut Env<'local>,
    info: &TrackInfo,
) -> jni::errors::Result<JObject<'local>> {
    let identifier = env.new_string(&info.identifier)?;
    let track = proxy::create(env, 3, identifier.as_ref())?;
    let track_id = proxy::track_id_from_proxy(env, &track)?;
    crate::with_engine(|engine| engine.replace_track_info(track_id, info.clone()))
        .map_err(|_| jni::errors::Error::NullPtr("could not apply native track metadata"))?;
    Ok(track)
}

fn create_nico_track<'local>(
    env: &mut Env<'local>,
    track: &NicoNicoSourceTrack,
    source: &JObject<'local>,
) -> jni::errors::Result<JObject<'local>> {
    let info = &track.info;
    let duration = i64::try_from(info.duration.as_millis())
        .map_err(|_| jni::errors::Error::NullPtr("NicoNico duration exceeds JVM range"))?;
    let title = JObject::from(env.new_string(&info.title)?);
    let author = JObject::from(env.new_string(&info.author)?);
    let identifier = JObject::from(env.new_string(&info.identifier)?);
    let uri = optional_java_string(env, info.uri.as_deref())?;
    let artwork = optional_java_string(env, info.artwork_url.as_deref())?;
    let isrc = optional_java_string(env, info.isrc.as_deref())?;
    let java_info = env.new_object(
        jni_str!("com/sedmelluq/discord/lavaplayer/track/AudioTrackInfo"),
        jni_sig!("(Ljava/lang/String;Ljava/lang/String;JLjava/lang/String;ZLjava/lang/String;Ljava/lang/String;Ljava/lang/String;)V"),
        &[
            JValue::Object(&title),
            JValue::Object(&author),
            JValue::Long(duration),
            JValue::Object(&identifier),
            JValue::Bool(info.is_stream),
            JValue::Object(&uri),
            JValue::Object(&artwork),
            JValue::Object(&isrc),
        ],
    )?;
    env.new_object(
        jni_str!("com/sedmelluq/discord/lavaplayer/source/nico/NicoAudioTrack"),
        jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackInfo;Lcom/sedmelluq/discord/lavaplayer/source/nico/NicoAudioSourceManager;)V"),
        &[JValue::Object(&java_info), JValue::Object(source)],
    )
}

fn create_twitch_track<'local>(
    env: &mut Env<'local>,
    track: &TwitchSourceTrack,
    source: &JObject<'local>,
) -> jni::errors::Result<JObject<'local>> {
    let info = &track.info;
    let duration = i64::try_from(info.duration.as_millis())
        .map_err(|_| jni::errors::Error::NullPtr("Twitch duration exceeds JVM range"))?;
    let title = JObject::from(env.new_string(&info.title)?);
    let author = JObject::from(env.new_string(&info.author)?);
    let identifier = JObject::from(env.new_string(&info.identifier)?);
    let uri = optional_java_string(env, info.uri.as_deref())?;
    let artwork = optional_java_string(env, info.artwork_url.as_deref())?;
    let isrc = optional_java_string(env, info.isrc.as_deref())?;
    let java_info = env.new_object(
        jni_str!("com/sedmelluq/discord/lavaplayer/track/AudioTrackInfo"),
        jni_sig!("(Ljava/lang/String;Ljava/lang/String;JLjava/lang/String;ZLjava/lang/String;Ljava/lang/String;Ljava/lang/String;)V"),
        &[
            JValue::Object(&title),
            JValue::Object(&author),
            JValue::Long(duration),
            JValue::Object(&identifier),
            JValue::Bool(info.is_stream),
            JValue::Object(&uri),
            JValue::Object(&artwork),
            JValue::Object(&isrc),
        ],
    )?;
    env.new_object(
        jni_str!("com/sedmelluq/discord/lavaplayer/source/twitch/TwitchStreamAudioTrack"),
        jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackInfo;Lcom/sedmelluq/discord/lavaplayer/source/twitch/TwitchStreamAudioSourceManager;)V"),
        &[JValue::Object(&java_info), JValue::Object(source)],
    )
}

fn create_youtube_playlist<'local>(
    env: &mut Env<'local>,
    registration: SourceRegistrationId,
    playlist: YoutubeSourcePlaylist,
) -> jni::errors::Result<JObject<'local>> {
    let tracks = playlist
        .tracks
        .into_iter()
        .map(|track| {
            (
                track.info.clone(),
                BridgeItem::Youtube(YoutubeSourceItem::Track(track)),
            )
        })
        .collect();
    create_native_playlist(
        env,
        registration,
        playlist.name,
        tracks,
        playlist.selected_track,
        playlist.is_search_result,
    )
}

fn create_yandex_playlist<'local>(
    env: &mut Env<'local>,
    registration: SourceRegistrationId,
    playlist: YandexMusicSourcePlaylist,
) -> jni::errors::Result<JObject<'local>> {
    let tracks = playlist
        .tracks
        .into_iter()
        .map(|track| {
            (
                track.info.clone(),
                BridgeItem::Yandex(YandexMusicSourceItem::Track(track)),
            )
        })
        .collect();
    create_native_playlist(
        env,
        registration,
        playlist.name,
        tracks,
        playlist.selected_track,
        playlist.is_search_result,
    )
}

fn create_native_playlist<'local>(
    env: &mut Env<'local>,
    registration: SourceRegistrationId,
    name: String,
    source_tracks: Vec<(TrackInfo, BridgeItem)>,
    selected_track: Option<usize>,
    is_search_result: bool,
) -> jni::errors::Result<JObject<'local>> {
    let tracks = env.new_object(jni_str!("java/util/ArrayList"), jni_sig!("()V"), &[])?;
    for (info, source_item) in source_tracks {
        env.with_local_frame(16, |env| {
            let track = create_native_track(env, &info)?;
            remember_source_item(env, &track, registration, source_item)?;
            let _ = env.call_method(
                &tracks,
                jni_str!("add"),
                jni_sig!("(Ljava/lang/Object;)Z"),
                &[JValue::Object(&track)],
            )?;
            Ok::<_, jni::errors::Error>(())
        })?;
    }
    let selected = selected_track.map_or_else(
        || Ok(JObject::null()),
        |index| {
            let index = i32::try_from(index)
                .map_err(|_| jni::errors::Error::NullPtr("playlist selection exceeds JVM index"))?;
            env.call_method(
                &tracks,
                jni_str!("get"),
                jni_sig!("(I)Ljava/lang/Object;"),
                &[JValue::Int(index)],
            )?
            .l()
        },
    )?;
    let name = env.new_string(name)?;
    env.new_object(
        jni_str!("com/sedmelluq/discord/lavaplayer/track/BasicAudioPlaylist"),
        jni_sig!("(Ljava/lang/String;Ljava/util/List;Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;Z)V"),
        &[
            JValue::Object(name.as_ref()),
            JValue::Object(&tracks),
            JValue::Object(&selected),
            JValue::Bool(is_search_result),
        ],
    )
}

pub(crate) fn tracked_source_item_count(env: &mut Env<'_>) -> jni::errors::Result<usize> {
    let mut state = runtime()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(runtime) = state.as_mut() else {
        return Ok(0);
    };
    prune_tracked_source_items(env, &mut runtime.tracked_items)?;
    Ok(runtime.tracked_items.len())
}

pub(crate) fn shutdown(env: &mut Env<'_>) {
    if shutdown_in_progress().swap(true, Ordering::AcqRel) {
        return;
    }
    let runtime = runtime()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    let Some(mut runtime) = runtime else {
        shutdown_in_progress().store(false, Ordering::Release);
        return;
    };
    drop(runtime.executor.take());
    if let Some(registry) = runtime.registry.take() {
        registry.shutdown();
    }
    let _ = runtime
        .ordering_keys
        .drain_releases(env, &runtime.ordering_releases);
    runtime.ordering_keys.shutdown();
    runtime.ordering_releases.shutdown();
    runtime.entries.clear();
    shutdown_in_progress().store(false, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_source_distinguishes_track_pending_and_missing() {
        let source = GateSourceManager;
        let track = SourceReference::new(Some("gate:track".to_owned()), false);
        assert!(matches!(
            source.load(&track).expect("track load"),
            Some(SourceLoad::Item(BridgeItem::Synthetic(identifier)))
                if identifier == "gate:track"
        ));
        let missing = SourceReference::new(Some("other:item".to_owned()), false);
        assert!(source.load(&missing).expect("missing load").is_none());
        assert_eq!(source.source_name(), "mantle-oracle");
        assert_eq!(
            source
                .encode(&BridgeItem::Synthetic("gate:track".to_owned()))
                .expect("track encoding"),
            b"\0\toracle-v1"
        );
    }

    #[test]
    fn youtube_gate_source_preserves_native_result_and_empty_detail_contracts() {
        let source = GateYoutubeSourceManager;
        let track = source
            .load(&SourceReference::new(
                Some("gate:youtube-track".to_owned()),
                false,
            ))
            .expect("YouTube track load")
            .expect("recognized YouTube fixture");
        let SourceLoad::Item(BridgeItem::Youtube(YoutubeSourceItem::Track(track))) = track else {
            panic!("expected a native YouTube track");
        };
        assert_eq!(track.info.identifier, "dQw4w9WgXcQ");
        assert_eq!(
            source
                .encode(&BridgeItem::Youtube(YoutubeSourceItem::Track(
                    track.clone()
                )))
                .expect("YouTube detail encoding"),
            Vec::<u8>::new()
        );
        assert!(matches!(
            source
                .decode_with_info(&track.info, &[])
                .expect("YouTube detail reconstruction"),
            BridgeItem::Youtube(YoutubeSourceItem::Track(decoded)) if decoded == track
        ));

        let playlist = source
            .load(&SourceReference::new(
                Some("gate:youtube-playlist".to_owned()),
                false,
            ))
            .expect("YouTube playlist load")
            .expect("recognized YouTube playlist");
        assert!(matches!(
            playlist,
            SourceLoad::Item(BridgeItem::Youtube(YoutubeSourceItem::Playlist(playlist)))
                if playlist.tracks.len() == 2 && playlist.selected_track == Some(1)
        ));
        assert_eq!(source.source_name(), "youtube");
    }

    #[test]
    fn yandex_gate_source_preserves_native_result_and_empty_detail_contracts() {
        let source = GateYandexSourceManager;
        let track = source
            .load(&SourceReference::new(
                Some("gate:yandex-track".to_owned()),
                false,
            ))
            .expect("Yandex track load")
            .expect("recognized Yandex fixture");
        let SourceLoad::Item(BridgeItem::Yandex(YandexMusicSourceItem::Track(track))) = track
        else {
            panic!("expected a native Yandex track");
        };
        assert_eq!(track.info.identifier, "71663565");
        assert_eq!(track.info.title, "Animals");
        assert_eq!(track.info.author, "Architects");
        assert_eq!(track.info.duration, Duration::from_millis(244_321));
        assert_eq!(
            source
                .encode(&BridgeItem::Yandex(YandexMusicSourceItem::Track(
                    track.clone()
                )))
                .expect("Yandex detail encoding"),
            Vec::<u8>::new()
        );
        assert!(matches!(
            source
                .decode_with_info(&track.info, &[])
                .expect("Yandex detail reconstruction"),
            BridgeItem::Yandex(YandexMusicSourceItem::Track(decoded)) if decoded == track
        ));

        let playlist = source
            .load(&SourceReference::new(
                Some("gate:yandex-playlist".to_owned()),
                false,
            ))
            .expect("Yandex playlist load")
            .expect("recognized Yandex playlist");
        assert!(matches!(
            playlist,
            SourceLoad::Item(BridgeItem::Yandex(YandexMusicSourceItem::Playlist(playlist)))
                if playlist.tracks.len() == 2
                    && playlist.selected_track.is_none()
                    && playlist.kind == YandexMusicPlaylistKind::Playlist
        ));

        let search = source
            .load(&SourceReference::new(
                Some("gate:yandex-search".to_owned()),
                false,
            ))
            .expect("Yandex search load")
            .expect("recognized Yandex search");
        assert!(matches!(
            search,
            SourceLoad::Item(BridgeItem::Yandex(YandexMusicSourceItem::Playlist(search)))
                if search.tracks.len() == 2
                    && search.selected_track.is_none()
                    && search.is_search_result
                    && search.kind == YandexMusicPlaylistKind::Search
        ));
        assert_eq!(source.source_name(), "yandex-music");
    }
}
