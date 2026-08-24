use std::time::{Duration, Instant};

use jni::Env;
use jni::objects::{JObject, JString, JValue};
use jni::{jni_sig, jni_str};
use mantle_audio::{
    COMPATIBLE_CHANNELS, COMPATIBLE_FRAME_DURATION, COMPATIBLE_PCM_SAMPLES, COMPATIBLE_SAMPLE_RATE,
    EncodedFrameSlot, MAX_RESAMPLER_BUFFERED_FRAMES, OpusEncodingQuality, PcmFormat, PcmFrame,
    PcmOpusEncoder, PcmResampler, ResamplingQuality, VolumeLevel,
};
use mantle_core::{SourceCancellation, SourceManager};
use mantle_media::{
    BandcampPlaybackSession, BandcampRoute, BandcampSourceManager, BandcampSourceOptions,
    BandcampSourceTrack, BeamErrorKind, BeamSourceManager, GetyarnErrorKind, GetyarnSourceManager,
    HttpRangeInput, HttpRangeOptions, MediaCancellation, MediaInfo, MediaLimits, MediaSession,
    NicoNicoPlaybackSession, NicoNicoSourceManager, NicoNicoSourceOptions, NicoNicoSourceTrack,
    SoundCloudAuthentication, SoundCloudPlaybackSession, SoundCloudSourceManager,
    SoundCloudSourceOptions, TwitchAuthentication, TwitchLivePlaybackOptions,
    TwitchLivePlaybackPoll, TwitchLivePlaybackSession, TwitchSourceManager, TwitchSourceOptions,
    TwitchSourceTrack, VimeoAuthentication, VimeoPlaybackSession, VimeoSourceManager,
    VimeoSourceOptions, VimeoSourceTrack, YandexMusicAuthentication, YandexMusicPlaybackSession,
    YandexMusicSourceManager, YandexMusicSourceOptions, YoutubeAudioSourceManager,
    YoutubeAuthentication, YoutubeLivePlaybackOptions, YoutubeLivePlaybackPoll,
    YoutubeLivePlaybackSession, YoutubePlaybackSession, YoutubeSourceOptions,
    route_bandcamp_identifier, route_twitch_identifier,
};

const TRANSCODE_INPUT_CHUNK_FRAMES: usize = 1_024;

pub(crate) fn process_bandcamp_track(
    env: &mut Env<'_>,
    track: &JObject<'_>,
    executor: &JObject<'_>,
) -> jni::errors::Result<()> {
    let java_info = env
        .get_field(
            track,
            jni_str!("trackInfo"),
            jni_sig!("Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackInfo;"),
        )?
        .l()?;
    let info = crate::track_info_from_java(env, &java_info)?;
    let options = BandcampSourceOptions::default();
    if !matches!(
        route_bandcamp_identifier(&info.identifier, &options),
        Some(BandcampRoute::Track(_))
    ) {
        return Err(failure("unsupported public Bandcamp track route"));
    }
    if executor.is_null() {
        return Err(failure("native playback requires a local track executor"));
    }
    let manager = BandcampSourceManager::new(options)
        .map_err(|_| failure("could not create current Bandcamp playback source"))?;
    let source_track = BandcampSourceTrack {
        info,
        playback: None,
    };
    let cancellation = MediaCancellation::new();
    let session = manager
        .open_track_playback(
            &source_track,
            HttpRangeOptions::default(),
            MediaLimits::default(),
            cancellation.clone(),
        )
        .map_err(|_| failure("current Bandcamp playback handoff failed"))?
        .ok_or_else(|| failure("current Bandcamp track has no compatible playback"))?;

    process_playback_session(env, executor, session, &cancellation)
}

pub(crate) fn process_beam_track(
    env: &mut Env<'_>,
    track: &JObject<'_>,
    _executor: &JObject<'_>,
) -> jni::errors::Result<()> {
    let java_info = env
        .get_field(
            track,
            jni_str!("trackInfo"),
            jni_sig!("Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackInfo;"),
        )?
        .l()?;
    let info = crate::track_info_from_java(env, &java_info)?;
    let manager = BeamSourceManager::default();
    let source_track = manager
        .decode_with_info(&info, &[])
        .map_err(|_| failure("invalid legacy Beam track details"))?;
    match manager.open_track_playback(&source_track, &SourceCancellation::new()) {
        Err(error) if error.kind() == BeamErrorKind::ServiceClosed => Err(failure(
            "Beam/Mixer service is closed; legacy track playback is unavailable",
        )),
        Err(_) => Err(failure("invalid legacy Beam playback state")),
        Ok(()) => Err(failure(
            "retired Beam playback unexpectedly became available",
        )),
    }
}

pub(crate) fn process_getyarn_track(
    env: &mut Env<'_>,
    track: &JObject<'_>,
    _executor: &JObject<'_>,
) -> jni::errors::Result<()> {
    let java_info = env
        .get_field(
            track,
            jni_str!("trackInfo"),
            jni_sig!("Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackInfo;"),
        )?
        .l()?;
    let info = crate::track_info_from_java(env, &java_info)?;
    let manager = GetyarnSourceManager::default();
    let source_track = manager
        .decode_with_info(&info, &[])
        .map_err(|_| failure("invalid legacy Getyarn track details"))?;
    match manager.open_track_playback(&source_track, &SourceCancellation::new()) {
        Err(error) if error.kind() == GetyarnErrorKind::UnsupportedPlayback => Err(failure(
            "Getyarn playback has no supported current protocol; legacy media playback is unavailable",
        )),
        Err(_) => Err(failure("invalid legacy Getyarn playback state")),
        Ok(()) => Err(failure(
            "retired Getyarn playback unexpectedly became available",
        )),
    }
}

pub(crate) fn process_http_track(
    env: &mut Env<'_>,
    track: &JObject<'_>,
    executor: &JObject<'_>,
) -> jni::errors::Result<()> {
    let java_info = env
        .get_field(
            track,
            jni_str!("trackInfo"),
            jni_sig!("Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackInfo;"),
        )?
        .l()?;
    let info = crate::track_info_from_java(env, &java_info)?;
    let extension = http_extension_hint(&info.identifier);
    let cancellation = MediaCancellation::new();
    let input = HttpRangeInput::open_with_cancellation(
        &info.identifier,
        HttpRangeOptions::default(),
        cancellation.clone(),
    )
    .map_err(|_| failure("current HTTP track playback failed"))?;
    let session = MediaSession::open_with_cancellation(
        Box::new(input),
        extension.as_deref(),
        MediaLimits::default(),
        cancellation.clone(),
    )
    .map_err(|_| failure("current HTTP track playback failed"))?;
    process_playback_session(env, executor, session, &cancellation)
}

pub(crate) fn process_nico_track(
    env: &mut Env<'_>,
    track: &JObject<'_>,
    executor: &JObject<'_>,
) -> jni::errors::Result<()> {
    let java_info = env
        .get_field(
            track,
            jni_str!("trackInfo"),
            jni_sig!("Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackInfo;"),
        )?
        .l()?;
    let info = crate::track_info_from_java(env, &java_info)?;
    let source_track = NicoNicoSourceTrack {
        info,
        playback_available: true,
    };
    let limits = MediaLimits::default();
    let cancellation = MediaCancellation::new();
    let manager = NicoNicoSourceManager::new(NicoNicoSourceOptions::default())
        .map_err(|_| failure("could not create current NicoNico playback source"))?;
    let session = manager
        .open_track_playback(&source_track, limits, cancellation.clone())
        .map_err(|_| failure("current NicoNico playback discovery failed"))?
        .ok_or_else(|| failure("current NicoNico track is unavailable"))?;

    process_playback_session(env, executor, session, &cancellation)
}

pub(crate) fn process_sound_cloud_track(
    env: &mut Env<'_>,
    track: &JObject<'_>,
    executor: &JObject<'_>,
) -> jni::errors::Result<()> {
    let client_id = system_property(env, "dev.mantle.soundcloud.clientId")?
        .ok_or_else(|| failure("SoundCloud playback requires dev.mantle.soundcloud.clientId"))?;
    let oauth_token = system_property(env, "dev.mantle.soundcloud.oauthToken")?;
    let authentication = SoundCloudAuthentication::with_oauth(client_id, oauth_token)
        .map_err(|_| failure("invalid explicit SoundCloud credentials"))?;
    let manager = SoundCloudSourceManager::new(SoundCloudSourceOptions::default(), authentication)
        .map_err(|_| failure("could not create current SoundCloud playback source"))?;
    let java_info = env
        .get_field(
            track,
            jni_str!("trackInfo"),
            jni_sig!("Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackInfo;"),
        )?
        .l()?;
    let info = crate::track_info_from_java(env, &java_info)?;
    let uri = info
        .uri
        .as_deref()
        .ok_or_else(|| failure("SoundCloud track requires a source URI"))?;
    let limits = MediaLimits::default();
    let cancellation = MediaCancellation::new();
    let source_track = manager
        .load_track_metadata(uri, &cancellation)
        .map_err(|_| failure("current SoundCloud playback discovery failed"))?
        .ok_or_else(|| failure("current SoundCloud track is unavailable"))?;
    let session = manager
        .open_track_playback(
            &source_track,
            HttpRangeOptions::default(),
            limits,
            cancellation.clone(),
        )
        .map_err(|_| failure("current SoundCloud playback handoff failed"))?
        .ok_or_else(|| failure("current SoundCloud track has no compatible playback"))?;

    process_playback_session(env, executor, session, &cancellation)
}

pub(crate) fn process_twitch_track(
    env: &mut Env<'_>,
    track: &JObject<'_>,
    executor: &JObject<'_>,
) -> jni::errors::Result<()> {
    let java_info = env
        .get_field(
            track,
            jni_str!("trackInfo"),
            jni_sig!("Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackInfo;"),
        )?
        .l()?;
    let info = crate::track_info_from_java(env, &java_info)?;
    let options = TwitchSourceOptions::default();
    let route = route_twitch_identifier(&info.identifier, &options)
        .ok_or_else(|| failure("current Twitch track has an unsupported channel route"))?;
    let client_id = system_property(env, "dev.mantle.twitch.clientId")?
        .ok_or_else(|| failure("Twitch playback requires dev.mantle.twitch.clientId"))?;
    let access_token = system_property(env, "dev.mantle.twitch.accessToken")?
        .ok_or_else(|| failure("Twitch playback requires dev.mantle.twitch.accessToken"))?;
    let device_id = system_property(env, "dev.mantle.twitch.deviceId")?;
    let authentication = TwitchAuthentication::with_device_id(client_id, access_token, device_id)
        .map_err(|_| failure("invalid explicit Twitch credentials"))?;
    if executor.is_null() {
        return Err(failure("native playback requires a local track executor"));
    }
    let manager = TwitchSourceManager::new(options, authentication)
        .map_err(|_| failure("could not create current Twitch playback source"))?;
    let source_track = TwitchSourceTrack {
        info,
        channel: route.channel,
    };
    let cancellation = MediaCancellation::new();
    let session = manager
        .open_live_playback(
            &source_track,
            TwitchLivePlaybackOptions::default(),
            cancellation.clone(),
        )
        .map_err(|_| failure("current Twitch playback discovery failed"))?;
    process_twitch_live_session(env, executor, session, &cancellation)
}

pub(crate) fn process_vimeo_track(
    env: &mut Env<'_>,
    track: &JObject<'_>,
    executor: &JObject<'_>,
) -> jni::errors::Result<()> {
    let options = VimeoSourceOptions::default();
    let manager = match system_property(env, "dev.mantle.vimeo.accessToken")? {
        Some(access_token) => {
            let authentication = VimeoAuthentication::new(access_token)
                .map_err(|_| failure("invalid Vimeo JVM access token"))?;
            VimeoSourceManager::with_authentication(options, authentication)
        }
        None => VimeoSourceManager::new(options),
    }
    .map_err(|_| failure("could not create current Vimeo playback source"))?;
    let java_info = env
        .get_field(
            track,
            jni_str!("trackInfo"),
            jni_sig!("Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackInfo;"),
        )?
        .l()?;
    let source_track = VimeoSourceTrack {
        info: crate::track_info_from_java(env, &java_info)?,
        playback: None,
    };
    let cancellation = MediaCancellation::new();
    let session = manager
        .open_track_playback(
            &source_track,
            HttpRangeOptions::default(),
            MediaLimits::default(),
            cancellation.clone(),
        )
        .map_err(|_| failure("current Vimeo playback handoff failed"))?
        .ok_or_else(|| failure("current Vimeo track has no compatible playback"))?;

    process_playback_session(env, executor, session, &cancellation)
}

pub(crate) fn process_yandex_music_track(
    env: &mut Env<'_>,
    track: &JObject<'_>,
    executor: &JObject<'_>,
) -> jni::errors::Result<()> {
    let access_token = system_property(env, "dev.mantle.yandex.accessToken")?
        .ok_or_else(|| failure("Yandex Music playback requires dev.mantle.yandex.accessToken"))?;
    let authentication = YandexMusicAuthentication::new(access_token)
        .map_err(|_| failure("invalid Yandex Music JVM access token"))?;
    let manager =
        YandexMusicSourceManager::new(YandexMusicSourceOptions::default(), authentication)
            .map_err(|_| failure("could not create current Yandex Music playback source"))?;
    let java_info = env
        .get_field(
            track,
            jni_str!("trackInfo"),
            jni_sig!("Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackInfo;"),
        )?
        .l()?;
    let info = crate::track_info_from_java(env, &java_info)?;
    let cancellation = MediaCancellation::new();
    let session = manager
        .open_track_playback(
            &info.identifier,
            HttpRangeOptions::default(),
            MediaLimits::default(),
            cancellation.clone(),
        )
        .map_err(|_| failure("current Yandex Music playback handoff failed"))?
        .ok_or_else(|| failure("current Yandex Music track has no compatible playback"))?;

    process_playback_session(env, executor, session, &cancellation)
}

pub(crate) fn process_youtube_track<'local>(
    env: &mut Env<'local>,
    track: &JObject<'local>,
    executor: &JObject<'local>,
) -> jni::errors::Result<()> {
    if executor.is_null() {
        return Err(failure("native playback requires a local track executor"));
    }
    let java_info = env
        .get_field(
            track,
            jni_str!("trackInfo"),
            jni_sig!("Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackInfo;"),
        )?
        .l()?;
    let info = crate::track_info_from_java(env, &java_info)?;
    let manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions::default(),
        YoutubeAuthentication::default(),
    )
    .map_err(|_| failure("could not create current YouTube playback source"))?;
    let cancellation = MediaCancellation::new();
    let formats = manager
        .discover_playback_formats(&info.identifier, &cancellation)
        .map_err(|_| failure("current YouTube playback discovery failed"))?;
    if info.is_stream || formats.selected().content_length().is_none() {
        let session = manager
            .open_selected_live_playback(
                &formats,
                YoutubeLivePlaybackOptions::default(),
                cancellation.clone(),
            )
            .map_err(|_| failure("current YouTube live playback handoff failed"))?;
        process_youtube_live_session(env, executor, session, &cancellation)
    } else {
        let session = manager
            .open_selected_playback(
                &formats,
                HttpRangeOptions::default(),
                MediaLimits::default(),
                cancellation.clone(),
            )
            .map_err(|_| failure("current YouTube playback handoff failed"))?;
        process_youtube_playback_session(env, executor, session, &cancellation)
    }
}

fn process_youtube_playback_session<'local>(
    env: &mut Env<'local>,
    executor: &JObject<'local>,
    mut session: YoutubePlaybackSession,
    cancellation: &MediaCancellation,
) -> jni::errors::Result<()> {
    let (buffer, format) = native_output_target(env, executor)?;
    let started = Instant::now();
    let mut encoded = EncodedFrameSlot::new();
    loop {
        if current_thread_interrupted(env)? {
            cancellation.cancel();
            return Err(failure("current native playback was cancelled"));
        }
        if !session
            .read_frame(&mut encoded)
            .map_err(|_| failure("current YouTube media playback failed"))?
        {
            return Ok(());
        }
        consume_native_frame(env, &buffer, &format, &encoded, started.elapsed())?;
    }
}

fn process_youtube_live_session<'local>(
    env: &mut Env<'local>,
    executor: &JObject<'local>,
    mut session: YoutubeLivePlaybackSession,
    cancellation: &MediaCancellation,
) -> jni::errors::Result<()> {
    let (buffer, format) = native_output_target(env, executor)?;
    let started = Instant::now();
    let mut encoded = EncodedFrameSlot::new();
    loop {
        if current_thread_interrupted(env)? {
            cancellation.cancel();
            return Err(failure("current native playback was cancelled"));
        }
        match session
            .poll_frame(started.elapsed(), &mut encoded)
            .map_err(|_| failure("current YouTube live playback failed"))?
        {
            YoutubeLivePlaybackPoll::Frame => {
                consume_native_frame(env, &buffer, &format, &encoded, started.elapsed())?;
            }
            YoutubeLivePlaybackPoll::WaitUntil(deadline) => {
                let remaining = deadline.saturating_sub(started.elapsed());
                if !remaining.is_zero() {
                    std::thread::park_timeout(remaining.min(Duration::from_millis(10)));
                }
            }
            YoutubeLivePlaybackPoll::Ended | YoutubeLivePlaybackPoll::Exhausted => return Ok(()),
        }
    }
}

fn native_output_target<'local>(
    env: &mut Env<'local>,
    executor: &JObject<'local>,
) -> jni::errors::Result<(JObject<'local>, JObject<'local>)> {
    let buffer = env
        .call_method(
            executor,
            jni_str!("getAudioBuffer"),
            jni_sig!("()Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrameBuffer;"),
            &[],
        )?
        .l()?;
    if buffer.is_null() {
        return Err(failure("native playback requires an audio frame buffer"));
    }
    let context = env
        .call_method(
            executor,
            jni_str!("getProcessingContext"),
            jni_sig!("()Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioProcessingContext;"),
            &[],
        )?
        .l()?;
    let format = validate_output_format(env, &context)?;
    Ok((buffer, format))
}

fn consume_native_frame(
    env: &mut Env<'_>,
    buffer: &JObject<'_>,
    format: &JObject<'_>,
    encoded: &EncodedFrameSlot,
    fallback_timecode: Duration,
) -> jni::errors::Result<()> {
    env.with_local_frame(4, |env| {
        let data = JObject::from(env.byte_array_from_slice(encoded.data())?);
        let timecode = encoded.timestamp().unwrap_or(fallback_timecode).as_millis();
        let timecode = i64::try_from(timecode)
            .map_err(|_| failure("native frame timecode exceeds the JVM range"))?;
        let frame = env.new_object(
            jni_str!("com/sedmelluq/discord/lavaplayer/track/playback/ImmutableAudioFrame"),
            jni_sig!("(J[BILcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;)V"),
            &[
                JValue::Long(timecode),
                JValue::Object(&data),
                JValue::Int(i32::from(encoded.volume().get())),
                JValue::Object(format),
            ],
        )?;
        let _ = env.call_method(
            buffer,
            jni_str!("consume"),
            jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrame;)V"),
            &[JValue::Object(&frame)],
        )?;
        Ok::<(), jni::errors::Error>(())
    })
}

fn process_playback_session<S: PcmPlaybackSession>(
    env: &mut Env<'_>,
    executor: &JObject<'_>,
    session: S,
    cancellation: &MediaCancellation,
) -> jni::errors::Result<()> {
    if executor.is_null() {
        return Err(failure("native playback requires a local track executor"));
    }
    let buffer = env
        .call_method(
            executor,
            jni_str!("getAudioBuffer"),
            jni_sig!("()Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrameBuffer;"),
            &[],
        )?
        .l()?;
    if buffer.is_null() {
        return Err(failure("native playback requires an audio frame buffer"));
    }
    let context = env
        .call_method(
            executor,
            jni_str!("getProcessingContext"),
            jni_sig!("()Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioProcessingContext;"),
            &[],
        )?
        .l()?;
    let format = validate_output_format(env, &context)?;
    let volume = player_volume(env, &context)?;
    let mut transcoder = PcmTranscoder::new(session, MediaLimits::default())?;
    let mut encoded = EncodedFrameSlot::new();

    while transcoder.read_frame(&mut encoded, volume)? {
        env.with_local_frame(4, |env| {
            if current_thread_interrupted(env)? {
                cancellation.cancel();
                return Err(failure("current native playback was cancelled"));
            }
            let data = JObject::from(env.byte_array_from_slice(encoded.data())?);
            let timecode = encoded
                .timestamp()
                .unwrap_or_else(|| transcoder.last_frame_timecode())
                .as_millis();
            let timecode = i64::try_from(timecode)
                .map_err(|_| failure("native frame timecode exceeds the JVM range"))?;
            let frame = env.new_object(
                jni_str!("com/sedmelluq/discord/lavaplayer/track/playback/ImmutableAudioFrame"),
                jni_sig!("(J[BILcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;)V"),
                &[
                    JValue::Long(timecode),
                    JValue::Object(&data),
                    JValue::Int(i32::from(encoded.volume().get())),
                    JValue::Object(&format),
                ],
            )?;
            let _ = env.call_method(
                &buffer,
                jni_str!("consume"),
                jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrame;)V"),
                &[JValue::Object(&frame)],
            )?;
            Ok(())
        })?;
    }
    Ok(())
}

fn process_twitch_live_session(
    env: &mut Env<'_>,
    executor: &JObject<'_>,
    mut session: TwitchLivePlaybackSession,
    cancellation: &MediaCancellation,
) -> jni::errors::Result<()> {
    if executor.is_null() {
        return Err(failure("native playback requires a local track executor"));
    }
    let buffer = env
        .call_method(
            executor,
            jni_str!("getAudioBuffer"),
            jni_sig!("()Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrameBuffer;"),
            &[],
        )?
        .l()?;
    if buffer.is_null() {
        return Err(failure("native playback requires an audio frame buffer"));
    }
    let context = env
        .call_method(
            executor,
            jni_str!("getProcessingContext"),
            jni_sig!("()Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioProcessingContext;"),
            &[],
        )?
        .l()?;
    let format = validate_output_format(env, &context)?;
    let started = Instant::now();
    let mut encoded = EncodedFrameSlot::new();

    loop {
        if current_thread_interrupted(env)? {
            cancellation.cancel();
            return Err(failure("current native playback was cancelled"));
        }
        match session
            .poll_frame(started.elapsed(), &mut encoded)
            .map_err(|_| failure("current Twitch live playback failed"))?
        {
            TwitchLivePlaybackPoll::Frame => env.with_local_frame(4, |env| {
                let data = JObject::from(env.byte_array_from_slice(encoded.data())?);
                let timecode = encoded
                    .timestamp()
                    .unwrap_or_else(|| started.elapsed())
                    .as_millis();
                let timecode = i64::try_from(timecode)
                    .map_err(|_| failure("native frame timecode exceeds the JVM range"))?;
                let frame = env.new_object(
                    jni_str!("com/sedmelluq/discord/lavaplayer/track/playback/ImmutableAudioFrame"),
                    jni_sig!("(J[BILcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;)V"),
                    &[
                        JValue::Long(timecode),
                        JValue::Object(&data),
                        JValue::Int(i32::from(encoded.volume().get())),
                        JValue::Object(&format),
                    ],
                )?;
                let _ = env.call_method(
                    &buffer,
                    jni_str!("consume"),
                    jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrame;)V"),
                    &[JValue::Object(&frame)],
                )?;
                Ok::<(), jni::errors::Error>(())
            })?,
            TwitchLivePlaybackPoll::WaitUntil(deadline) => {
                let remaining = deadline.saturating_sub(started.elapsed());
                if !remaining.is_zero() {
                    std::thread::park_timeout(remaining.min(Duration::from_millis(10)));
                }
            }
            TwitchLivePlaybackPoll::Ended | TwitchLivePlaybackPoll::Exhausted => return Ok(()),
        }
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

fn validate_output_format<'local>(
    env: &mut Env<'local>,
    context: &JObject<'local>,
) -> jni::errors::Result<JObject<'local>> {
    if context.is_null() {
        return Err(failure(
            "native playback requires an audio processing context",
        ));
    }
    let format = env
        .get_field(
            context,
            jni_str!("outputFormat"),
            jni_sig!("Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;"),
        )?
        .l()?;
    if format.is_null() {
        return Err(failure("native playback requires an output format"));
    }
    let codec = env
        .call_method(
            &format,
            jni_str!("codecName"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?
        .l()?;
    let codec = JString::cast_local(env, codec)?.try_to_string(env)?;
    let channels = env
        .get_field(&format, jni_str!("channelCount"), jni_sig!("I"))?
        .i()?;
    let sample_rate = env
        .get_field(&format, jni_str!("sampleRate"), jni_sig!("I"))?
        .i()?;
    let chunk_samples = env
        .get_field(&format, jni_str!("chunkSampleCount"), jni_sig!("I"))?
        .i()?;
    if codec != "OPUS"
        || channels != i32::from(COMPATIBLE_CHANNELS)
        || sample_rate != i32::try_from(COMPATIBLE_SAMPLE_RATE).unwrap_or(i32::MAX)
        || chunk_samples
            != i32::try_from(mantle_audio::COMPATIBLE_SAMPLES_PER_CHANNEL).unwrap_or(i32::MAX)
    {
        return Err(failure(
            "current native playback requires 48 kHz stereo 20 ms Opus output",
        ));
    }
    Ok(format)
}

fn player_volume(env: &mut Env<'_>, context: &JObject<'_>) -> jni::errors::Result<VolumeLevel> {
    let options = env
        .get_field(
            context,
            jni_str!("playerOptions"),
            jni_sig!("Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayerOptions;"),
        )?
        .l()?;
    if options.is_null() {
        return Ok(VolumeLevel::NORMAL);
    }
    let volume = env
        .get_field(
            &options,
            jni_str!("volumeLevel"),
            jni_sig!("Ljava/util/concurrent/atomic/AtomicInteger;"),
        )?
        .l()?;
    let value = env
        .call_method(&volume, jni_str!("get"), jni_sig!("()I"), &[])?
        .i()?;
    Ok(VolumeLevel::new(value))
}

fn current_thread_interrupted(env: &mut Env<'_>) -> jni::errors::Result<bool> {
    let thread = env
        .call_static_method(
            jni_str!("java/lang/Thread"),
            jni_str!("currentThread"),
            jni_sig!("()Ljava/lang/Thread;"),
            &[],
        )?
        .l()?;
    env.call_method(&thread, jni_str!("isInterrupted"), jni_sig!("()Z"), &[])?
        .z()
}

trait PcmPlaybackSession {
    fn info(&self) -> &MediaInfo;
    fn read_pcm(&mut self, output: &mut PcmFrame) -> jni::errors::Result<bool>;
}

impl PcmPlaybackSession for BandcampPlaybackSession {
    fn info(&self) -> &MediaInfo {
        self.info()
    }

    fn read_pcm(&mut self, output: &mut PcmFrame) -> jni::errors::Result<bool> {
        self.read_pcm(output)
            .map_err(|_| failure("Bandcamp media decoding failed"))
    }
}

impl PcmPlaybackSession for NicoNicoPlaybackSession {
    fn info(&self) -> &MediaInfo {
        self.info()
    }

    fn read_pcm(&mut self, output: &mut PcmFrame) -> jni::errors::Result<bool> {
        self.read_pcm(output)
            .map_err(|_| failure("NicoNico media decoding failed"))
    }
}

impl PcmPlaybackSession for SoundCloudPlaybackSession {
    fn info(&self) -> &MediaInfo {
        self.info()
    }

    fn read_pcm(&mut self, output: &mut PcmFrame) -> jni::errors::Result<bool> {
        self.read_pcm(output)
            .map_err(|_| failure("SoundCloud media decoding failed"))
    }
}

impl PcmPlaybackSession for VimeoPlaybackSession {
    fn info(&self) -> &MediaInfo {
        self.info()
    }

    fn read_pcm(&mut self, output: &mut PcmFrame) -> jni::errors::Result<bool> {
        self.read_pcm(output)
            .map_err(|_| failure("Vimeo media decoding failed"))
    }
}

impl PcmPlaybackSession for YandexMusicPlaybackSession {
    fn info(&self) -> &MediaInfo {
        self.info()
    }

    fn read_pcm(&mut self, output: &mut PcmFrame) -> jni::errors::Result<bool> {
        self.read_pcm(output)
            .map_err(|_| failure("Yandex Music media decoding failed"))
    }
}

impl PcmPlaybackSession for MediaSession {
    fn info(&self) -> &MediaInfo {
        self.info()
    }

    fn read_pcm(&mut self, output: &mut PcmFrame) -> jni::errors::Result<bool> {
        self.read_pcm(output)
            .map_err(|_| failure("HTTP media decoding failed"))
    }
}

fn http_extension_hint(identifier: &str) -> Option<String> {
    let path = identifier.split(['?', '#']).next().unwrap_or(identifier);
    let name = path.rsplit('/').next()?;
    let (_, extension) = name.rsplit_once('.')?;
    (!extension.is_empty()).then(|| extension.to_owned())
}

struct PcmTranscoder<S> {
    session: S,
    source_format: PcmFormat,
    decoded: PcmFrame,
    decoded_offset: usize,
    resampler: Option<PcmResampler>,
    resampled: PcmFrame,
    resampled_offset: usize,
    assembled: [f32; COMPATIBLE_PCM_SAMPLES],
    assembled_len: usize,
    encoder_input: PcmFrame,
    encoder: PcmOpusEncoder,
    input_eof: bool,
    timestamp_initialized: bool,
    base_timestamp: Option<Duration>,
    frames_encoded: u64,
}

impl<S: PcmPlaybackSession> PcmTranscoder<S> {
    fn new(session: S, limits: MediaLimits) -> jni::errors::Result<Self> {
        let source_format = PcmFormat::new(session.info().sample_rate, session.info().channels)
            .map_err(|_| failure("native playback returned an incompatible PCM format"))?;
        let resampler = (source_format.sample_rate() != COMPATIBLE_SAMPLE_RATE)
            .then(|| {
                PcmResampler::new(
                    source_format,
                    COMPATIBLE_SAMPLE_RATE,
                    ResamplingQuality::Medium,
                    TRANSCODE_INPUT_CHUNK_FRAMES,
                    mantle_audio::COMPATIBLE_SAMPLES_PER_CHANNEL,
                    MAX_RESAMPLER_BUFFERED_FRAMES,
                )
            })
            .transpose()
            .map_err(|_| failure("could not create the native playback resampler"))?;
        Ok(Self {
            session,
            source_format,
            decoded: PcmFrame::with_capacity(limits.max_pcm_samples_per_frame),
            decoded_offset: 0,
            resampler,
            resampled: PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES),
            resampled_offset: 0,
            assembled: [0.0; COMPATIBLE_PCM_SAMPLES],
            assembled_len: 0,
            encoder_input: PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES),
            encoder: PcmOpusEncoder::new(OpusEncodingQuality::MAXIMUM)
                .map_err(|_| failure("could not create the native playback Opus encoder"))?,
            input_eof: false,
            timestamp_initialized: false,
            base_timestamp: None,
            frames_encoded: 0,
        })
    }

    fn read_frame(
        &mut self,
        output: &mut EncodedFrameSlot,
        volume: VolumeLevel,
    ) -> jni::errors::Result<bool> {
        while self.assembled_len < COMPATIBLE_PCM_SAMPLES {
            if self.resampler.is_some() {
                if self.append_resampled()? {
                    continue;
                }
                if self.input_eof {
                    break;
                }
                if self.session.read_pcm(&mut self.decoded)? {
                    self.validate_decoded_format()?;
                    self.resampler
                        .as_mut()
                        .expect("resampler path")
                        .push(&self.decoded)
                        .map_err(|_| failure("native playback resampling failed"))?;
                } else {
                    self.input_eof = true;
                    self.resampler
                        .as_mut()
                        .expect("resampler path")
                        .finish()
                        .map_err(|_| failure("native playback resampler finalization failed"))?;
                }
            } else if !self.append_decoded()? {
                if self.input_eof {
                    break;
                }
                if self.session.read_pcm(&mut self.decoded)? {
                    self.validate_decoded_format()?;
                    self.decoded_offset = 0;
                } else {
                    self.input_eof = true;
                }
            }
        }

        if self.assembled_len == 0 {
            output.clear();
            return Ok(false);
        }
        self.assembled[self.assembled_len..].fill(0.0);
        let timestamp = self.base_timestamp.map(|base| {
            base.saturating_add(
                COMPATIBLE_FRAME_DURATION
                    .saturating_mul(u32::try_from(self.frames_encoded).unwrap_or(u32::MAX)),
            )
        });
        let output_format = PcmFormat::new(COMPATIBLE_SAMPLE_RATE, COMPATIBLE_CHANNELS)
            .map_err(|_| failure("could not create the native playback output format"))?;
        self.encoder_input
            .copy_from_interleaved(&self.assembled, output_format, timestamp)
            .map_err(|_| failure("could not assemble the native playback Opus frame"))?;
        self.encoder
            .encode(&self.encoder_input, output, volume)
            .map_err(|_| failure("native playback Opus encoding failed"))?;
        self.assembled_len = 0;
        self.frames_encoded = self.frames_encoded.saturating_add(1);
        Ok(true)
    }

    fn last_frame_timecode(&self) -> Duration {
        COMPATIBLE_FRAME_DURATION.saturating_mul(
            u32::try_from(self.frames_encoded.saturating_sub(1)).unwrap_or(u32::MAX),
        )
    }

    fn append_decoded(&mut self) -> jni::errors::Result<bool> {
        if self.decoded_offset >= self.decoded.samples().len() {
            return Ok(false);
        }
        if !self.timestamp_initialized {
            self.base_timestamp = self.decoded.timestamp();
            self.timestamp_initialized = true;
        }
        append_interleaved(
            self.decoded.samples(),
            self.source_format.channels(),
            &mut self.decoded_offset,
            &mut self.assembled,
            &mut self.assembled_len,
        )?;
        Ok(true)
    }

    fn append_resampled(&mut self) -> jni::errors::Result<bool> {
        if self.resampled_offset >= self.resampled.samples().len() {
            if !self
                .resampler
                .as_mut()
                .expect("resampler path")
                .read(&mut self.resampled)
                .map_err(|_| failure("native playback resampling failed"))?
            {
                return Ok(false);
            }
            self.resampled_offset = 0;
        }
        if !self.timestamp_initialized {
            self.base_timestamp = self.resampled.timestamp();
            self.timestamp_initialized = true;
        }
        append_interleaved(
            self.resampled.samples(),
            self.source_format.channels(),
            &mut self.resampled_offset,
            &mut self.assembled,
            &mut self.assembled_len,
        )?;
        Ok(true)
    }

    fn validate_decoded_format(&self) -> jni::errors::Result<()> {
        if self.decoded.format() == Some(self.source_format) {
            Ok(())
        } else {
            Err(failure(
                "native playback changed PCM format during playback",
            ))
        }
    }
}

fn append_interleaved(
    input: &[f32],
    channels: u16,
    input_offset: &mut usize,
    output: &mut [f32; COMPATIBLE_PCM_SAMPLES],
    output_len: &mut usize,
) -> jni::errors::Result<()> {
    let channels = usize::from(channels);
    if channels == 0
        || *input_offset > input.len()
        || !input.len().is_multiple_of(channels)
        || !input_offset.is_multiple_of(channels)
    {
        return Err(failure("NicoNico returned invalid interleaved PCM"));
    }
    let available_frames = (input.len() - *input_offset) / channels;
    let output_frames = (COMPATIBLE_PCM_SAMPLES - *output_len) / usize::from(COMPATIBLE_CHANNELS);
    let frames = available_frames.min(output_frames);
    match channels {
        1 => {
            for sample in &input[*input_offset..*input_offset + frames] {
                output[*output_len] = *sample;
                output[*output_len + 1] = *sample;
                *output_len += 2;
            }
            *input_offset += frames;
        }
        2 => {
            let samples = frames * 2;
            output[*output_len..*output_len + samples]
                .copy_from_slice(&input[*input_offset..*input_offset + samples]);
            *output_len += samples;
            *input_offset += samples;
        }
        _ => return Err(failure("NicoNico returned unsupported PCM channels")),
    }
    Ok(())
}

const fn failure(message: &'static str) -> jni::errors::Error {
    jni::errors::Error::NullPtr(message)
}

#[cfg(test)]
mod tests {
    use super::append_interleaved;
    use mantle_audio::COMPATIBLE_PCM_SAMPLES;

    #[test]
    fn appends_stereo_and_expands_mono_without_crossing_frame_bounds() {
        let mut output = [0.0; COMPATIBLE_PCM_SAMPLES];
        let mut output_len = 0;
        let mut stereo_offset = 0;
        append_interleaved(
            &[0.1, 0.2, 0.3, 0.4],
            2,
            &mut stereo_offset,
            &mut output,
            &mut output_len,
        )
        .unwrap();
        assert_eq!(stereo_offset, 4);
        assert_eq!(output_len, 4);
        assert_eq!(&output[..4], &[0.1, 0.2, 0.3, 0.4]);

        let mut mono_offset = 0;
        append_interleaved(
            &[0.5, 0.6],
            1,
            &mut mono_offset,
            &mut output,
            &mut output_len,
        )
        .unwrap();
        assert_eq!(mono_offset, 2);
        assert_eq!(output_len, 8);
        assert_eq!(&output[4..8], &[0.5, 0.5, 0.6, 0.6]);
    }

    #[test]
    fn rejects_misaligned_or_unsupported_pcm() {
        let mut output = [0.0; COMPATIBLE_PCM_SAMPLES];
        let mut output_len = 0;
        let mut offset = 0;
        assert!(append_interleaved(&[0.1], 2, &mut offset, &mut output, &mut output_len).is_err());
        assert!(
            append_interleaved(
                &[0.1, 0.2, 0.3],
                3,
                &mut offset,
                &mut output,
                &mut output_len
            )
            .is_err()
        );
    }
}
