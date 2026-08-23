mod emitter;

use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mantle-jvm-gate: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let command = args.first().map(String::as_str);
    if let Some(source) = command.and_then(consumer_source) {
        return write_consumer(&args, source);
    }
    match command {
        Some("emit") => emitter::emit(
            &required_path(&args, "--reference-jar")?,
            &required_path(&args, "--output")?,
            required_value(&args, "--expected-abi")?.parse()?,
            optional_path(&args, "--manifest-output").as_deref(),
        ),
        Some("emit-reference-slice") => emitter::emit_reference_slice(
            &required_path(&args, "--reference-jar")?,
            &required_path(&args, "--output")?,
        ),
        Some("verify-structure") => emitter::verify_structure(
            &required_path(&args, "--reference-jar")?,
            &required_path(&args, "--candidate-jar")?,
        ),
        _ => Err(
            "usage: mantle-jvm-gate <emit|write-smoke-consumer|write-probe-consumer> [options]"
                .into(),
        ),
    }
}

fn consumer_source(command: &str) -> Option<&'static str> {
    match command {
        "write-smoke-consumer" => Some(SMOKE_CONSUMER),
        "write-probe-consumer" => Some(PROBE_CONSUMER),
        "write-integration-consumer" => Some(INTEGRATION_CONSUMER),
        "write-classloader-consumer" => Some(CLASSLOADER_CONSUMER),
        "write-event-consumer" => Some(EVENT_CONSUMER),
        "write-track-value-consumer" => Some(TRACK_VALUE_CONSUMER),
        "write-track-enum-consumer" => Some(TRACK_ENUM_CONSUMER),
        "write-track-contract-consumer" => Some(TRACK_CONTRACT_CONSUMER),
        "write-audio-frame-consumer" => Some(AUDIO_FRAME_CONSUMER),
        "write-audio-configuration-consumer" => Some(AUDIO_CONFIGURATION_CONSUMER),
        "write-frame-buffer-factory-consumer" => Some(FRAME_BUFFER_FACTORY_CONSUMER),
        "write-audio-frame-buffer-consumer" => Some(AUDIO_FRAME_BUFFER_CONSUMER),
        "write-audio-frame-rebuilder-consumer" => Some(AUDIO_FRAME_REBUILDER_CONSUMER),
        "write-audio-frame-provider-tools-consumer" => Some(AUDIO_FRAME_PROVIDER_TOOLS_CONSUMER),
        "write-audio-processing-context-consumer" => Some(AUDIO_PROCESSING_CONTEXT_CONSUMER),
        "write-audio-player-options-consumer" => Some(AUDIO_PLAYER_OPTIONS_CONSUMER),
        "write-decoded-track-holder-consumer" => Some(DECODED_TRACK_HOLDER_CONSUMER),
        "write-track-state-listener-consumer" => Some(TRACK_STATE_LISTENER_CONSUMER),
        "write-audio-output-hook-consumer" => Some(AUDIO_OUTPUT_HOOK_CONSUMER),
        "write-audio-load-result-handler-consumer" => Some(AUDIO_LOAD_RESULT_HANDLER_CONSUMER),
        "write-functional-result-handler-consumer" => Some(FUNCTIONAL_RESULT_HANDLER_CONSUMER),
        "write-audio-player-lifecycle-manager-consumer" => {
            Some(AUDIO_PLAYER_LIFECYCLE_MANAGER_CONSUMER)
        }
        "write-audio-player-interface-consumer" => Some(AUDIO_PLAYER_INTERFACE_CONSUMER),
        "write-audio-player-manager-interface-consumer" => {
            Some(AUDIO_PLAYER_MANAGER_INTERFACE_CONSUMER)
        }
        "write-default-audio-player-consumer" => Some(DEFAULT_AUDIO_PLAYER_CONSUMER),
        "write-default-audio-player-manager-consumer" => {
            Some(DEFAULT_AUDIO_PLAYER_MANAGER_CONSUMER)
        }
        "write-internal-audio-track-consumer" => Some(INTERNAL_AUDIO_TRACK_CONSUMER),
        "write-audio-track-executor-consumer" => Some(AUDIO_TRACK_EXECUTOR_CONSUMER),
        "write-local-audio-track-executor-callback-consumer" => {
            Some(LOCAL_AUDIO_TRACK_EXECUTOR_CALLBACK_CONSUMER)
        }
        "write-local-audio-track-executor-consumer" => Some(LOCAL_AUDIO_TRACK_EXECUTOR_CONSUMER),
        "write-track-marker-tracker-consumer" => Some(TRACK_MARKER_TRACKER_CONSUMER),
        "write-base-audio-track-consumer" => Some(BASE_AUDIO_TRACK_CONSUMER),
        "write-primordial-audio-track-executor-consumer" => {
            Some(PRIMORDIAL_AUDIO_TRACK_EXECUTOR_CONSUMER)
        }
        "write-delegated-audio-track-consumer" => Some(DELEGATED_AUDIO_TRACK_CONSUMER),
        "write-audio-track-info-builder-consumer" => Some(AUDIO_TRACK_INFO_BUILDER_CONSUMER),
        "write-abstract-audio-frame-buffer-consumer" => Some(ABSTRACT_AUDIO_FRAME_BUFFER_CONSUMER),
        "write-allocating-audio-frame-buffer-consumer" => {
            Some(ALLOCATING_AUDIO_FRAME_BUFFER_CONSUMER)
        }
        "write-non-allocating-audio-frame-buffer-consumer" => {
            Some(NON_ALLOCATING_AUDIO_FRAME_BUFFER_CONSUMER)
        }
        "write-audio-source-manager-interface-consumer" => {
            Some(AUDIO_SOURCE_MANAGER_INTERFACE_CONSUMER)
        }
        "write-audio-source-managers-consumer" => Some(AUDIO_SOURCE_MANAGERS_CONSUMER),
        "write-probing-audio-source-manager-consumer" => {
            Some(PROBING_AUDIO_SOURCE_MANAGER_CONSUMER)
        }
        "write-local-audio-source-manager-consumer" => Some(LOCAL_AUDIO_SOURCE_MANAGER_CONSUMER),
        "write-local-audio-track-consumer" => Some(LOCAL_AUDIO_TRACK_CONSUMER),
        "write-local-seekable-input-stream-consumer" => Some(LOCAL_SEEKABLE_INPUT_STREAM_CONSUMER),
        "write-heartbeating-http-stream-consumer" => Some(HEARTBEATING_HTTP_STREAM_CONSUMER),
        "write-nico-audio-source-manager-consumer" => Some(NICO_AUDIO_SOURCE_MANAGER_CONSUMER),
        "write-nico-audio-track-consumer" => Some(NICO_AUDIO_TRACK_CONSUMER),
        "write-terminator-audio-frame-consumer" => Some(TERMINATOR_AUDIO_FRAME_CONSUMER),
        "write-reference-mutable-audio-frame-consumer" => {
            Some(REFERENCE_MUTABLE_AUDIO_FRAME_CONSUMER)
        }
        _ => sound_cloud_consumer_source(command),
    }
}

#[allow(clippy::too_many_lines)]
fn sound_cloud_consumer_source(command: &str) -> Option<&'static str> {
    match command {
        "write-default-sound-cloud-data-loader-consumer" => {
            Some(DEFAULT_SOUND_CLOUD_DATA_LOADER_CONSUMER)
        }
        "write-default-sound-cloud-data-reader-consumer" => {
            Some(DEFAULT_SOUND_CLOUD_DATA_READER_CONSUMER)
        }
        "write-default-sound-cloud-format-handler-consumer" => {
            Some(DEFAULT_SOUND_CLOUD_FORMAT_HANDLER_CONSUMER)
        }
        "write-default-sound-cloud-playlist-loader-consumer" => {
            Some(DEFAULT_SOUND_CLOUD_PLAYLIST_LOADER_CONSUMER)
        }
        "write-default-sound-cloud-track-format-consumer" => {
            Some(DEFAULT_SOUND_CLOUD_TRACK_FORMAT_CONSUMER)
        }
        "write-sound-cloud-audio-source-manager-consumer" => {
            Some(SOUND_CLOUD_AUDIO_SOURCE_MANAGER_CONSUMER)
        }
        "write-sound-cloud-audio-source-manager-builder-consumer" => {
            Some(SOUND_CLOUD_AUDIO_SOURCE_MANAGER_BUILDER_CONSUMER)
        }
        "write-sound-cloud-audio-track-consumer" => Some(SOUND_CLOUD_AUDIO_TRACK_CONSUMER),
        "write-sound-cloud-client-id-tracker-consumer" => {
            Some(SOUND_CLOUD_CLIENT_ID_TRACKER_CONSUMER)
        }
        "write-sound-cloud-data-loader-consumer" => Some(SOUND_CLOUD_DATA_LOADER_CONSUMER),
        "write-sound-cloud-data-reader-consumer" => Some(SOUND_CLOUD_DATA_READER_CONSUMER),
        "write-sound-cloud-format-handler-consumer" => Some(SOUND_CLOUD_FORMAT_HANDLER_CONSUMER),
        "write-sound-cloud-helper-consumer" => Some(SOUND_CLOUD_HELPER_CONSUMER),
        "write-sound-cloud-http-context-filter-consumer" => {
            Some(SOUND_CLOUD_HTTP_CONTEXT_FILTER_CONSUMER)
        }
        "write-sound-cloud-m3u-audio-track-consumer" => Some(SOUND_CLOUD_M3U_AUDIO_TRACK_CONSUMER),
        "write-sound-cloud-m3u-info-consumer" => Some(SOUND_CLOUD_M3U_INFO_CONSUMER),
        "write-sound-cloud-mp3-segment-decoder-consumer" => {
            Some(SOUND_CLOUD_MP3_SEGMENT_DECODER_CONSUMER)
        }
        "write-sound-cloud-opus-segment-decoder-consumer" => {
            Some(SOUND_CLOUD_OPUS_SEGMENT_DECODER_CONSUMER)
        }
        "write-sound-cloud-playlist-loader-consumer" => Some(SOUND_CLOUD_PLAYLIST_LOADER_CONSUMER),
        "write-sound-cloud-segment-decoder-consumer" => Some(SOUND_CLOUD_SEGMENT_DECODER_CONSUMER),
        "write-sound-cloud-segment-decoder-factory-consumer" => {
            Some(SOUND_CLOUD_SEGMENT_DECODER_FACTORY_CONSUMER)
        }
        "write-sound-cloud-track-format-consumer" => Some(SOUND_CLOUD_TRACK_FORMAT_CONSUMER),
        "write-m3u-stream-audio-track-consumer" => Some(M3U_STREAM_AUDIO_TRACK_CONSUMER),
        "write-m3u-stream-segment-url-provider-consumer" => {
            Some(M3U_STREAM_SEGMENT_URL_PROVIDER_CONSUMER)
        }
        "write-mpeg-ts-m3u-stream-audio-track-consumer" => {
            Some(MPEG_TS_M3U_STREAM_AUDIO_TRACK_CONSUMER)
        }
        "write-twitch-constants-consumer" => Some(TWITCH_CONSTANTS_CONSUMER),
        "write-twitch-stream-audio-source-manager-consumer" => {
            Some(TWITCH_STREAM_AUDIO_SOURCE_MANAGER_CONSUMER)
        }
        "write-twitch-stream-audio-track-consumer" => Some(TWITCH_STREAM_AUDIO_TRACK_CONSUMER),
        "write-twitch-stream-segment-url-provider-consumer" => {
            Some(TWITCH_STREAM_SEGMENT_URL_PROVIDER_CONSUMER)
        }
        "write-vimeo-audio-source-manager-consumer" => Some(VIMEO_AUDIO_SOURCE_MANAGER_CONSUMER),
        "write-vimeo-playback-format-consumer" => Some(VIMEO_PLAYBACK_FORMAT_CONSUMER),
        "write-vimeo-audio-track-consumer" => Some(VIMEO_AUDIO_TRACK_CONSUMER),
        "write-abstract-yandex-music-api-loader-consumer" => {
            Some(ABSTRACT_YANDEX_MUSIC_API_LOADER_CONSUMER)
        }
        "write-yandex-music-api-extractor-consumer" => Some(YANDEX_MUSIC_API_EXTRACTOR_CONSUMER),
        "write-default-yandex-music-direct-url-loader-consumer" => {
            Some(DEFAULT_YANDEX_MUSIC_DIRECT_URL_LOADER_CONSUMER)
        }
        "write-default-yandex-music-playlist-loader-consumer" => {
            Some(DEFAULT_YANDEX_MUSIC_PLAYLIST_LOADER_CONSUMER)
        }
        "write-default-yandex-music-track-loader-consumer" => {
            Some(DEFAULT_YANDEX_MUSIC_TRACK_LOADER_CONSUMER)
        }
        "write-default-yandex-search-provider-consumer" => {
            Some(DEFAULT_YANDEX_SEARCH_PROVIDER_CONSUMER)
        }
        "write-yandex-http-context-filter-consumer" => Some(YANDEX_HTTP_CONTEXT_FILTER_CONSUMER),
        "write-yandex-music-api-loader-consumer" => Some(YANDEX_MUSIC_API_LOADER_CONSUMER),
        "write-yandex-music-audio-source-manager-consumer" => {
            Some(YANDEX_MUSIC_AUDIO_SOURCE_MANAGER_CONSUMER)
        }
        "write-yandex-music-audio-track-consumer" => Some(YANDEX_MUSIC_AUDIO_TRACK_CONSUMER),
        "write-yandex-music-direct-url-loader-consumer" => {
            Some(YANDEX_MUSIC_DIRECT_URL_LOADER_CONSUMER)
        }
        "write-yandex-music-playlist-loader-consumer" => {
            Some(YANDEX_MUSIC_PLAYLIST_LOADER_CONSUMER)
        }
        "write-yandex-music-search-result-loader-consumer" => {
            Some(YANDEX_MUSIC_SEARCH_RESULT_LOADER_CONSUMER)
        }
        "write-yandex-music-track-loader-consumer" => Some(YANDEX_MUSIC_TRACK_LOADER_CONSUMER),
        "write-yandex-music-utils-consumer" => Some(YANDEX_MUSIC_UTILS_CONSUMER),
        "write-default-youtube-link-router-consumer" => Some(DEFAULT_YOUTUBE_LINK_ROUTER_CONSUMER),
        "write-default-youtube-playlist-loader-consumer" => {
            Some(DEFAULT_YOUTUBE_PLAYLIST_LOADER_CONSUMER)
        }
        "write-default-youtube-track-details-consumer" => {
            Some(DEFAULT_YOUTUBE_TRACK_DETAILS_CONSUMER)
        }
        "write-default-youtube-track-details-loader-consumer" => {
            Some(DEFAULT_YOUTUBE_TRACK_DETAILS_LOADER_CONSUMER)
        }
        "write-youtube-cached-player-script-consumer" => {
            Some(YOUTUBE_CACHED_PLAYER_SCRIPT_CONSUMER)
        }
        "write-youtube-info-status-consumer" => Some(YOUTUBE_INFO_STATUS_CONSUMER),
        "write-youtube-access-token-tracker-consumer" => {
            Some(YOUTUBE_ACCESS_TOKEN_TRACKER_CONSUMER)
        }
        "write-youtube-cached-auth-script-consumer" => Some(YOUTUBE_CACHED_AUTH_SCRIPT_CONSUMER),
        "write-youtube-audio-source-manager-consumer" => {
            Some(YOUTUBE_AUDIO_SOURCE_MANAGER_CONSUMER)
        }
        _ => None,
    }
}

fn write_consumer(args: &[String], source: &str) -> Result<()> {
    let output = required_path(args, "--output")?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, source)?;
    Ok(())
}

const SMOKE_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.player.DefaultAudioPlayerManager;

public final class GateSmoke {
  public static void main(String[] args) {
    System.load(args[0]);
    DefaultAudioPlayerManager manager = new DefaultAudioPlayerManager();
    manager.shutdown();
    System.out.println("gate-smoke-ok");
  }
}
"#;

const EVENT_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.player.AudioPlayer;
import com.sedmelluq.discord.lavaplayer.player.event.AudioEvent;
import com.sedmelluq.discord.lavaplayer.player.event.AudioEventAdapter;
import com.sedmelluq.discord.lavaplayer.player.event.PlayerPauseEvent;
import com.sedmelluq.discord.lavaplayer.player.event.PlayerResumeEvent;
import com.sedmelluq.discord.lavaplayer.player.event.TrackEndEvent;
import com.sedmelluq.discord.lavaplayer.player.event.TrackExceptionEvent;
import com.sedmelluq.discord.lavaplayer.player.event.TrackStartEvent;
import com.sedmelluq.discord.lavaplayer.player.event.TrackStuckEvent;
import com.sedmelluq.discord.lavaplayer.tools.FriendlyException;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import java.lang.reflect.Field;
import java.lang.reflect.Proxy;

public final class GateEvents {
  private static AudioPlayer player;
  private static AudioTrack track;
  private static FriendlyException exception;
  private static StackTraceElement[] stackTrace;

  public static void main(String[] args) throws Exception {
    player = proxy(AudioPlayer.class);
    track = proxy(AudioTrack.class);
    exception = allocate(FriendlyException.class);
    stackTrace = new StackTraceElement[] {
      new StackTraceElement("GateEvents", "main", "GateEvents.java", 1)
    };

    PlayerPauseEvent pause = new PlayerPauseEvent(player);
    PlayerResumeEvent resume = new PlayerResumeEvent(player);
    TrackStartEvent start = new TrackStartEvent(player, track);
    TrackEndEvent end = new TrackEndEvent(player, track, null);
    TrackExceptionEvent failed = new TrackExceptionEvent(player, track, exception);
    TrackStuckEvent stuck = new TrackStuckEvent(player, track, 321L, stackTrace);
    check(pause.player == player && resume.player == player, "player fields");
    check(start.player == player && start.track == track, "start fields");
    check(end.player == player && end.track == track && end.endReason == null, "end fields");
    check(failed.player == player && failed.track == track && failed.exception == exception,
        "exception fields");
    check(stuck.player == player && stuck.track == track && stuck.thresholdMs == 321L
        && stuck.stackTrace == stackTrace, "stuck fields");

    StringBuilder observed = new StringBuilder();
    AudioEventAdapter adapter = new AudioEventAdapter() {
      public void onPlayerPause(AudioPlayer value) {
        check(value == player, "pause player"); observed.append("pause,");
      }
      public void onPlayerResume(AudioPlayer value) {
        check(value == player, "resume player"); observed.append("resume,");
      }
      public void onTrackStart(AudioPlayer value, AudioTrack item) {
        check(value == player && item == track, "start values"); observed.append("start,");
      }
      public void onTrackEnd(AudioPlayer value, AudioTrack item,
                             com.sedmelluq.discord.lavaplayer.track.AudioTrackEndReason reason) {
        check(value == player && item == track && reason == null, "end values");
        observed.append("end,");
      }
      public void onTrackException(AudioPlayer value, AudioTrack item, FriendlyException error) {
        check(value == player && item == track && error == exception, "exception values");
        observed.append("exception,");
      }
      public void onTrackStuck(AudioPlayer value, AudioTrack item, long threshold,
                               StackTraceElement[] trace) {
        check(value == player && item == track && threshold == 321L && trace == stackTrace,
            "stuck values");
        observed.append("stuck,");
      }
    };
    adapter.onEvent(pause);
    adapter.onEvent(resume);
    adapter.onEvent(start);
    adapter.onEvent(end);
    adapter.onEvent(failed);
    adapter.onEvent(stuck);
    adapter.onEvent(new AudioEvent(player) {});

    StringBuilder legacy = new StringBuilder();
    AudioEventAdapter legacyAdapter = new AudioEventAdapter() {
      public void onTrackStuck(AudioPlayer value, AudioTrack item, long threshold) {
        check(value == player && item == track && threshold == 654L, "legacy stuck values");
        legacy.append("legacy-stuck");
      }
    };
    legacyAdapter.onTrackStuck(player, track, 654L, stackTrace);
    new AudioEventAdapter() {}.onEvent(start);

    System.out.println(observed + "|" + legacy);
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type) {
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] { type },
        (instance, method, arguments) -> defaultValue(method.getReturnType()));
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == byte.class) return (byte) 0;
    if (type == short.class) return (short) 0;
    if (type == int.class) return 0;
    if (type == long.class) return 0L;
    if (type == float.class) return 0.0f;
    if (type == double.class) return 0.0d;
    if (type == char.class) return (char) 0;
    return null;
  }

  private static <T> T allocate(Class<T> type) throws Exception {
    Class<?> unsafeType = Class.forName("sun.misc.Unsafe");
    Field singleton = unsafeType.getDeclaredField("theUnsafe");
    singleton.setAccessible(true);
    Object unsafe = singleton.get(null);
    return type.cast(unsafeType.getMethod("allocateInstance", Class.class).invoke(unsafe, type));
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const TRACK_VALUE_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.container.MediaContainerDescriptor;
import com.sedmelluq.discord.lavaplayer.track.AudioItem;
import com.sedmelluq.discord.lavaplayer.track.AudioPlaylist;
import com.sedmelluq.discord.lavaplayer.track.AudioReference;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import com.sedmelluq.discord.lavaplayer.track.BasicAudioPlaylist;
import com.sedmelluq.discord.lavaplayer.track.TrackMarker;
import com.sedmelluq.discord.lavaplayer.track.TrackMarkerHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.List;

public final class GateTrackValues {
  public static void main(String[] args) throws Exception {
    MediaContainerDescriptor descriptor = new MediaContainerDescriptor(null, "container-params");
    AudioReference full = new AudioReference("identifier", "title", descriptor);
    check(full.identifier.equals("identifier"), "reference identifier field");
    check(full.title.equals("title"), "reference title field");
    check(full.containerDescriptor == descriptor, "reference descriptor identity");
    check(full.getIdentifier().equals("identifier") && full.getUri().equals("identifier"),
        "reference identifier accessors");
    check(full.getTitle().equals("title"), "reference title accessor");
    check(full.getAuthor() == null && full.getLength() == null && full.getArtworkUrl() == null
        && full.getISRC() == null, "reference nullable accessors");

    AudioReference shortReference = new AudioReference("short-id", "short-title");
    check(shortReference.containerDescriptor == null, "short reference descriptor");
    check(AudioReference.NO_TRACK != null && AudioReference.NO_TRACK == AudioReference.NO_TRACK,
        "no-track singleton");
    check(AudioReference.NO_TRACK.identifier == null && AudioReference.NO_TRACK.title == null
        && AudioReference.NO_TRACK.containerDescriptor == null, "no-track fields");

    AudioTrackInfo rich = new AudioTrackInfo(
        "track-title", "author", 123456789L, "track-id", true,
        "https://example.invalid/track", "https://example.invalid/art", "ISRC-1");
    check(rich.title.equals("track-title") && rich.author.equals("author"), "track info text");
    check(rich.length == 123456789L && rich.identifier.equals("track-id") && rich.isStream,
        "track info scalar fields");
    check(rich.uri.endsWith("/track") && rich.artworkUrl.endsWith("/art")
        && rich.isrc.equals("ISRC-1"), "track info optional fields");
    AudioTrackInfo shortInfo = new AudioTrackInfo(
        "short-track", "short-author", 7L, "short-track-id", false, null);
    check(shortInfo.uri == null && shortInfo.artworkUrl == null && shortInfo.isrc == null,
        "short track info defaults");

    AudioTrack track = proxy(AudioTrack.class);
    List<AudioTrack> tracks = new ArrayList<>();
    tracks.add(track);
    AudioPlaylist playlist = new BasicAudioPlaylist("playlist", tracks, track, true);
    check(playlist.getName().equals("playlist"), "playlist name");
    check(playlist.getTracks() == tracks && playlist.getSelectedTrack() == track,
        "playlist object identity");
    check(playlist.isSearchResult(), "playlist search flag");
    tracks.clear();
    check(playlist.getTracks().isEmpty(), "playlist retains caller list");
    check(playlist instanceof AudioItem, "playlist item inheritance");
    check(AudioPlaylist.class.isInterface()
        && AudioPlaylist.class.getDeclaredMethods().length == 4,
        "playlist interface structure");
    check(AudioPlaylist.class.getInterfaces().length == 1
        && AudioPlaylist.class.getInterfaces()[0] == AudioItem.class,
        "playlist interface parent");
    for (Method method : AudioPlaylist.class.getDeclaredMethods()) {
      check(Modifier.isPublic(method.getModifiers())
          && Modifier.isAbstract(method.getModifiers()) && !method.isDefault(),
          "playlist abstract method");
    }
    Method tracksMethod = AudioPlaylist.class.getMethod("getTracks");
    check(tracksMethod.getGenericReturnType().getTypeName().equals(
        "java.util.List<com.sedmelluq.discord.lavaplayer.track.AudioTrack>"),
        "playlist generic track list");

    TrackMarkerHandler.MarkerState[] handled = new TrackMarkerHandler.MarkerState[1];
    TrackMarkerHandler handler = state -> handled[0] = state;
    TrackMarker marker = new TrackMarker(987654321L, handler);
    check(marker.timecode == 987654321L && marker.handler == handler, "marker fields");
    handler.handle(TrackMarkerHandler.MarkerState.BYPASSED);
    check(handled[0] == TrackMarkerHandler.MarkerState.BYPASSED, "marker handler dispatch");
    check(TrackMarkerHandler.class.isInterface()
        && TrackMarkerHandler.class.getDeclaredMethods().length == 1,
        "marker handler structure");
    Method handleMethod = TrackMarkerHandler.class.getMethod(
        "handle", TrackMarkerHandler.MarkerState.class);
    check(Modifier.isPublic(handleMethod.getModifiers())
        && Modifier.isAbstract(handleMethod.getModifiers()) && !handleMethod.isDefault(),
        "marker handler method");
    check(handleMethod.getReturnType() == void.class
        && handleMethod.getParameterTypes()[0] == TrackMarkerHandler.MarkerState.class,
        "marker handler descriptor");
    check(TrackMarkerHandler.MarkerState.class.getDeclaringClass() == TrackMarkerHandler.class
        && Modifier.isStatic(TrackMarkerHandler.MarkerState.class.getModifiers()),
        "marker handler nested enum");

    System.out.println(
        "reference=identifier,title,container-params,null-defaults;"
        + "info=123456789,true,optional-fields;"
        + "playlist=identity,mutable,true;marker=987654321,identity;"
        + "playlist-contract=AudioItem,4,List<AudioTrack>;"
        + "marker-handler=BYPASSED,public-abstract,void(MarkerState),nested-static");
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type) {
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] { type },
        (instance, method, arguments) -> defaultValue(method.getReturnType()));
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == byte.class) return (byte) 0;
    if (type == short.class) return (short) 0;
    if (type == int.class) return 0;
    if (type == long.class) return 0L;
    if (type == float.class) return 0.0f;
    if (type == double.class) return 0.0d;
    if (type == char.class) return (char) 0;
    return null;
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const TRACK_ENUM_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.track.AudioTrackEndReason;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackState;
import com.sedmelluq.discord.lavaplayer.track.TrackMarkerHandler.MarkerState;
import java.util.Arrays;

public final class GateTrackEnums {
  public static void main(String[] args) {
    AudioTrackEndReason[] reasons = AudioTrackEndReason.values();
    check(Arrays.toString(reasons).equals(
        "[FINISHED, LOAD_FAILED, STOPPED, REPLACED, CLEANUP]"), "reason order");
    check(flags(reasons).equals("true,true,false,false,false"), "reason flags");
    check(reasons[0] == AudioTrackEndReason.FINISHED
        && reasons[1] == AudioTrackEndReason.valueOf("LOAD_FAILED"), "reason lookup");
    check(reasons[4].name().equals("CLEANUP") && reasons[4].ordinal() == 4,
        "reason inherited enum behavior");
    reasons[0] = null;
    check(AudioTrackEndReason.values()[0] == AudioTrackEndReason.FINISHED,
        "reason values copy");

    AudioTrackState[] states = AudioTrackState.values();
    check(Arrays.toString(states).equals(
        "[INACTIVE, LOADING, PLAYING, SEEKING, STOPPING, FINISHED]"), "state order");
    check(states[2] == AudioTrackState.valueOf("PLAYING") && states[5].ordinal() == 5,
        "state lookup");
    states[1] = null;
    check(AudioTrackState.values()[1] == AudioTrackState.LOADING, "state values copy");

    MarkerState[] markers = MarkerState.values();
    check(Arrays.toString(markers).equals(
        "[REACHED, REMOVED, OVERWRITTEN, BYPASSED, STOPPED, LATE, ENDED]"),
        "marker order");
    check(markers[2] == MarkerState.valueOf("OVERWRITTEN") && markers[6].ordinal() == 6,
        "marker lookup");
    markers[0] = null;
    check(MarkerState.values()[0] == MarkerState.REACHED, "marker values copy");

    expect(IllegalArgumentException.class, () -> AudioTrackState.valueOf("missing"));
    expect(NullPointerException.class, () -> MarkerState.valueOf(null));
    check(AudioTrackEndReason.class.getEnumConstants().length == 5, "reflection reasons");
    check(AudioTrackState.class.getEnumConstants().length == 6, "reflection states");
    check(MarkerState.class.getEnumConstants().length == 7, "reflection markers");

    System.out.println(
        "reasons=FINISHED,LOAD_FAILED,STOPPED,REPLACED,CLEANUP:true,true,false,false,false;"
        + "states=INACTIVE,LOADING,PLAYING,SEEKING,STOPPING,FINISHED;"
        + "markers=REACHED,REMOVED,OVERWRITTEN,BYPASSED,STOPPED,LATE,ENDED;"
        + "copy=true;lookup-errors=iae,npe;reflection=5,6,7");
  }

  private static String flags(AudioTrackEndReason[] values) {
    StringBuilder result = new StringBuilder();
    for (AudioTrackEndReason value : values) {
      if (result.length() > 0) result.append(',');
      result.append(value.mayStartNext);
    }
    return result.toString();
  }

  private static void expect(Class<? extends Throwable> type, Runnable operation) {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const TRACK_CONTRACT_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.track.AudioItem;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackState;
import com.sedmelluq.discord.lavaplayer.track.TrackMarker;
import com.sedmelluq.discord.lavaplayer.track.info.AudioTrackInfoProvider;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;

public final class GateTrackContracts {
  public static void main(String[] args) throws Exception {
    StubTrack track = new StubTrack();
    AudioItem item = track;
    check(item instanceof AudioTrack, "audio item marker interface");
    check(track.getInfo().identifier.equals("contract-id"), "track info linkage");
    check(track.getIdentifier().equals("contract-id"), "track identifier");
    check(track.getState() == AudioTrackState.INACTIVE, "track state");
    check(track.isSeekable() && track.getDuration() == 654321L, "track scalar methods");
    track.setPosition(321L);
    check(track.getPosition() == 321L, "track position");

    TrackMarker marker = new TrackMarker(17L, state -> { });
    track.setMarker(marker);
    track.addMarker(marker);
    track.removeMarker(marker);
    check(track.marker == marker && track.markerOperations.equals("set,add,remove"),
        "track marker methods");
    track.stop();
    check(track.stopped, "track stop");
    check(track.makeClone() == track && track.getSourceManager() == null,
        "track clone and source");

    StringBuilder userData = new StringBuilder("payload");
    track.setUserData(userData);
    check(track.getUserData() == userData, "untyped user data");
    check(track.getUserData(StringBuilder.class) == userData, "typed user data");
    check(track.getUserData(String.class) == null, "typed user data mismatch");

    AudioTrackInfoProvider provider = new StubInfoProvider();
    check(provider.getTitle().equals("title") && provider.getAuthor().equals("author"),
        "provider title and author");
    check(provider.getLength().equals(123L) && provider.getIdentifier().equals("provider-id"),
        "provider length and identifier");
    check(provider.getUri().equals("uri") && provider.getArtworkUrl().equals("art")
        && provider.getISRC().equals("isrc"), "provider optional metadata");

    check(AudioItem.class.isInterface() && AudioItem.class.getDeclaredMethods().length == 0,
        "audio item structure");
    check(AudioTrack.class.isInterface() && AudioTrack.class.getDeclaredMethods().length == 16,
        "audio track structure");
    check(AudioTrack.class.getInterfaces().length == 1
        && AudioTrack.class.getInterfaces()[0] == AudioItem.class, "audio track parent");
    check(AudioTrackInfoProvider.class.isInterface()
        && AudioTrackInfoProvider.class.getDeclaredMethods().length == 7,
        "info provider structure");
    check(allAbstract(AudioTrack.class) && allAbstract(AudioTrackInfoProvider.class),
        "abstract interface methods");
    Method typedUserData = AudioTrack.class.getMethod("getUserData", Class.class);
    check(typedUserData.getTypeParameters().length == 1
        && typedUserData.getGenericReturnType().getTypeName().equals("T")
        && typedUserData.getGenericParameterTypes()[0].getTypeName().equals("java.lang.Class<T>"),
        "typed user data signature");

    System.out.println(
        "track=info,id,INACTIVE,true,321,654321,set-add-remove,stopped,clone,userdata;"
        + "provider=title,author,123,provider-id,uri,art,isrc;"
        + "reflection=0,16,7,T,java.lang.Class<T>");
  }

  private static boolean allAbstract(Class<?> type) {
    for (Method method : type.getDeclaredMethods()) {
      if (!Modifier.isPublic(method.getModifiers())
          || !Modifier.isAbstract(method.getModifiers()) || method.isDefault()) return false;
    }
    return true;
  }

  private static final class StubTrack implements AudioTrack {
    private final AudioTrackInfo info = new AudioTrackInfo(
        "contract-title", "contract-author", 654321L, "contract-id", false, "contract-uri");
    private long position;
    private Object userData;
    private TrackMarker marker;
    private String markerOperations = "";
    private boolean stopped;

    public AudioTrackInfo getInfo() { return info; }
    public String getIdentifier() { return info.identifier; }
    public AudioTrackState getState() { return AudioTrackState.INACTIVE; }
    public void stop() { stopped = true; }
    public boolean isSeekable() { return true; }
    public long getPosition() { return position; }
    public void setPosition(long value) { position = value; }
    public void setMarker(TrackMarker value) { marker = value; record("set"); }
    public void addMarker(TrackMarker value) { marker = value; record("add"); }
    public void removeMarker(TrackMarker value) { marker = value; record("remove"); }
    public long getDuration() { return info.length; }
    public AudioTrack makeClone() { return this; }
    public AudioSourceManager getSourceManager() { return null; }
    public void setUserData(Object value) { userData = value; }
    public Object getUserData() { return userData; }
    public <T> T getUserData(Class<T> type) {
      return type.isInstance(userData) ? type.cast(userData) : null;
    }

    private void record(String operation) {
      if (!markerOperations.isEmpty()) markerOperations += ',';
      markerOperations += operation;
    }
  }

  private static final class StubInfoProvider implements AudioTrackInfoProvider {
    public String getTitle() { return "title"; }
    public String getAuthor() { return "author"; }
    public Long getLength() { return 123L; }
    public String getIdentifier() { return "provider-id"; }
    public String getUri() { return "uri"; }
    public String getArtworkUrl() { return "art"; }
    public String getISRC() { return "isrc"; }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const AUDIO_FRAME_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.track.playback.AbstractMutableAudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameProvider;
import com.sedmelluq.discord.lavaplayer.track.playback.ImmutableAudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.MutableAudioFrame;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.nio.ByteBuffer;
import java.util.Arrays;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;

public final class GateAudioFrames {
  public static void main(String[] args) throws Exception {
    byte[] data = new byte[] { 1, 2, 3 };
    ImmutableAudioFrame immutableValue = new ImmutableAudioFrame(42L, data, 77, null);
    AudioFrame immutable = immutableValue;
    check(immutable.getTimecode() == 42L && immutable.getVolume() == 77,
        "immutable scalar accessors");
    check(immutable.getDataLength() == 3 && immutable.getData() == data,
        "immutable data identity");
    check(immutable.getFormat() == null && !immutable.isTerminator(),
        "immutable format and terminator");
    check(immutableValue.timecode == 42L && immutableValue.data == data
        && immutableValue.volume == 77 && immutableValue.format == null,
        "immutable public fields");
    byte[] immutableCopy = new byte[5];
    immutable.getData(immutableCopy, 1);
    check(Arrays.equals(immutableCopy, new byte[] { 0, 1, 2, 3, 0 }),
        "immutable copy offset");
    data[0] = 9;
    check(immutable.getData()[0] == 9, "immutable retains source array");
    expect(ArrayIndexOutOfBoundsException.class,
        () -> immutable.getData(new byte[2], 0));
    expect(NullPointerException.class,
        () -> new ImmutableAudioFrame(0L, null, 0, null).getDataLength());

    ByteBuffer buffer = ByteBuffer.wrap(new byte[] { 9, 8, 7, 6, 5 });
    buffer.position(1);
    buffer.limit(4);
    MutableAudioFrame mutable = new MutableAudioFrame(buffer);
    check(mutable.getDataLength() == 3
        && Arrays.equals(mutable.getData(), new byte[] { 8, 7, 6 }),
        "mutable initial buffer window");
    check(buffer.position() == 1, "mutable read preserves buffer position");
    mutable.setTimecode(99L);
    mutable.setVolume(55);
    mutable.setFormat(null);
    mutable.setTerminator(true);
    check(mutable.getTimecode() == 99L && mutable.getVolume() == 55
        && mutable.getFormat() == null && mutable.isTerminator(),
        "mutable inherited state");
    mutable.store(new byte[] { 0, 2, 3, 4, 0 }, 1, 3);
    check(mutable.getDataLength() == 3
        && Arrays.equals(mutable.getData(), new byte[] { 2, 3, 4 }),
        "mutable store");
    check(buffer.position() == 4 && buffer.limit() == buffer.capacity(),
        "mutable store buffer state");
    byte[] mutableCopy = new byte[7];
    mutable.getData(mutableCopy, 2);
    check(Arrays.equals(mutableCopy, new byte[] { 0, 0, 2, 3, 4, 0, 0 })
        && buffer.position() == 4, "mutable copy offset and position");

    ImmutableAudioFrame frozen = mutable.freeze();
    check(frozen.getTimecode() == 99L && frozen.getVolume() == 55
        && Arrays.equals(frozen.getData(), new byte[] { 2, 3, 4 })
        && frozen.getFormat() == null && !frozen.isTerminator(), "mutable freeze");
    mutable.store(new byte[] { 5, 6, 7 }, 0, 3);
    check(Arrays.equals(frozen.getData(), new byte[] { 2, 3, 4 }),
        "freeze owns copied data");
    MutableAudioFrame empty = new MutableAudioFrame();
    check(empty.getDataLength() == 0 && empty.getTimecode() == 0L
        && empty.getVolume() == 0 && empty.getFormat() == null && !empty.isTerminator(),
        "mutable defaults");
    expect(NullPointerException.class, empty::getData);

    StubProvider implementation = new StubProvider(immutable);
    AudioFrameProvider provider = implementation;
    check(provider.provide() == immutable, "provider immediate frame");
    check(provider.provide(7L, TimeUnit.MILLISECONDS) == immutable
        && implementation.timeout == 7L && implementation.unit == TimeUnit.MILLISECONDS,
        "provider timed frame");
    check(provider.provide(mutable), "provider mutable frame");
    check(provider.provide(mutable, 8L, TimeUnit.SECONDS)
        && implementation.mutable == mutable && implementation.timeout == 8L
        && implementation.unit == TimeUnit.SECONDS, "provider timed mutable frame");

    checkInterface(AudioFrame.class, 7);
    checkInterface(AudioFrameProvider.class, 4);
    Method timedProvide = AudioFrameProvider.class.getMethod(
        "provide", long.class, TimeUnit.class);
    check(Arrays.equals(timedProvide.getExceptionTypes(),
        new Class<?>[] { TimeoutException.class, InterruptedException.class }),
        "provider checked exceptions");
    check(Modifier.isPublic(AbstractMutableAudioFrame.class.getModifiers())
        && Modifier.isAbstract(AbstractMutableAudioFrame.class.getModifiers())
        && AbstractMutableAudioFrame.class.getInterfaces().length == 1
        && AbstractMutableAudioFrame.class.getInterfaces()[0] == AudioFrame.class,
        "abstract mutable structure");
    check(AbstractMutableAudioFrame.class.getDeclaredMethods().length == 9
        && AbstractMutableAudioFrame.class.getConstructors().length == 1,
        "abstract mutable members");
    check(ImmutableAudioFrame.class.getDeclaredFields().length == 4
        && ImmutableAudioFrame.class.getDeclaredMethods().length == 7
        && ImmutableAudioFrame.class.getConstructors().length == 1,
        "immutable members");
    check(MutableAudioFrame.class.getSuperclass() == AbstractMutableAudioFrame.class
        && MutableAudioFrame.class.getDeclaredMethods().length == 5
        && MutableAudioFrame.class.getConstructors().length == 2,
        "mutable members");

    System.out.println(
        "immutable=42,77,identity,copy,false,exceptions;"
        + "mutable=window,state,store,position,freeze,defaults;"
        + "provider=immediate,timed,mutable,timed-mutable,exceptions;"
        + "reflection=7,4,9+1,4+7+1,5+2");
  }

  private static void checkInterface(Class<?> type, int methodCount) {
    check(type.isInterface() && type.getDeclaredMethods().length == methodCount,
        type.getName() + " structure");
    for (Method method : type.getDeclaredMethods()) {
      check(Modifier.isPublic(method.getModifiers())
          && Modifier.isAbstract(method.getModifiers()) && !method.isDefault(),
          type.getName() + " method " + method.getName());
    }
  }

  private static final class StubProvider implements AudioFrameProvider {
    private final AudioFrame frame;
    private MutableAudioFrame mutable;
    private long timeout;
    private TimeUnit unit;

    private StubProvider(AudioFrame frame) { this.frame = frame; }
    public AudioFrame provide() { return frame; }
    public AudioFrame provide(long value, TimeUnit valueUnit) {
      timeout = value;
      unit = valueUnit;
      return frame;
    }
    public boolean provide(MutableAudioFrame value) {
      mutable = value;
      return true;
    }
    public boolean provide(MutableAudioFrame value, long timeoutValue, TimeUnit valueUnit) {
      mutable = value;
      timeout = timeoutValue;
      unit = valueUnit;
      return true;
    }
  }

  private static void expect(Class<? extends Throwable> type, Runnable operation) {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const PROBE_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.player.DefaultAudioPlayerManager;
import java.lang.ref.PhantomReference;
import java.lang.ref.Reference;
import java.lang.ref.ReferenceQueue;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;

public final class GateProbe {
  private static Class<?> nativeClass;

  private static final class HandlePhantom extends PhantomReference<Object> {
    final long handle;
    HandlePhantom(Object referent, ReferenceQueue<Object> queue, long handle) {
      super(referent, queue);
      this.handle = handle;
    }
  }

  public static void main(String[] args) throws Exception {
    System.load(args[0]);
    nativeClass = Class.forName("dev.mantle.internal.MantleNative");
    switch (args[1]) {
      case "callbacks": callbacks(); break;
      case "lifetime": lifetime(); break;
      case "dispatcher-exit": dispatcherExit(); break;
      case "leak-manager": new DefaultAudioPlayerManager(); break;
      default: throw new IllegalArgumentException("unknown probe mode " + args[1]);
    }
  }

  private static Method method(String name, Class<?>... types) throws Exception {
    return nativeClass.getMethod(name, types);
  }

  private static Object invoke(String name, Class<?>[] types, Object... args) throws Exception {
    try {
      return method(name, types).invoke(null, args);
    } catch (InvocationTargetException error) {
      Throwable cause = error.getCause();
      if (cause instanceof Exception) throw (Exception) cause;
      throw error;
    }
  }

  private static void callbacks() throws Exception {
    final int warmup = 2_000;
    final int iterations = 50_000;
    invoke("resetCallbackExceptions", new Class<?>[0]);
    callbackRound(false, warmup);
    callbackRound(true, warmup);
    long dispatcherNanos = callbackRound(false, iterations);
    long directNanos = callbackRound(true, iterations);

    Runnable throwsOnCallback = () -> { throw new IllegalStateException("callback-spike"); };
    Thread dispatcher = new Thread(() -> {
      try {
        method("dispatchOnCurrentThread", Runnable.class, int.class)
            .invoke(null, throwsOnCallback, 3);
      } catch (InvocationTargetException expected) {
        // The JNI boundary must contain the exception; this Java worker owns policy.
      } catch (Exception error) {
        throw new RuntimeException(error);
      }
    }, "mantle-java-dispatcher-exception-probe");
    dispatcher.setDaemon(true);
    dispatcher.start();
    dispatcher.join(5_000);
    invoke("dispatchOnNativeDaemon", new Class<?>[] { Runnable.class, int.class }, throwsOnCallback, 3);
    long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5);
    int exceptions;
    do {
      exceptions = (Integer) invoke("callbackExceptions", new Class<?>[0]);
      if (exceptions >= 6) break;
      Thread.onSpinWait();
    } while (System.nanoTime() < deadline);
    if (exceptions != 6) throw new AssertionError("callback exceptions=" + exceptions);

    System.out.printf(
        "{\"probe\":\"callbacks\",\"iterations\":%d,\"dispatcher_ns_per_callback\":%.3f,\"direct_ns_per_callback\":%.3f,\"exceptions\":%d}%n",
        iterations,
        (double) dispatcherNanos / iterations,
        (double) directNanos / iterations,
        exceptions);
  }

  private static long callbackRound(boolean direct, int iterations) throws Exception {
    CountDownLatch complete = new CountDownLatch(iterations);
    AtomicInteger order = new AtomicInteger();
    AtomicReference<Throwable> failure = new AtomicReference<>();
    Object identity = new Object();
    Runnable callback = () -> {
      try {
        if (!Thread.currentThread().isDaemon()) throw new AssertionError("callback thread is not daemon");
        int sequence = order.getAndIncrement();
        if (sequence < 0 || sequence >= iterations) throw new AssertionError("callback order");
        Object returned = method("identity", Object.class).invoke(null, identity);
        if (returned != identity) throw new AssertionError("identity lost across callback");
      } catch (Throwable error) {
        failure.compareAndSet(null, error);
      } finally {
        complete.countDown();
      }
    };
    long start = System.nanoTime();
    if (direct) {
      Object spawned = invoke(
          "dispatchOnNativeDaemon", new Class<?>[] { Runnable.class, int.class }, callback, iterations);
      if (!Boolean.TRUE.equals(spawned)) throw new AssertionError("direct worker not spawned");
    } else {
      Thread thread = new Thread(() -> {
        try {
          method("dispatchOnCurrentThread", Runnable.class, int.class)
              .invoke(null, callback, iterations);
        } catch (Throwable error) {
          failure.compareAndSet(null, error);
        }
      }, "mantle-java-callback-dispatcher");
      thread.setDaemon(true);
      thread.start();
    }
    if (!complete.await(30, TimeUnit.SECONDS)) throw new AssertionError("callback timeout");
    long elapsed = System.nanoTime() - start;
    if (failure.get() != null) throw new AssertionError("callback failure", failure.get());
    if (order.get() != iterations) throw new AssertionError("callback count=" + order.get());
    return elapsed;
  }

  private static void dispatcherExit() throws Exception {
    CountDownLatch entered = new CountDownLatch(1);
    Runnable callback = () -> {
      entered.countDown();
      try {
        Thread.sleep(1);
      } catch (InterruptedException interrupted) {
        Thread.currentThread().interrupt();
      }
    };
    Thread dispatcher = new Thread(() -> {
      try {
        method("dispatchOnCurrentThread", Runnable.class, int.class)
            .invoke(null, callback, 1_000_000);
      } catch (Exception error) {
        throw new RuntimeException(error);
      }
    }, "mantle-java-dispatcher-exit-probe");
    dispatcher.setDaemon(true);
    dispatcher.start();
    if (!entered.await(5, TimeUnit.SECONDS)) throw new AssertionError("dispatcher did not start");
  }

  private static void lifetime() throws Exception {
    Class<?> probeClass = Class.forName("dev.mantle.internal.NativeHandleProbe");
    Method close = probeClass.getMethod("close");
    Method nativeHandle = probeClass.getMethod("nativeHandle");
    Method liveHandles = method("liveHandles");
    int baseline = (Integer) liveHandles.invoke(null);
    final int explicitCount = 1_000_000;
    long explicitStart = System.nanoTime();
    long stale = 0;
    for (int index = 0; index < explicitCount; index++) {
      Object probe = probeClass.getConstructor().newInstance();
      stale = (Long) nativeHandle.invoke(probe);
      close.invoke(probe);
      close.invoke(probe);
    }
    long explicitNanos = System.nanoTime() - explicitStart;
    Object current = probeClass.getConstructor().newInstance();
    long currentHandle = (Long) nativeHandle.invoke(current);
    if (stale == currentHandle) throw new AssertionError("generation did not change");
    try {
      method("validateHandle", long.class, int.class).invoke(null, stale, 5);
      throw new AssertionError("stale handle accepted");
    } catch (InvocationTargetException expected) {
      if (!(expected.getCause() instanceof RuntimeException)) throw expected;
    }
    try {
      method("validateHandle", long.class, int.class).invoke(null, currentHandle, 2);
      throw new AssertionError("wrong handle type accepted");
    } catch (InvocationTargetException expected) {
      if (!(expected.getCause() instanceof RuntimeException)) throw expected;
    }
    close.invoke(current);
    if ((Integer) liveHandles.invoke(null) != baseline) throw new AssertionError("explicit handle leak");

    final int gcCount = 100_000;
    long cleanerStart = System.nanoTime();
    for (int index = 0; index < gcCount; index++) probeClass.getConstructor().newInstance();
    awaitHandles(liveHandles, baseline, 30_000);
    long cleanerNanos = System.nanoTime() - cleanerStart;

    ReferenceQueue<Object> queue = new ReferenceQueue<>();
    List<HandlePhantom> references = new ArrayList<>();
    Object[] tokens = new Object[gcCount];
    Method createHandle = method("createHandle", int.class);
    Method release = method("release", long.class);
    long phantomStart = System.nanoTime();
    for (int index = 0; index < gcCount; index++) {
      Object token = new Object();
      tokens[index] = token;
      long handle = (Long) createHandle.invoke(null, 5);
      references.add(new HandlePhantom(token, queue, handle));
      token = null;
    }
    Arrays.fill(tokens, null);
    int released = 0;
    long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(30);
    while (released < gcCount && System.nanoTime() < deadline) {
      System.gc();
      HandlePhantom reference;
      while ((reference = (HandlePhantom) queue.poll()) != null) {
        release.invoke(null, reference.handle);
        released++;
      }
      Reference.reachabilityFence(references);
      Thread.sleep(2);
    }
    long phantomNanos = System.nanoTime() - phantomStart;
    if (released != gcCount) throw new AssertionError("phantom releases=" + released);
    if ((Integer) liveHandles.invoke(null) != baseline) throw new AssertionError("phantom handle leak");
    System.out.printf(
        "{\"probe\":\"lifetime\",\"explicit_wrappers\":%d,\"gc_wrappers\":%d,\"explicit_ns_per_wrapper\":%.3f,\"cleaner_ns_per_wrapper\":%.3f,\"phantom_ns_per_wrapper\":%.3f,\"live_handles\":%d}%n",
        explicitCount,
        gcCount,
        (double) explicitNanos / explicitCount,
        (double) cleanerNanos / gcCount,
        (double) phantomNanos / gcCount,
        (Integer) liveHandles.invoke(null));
  }

  private static void awaitHandles(Method liveHandles, int expected, long timeoutMillis) throws Exception {
    long deadline = System.nanoTime() + TimeUnit.MILLISECONDS.toNanos(timeoutMillis);
    int live;
    do {
      System.gc();
      Thread.sleep(5);
      live = (Integer) liveHandles.invoke(null);
      if (live == expected) return;
    } while (System.nanoTime() < deadline);
    throw new AssertionError("live handles=" + live + ", expected=" + expected);
  }
}
"#;

const INTEGRATION_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.player.AudioLoadResultHandler;
import com.sedmelluq.discord.lavaplayer.player.AudioPlayer;
import com.sedmelluq.discord.lavaplayer.player.DefaultAudioPlayerManager;
import com.sedmelluq.discord.lavaplayer.player.event.AudioEventListener;
import com.sedmelluq.discord.lavaplayer.player.event.TrackStartEvent;
import com.sedmelluq.discord.lavaplayer.tools.FriendlyException;
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.track.AudioItem;
import com.sedmelluq.discord.lavaplayer.track.AudioPlaylist;
import com.sedmelluq.discord.lavaplayer.track.AudioReference;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.TrackMarker;
import com.sedmelluq.discord.lavaplayer.track.TrackMarkerHandler.MarkerState;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrame;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;
import java.io.DataInput;
import java.io.DataOutput;
import java.lang.reflect.Method;

public final class GateIntegration {
  public static void main(String[] args) throws Exception {
    System.load(args[0]);
    DefaultAudioPlayerManager manager = new DefaultAudioPlayerManager();
    AudioPlayer player = manager.createPlayer();
    AtomicReference<AudioTrack> loaded = new AtomicReference<>();
    AtomicInteger starts = new AtomicInteger();
    AtomicInteger markers = new AtomicInteger();
    AtomicInteger sourceLoads = new AtomicInteger();
    AtomicInteger sourceShutdowns = new AtomicInteger();
    Object userData = new Object();
    AudioPlaylist registeredPlaylist = new AudioPlaylist() {
      public String getName() { return "JVM playlist"; }
      public List<AudioTrack> getTracks() { return Collections.singletonList(loaded.get()); }
      public AudioTrack getSelectedTrack() { return loaded.get(); }
      public boolean isSearchResult() { return false; }
    };

    AudioSourceManager[] sourceHolder = new AudioSourceManager[1];
    AudioSourceManager registeredSource = new AudioSourceManager() {
      public String getSourceName() { return "gate-jvm"; }
      public AudioItem loadItem(com.sedmelluq.discord.lavaplayer.player.AudioPlayerManager ignored,
                                AudioReference reference) {
        sourceLoads.incrementAndGet();
        if ("jvm:track".equals(reference.identifier)) {
          return new AudioReference("gate:track", "referred");
        }
        if ("jvm:direct".equals(reference.identifier)) return loaded.get();
        if ("jvm:playlist".equals(reference.identifier)) return registeredPlaylist;
        if ("jvm:reentrant".equals(reference.identifier)) {
          manager.registerSourceManager(sourceHolder[0]);
          return loaded.get();
        }
        if ("jvm:fail".equals(reference.identifier)) throw new IllegalStateException("gate failure");
        return null;
      }
      public boolean isTrackEncodable(AudioTrack track) { return track == loaded.get(); }
      public void encodeTrack(AudioTrack track, DataOutput output) throws java.io.IOException {
        output.writeUTF("jvm-v1");
      }
      public AudioTrack decodeTrack(com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo info,
                                    DataInput input) throws java.io.IOException {
        if (!"jvm-v1".equals(input.readUTF())) throw new java.io.IOException("wrong details");
        return loaded.get();
      }
      public void shutdown() { sourceShutdowns.incrementAndGet(); }
    };
    sourceHolder[0] = registeredSource;
    manager.registerSourceManager(registeredSource);
    if (manager.getSourceManagers().size() != 1
        || manager.getSourceManagers().get(0) != registeredSource) {
      throw new AssertionError("registered source visibility");
    }
    try {
      manager.getSourceManagers().clear();
      throw new AssertionError("registered source view is mutable");
    } catch (UnsupportedOperationException expected) {
      // The reference returns an unmodifiable live view.
    }
    @SuppressWarnings({"rawtypes", "unchecked"})
    AudioSourceManager foundSource = manager.source((Class) registeredSource.getClass());
    if (foundSource != registeredSource) throw new AssertionError("source class lookup");

    AudioEventListener[] listener = new AudioEventListener[1];
    listener[0] = event -> {
      if (!(event instanceof TrackStartEvent)) throw new AssertionError("wrong event " + event);
      TrackStartEvent start = (TrackStartEvent) event;
      if (start.track != loaded.get()) throw new AssertionError("event track identity");
      starts.incrementAndGet();
      player.removeListener(listener[0]);
      player.setPaused(true);
    };
    player.addListener(listener[0]);

    AudioLoadResultHandler handler = new AudioLoadResultHandler() {
      public void trackLoaded(AudioTrack track) {
        loaded.set(track);
        track.setUserData(userData);
        if (track.getUserData() != userData) throw new AssertionError("user data identity");
        if (track.getUserData(Object.class) != userData) throw new AssertionError("typed user data");
        if (track.getUserData(String.class) != null) throw new AssertionError("typed mismatch");
        track.setMarker(new TrackMarker(10, state -> {
          if (state != MarkerState.BYPASSED) throw new AssertionError("marker state " + state);
          markers.incrementAndGet();
        }));
        track.setPosition(10);
        player.playTrack(track);
      }
      public void playlistLoaded(AudioPlaylist playlist) { throw new AssertionError("playlist"); }
      public void noMatches() { throw new AssertionError("no matches"); }
      public void loadFailed(FriendlyException error) { throw new AssertionError(error); }
    };

    Future<Void> completed = manager.loadItem("jvm:track", handler);
    completed.get();
    if (!completed.isDone() || completed.isCancelled()) throw new AssertionError("future completion");
    if (loaded.get() == null || !"gate:track".equals(loaded.get().getIdentifier())
        || sourceLoads.get() != 1) {
      throw new AssertionError("track identifier:" + loaded.get() + ":" + sourceLoads.get());
    }
    if (starts.get() != 1 || !player.isPaused()) throw new AssertionError(
        "reentrant start callback:" + starts.get() + ":" + player.isPaused()
            + ":" + player.getClass().getName());
    if (markers.get() != 1) throw new AssertionError("marker callback");
    player.setPaused(false);

    AudioItem syncItem = manager.loadItemSync(new AudioReference("jvm:direct", null));
    if (syncItem != loaded.get() || sourceLoads.get() != 2) {
      throw new AssertionError("synchronous JVM source item");
    }
    AtomicReference<AudioPlaylist> syncPlaylist = new AtomicReference<>();
    manager.loadItemSync(new AudioReference("jvm:playlist", null), new AudioLoadResultHandler() {
      public void trackLoaded(AudioTrack track) { throw new AssertionError("track"); }
      public void playlistLoaded(AudioPlaylist value) { syncPlaylist.set(value); }
      public void noMatches() { throw new AssertionError("no matches"); }
      public void loadFailed(FriendlyException error) { throw new AssertionError(error); }
    });
    if (syncPlaylist.get() != registeredPlaylist || sourceLoads.get() != 3) {
      throw new AssertionError("synchronous JVM source callback");
    }

    manager.registerSourceManager(registeredSource);
    if (manager.getSourceManagers().size() != 2) {
      throw new AssertionError("late duplicate source registration");
    }

    AtomicReference<AudioTrack> direct = new AtomicReference<>();
    Future<Void> directFuture = manager.loadItem(
        new AudioReference("jvm:direct", null), new AudioLoadResultHandler() {
      public void trackLoaded(AudioTrack track) { direct.set(track); }
      public void playlistLoaded(AudioPlaylist playlist) { throw new AssertionError("playlist"); }
      public void noMatches() { throw new AssertionError("no matches"); }
      public void loadFailed(FriendlyException error) { throw new AssertionError(error); }
    });
    directFuture.get();
    if (direct.get() != loaded.get() || sourceLoads.get() != 4) {
      throw new AssertionError("direct JVM source item");
    }

    AtomicReference<AudioPlaylist> playlist = new AtomicReference<>();
    manager.loadItem("jvm:playlist", new AudioLoadResultHandler() {
      public void trackLoaded(AudioTrack track) { throw new AssertionError("track"); }
      public void playlistLoaded(AudioPlaylist value) { playlist.set(value); }
      public void noMatches() { throw new AssertionError("no matches"); }
      public void loadFailed(FriendlyException error) { throw new AssertionError(error); }
    }).get();
    if (playlist.get() != registeredPlaylist || sourceLoads.get() != 5) {
      throw new AssertionError("direct JVM playlist item");
    }

    AtomicInteger failures = new AtomicInteger();
    manager.loadItem("jvm:fail", new AudioLoadResultHandler() {
      public void trackLoaded(AudioTrack track) { throw new AssertionError("track"); }
      public void playlistLoaded(AudioPlaylist value) { throw new AssertionError("playlist"); }
      public void noMatches() { throw new AssertionError("no matches"); }
      public void loadFailed(FriendlyException error) { failures.incrementAndGet(); }
    }).get();
    if (failures.get() != 1 || sourceLoads.get() != 6) {
      throw new AssertionError("JVM source failure callback");
    }

    AtomicInteger noMatches = new AtomicInteger();
    manager.loadItem("jvm:missing", new AudioLoadResultHandler() {
      public void trackLoaded(AudioTrack track) { throw new AssertionError("track"); }
      public void playlistLoaded(AudioPlaylist value) { throw new AssertionError("playlist"); }
      public void noMatches() { noMatches.incrementAndGet(); }
      public void loadFailed(FriendlyException error) { throw new AssertionError(error); }
    }).get();
    if (noMatches.get() != 1) throw new AssertionError("JVM source no-match callback");

    AtomicReference<AudioTrack> reentrant = new AtomicReference<>();
    manager.loadItem("jvm:reentrant", new AudioLoadResultHandler() {
      public void trackLoaded(AudioTrack track) { reentrant.set(track); }
      public void playlistLoaded(AudioPlaylist value) { throw new AssertionError("playlist"); }
      public void noMatches() { throw new AssertionError("no matches"); }
      public void loadFailed(FriendlyException error) { throw new AssertionError(error); }
    }).get();
    if (reentrant.get() != loaded.get() || manager.getSourceManagers().size() != 3) {
      throw new AssertionError("reentrant JVM source registration");
    }

    byte[] details = manager.encodeTrackDetails(direct.get());
    byte[] expectedDetails = new byte[] {
        0, 8, 'g', 'a', 't', 'e', '-', 'j', 'v', 'm',
        0, 6, 'j', 'v', 'm', '-', 'v', '1'
    };
    if (!Arrays.equals(details, expectedDetails)) throw new AssertionError("track detail bytes");
    AudioTrack decoded = manager.decodeTrackDetails(loaded.get().getInfo(), details);
    if (!loaded.get().getIdentifier().equals(decoded.getIdentifier())
        || !loaded.get().getInfo().title.equals(decoded.getInfo().title)) {
      throw new AssertionError("decoded track metadata");
    }
    try {
      manager.decodeTrackDetails(loaded.get().getInfo(), Arrays.copyOf(details, 1));
      throw new AssertionError("truncated track details accepted");
    } catch (RuntimeException expected) {
      // Malformed compatibility input must fail without crossing JNI as a Rust panic.
    }

    Class<?> nativeClass = Class.forName("dev.mantle.internal.MantleNative");
    Method liveHandles = nativeClass.getMethod("liveHandles");
    Method trackedSourceItems = nativeClass.getMethod("trackedSourceItemCount");
    int handleBaseline = (Integer) liveHandles.invoke(null);
    int trackedBaseline = (Integer) trackedSourceItems.invoke(null);
    final int youtubeResourceRounds = 5_000;
    for (int batch = 0; batch < 20; batch++) {
      youtubeSerializationBatch(manager, 250);
      awaitNativeSourceResources(
          liveHandles, trackedSourceItems, handleBaseline, trackedBaseline, 10_000);
    }
    final int yandexResourceRounds = 5_000;
    for (int batch = 0; batch < 20; batch++) {
      yandexSerializationBatch(manager, 250);
      awaitNativeSourceResources(
          liveHandles, trackedSourceItems, handleBaseline, trackedBaseline, 10_000);
    }

    AudioItem youtubeItem = manager.loadItemSync(
        new AudioReference("gate:youtube-track", null));
    if (!(youtubeItem instanceof AudioTrack)) throw new AssertionError("YouTube track result");
    AudioTrack youtubeTrack = (AudioTrack) youtubeItem;
    if (!"dQw4w9WgXcQ".equals(youtubeTrack.getIdentifier())
        || !"YouTube fixture".equals(youtubeTrack.getInfo().title)
        || !"Fixture channel".equals(youtubeTrack.getInfo().author)
        || youtubeTrack.getInfo().length != 213_000L
        || youtubeTrack.getInfo().isStream
        || !"https://www.youtube.com/watch?v=dQw4w9WgXcQ".equals(youtubeTrack.getInfo().uri)
        || !"https://i.ytimg.com/fixture.jpg".equals(youtubeTrack.getInfo().artworkUrl)) {
      throw new AssertionError("YouTube track metadata");
    }
    byte[] youtubeDetails = manager.encodeTrackDetails(youtubeTrack);
    byte[] expectedYoutubeDetails = new byte[] {
        0, 7, 'y', 'o', 'u', 't', 'u', 'b', 'e'
    };
    if (!Arrays.equals(youtubeDetails, expectedYoutubeDetails)) {
      throw new AssertionError("YouTube detail bytes");
    }
    AudioTrack decodedYoutube = manager.decodeTrackDetails(
        youtubeTrack.getInfo(), youtubeDetails);
    if (!youtubeTrack.getIdentifier().equals(decodedYoutube.getIdentifier())
        || !youtubeTrack.getInfo().title.equals(decodedYoutube.getInfo().title)) {
      throw new AssertionError("decoded YouTube metadata");
    }

    AudioItem youtubePlaylistItem = manager.loadItemSync(
        new AudioReference("gate:youtube-playlist", null));
    if (!(youtubePlaylistItem instanceof AudioPlaylist)) {
      throw new AssertionError("YouTube playlist result");
    }
    AudioPlaylist youtubePlaylist = (AudioPlaylist) youtubePlaylistItem;
    if (!"YouTube fixture playlist".equals(youtubePlaylist.getName())
        || youtubePlaylist.isSearchResult()
        || youtubePlaylist.getTracks().size() != 2
        || youtubePlaylist.getSelectedTrack() != youtubePlaylist.getTracks().get(1)) {
      throw new AssertionError("YouTube playlist shape");
    }
    if (!Arrays.equals(
        manager.encodeTrackDetails(youtubePlaylist.getTracks().get(0)),
        expectedYoutubeDetails)) {
      throw new AssertionError("YouTube playlist member ownership");
    }

    AudioPlaylist youtubeSearch = (AudioPlaylist) manager.loadItemSync(
        new AudioReference("gate:youtube-search", null));
    if (!youtubeSearch.isSearchResult() || youtubeSearch.getSelectedTrack() != null
        || youtubeSearch.getTracks().size() != 2) {
      throw new AssertionError("YouTube search result shape");
    }

    AudioItem yandexItem = manager.loadItemSync(
        new AudioReference("gate:yandex-track", null));
    if (!(yandexItem instanceof AudioTrack)) throw new AssertionError("Yandex track result");
    AudioTrack yandexTrack = (AudioTrack) yandexItem;
    if (!"71663565".equals(yandexTrack.getIdentifier())
        || !"Animals".equals(yandexTrack.getInfo().title)
        || !"Architects".equals(yandexTrack.getInfo().author)
        || yandexTrack.getInfo().length != 244_321L
        || yandexTrack.getInfo().isStream
        || !"https://music.yandex.ru/album/1/track/71663565".equals(yandexTrack.getInfo().uri)
        || !"https://avatars.yandex.net/get-music-content/fixture/400x400".equals(
            yandexTrack.getInfo().artworkUrl)) {
      throw new AssertionError("Yandex track metadata");
    }
    byte[] yandexDetails = manager.encodeTrackDetails(yandexTrack);
    byte[] expectedYandexDetails = new byte[] {
        0, 12, 'y', 'a', 'n', 'd', 'e', 'x', '-', 'm', 'u', 's', 'i', 'c'
    };
    if (!Arrays.equals(yandexDetails, expectedYandexDetails)) {
      throw new AssertionError("Yandex detail bytes");
    }
    AudioTrack decodedYandex = manager.decodeTrackDetails(
        yandexTrack.getInfo(), yandexDetails);
    if (!yandexTrack.getIdentifier().equals(decodedYandex.getIdentifier())
        || !yandexTrack.getInfo().title.equals(decodedYandex.getInfo().title)) {
      throw new AssertionError("decoded Yandex metadata");
    }

    AudioItem yandexPlaylistItem = manager.loadItemSync(
        new AudioReference("gate:yandex-playlist", null));
    if (!(yandexPlaylistItem instanceof AudioPlaylist)) {
      throw new AssertionError("Yandex playlist result");
    }
    AudioPlaylist yandexPlaylist = (AudioPlaylist) yandexPlaylistItem;
    if (!"Yandex fixture playlist".equals(yandexPlaylist.getName())
        || yandexPlaylist.isSearchResult()
        || yandexPlaylist.getTracks().size() != 2
        || yandexPlaylist.getSelectedTrack() != null) {
      throw new AssertionError("Yandex playlist shape");
    }
    if (!Arrays.equals(
        manager.encodeTrackDetails(yandexPlaylist.getTracks().get(0)),
        expectedYandexDetails)) {
      throw new AssertionError("Yandex playlist member ownership");
    }

    AudioPlaylist yandexSearch = (AudioPlaylist) manager.loadItemSync(
        new AudioReference("gate:yandex-search", null));
    if (!yandexSearch.isSearchResult() || yandexSearch.getSelectedTrack() != null
        || yandexSearch.getTracks().size() != 2) {
      throw new AssertionError("Yandex search result shape");
    }

    AudioFrame frame = player.provide();
    if (frame == null || frame.getTimecode() != 0 || frame.getVolume() != 100
        || frame.getDataLength() != 4 || !Arrays.equals(frame.getData(), new byte[] {1, 2, 3, 4})) {
      throw new AssertionError("frame behavior");
    }
    if (player.provide() != null) throw new AssertionError("frame should be consumed");

    Future<Void> pending = manager.loadItemOrdered(
        new String("gate-key"), new AudioReference("gate:pending", null), handler);
    if (pending.isDone()) throw new AssertionError("pending future completed");
    if (!pending.cancel(true) || !pending.isCancelled()) throw new AssertionError("future cancellation");
    Method orderingKeyCount = nativeClass.getMethod("orderingKeyCount");
    long keyDeadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5);
    while ((Integer) orderingKeyCount.invoke(null) != 0 && System.nanoTime() < keyDeadline) {
      Thread.sleep(1);
    }
    if ((Integer) orderingKeyCount.invoke(null) != 0) throw new AssertionError("ordering-key leak");

    List<Integer> orderedCallbacks = Collections.synchronizedList(new ArrayList<>());
    List<String> callbackThreads = Collections.synchronizedList(new ArrayList<>());
    @SuppressWarnings("unchecked")
    Future<Void>[] ordered = new Future[16];
    Object sharedKey = new String("shared-key");
    for (int index = 0; index < ordered.length; index++) {
      final int callbackIndex = index;
      ordered[index] = manager.loadItemOrdered(sharedKey, "gate:track", new AudioLoadResultHandler() {
        public void trackLoaded(AudioTrack track) {
          orderedCallbacks.add(callbackIndex);
          callbackThreads.add(Thread.currentThread().getName());
        }
        public void playlistLoaded(AudioPlaylist playlist) { throw new AssertionError("playlist"); }
        public void noMatches() { throw new AssertionError("no matches"); }
        public void loadFailed(FriendlyException error) { throw new AssertionError(error); }
      });
    }
    for (Future<Void> orderedFuture : ordered) orderedFuture.get();
    for (int index = 0; index < ordered.length; index++) {
      if (orderedCallbacks.get(index) != index) throw new AssertionError("ordered callback FIFO");
      if (callbackThreads.get(index).startsWith("mantle-info-loader-")) {
        throw new AssertionError("callback ran on native loader");
      }
    }
    long orderedDeadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5);
    while ((Integer) orderingKeyCount.invoke(null) != 0 && System.nanoTime() < orderedDeadline) {
      Thread.sleep(1);
    }
    if ((Integer) orderingKeyCount.invoke(null) != 0) throw new AssertionError("ordered key leak");

    loaded.get().stop();
    decoded.stop();
    youtubeTrack.stop();
    decodedYoutube.stop();
    for (AudioTrack track : youtubePlaylist.getTracks()) track.stop();
    for (AudioTrack track : youtubeSearch.getTracks()) track.stop();
    yandexTrack.stop();
    decodedYandex.stop();
    for (AudioTrack track : yandexPlaylist.getTracks()) track.stop();
    for (AudioTrack track : yandexSearch.getTracks()) track.stop();
    player.destroy();
    manager.shutdown();
    if (sourceShutdowns.get() != 3) throw new AssertionError("source shutdown count");
    System.out.printf(
        "{\"probe\":\"integration\",\"starts\":%d,\"markers\":%d,\"serialization\":true,\"youtube_results\":true,\"youtube_resource_rounds\":%d,\"yandex_results\":true,\"yandex_resource_rounds\":%d,\"future_complete\":true,\"future_cancel\":true,\"ordered_callbacks\":%d}%n",
        starts.get(), markers.get(), youtubeResourceRounds, yandexResourceRounds,
        orderedCallbacks.size());
  }

  private static void youtubeSerializationBatch(
      DefaultAudioPlayerManager manager, int count) throws Exception {
    byte[] expected = new byte[] {0, 7, 'y', 'o', 'u', 't', 'u', 'b', 'e'};
    for (int index = 0; index < count; index++) {
      AudioTrack track = (AudioTrack) manager.loadItemSync(
          new AudioReference("gate:youtube-track", null));
      byte[] details = manager.encodeTrackDetails(track);
      if (!Arrays.equals(details, expected)) throw new AssertionError("YouTube resource details");
      AudioTrack decoded = manager.decodeTrackDetails(track.getInfo(), details);
      track.stop();
      decoded.stop();
    }
  }

  private static void yandexSerializationBatch(
      DefaultAudioPlayerManager manager, int count) throws Exception {
    byte[] expected = new byte[] {
        0, 12, 'y', 'a', 'n', 'd', 'e', 'x', '-', 'm', 'u', 's', 'i', 'c'
    };
    for (int index = 0; index < count; index++) {
      AudioTrack track = (AudioTrack) manager.loadItemSync(
          new AudioReference("gate:yandex-track", null));
      byte[] details = manager.encodeTrackDetails(track);
      if (!Arrays.equals(details, expected)) throw new AssertionError("Yandex resource details");
      AudioTrack decoded = manager.decodeTrackDetails(track.getInfo(), details);
      track.stop();
      decoded.stop();
    }
  }

  private static void awaitNativeSourceResources(
      Method liveHandles, Method trackedSourceItems, int expectedHandles, int expectedTracked,
      long timeoutMillis) throws Exception {
    long deadline = System.nanoTime() + TimeUnit.MILLISECONDS.toNanos(timeoutMillis);
    int handles;
    int tracked;
    do {
      System.gc();
      Thread.sleep(2);
      handles = (Integer) liveHandles.invoke(null);
      tracked = (Integer) trackedSourceItems.invoke(null);
      if (handles == expectedHandles && tracked <= expectedTracked) return;
    } while (System.nanoTime() < deadline);
    throw new AssertionError(
        "native source resource leak handles=" + handles + ", tracked=" + tracked);
  }
}
"#;

const AUDIO_CONFIGURATION_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.format.AudioDataFormat;
import com.sedmelluq.discord.lavaplayer.player.AudioConfiguration;
import com.sedmelluq.discord.lavaplayer.player.AudioConfiguration.ResamplingQuality;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameBuffer;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameBufferFactory;
import java.lang.reflect.Modifier;
import java.util.Arrays;
import java.util.concurrent.atomic.AtomicBoolean;

public final class GateAudioConfiguration {
  public static void main(String[] args) {
    AudioConfiguration configuration = new AudioConfiguration();
    check(AudioConfiguration.OPUS_QUALITY_MAX == 10, "opus max constant");
    check(configuration.getResamplingQuality() == ResamplingQuality.LOW,
        "default resampling quality");
    check(configuration.getOpusEncodingQuality() == 10,
        "default opus quality");
    AudioDataFormat format = configuration.getOutputFormat();
    check(format != null && format.channelCount == 2 && format.sampleRate == 48000
        && format.chunkSampleCount == 960 && "OPUS".equals(format.codecName()),
        "default output format");
    check(!configuration.isFilterHotSwapEnabled(), "default hot swap");
    check(configuration.getFrameBufferFactory() != null, "default frame factory");

    configuration.setResamplingQuality(ResamplingQuality.HIGH);
    check(configuration.getResamplingQuality() == ResamplingQuality.HIGH,
        "resampling setter");
    configuration.setResamplingQuality(null);
    check(configuration.getResamplingQuality() == null, "null resampling setter");
    configuration.setOpusEncodingQuality(-1);
    check(configuration.getOpusEncodingQuality() == 0, "low opus clamp");
    configuration.setOpusEncodingQuality(7);
    check(configuration.getOpusEncodingQuality() == 7, "middle opus quality");
    configuration.setOpusEncodingQuality(99);
    check(configuration.getOpusEncodingQuality() == 10, "high opus clamp");
    configuration.setOutputFormat(null);
    check(configuration.getOutputFormat() == null, "null output setter");
    configuration.setFilterHotSwapEnabled(true);
    check(configuration.isFilterHotSwapEnabled(), "hot swap setter");

    AudioFrameBufferFactory factory = new AudioFrameBufferFactory() {
      public AudioFrameBuffer create(int duration, AudioDataFormat dataFormat,
          AtomicBoolean stopping) {
        return null;
      }
    };
    configuration.setFrameBufferFactory(factory);
    check(configuration.getFrameBufferFactory() == factory, "factory setter");

    configuration.setResamplingQuality(ResamplingQuality.MEDIUM);
    configuration.setOpusEncodingQuality(6);
    AudioConfiguration copy = configuration.copy();
    check(copy != configuration && copy.getResamplingQuality() == ResamplingQuality.MEDIUM
        && copy.getOpusEncodingQuality() == 6 && copy.getOutputFormat() == null
        && copy.isFilterHotSwapEnabled() && copy.getFrameBufferFactory() == factory,
        "copy state");
    configuration.setResamplingQuality(ResamplingQuality.LOW);
    configuration.setOpusEncodingQuality(2);
    configuration.setFilterHotSwapEnabled(false);
    configuration.setFrameBufferFactory(null);
    check(copy.getResamplingQuality() == ResamplingQuality.MEDIUM
        && copy.getOpusEncodingQuality() == 6 && copy.isFilterHotSwapEnabled()
        && copy.getFrameBufferFactory() == factory, "copy independence");

    ResamplingQuality[] values = ResamplingQuality.values();
    check(Arrays.equals(values, new ResamplingQuality[] {
        ResamplingQuality.HIGH, ResamplingQuality.MEDIUM, ResamplingQuality.LOW }),
        "enum order");
    values[0] = ResamplingQuality.LOW;
    check(ResamplingQuality.values()[0] == ResamplingQuality.HIGH, "enum values copy");
    check(ResamplingQuality.valueOf("MEDIUM") == ResamplingQuality.MEDIUM
        && ResamplingQuality.HIGH.ordinal() == 0
        && ResamplingQuality.LOW.ordinal() == 2, "enum lookup and ordinals");
    expect(IllegalArgumentException.class, () -> ResamplingQuality.valueOf("missing"));
    expect(NullPointerException.class, () -> ResamplingQuality.valueOf(null));
    check(ResamplingQuality.class.isEnum()
        && ResamplingQuality.class.getEnumConstants().length == 3,
        "enum reflection");
    check(Modifier.isPublic(AudioConfiguration.class.getModifiers())
        && AudioConfiguration.class.getDeclaredMethods().length == 11
        && AudioConfiguration.class.getConstructors().length == 1,
        "configuration reflection");

    System.out.println(
        "defaults=LOW,10,OPUS,2x48000x960,false,factory;"
        + "mutation=null,clamp,format,hot-swap,factory;copy=independent;"
        + "enum=HIGH,MEDIUM,LOW,lookup-errors,reflection;surface=11+1");
  }

  private static void expect(Class<? extends Throwable> type, Runnable operation) {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const FRAME_BUFFER_FACTORY_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.format.AudioDataFormat;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameBuffer;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameBufferFactory;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;
import java.util.concurrent.atomic.AtomicBoolean;

public final class GateFrameBufferFactory {
  private static int duration;
  private static AudioDataFormat format;
  private static AtomicBoolean stopping;

  public static void main(String[] args) throws Exception {
    AudioDataFormat expectedFormat = null;
    AtomicBoolean expectedStopping = new AtomicBoolean(true);
    AudioFrameBufferFactory factory = new AudioFrameBufferFactory() {
      public AudioFrameBuffer create(int value, AudioDataFormat valueFormat,
          AtomicBoolean valueStopping) {
        duration = value;
        format = valueFormat;
        stopping = valueStopping;
        return null;
      }
    };
    check(factory.create(2_500, expectedFormat, expectedStopping) == null,
        "factory return dispatch");
    check(duration == 2_500 && format == expectedFormat && stopping == expectedStopping,
        "factory argument dispatch");

    check(AudioFrameBufferFactory.class.isInterface()
        && Modifier.isPublic(AudioFrameBufferFactory.class.getModifiers())
        && Modifier.isAbstract(AudioFrameBufferFactory.class.getModifiers())
        && AudioFrameBufferFactory.class.getInterfaces().length == 0
        && AudioFrameBufferFactory.class.getDeclaredFields().length == 0
        && AudioFrameBufferFactory.class.getDeclaredMethods().length == 1,
        "factory structure");
    Method create = AudioFrameBufferFactory.class.getDeclaredMethod(
        "create", int.class, AudioDataFormat.class, AtomicBoolean.class);
    check(Modifier.isPublic(create.getModifiers()) && Modifier.isAbstract(create.getModifiers())
        && !create.isDefault() && !create.isBridge() && !create.isSynthetic()
        && create.getReturnType() == AudioFrameBuffer.class
        && Arrays.equals(create.getParameterTypes(), new Class<?>[] {
            int.class, AudioDataFormat.class, AtomicBoolean.class })
        && create.getExceptionTypes().length == 0
        && create.getTypeParameters().length == 0,
        "factory method metadata");

    System.out.println(
        "dispatch=duration,format-identity,stopping-identity,null-return;"
        + "reflection=public-abstract-interface,0-fields,1-method,0-exceptions");
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const AUDIO_FRAME_BUFFER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameBuffer;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameConsumer;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameProvider;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameRebuilder;
import com.sedmelluq.discord.lavaplayer.track.playback.ImmutableAudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.MutableAudioFrame;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;

public final class GateAudioFrameBuffer {
  public static void main(String[] args) throws Exception {
    AudioFrame frame = new ImmutableAudioFrame(42L, new byte[] { 1 }, 100, null);
    StubBuffer implementation = new StubBuffer(frame);
    AudioFrameBuffer buffer = implementation;
    check(buffer.getRemainingCapacity() == 7 && buffer.getFullCapacity() == 9,
        "capacity dispatch");
    buffer.waitForTermination();
    buffer.setTerminateOnEmpty();
    buffer.setClearOnInsert();
    check(buffer.hasClearOnInsert(), "clear-on-insert dispatch");
    buffer.clear();
    buffer.lockBuffer();
    check(buffer.hasReceivedFrames() && buffer.getLastInputTimecode().equals(42L),
        "buffer state dispatch");
    buffer.consume(frame);
    buffer.rebuild(null);
    check(implementation.consumed == frame && implementation.rebuilder == null,
        "consumer dispatch");
    check(buffer.provide() == frame
        && buffer.provide(3L, TimeUnit.MILLISECONDS) == frame,
        "provider frame dispatch");
    MutableAudioFrame mutable = new MutableAudioFrame();
    check(buffer.provide(mutable)
        && buffer.provide(mutable, 4L, TimeUnit.SECONDS)
        && implementation.mutable == mutable && implementation.timeout == 4L
        && implementation.unit == TimeUnit.SECONDS, "provider mutable dispatch");
    check("wait,terminate,clear-on-insert,clear,lock,consume,rebuild".equals(
        implementation.operations), "operation order");

    checkInterface(AudioFrameConsumer.class, 2);
    checkInterface(AudioFrameBuffer.class, 10);
    check(Arrays.equals(AudioFrameBuffer.class.getInterfaces(), new Class<?>[] {
        AudioFrameProvider.class, AudioFrameConsumer.class }), "buffer inheritance");
    Method consume = AudioFrameConsumer.class.getMethod("consume", AudioFrame.class);
    Method rebuild = AudioFrameConsumer.class.getMethod("rebuild", AudioFrameRebuilder.class);
    Method waitForTermination = AudioFrameBuffer.class.getMethod("waitForTermination");
    check(Arrays.equals(consume.getExceptionTypes(), new Class<?>[] {
        InterruptedException.class }) && rebuild.getExceptionTypes().length == 0
        && Arrays.equals(waitForTermination.getExceptionTypes(), new Class<?>[] {
            InterruptedException.class }), "checked exceptions");
    check(AudioFrameBuffer.class.getMethods().length == 16,
        "declared and inherited method count");

    System.out.println(
        "dispatch=capacity,lifecycle,consumer,provider,order;"
        + "reflection=consumer-2,buffer-10,inherited-16,exceptions");
  }

  private static void checkInterface(Class<?> type, int methodCount) {
    check(type.isInterface() && Modifier.isPublic(type.getModifiers())
        && Modifier.isAbstract(type.getModifiers())
        && type.getDeclaredFields().length == 0
        && type.getDeclaredMethods().length == methodCount,
        type.getName() + " structure");
    for (Method method : type.getDeclaredMethods()) {
      check(Modifier.isPublic(method.getModifiers())
          && Modifier.isAbstract(method.getModifiers()) && !method.isDefault()
          && !method.isBridge() && !method.isSynthetic(),
          type.getName() + " method " + method.getName());
    }
  }

  private static final class StubBuffer implements AudioFrameBuffer {
    private final AudioFrame frame;
    private String operations = "";
    private AudioFrame consumed;
    private AudioFrameRebuilder rebuilder;
    private MutableAudioFrame mutable;
    private long timeout;
    private TimeUnit unit;

    private StubBuffer(AudioFrame value) { frame = value; }
    public int getRemainingCapacity() { return 7; }
    public int getFullCapacity() { return 9; }
    public void waitForTermination() { record("wait"); }
    public void setTerminateOnEmpty() { record("terminate"); }
    public void setClearOnInsert() { record("clear-on-insert"); }
    public boolean hasClearOnInsert() { return true; }
    public void clear() { record("clear"); }
    public void lockBuffer() { record("lock"); }
    public boolean hasReceivedFrames() { return true; }
    public Long getLastInputTimecode() { return 42L; }
    public void consume(AudioFrame value) { consumed = value; record("consume"); }
    public void rebuild(AudioFrameRebuilder value) { rebuilder = value; record("rebuild"); }
    public AudioFrame provide() { return frame; }
    public AudioFrame provide(long value, TimeUnit valueUnit) {
      timeout = value;
      unit = valueUnit;
      return frame;
    }
    public boolean provide(MutableAudioFrame value) { mutable = value; return true; }
    public boolean provide(MutableAudioFrame value, long timeoutValue, TimeUnit valueUnit) {
      mutable = value;
      timeout = timeoutValue;
      unit = valueUnit;
      return true;
    }
    private void record(String value) {
      if (!operations.isEmpty()) operations += ',';
      operations += value;
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const AUDIO_FRAME_REBUILDER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameRebuilder;
import com.sedmelluq.discord.lavaplayer.track.playback.ImmutableAudioFrame;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;

public final class GateAudioFrameRebuilder {
  private static AudioFrame input;

  public static void main(String[] args) throws Exception {
    AudioFrame original = new ImmutableAudioFrame(1L, new byte[] { 1 }, 100, null);
    AudioFrame replacement = new ImmutableAudioFrame(2L, new byte[] { 2 }, 50, null);
    AudioFrameRebuilder rebuilder = value -> {
      input = value;
      return value == null ? original : replacement;
    };
    check(rebuilder.rebuild(original) == replacement && input == original,
        "frame identity and return dispatch");
    check(rebuilder.rebuild(null) == original && input == null,
        "null argument and return dispatch");

    check(AudioFrameRebuilder.class.isInterface()
        && Modifier.isPublic(AudioFrameRebuilder.class.getModifiers())
        && Modifier.isAbstract(AudioFrameRebuilder.class.getModifiers())
        && AudioFrameRebuilder.class.getInterfaces().length == 0
        && AudioFrameRebuilder.class.getDeclaredFields().length == 0
        && AudioFrameRebuilder.class.getDeclaredMethods().length == 1,
        "rebuilder structure");
    Method rebuild = AudioFrameRebuilder.class.getDeclaredMethod("rebuild", AudioFrame.class);
    check(Modifier.isPublic(rebuild.getModifiers()) && Modifier.isAbstract(rebuild.getModifiers())
        && !rebuild.isDefault() && !rebuild.isBridge() && !rebuild.isSynthetic()
        && rebuild.getReturnType() == AudioFrame.class
        && Arrays.equals(rebuild.getParameterTypes(), new Class<?>[] { AudioFrame.class })
        && rebuild.getExceptionTypes().length == 0
        && rebuild.getTypeParameters().length == 0,
        "rebuilder method metadata");

    System.out.println(
        "dispatch=frame-identity,null-identity,return-identity;"
        + "reflection=public-abstract-interface,0-fields,1-method,0-exceptions");
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const TERMINATOR_AUDIO_FRAME_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.TerminatorAudioFrame;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;

public final class GateTerminatorAudioFrame {
  public static void main(String[] args) throws Exception {
    TerminatorAudioFrame singleton = TerminatorAudioFrame.INSTANCE;
    check(singleton != null && singleton == TerminatorAudioFrame.INSTANCE,
        "stable singleton");
    TerminatorAudioFrame fresh = new TerminatorAudioFrame();
    check(fresh != singleton && singleton.isTerminator() && fresh.isTerminator(),
        "public constructor and terminator state");

    UnsupportedOperationException[] failures = {
      expectUnsupported(() -> { singleton.getTimecode(); }),
      expectUnsupported(() -> { singleton.getVolume(); }),
      expectUnsupported(() -> { singleton.getDataLength(); }),
      expectUnsupported(() -> { singleton.getData(); }),
      expectUnsupported(() -> { singleton.getData(null, -1); }),
      expectUnsupported(() -> { singleton.getFormat(); })
    };
    for (int index = 0; index < failures.length; index++) {
      check(failures[index].getMessage() == null && failures[index].getCause() == null,
          "message-less unsupported accessor " + index);
      if (index > 0) check(failures[index] != failures[index - 1], "fresh exception " + index);
    }

    Class<TerminatorAudioFrame> type = TerminatorAudioFrame.class;
    int modifiers = type.getModifiers();
    check(Modifier.isPublic(modifiers) && !Modifier.isFinal(modifiers)
        && !Modifier.isAbstract(modifiers) && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] { AudioFrame.class }),
        "class structure");
    check(type.getDeclaredFields().length == 1 && type.getDeclaredMethods().length == 7
        && type.getDeclaredConstructors().length == 1, "member counts");
    Field instance = type.getDeclaredField("INSTANCE");
    check(instance.getType() == type && Modifier.isPublic(instance.getModifiers())
        && Modifier.isStatic(instance.getModifiers()) && Modifier.isFinal(instance.getModifiers()),
        "singleton field metadata");
    Constructor<TerminatorAudioFrame> constructor = type.getDeclaredConstructor();
    check(Modifier.isPublic(constructor.getModifiers())
        && constructor.getExceptionTypes().length == 0, "constructor metadata");
    for (Method method : type.getDeclaredMethods()) {
      check(Modifier.isPublic(method.getModifiers()) && !Modifier.isStatic(method.getModifiers())
          && !Modifier.isAbstract(method.getModifiers()) && !method.isBridge()
          && !method.isSynthetic() && method.getExceptionTypes().length == 0,
          "method metadata " + method);
    }

    System.out.println(
        "singleton=stable,fresh-public;accessors=6-unsupported-null-message;"
        + "reflection=1-field,7-methods,1-constructor");
  }

  private static UnsupportedOperationException expectUnsupported(Operation operation) {
    try {
      operation.run();
      throw new AssertionError("expected UnsupportedOperationException");
    } catch (UnsupportedOperationException error) {
      return error;
    }
  }

  private interface Operation { void run(); }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const REFERENCE_MUTABLE_AUDIO_FRAME_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.track.playback.AbstractMutableAudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.ImmutableAudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.ReferenceMutableAudioFrame;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;

public final class GateReferenceMutableAudioFrame {
  public static void main(String[] args) throws Exception {
    ReferenceMutableAudioFrame frame = new ReferenceMutableAudioFrame();
    check(frame.getFrameBuffer() == null && frame.getFrameOffset() == 0
        && frame.getFrameEndOffset() == 0 && frame.getDataLength() == 0,
        "default state");
    expect(NullPointerException.class, () -> { frame.getData(); });

    byte[] backing = { 9, 1, 2, 3, 8 };
    frame.setDataReference(backing, 1, 3);
    check(frame.getFrameBuffer() == backing && frame.getFrameOffset() == 1
        && frame.getFrameEndOffset() == 4 && frame.getDataLength() == 3,
        "reference window");
    byte[] firstCopy = frame.getData();
    byte[] secondCopy = frame.getData();
    check(Arrays.equals(firstCopy, new byte[] { 1, 2, 3 })
        && Arrays.equals(secondCopy, firstCopy) && firstCopy != secondCopy,
        "independent copies");
    byte[] destination = { 7, 7, 7, 7, 7 };
    frame.getData(destination, 1);
    check(Arrays.equals(destination, new byte[] { 7, 1, 2, 3, 7 }),
        "destination offset");
    backing[2] = 6;
    check(Arrays.equals(frame.getData(), new byte[] { 1, 6, 3 }),
        "backing mutation remains visible");

    frame.setTimecode(42L);
    frame.setVolume(73);
    frame.setFormat(null);
    frame.setTerminator(true);
    ImmutableAudioFrame frozen = frame.freeze();
    check(frozen.getTimecode() == 42L && frozen.getVolume() == 73
        && frozen.getFormat() == null && !frozen.isTerminator()
        && Arrays.equals(frozen.getData(), new byte[] { 1, 6, 3 }),
        "inherited state and freeze");
    backing[2] = 2;
    check(Arrays.equals(frozen.getData(), new byte[] { 1, 6, 3 }),
        "freeze owns copied data");

    frame.setDataReference(null, 7, -2);
    check(frame.getFrameBuffer() == null && frame.getFrameOffset() == 7
        && frame.getFrameEndOffset() == 5 && frame.getDataLength() == -2,
        "invalid state stored verbatim");
    expect(NegativeArraySizeException.class, () -> { frame.getData(); });
    frame.setDataReference(new byte[] { 1, 2 }, 1, 3);
    expect(ArrayIndexOutOfBoundsException.class, () -> { frame.getData(); });
    frame.setDataReference(new byte[0], Integer.MAX_VALUE, 1);
    check(frame.getFrameEndOffset() == Integer.MIN_VALUE, "end offset overflow");

    Class<ReferenceMutableAudioFrame> type = ReferenceMutableAudioFrame.class;
    int modifiers = type.getModifiers();
    check(Modifier.isPublic(modifiers) && !Modifier.isFinal(modifiers)
        && !Modifier.isAbstract(modifiers)
        && type.getSuperclass() == AbstractMutableAudioFrame.class
        && type.getInterfaces().length == 0, "class structure");
    check(type.getDeclaredFields().length == 3 && type.getDeclaredMethods().length == 7
        && type.getDeclaredConstructors().length == 1, "member counts");
    checkField(type.getDeclaredField("frameBuffer"), byte[].class);
    checkField(type.getDeclaredField("frameOffset"), int.class);
    checkField(type.getDeclaredField("frameLength"), int.class);
    Constructor<ReferenceMutableAudioFrame> constructor = type.getDeclaredConstructor();
    check(Modifier.isPublic(constructor.getModifiers())
        && constructor.getExceptionTypes().length == 0, "constructor metadata");
    for (Method method : type.getDeclaredMethods()) {
      check(Modifier.isPublic(method.getModifiers()) && !Modifier.isStatic(method.getModifiers())
          && !Modifier.isAbstract(method.getModifiers()) && !method.isBridge()
          && !method.isSynthetic() && method.getExceptionTypes().length == 0,
          "method metadata " + method);
    }

    System.out.println(
        "reference=identity,window,copy,mutation,freeze;"
        + "invalid=deferred,negative,range,overflow;reflection=3-fields,7-methods,1-constructor");
  }

  private static void checkField(Field field, Class<?> type) {
    check(field.getType() == type && Modifier.isPrivate(field.getModifiers())
        && !Modifier.isStatic(field.getModifiers()) && !Modifier.isFinal(field.getModifiers()),
        "field metadata " + field.getName());
  }

  private static void expect(Class<? extends Throwable> type, Runnable operation) {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const AUDIO_FRAME_PROVIDER_TOOLS_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameProvider;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameProviderTools;
import com.sedmelluq.discord.lavaplayer.track.playback.ImmutableAudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.MutableAudioFrame;
import java.lang.reflect.Constructor;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;

public final class GateAudioFrameProviderTools {
  public static void main(String[] args) throws Exception {
    AudioFrame frame = new ImmutableAudioFrame(42L, new byte[] { 1 }, 100, null);
    StubProvider provider = new StubProvider(frame);
    check(AudioFrameProviderTools.delegateToTimedProvide(provider) == frame
        && provider.timeout == 0L && provider.unit == TimeUnit.MILLISECONDS,
        "timed delegation and return identity");

    provider.failure = new TimeoutException("timeout");
    RuntimeException timeout = expectRuntime(() ->
        AudioFrameProviderTools.delegateToTimedProvide(provider));
    check(timeout.getClass() == RuntimeException.class && timeout.getCause() == provider.failure
        && timeout.getMessage().equals(provider.failure.toString())
        && !Thread.currentThread().isInterrupted(), "timeout wrapping");

    Thread.interrupted();
    provider.failure = new InterruptedException("interrupted");
    RuntimeException interrupted = expectRuntime(() ->
        AudioFrameProviderTools.delegateToTimedProvide(provider));
    check(interrupted.getClass() == RuntimeException.class
        && interrupted.getCause() == provider.failure && Thread.interrupted(),
        "interruption wrapping and restoration");

    IllegalStateException unchecked = new IllegalStateException("unchecked");
    provider.failure = unchecked;
    try {
      AudioFrameProviderTools.delegateToTimedProvide(provider);
      throw new AssertionError("expected unchecked failure");
    } catch (IllegalStateException error) {
      check(error == unchecked, "unchecked failure identity");
    }
    expect(NullPointerException.class, () ->
        AudioFrameProviderTools.delegateToTimedProvide(null));

    Class<AudioFrameProviderTools> type = AudioFrameProviderTools.class;
    int modifiers = type.getModifiers();
    check(Modifier.isPublic(modifiers) && !Modifier.isFinal(modifiers)
        && !Modifier.isAbstract(modifiers) && type.getSuperclass() == Object.class
        && type.getInterfaces().length == 0 && type.getDeclaredFields().length == 0
        && type.getDeclaredMethods().length == 1
        && type.getDeclaredConstructors().length == 1, "class structure");
    Constructor<AudioFrameProviderTools> constructor = type.getDeclaredConstructor();
    check(Modifier.isPublic(constructor.getModifiers())
        && constructor.getExceptionTypes().length == 0, "constructor metadata");
    Method delegate = type.getDeclaredMethod("delegateToTimedProvide", AudioFrameProvider.class);
    check(Modifier.isPublic(delegate.getModifiers()) && Modifier.isStatic(delegate.getModifiers())
        && !Modifier.isAbstract(delegate.getModifiers()) && !delegate.isBridge()
        && !delegate.isSynthetic() && delegate.getReturnType() == AudioFrame.class
        && Arrays.equals(delegate.getParameterTypes(), new Class<?>[] { AudioFrameProvider.class })
        && delegate.getExceptionTypes().length == 0, "delegate metadata");

    System.out.println(
        "delegate=zero-milliseconds,return-identity,null;"
        + "failures=timeout-wrap,interrupt-wrap-restore,unchecked-identity;"
        + "reflection=0-fields,1-method,1-constructor");
  }

  private static RuntimeException expectRuntime(Runnable operation) {
    try {
      operation.run();
      throw new AssertionError("expected RuntimeException");
    } catch (RuntimeException error) {
      return error;
    }
  }

  private static void expect(Class<? extends Throwable> type, Runnable operation) {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
    }
  }

  private static final class StubProvider implements AudioFrameProvider {
    private final AudioFrame frame;
    private Throwable failure;
    private long timeout;
    private TimeUnit unit;

    private StubProvider(AudioFrame value) { frame = value; }
    public AudioFrame provide() { throw new AssertionError("untimed provide called"); }
    public AudioFrame provide(long value, TimeUnit valueUnit)
        throws TimeoutException, InterruptedException {
      timeout = value;
      unit = valueUnit;
      if (failure instanceof TimeoutException) throw (TimeoutException) failure;
      if (failure instanceof InterruptedException) throw (InterruptedException) failure;
      if (failure instanceof RuntimeException) throw (RuntimeException) failure;
      return frame;
    }
    public boolean provide(MutableAudioFrame targetFrame) {
      throw new AssertionError("mutable provide called");
    }
    public boolean provide(MutableAudioFrame targetFrame, long value, TimeUnit valueUnit) {
      throw new AssertionError("timed mutable provide called");
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const AUDIO_PROCESSING_CONTEXT_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.format.AudioDataFormat;
import com.sedmelluq.discord.lavaplayer.player.AudioConfiguration;
import com.sedmelluq.discord.lavaplayer.player.AudioPlayerOptions;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameBuffer;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioProcessingContext;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;

public final class GateAudioProcessingContext {
  public static void main(String[] args) throws Exception {
    AudioConfiguration configuration = new AudioConfiguration();
    configuration.setFilterHotSwapEnabled(true);
    AudioFrameBuffer buffer = (AudioFrameBuffer) Proxy.newProxyInstance(
        GateAudioProcessingContext.class.getClassLoader(),
        new Class<?>[] { AudioFrameBuffer.class }, (proxy, method, values) -> null);
    AudioPlayerOptions options = new AudioPlayerOptions();
    AudioDataFormat format = configuration.getOutputFormat();

    AudioProcessingContext context =
        new AudioProcessingContext(configuration, buffer, options, format);
    check(context.configuration == configuration && context.frameBuffer == buffer
        && context.playerOptions == options && context.outputFormat == format,
        "constructor identity");
    check(context.filterHotSwapEnabled, "initial hot-swap snapshot");
    configuration.setFilterHotSwapEnabled(false);
    check(context.filterHotSwapEnabled, "snapshot remains stable");
    check(!new AudioProcessingContext(configuration, buffer, options, format)
        .filterHotSwapEnabled, "later snapshot observes mutation");

    AudioProcessingContext nullable =
        new AudioProcessingContext(configuration, null, null, null);
    check(nullable.frameBuffer == null && nullable.playerOptions == null
        && nullable.outputFormat == null, "nullable auxiliary fields");
    expect(NullPointerException.class,
        () -> new AudioProcessingContext(null, buffer, options, format));

    Class<AudioProcessingContext> type = AudioProcessingContext.class;
    int modifiers = type.getModifiers();
    check(Modifier.isPublic(modifiers) && !Modifier.isFinal(modifiers)
        && !Modifier.isAbstract(modifiers) && type.getSuperclass() == Object.class
        && type.getInterfaces().length == 0, "class structure");
    check(type.getDeclaredFields().length == 5 && type.getDeclaredMethods().length == 0
        && type.getDeclaredConstructors().length == 1, "member counts");
    checkField(type.getDeclaredField("configuration"), AudioConfiguration.class);
    checkField(type.getDeclaredField("frameBuffer"), AudioFrameBuffer.class);
    checkField(type.getDeclaredField("playerOptions"), AudioPlayerOptions.class);
    checkField(type.getDeclaredField("outputFormat"), AudioDataFormat.class);
    checkField(type.getDeclaredField("filterHotSwapEnabled"), boolean.class);
    Constructor<AudioProcessingContext> constructor = type.getDeclaredConstructor(
        AudioConfiguration.class, AudioFrameBuffer.class,
        AudioPlayerOptions.class, AudioDataFormat.class);
    check(Modifier.isPublic(constructor.getModifiers())
        && Arrays.equals(constructor.getParameterTypes(), new Class<?>[] {
            AudioConfiguration.class, AudioFrameBuffer.class,
            AudioPlayerOptions.class, AudioDataFormat.class })
        && constructor.getExceptionTypes().length == 0
        && constructor.getTypeParameters().length == 0, "constructor metadata");

    System.out.println(
        "identity=configuration,buffer,options,format;filter=snapshot,true,false;"
        + "nulls=optional,configuration-npe;reflection=5-fields,0-methods,1-constructor");
  }

  private static void checkField(Field field, Class<?> type) {
    check(field.getType() == type && Modifier.isPublic(field.getModifiers())
        && Modifier.isFinal(field.getModifiers()) && !Modifier.isStatic(field.getModifiers())
        && !field.isSynthetic(), "field metadata " + field.getName());
  }

  private static void expect(Class<? extends Throwable> type, Runnable operation) {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const AUDIO_PLAYER_OPTIONS_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.filter.PcmFilterFactory;
import com.sedmelluq.discord.lavaplayer.player.AudioPlayerOptions;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;

public final class GateAudioPlayerOptions {
  public static void main(String[] args) throws Exception {
    AudioPlayerOptions options = new AudioPlayerOptions();
    check(options.volumeLevel != null && options.volumeLevel.get() == 100,
        "volume default");
    check(options.filterFactory != null && options.filterFactory.get() == null,
        "filter default");
    check(options.frameBufferDuration != null && options.frameBufferDuration.get() == null,
        "duration default");
    check(options.filterFactory != (Object) options.frameBufferDuration,
        "distinct reference holders");

    PcmFilterFactory factory = (PcmFilterFactory) Proxy.newProxyInstance(
        GateAudioPlayerOptions.class.getClassLoader(),
        new Class<?>[] { PcmFilterFactory.class }, (proxy, method, values) -> null);
    options.volumeLevel.set(Integer.MIN_VALUE);
    options.filterFactory.set(factory);
    options.frameBufferDuration.set(-1);
    check(options.volumeLevel.get() == Integer.MIN_VALUE
        && options.filterFactory.get() == factory
        && options.frameBufferDuration.get().equals(-1), "atomic mutation");

    AudioPlayerOptions second = new AudioPlayerOptions();
    check(second.volumeLevel != options.volumeLevel
        && second.filterFactory != options.filterFactory
        && second.frameBufferDuration != options.frameBufferDuration,
        "per-instance holders");
    check(second.volumeLevel.get() == 100 && second.filterFactory.get() == null
        && second.frameBufferDuration.get() == null, "independent defaults");

    Class<AudioPlayerOptions> type = AudioPlayerOptions.class;
    int modifiers = type.getModifiers();
    check(Modifier.isPublic(modifiers) && !Modifier.isFinal(modifiers)
        && !Modifier.isAbstract(modifiers) && type.getSuperclass() == Object.class
        && type.getInterfaces().length == 0, "class structure");
    check(type.getDeclaredFields().length == 3 && type.getDeclaredMethods().length == 0
        && type.getDeclaredConstructors().length == 1, "member counts");
    checkField(type.getDeclaredField("volumeLevel"), AtomicInteger.class,
        "java.util.concurrent.atomic.AtomicInteger");
    checkField(type.getDeclaredField("filterFactory"), AtomicReference.class,
        "java.util.concurrent.atomic.AtomicReference<com.sedmelluq.discord.lavaplayer.filter.PcmFilterFactory>");
    checkField(type.getDeclaredField("frameBufferDuration"), AtomicReference.class,
        "java.util.concurrent.atomic.AtomicReference<java.lang.Integer>");
    Constructor<AudioPlayerOptions> constructor = type.getDeclaredConstructor();
    check(Modifier.isPublic(constructor.getModifiers())
        && constructor.getExceptionTypes().length == 0
        && constructor.getTypeParameters().length == 0, "constructor metadata");

    System.out.println(
        "defaults=100,null,null;holders=distinct,per-instance;"
        + "mutation=minimum,factory,-1;reflection=3-fields,0-methods,1-constructor,generics");
  }

  private static void checkField(Field field, Class<?> type, String genericType) {
    check(field.getType() == type && field.getGenericType().getTypeName().equals(genericType)
        && Modifier.isPublic(field.getModifiers()) && Modifier.isFinal(field.getModifiers())
        && !Modifier.isStatic(field.getModifiers()) && !field.isSynthetic(),
        "field metadata " + field.getName());
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const DECODED_TRACK_HOLDER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.DecodedTrackHolder;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;

public final class GateDecodedTrackHolder {
  public static void main(String[] args) throws Exception {
    AudioTrack track = (AudioTrack) Proxy.newProxyInstance(
        GateDecodedTrackHolder.class.getClassLoader(),
        new Class<?>[] { AudioTrack.class }, (proxy, method, values) -> null);
    DecodedTrackHolder holder = new DecodedTrackHolder(track);
    check(holder.decodedTrack == track, "track identity");
    check(new DecodedTrackHolder(null).decodedTrack == null, "null track");

    Class<DecodedTrackHolder> type = DecodedTrackHolder.class;
    int modifiers = type.getModifiers();
    check(Modifier.isPublic(modifiers) && !Modifier.isFinal(modifiers)
        && !Modifier.isAbstract(modifiers) && type.getSuperclass() == Object.class
        && type.getInterfaces().length == 0, "class structure");
    check(type.getDeclaredFields().length == 1 && type.getDeclaredMethods().length == 0
        && type.getDeclaredConstructors().length == 1, "member counts");
    Field field = type.getDeclaredField("decodedTrack");
    check(field.getType() == AudioTrack.class && Modifier.isPublic(field.getModifiers())
        && Modifier.isFinal(field.getModifiers()) && !Modifier.isStatic(field.getModifiers())
        && !field.isSynthetic(), "field metadata");
    Constructor<DecodedTrackHolder> constructor =
        type.getDeclaredConstructor(AudioTrack.class);
    check(Modifier.isPublic(constructor.getModifiers())
        && Arrays.equals(constructor.getParameterTypes(), new Class<?>[] { AudioTrack.class })
        && constructor.getExceptionTypes().length == 0
        && constructor.getTypeParameters().length == 0, "constructor metadata");

    System.out.println(
        "holder=track-identity,null;reflection=1-field,0-methods,1-constructor");
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const TRACK_STATE_LISTENER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.tools.FriendlyException;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.TrackStateListener;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;

public final class GateTrackStateListener {
  public static void main(String[] args) throws Exception {
    AudioTrack track = (AudioTrack) Proxy.newProxyInstance(
        GateTrackStateListener.class.getClassLoader(),
        new Class<?>[] { AudioTrack.class }, (proxy, method, values) -> null);
    FriendlyException exception = allocate(FriendlyException.class);
    StringBuilder calls = new StringBuilder();
    TrackStateListener listener = new TrackStateListener() {
      public void onTrackException(AudioTrack value, FriendlyException error) {
        if (calls.length() == 0) {
          check(value == track && error == exception, "exception identities");
          calls.append("exception,");
        } else {
          check(value == null && error == null, "nullable exception values");
          calls.append("exception-null,");
        }
      }
      public void onTrackStuck(AudioTrack value, long thresholdMs) {
        if (thresholdMs == Long.MIN_VALUE) {
          check(value == track, "stuck track identity");
          calls.append("stuck-min,");
        } else {
          check(value == null && thresholdMs == Long.MAX_VALUE, "nullable stuck values");
          calls.append("stuck-max");
        }
      }
    };
    listener.onTrackException(track, exception);
    listener.onTrackStuck(track, Long.MIN_VALUE);
    listener.onTrackException(null, null);
    listener.onTrackStuck(null, Long.MAX_VALUE);
    check(calls.toString().equals("exception,stuck-min,exception-null,stuck-max"),
        "callback order");

    Class<TrackStateListener> type = TrackStateListener.class;
    int modifiers = type.getModifiers();
    check(Modifier.isPublic(modifiers) && Modifier.isInterface(modifiers)
        && Modifier.isAbstract(modifiers) && !Modifier.isFinal(modifiers)
        && type.getSuperclass() == null && type.getInterfaces().length == 0,
        "interface structure");
    check(type.getDeclaredFields().length == 0 && type.getDeclaredMethods().length == 2
        && type.getDeclaredConstructors().length == 0, "member counts");
    checkMethod(type.getDeclaredMethod("onTrackException",
        AudioTrack.class, FriendlyException.class),
        new Class<?>[] { AudioTrack.class, FriendlyException.class });
    checkMethod(type.getDeclaredMethod("onTrackStuck", AudioTrack.class, long.class),
        new Class<?>[] { AudioTrack.class, long.class });

    System.out.println(
        "dispatch=exception,stuck-min,nullable,stuck-max;"
        + "reflection=interface,0-fields,2-methods,0-constructors");
  }

  private static void checkMethod(Method method, Class<?>[] parameters) {
    check(Modifier.isPublic(method.getModifiers()) && Modifier.isAbstract(method.getModifiers())
        && !Modifier.isStatic(method.getModifiers()) && !method.isDefault()
        && !method.isBridge() && !method.isSynthetic() && method.getReturnType() == void.class
        && Arrays.equals(method.getParameterTypes(), parameters)
        && method.getExceptionTypes().length == 0 && method.getTypeParameters().length == 0,
        "method metadata " + method.getName());
  }

  private static <T> T allocate(Class<T> type) throws Exception {
    Class<?> unsafeType = Class.forName("sun.misc.Unsafe");
    Field singleton = unsafeType.getDeclaredField("theUnsafe");
    singleton.setAccessible(true);
    Object unsafe = singleton.get(null);
    return type.cast(unsafeType.getMethod("allocateInstance", Class.class).invoke(unsafe, type));
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const AUDIO_OUTPUT_HOOK_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.player.AudioPlayer;
import com.sedmelluq.discord.lavaplayer.player.hook.AudioOutputHook;
import com.sedmelluq.discord.lavaplayer.player.hook.AudioOutputHookFactory;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.ImmutableAudioFrame;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;

public final class GateAudioOutputHook {
  public static void main(String[] args) throws Exception {
    AudioPlayer player = (AudioPlayer) Proxy.newProxyInstance(
        GateAudioOutputHook.class.getClassLoader(), new Class<?>[] { AudioPlayer.class },
        (proxy, method, values) -> null);
    AudioFrame input = new ImmutableAudioFrame(1L, new byte[] { 1 }, 100, null);
    AudioFrame replacement = new ImmutableAudioFrame(2L, new byte[] { 2 }, 50, null);
    int[] calls = { 0 };
    AudioOutputHook hook = (value, frame) -> {
      calls[0]++;
      if (calls[0] == 1) {
        check(value == player && frame == input, "hook argument identity");
        return replacement;
      }
      if (calls[0] == 2) {
        check(value == player && frame == replacement, "hook passthrough arguments");
        return frame;
      }
      check(value == null && frame == null, "hook nullable arguments");
      return null;
    };
    check(hook.outgoingFrame(player, input) == replacement, "replacement identity");
    check(hook.outgoingFrame(player, replacement) == replacement, "passthrough identity");
    check(hook.outgoingFrame(null, null) == null && calls[0] == 3, "nullable return");

    AudioOutputHookFactory factory = () -> hook;
    check(factory.createOutputHook() == hook, "factory hook identity");
    AudioOutputHookFactory nullFactory = () -> null;
    check(nullFactory.createOutputHook() == null, "factory nullable return");

    checkInterface(AudioOutputHook.class, 1);
    checkInterface(AudioOutputHookFactory.class, 1);
    Method outgoing = AudioOutputHook.class.getDeclaredMethod(
        "outgoingFrame", AudioPlayer.class, AudioFrame.class);
    checkMethod(outgoing, AudioFrame.class,
        new Class<?>[] { AudioPlayer.class, AudioFrame.class });
    Method create = AudioOutputHookFactory.class.getDeclaredMethod("createOutputHook");
    checkMethod(create, AudioOutputHook.class, new Class<?>[0]);

    System.out.println(
        "hook=replacement,passthrough,null;factory=identity,null;"
        + "reflection=2-interfaces,0-fields,2-methods,0-constructors");
  }

  private static void checkInterface(Class<?> type, int methodCount) {
    int modifiers = type.getModifiers();
    check(Modifier.isPublic(modifiers) && Modifier.isInterface(modifiers)
        && Modifier.isAbstract(modifiers) && !Modifier.isFinal(modifiers)
        && type.getSuperclass() == null && type.getInterfaces().length == 0
        && type.getDeclaredFields().length == 0
        && type.getDeclaredMethods().length == methodCount
        && type.getDeclaredConstructors().length == 0,
        "interface structure " + type.getName());
  }

  private static void checkMethod(Method method, Class<?> returnType, Class<?>[] parameters) {
    check(Modifier.isPublic(method.getModifiers()) && Modifier.isAbstract(method.getModifiers())
        && !Modifier.isStatic(method.getModifiers()) && !method.isDefault()
        && !method.isBridge() && !method.isSynthetic() && method.getReturnType() == returnType
        && Arrays.equals(method.getParameterTypes(), parameters)
        && method.getExceptionTypes().length == 0 && method.getTypeParameters().length == 0,
        "method metadata " + method.getName());
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const AUDIO_LOAD_RESULT_HANDLER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.player.AudioLoadResultHandler;
import com.sedmelluq.discord.lavaplayer.tools.FriendlyException;
import com.sedmelluq.discord.lavaplayer.track.AudioPlaylist;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;

public final class GateAudioLoadResultHandler {
  public static void main(String[] args) throws Exception {
    AudioTrack track = proxy(AudioTrack.class);
    AudioPlaylist playlist = proxy(AudioPlaylist.class);
    FriendlyException exception = allocate(FriendlyException.class);
    StringBuilder calls = new StringBuilder();
    AudioLoadResultHandler handler = new AudioLoadResultHandler() {
      private int trackCalls;
      private int playlistCalls;
      private int failureCalls;

      public void trackLoaded(AudioTrack value) {
        check(value == (trackCalls++ == 0 ? track : null), "track value");
        calls.append(value == null ? "track-null," : "track,");
      }
      public void playlistLoaded(AudioPlaylist value) {
        check(value == (playlistCalls++ == 0 ? playlist : null), "playlist value");
        calls.append(value == null ? "playlist-null," : "playlist,");
      }
      public void noMatches() {
        calls.append("none,");
      }
      public void loadFailed(FriendlyException value) {
        check(value == (failureCalls++ == 0 ? exception : null), "failure value");
        calls.append(value == null ? "failed-null" : "failed,");
      }
    };
    handler.trackLoaded(track);
    handler.playlistLoaded(playlist);
    handler.noMatches();
    handler.loadFailed(exception);
    handler.trackLoaded(null);
    handler.playlistLoaded(null);
    handler.loadFailed(null);
    check(calls.toString().equals(
        "track,playlist,none,failed,track-null,playlist-null,failed-null"),
        "callback order");

    Class<AudioLoadResultHandler> type = AudioLoadResultHandler.class;
    int modifiers = type.getModifiers();
    check(Modifier.isPublic(modifiers) && Modifier.isInterface(modifiers)
        && Modifier.isAbstract(modifiers) && !Modifier.isFinal(modifiers)
        && type.getSuperclass() == null && type.getInterfaces().length == 0,
        "interface structure");
    check(type.getDeclaredFields().length == 0 && type.getDeclaredMethods().length == 4
        && type.getDeclaredConstructors().length == 0, "member counts");
    checkMethod(type.getDeclaredMethod("trackLoaded", AudioTrack.class),
        new Class<?>[] { AudioTrack.class });
    checkMethod(type.getDeclaredMethod("playlistLoaded", AudioPlaylist.class),
        new Class<?>[] { AudioPlaylist.class });
    checkMethod(type.getDeclaredMethod("noMatches"), new Class<?>[0]);
    checkMethod(type.getDeclaredMethod("loadFailed", FriendlyException.class),
        new Class<?>[] { FriendlyException.class });

    System.out.println(
        "dispatch=track,playlist,none,failed,nulls,ordered;"
        + "reflection=interface,0-fields,4-methods,0-constructors");
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type) {
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] { type },
        (instance, method, arguments) -> null);
  }

  private static <T> T allocate(Class<T> type) throws Exception {
    Class<?> unsafeType = Class.forName("sun.misc.Unsafe");
    Field singleton = unsafeType.getDeclaredField("theUnsafe");
    singleton.setAccessible(true);
    Object unsafe = singleton.get(null);
    return type.cast(unsafeType.getMethod("allocateInstance", Class.class).invoke(unsafe, type));
  }

  private static void checkMethod(Method method, Class<?>[] parameters) {
    check(Modifier.isPublic(method.getModifiers()) && Modifier.isAbstract(method.getModifiers())
        && !Modifier.isStatic(method.getModifiers()) && !method.isDefault()
        && !method.isBridge() && !method.isSynthetic() && method.getReturnType() == void.class
        && Arrays.equals(method.getParameterTypes(), parameters)
        && method.getExceptionTypes().length == 0 && method.getTypeParameters().length == 0,
        "method metadata " + method.getName());
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const FUNCTIONAL_RESULT_HANDLER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.player.AudioLoadResultHandler;
import com.sedmelluq.discord.lavaplayer.player.FunctionalResultHandler;
import com.sedmelluq.discord.lavaplayer.tools.FriendlyException;
import com.sedmelluq.discord.lavaplayer.track.AudioPlaylist;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.lang.reflect.Type;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.function.Consumer;

public final class GateFunctionalResultHandler {
  public static void main(String[] args) throws Exception {
    AudioTrack track = proxy(AudioTrack.class);
    AudioPlaylist playlist = proxy(AudioPlaylist.class);
    FriendlyException exception = allocate(FriendlyException.class);
    List<String> calls = new ArrayList<>();

    FunctionalResultHandler handler = new FunctionalResultHandler(
        value -> {
          check(value == track || value == null, "track identity");
          calls.add(value == null ? "track-null" : "track");
        },
        value -> {
          check(value == playlist || value == null, "playlist identity");
          calls.add(value == null ? "playlist-null" : "playlist");
        },
        () -> calls.add("none"),
        value -> {
          check(value == exception || value == null, "exception identity");
          calls.add(value == null ? "failed-null" : "failed");
        });
    handler.trackLoaded(track);
    handler.playlistLoaded(playlist);
    handler.noMatches();
    handler.loadFailed(exception);
    handler.trackLoaded(null);
    handler.playlistLoaded(null);
    handler.loadFailed(null);
    check(calls.equals(Arrays.asList(
        "track", "playlist", "none", "failed", "track-null", "playlist-null", "failed-null")),
        "callback order");

    FunctionalResultHandler empty = new FunctionalResultHandler(null, null, null, null);
    empty.trackLoaded(track);
    empty.trackLoaded(null);
    empty.playlistLoaded(playlist);
    empty.playlistLoaded(null);
    empty.noMatches();
    empty.loadFailed(exception);
    empty.loadFailed(null);
    check(calls.size() == 7, "null callbacks skipped");

    RuntimeException sentinel = new RuntimeException("sentinel");
    expectSame(sentinel,
        () -> new FunctionalResultHandler(value -> { throw sentinel; }, null, null, null)
            .trackLoaded(track));
    expectSame(sentinel,
        () -> new FunctionalResultHandler(null, value -> { throw sentinel; }, null, null)
            .playlistLoaded(playlist));
    expectSame(sentinel,
        () -> new FunctionalResultHandler(null, null, () -> { throw sentinel; }, null)
            .noMatches());
    expectSame(sentinel,
        () -> new FunctionalResultHandler(null, null, null, value -> { throw sentinel; })
            .loadFailed(exception));

    checkReflection();
    System.out.println(
        "dispatch=track,playlist,none,failed,nulls,ordered;"
        + "callbacks=nullable,exceptions-propagated;"
        + "reflection=class,4-fields,4-methods,1-constructor");
  }

  private static void checkReflection() throws Exception {
    Class<FunctionalResultHandler> type = FunctionalResultHandler.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] { AudioLoadResultHandler.class })
        && type.getTypeParameters().length == 0 && type.getDeclaredAnnotations().length == 0,
        "class structure");
    check(type.getDeclaredFields().length == 4 && type.getDeclaredMethods().length == 4
        && type.getDeclaredConstructors().length == 1, "member counts");
    checkField(type, "trackConsumer", Consumer.class,
        "java.util.function.Consumer<com.sedmelluq.discord.lavaplayer.track.AudioTrack>");
    checkField(type, "playlistConsumer", Consumer.class,
        "java.util.function.Consumer<com.sedmelluq.discord.lavaplayer.track.AudioPlaylist>");
    checkField(type, "emptyResultHandler", Runnable.class, "java.lang.Runnable");
    checkField(type, "exceptionConsumer", Consumer.class,
        "java.util.function.Consumer<com.sedmelluq.discord.lavaplayer.tools.FriendlyException>");

    Constructor<FunctionalResultHandler> constructor = type.getDeclaredConstructor(
        Consumer.class, Consumer.class, Runnable.class, Consumer.class);
    check(constructor.getModifiers() == Modifier.PUBLIC && !constructor.isSynthetic()
        && !constructor.isVarArgs() && constructor.getExceptionTypes().length == 0
        && constructor.getTypeParameters().length == 0, "constructor metadata");
    Type[] genericParameters = constructor.getGenericParameterTypes();
    check(genericParameters.length == 4
        && genericParameters[0].getTypeName().equals(
            "java.util.function.Consumer<com.sedmelluq.discord.lavaplayer.track.AudioTrack>")
        && genericParameters[1].getTypeName().equals(
            "java.util.function.Consumer<com.sedmelluq.discord.lavaplayer.track.AudioPlaylist>")
        && genericParameters[2].getTypeName().equals("java.lang.Runnable")
        && genericParameters[3].getTypeName().equals(
            "java.util.function.Consumer<com.sedmelluq.discord.lavaplayer.tools.FriendlyException>"),
        "constructor generic parameters");
    checkMethod(type.getDeclaredMethod("trackLoaded", AudioTrack.class),
        new Class<?>[] { AudioTrack.class });
    checkMethod(type.getDeclaredMethod("playlistLoaded", AudioPlaylist.class),
        new Class<?>[] { AudioPlaylist.class });
    checkMethod(type.getDeclaredMethod("noMatches"), new Class<?>[0]);
    checkMethod(type.getDeclaredMethod("loadFailed", FriendlyException.class),
        new Class<?>[] { FriendlyException.class });
  }

  private static void checkField(
      Class<?> owner, String name, Class<?> fieldType, String genericType) throws Exception {
    Field field = owner.getDeclaredField(name);
    check(field.getType() == fieldType
        && field.getModifiers() == (Modifier.PRIVATE | Modifier.FINAL)
        && field.getGenericType().getTypeName().equals(genericType)
        && !field.isSynthetic() && field.getDeclaredAnnotations().length == 0,
        "field metadata " + name);
  }

  private static void checkMethod(Method method, Class<?>[] parameters) {
    check(method.getModifiers() == Modifier.PUBLIC && method.getReturnType() == void.class
        && Arrays.equals(method.getParameterTypes(), parameters)
        && method.getExceptionTypes().length == 0 && method.getTypeParameters().length == 0
        && !method.isBridge() && !method.isSynthetic() && !method.isDefault()
        && !method.isVarArgs(), "method metadata " + method.getName());
  }

  private static void expectSame(RuntimeException sentinel, Runnable invocation) {
    try {
      invocation.run();
      throw new AssertionError("callback exception swallowed");
    } catch (RuntimeException actual) {
      check(actual == sentinel, "callback exception identity");
    }
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type) {
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] { type },
        (instance, method, arguments) -> null);
  }

  private static <T> T allocate(Class<T> type) throws Exception {
    Class<?> unsafeType = Class.forName("sun.misc.Unsafe");
    Field singleton = unsafeType.getDeclaredField("theUnsafe");
    singleton.setAccessible(true);
    Object unsafe = singleton.get(null);
    return type.cast(unsafeType.getMethod("allocateInstance", Class.class).invoke(unsafe, type));
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const AUDIO_PLAYER_LIFECYCLE_MANAGER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.player.AudioPlayer;
import com.sedmelluq.discord.lavaplayer.player.AudioPlayerLifecycleManager;
import com.sedmelluq.discord.lavaplayer.player.event.AudioEvent;
import com.sedmelluq.discord.lavaplayer.player.event.AudioEventListener;
import com.sedmelluq.discord.lavaplayer.player.event.PlayerPauseEvent;
import com.sedmelluq.discord.lavaplayer.player.event.TrackEndEvent;
import com.sedmelluq.discord.lavaplayer.player.event.TrackStartEvent;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackEndReason;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.ConcurrentMap;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.ScheduledFuture;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicLong;
import java.util.concurrent.atomic.AtomicReference;

public final class GateAudioPlayerLifecycleManager {
  public static void main(String[] args) throws Exception {
    SchedulerHandler schedulerCalls = new SchedulerHandler();
    ScheduledExecutorService scheduler = proxy(ScheduledExecutorService.class, schedulerCalls);
    AtomicLong cleanupThreshold = new AtomicLong(37L);
    AudioPlayerLifecycleManager manager =
        new AudioPlayerLifecycleManager(scheduler, cleanupThreshold);
    schedulerCalls.expectedRunnable = manager;

    manager.shutdown();
    check(schedulerCalls.tasks.isEmpty(), "shutdown before initialise");
    manager.initialise();
    manager.initialise();
    check(schedulerCalls.tasks.size() == 2, "initialise scheduling count");
    check(schedulerCalls.tasks.get(0).cancelValues.isEmpty(), "stored task not cancelled");
    check(schedulerCalls.tasks.get(1).cancelValues.equals(Arrays.asList(false)),
        "duplicate task cancelled without interrupt");

    PlayerHandler firstCalls = new PlayerHandler();
    PlayerHandler secondCalls = new PlayerHandler();
    AudioPlayer first = proxy(AudioPlayer.class, firstCalls);
    AudioPlayer second = proxy(AudioPlayer.class, secondCalls);
    AudioTrack track = proxy(AudioTrack.class, new DefaultHandler());

    manager.onEvent(null);
    manager.onEvent(new PlayerPauseEvent(first));
    manager.run();
    check(firstCalls.thresholds.isEmpty() && secondCalls.thresholds.isEmpty(),
        "unrelated event ignored");

    manager.onEvent(new TrackStartEvent(first, track));
    manager.onEvent(new TrackStartEvent(second, track));
    manager.onEvent(new TrackStartEvent(first, track));
    manager.run();
    check(firstCalls.thresholds.equals(Arrays.asList(37L))
        && secondCalls.thresholds.equals(Arrays.asList(37L)), "start and deduplicate");

    cleanupThreshold.set(Long.MIN_VALUE);
    manager.run();
    check(firstCalls.thresholds.equals(Arrays.asList(37L, Long.MIN_VALUE))
        && secondCalls.thresholds.equals(Arrays.asList(37L, Long.MIN_VALUE)),
        "live threshold");

    manager.onEvent(new TrackEndEvent(first, track, AudioTrackEndReason.FINISHED));
    manager.run();
    check(firstCalls.thresholds.size() == 2 && secondCalls.thresholds.size() == 3
        && secondCalls.thresholds.get(2) == Long.MIN_VALUE, "end removes one player");
    manager.onEvent(new PlayerPauseEvent(second));
    manager.onEvent(new TrackEndEvent(second, track, AudioTrackEndReason.STOPPED));
    manager.run();
    check(secondCalls.thresholds.size() == 3, "end removes final player");
    expectNullPlayerFailure(manager, track);

    manager.shutdown();
    manager.shutdown();
    check(schedulerCalls.tasks.get(0).cancelValues.equals(Arrays.asList(false)),
        "stored task cancelled exactly once");
    manager.initialise();
    manager.shutdown();
    check(schedulerCalls.tasks.size() == 3
        && schedulerCalls.tasks.get(2).cancelValues.equals(Arrays.asList(false)),
        "restart after shutdown");

    checkReflection();
    System.out.println(
        "schedule=fixed-rate,duplicate-cancel,restart;"
        + "players=start,end,deduplicate,live-threshold,null-event,null-player;"
        + "shutdown=idempotent;reflection=class,5-fields,4-methods,1-constructor");
  }

  private static void expectNullPlayerFailure(
      AudioPlayerLifecycleManager manager, AudioTrack track) {
    try {
      manager.onEvent(new TrackStartEvent(null, track));
      throw new AssertionError("null player accepted");
    } catch (NullPointerException expected) {
      // ConcurrentHashMap rejects the null player used as both key and value.
    }
  }

  private static void checkReflection() throws Exception {
    Class<AudioPlayerLifecycleManager> type = AudioPlayerLifecycleManager.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(),
            new Class<?>[] { Runnable.class, AudioEventListener.class })
        && type.getTypeParameters().length == 0 && type.getDeclaredAnnotations().length == 0,
        "class structure");
    check(type.getDeclaredFields().length == 5 && type.getDeclaredMethods().length == 4
        && type.getDeclaredConstructors().length == 1, "member counts");
    checkField(type, "CHECK_INTERVAL", long.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL, "long");
    Field interval = type.getDeclaredField("CHECK_INTERVAL");
    interval.setAccessible(true);
    check(interval.getLong(null) == 10_000L, "check interval constant");
    checkField(type, "activePlayers", ConcurrentMap.class, Modifier.PRIVATE | Modifier.FINAL,
        "java.util.concurrent.ConcurrentMap<com.sedmelluq.discord.lavaplayer.player.AudioPlayer, "
        + "com.sedmelluq.discord.lavaplayer.player.AudioPlayer>");
    checkField(type, "scheduler", ScheduledExecutorService.class,
        Modifier.PRIVATE | Modifier.FINAL, "java.util.concurrent.ScheduledExecutorService");
    checkField(type, "cleanupThreshold", AtomicLong.class, Modifier.PRIVATE | Modifier.FINAL,
        "java.util.concurrent.atomic.AtomicLong");
    checkField(type, "scheduledTask", AtomicReference.class, Modifier.PRIVATE | Modifier.FINAL,
        "java.util.concurrent.atomic.AtomicReference<java.util.concurrent.ScheduledFuture<?>>");

    Constructor<AudioPlayerLifecycleManager> constructor =
        type.getDeclaredConstructor(ScheduledExecutorService.class, AtomicLong.class);
    check(constructor.getModifiers() == Modifier.PUBLIC && !constructor.isSynthetic()
        && !constructor.isVarArgs() && constructor.getExceptionTypes().length == 0
        && constructor.getTypeParameters().length == 0, "constructor metadata");
    checkMethod(type.getDeclaredMethod("initialise"), new Class<?>[0]);
    checkMethod(type.getDeclaredMethod("shutdown"), new Class<?>[0]);
    checkMethod(type.getDeclaredMethod("onEvent", AudioEvent.class),
        new Class<?>[] { AudioEvent.class });
    checkMethod(type.getDeclaredMethod("run"), new Class<?>[0]);
  }

  private static void checkField(Class<?> owner, String name, Class<?> fieldType, int modifiers,
      String genericType) throws Exception {
    Field field = owner.getDeclaredField(name);
    check(field.getType() == fieldType && field.getModifiers() == modifiers
        && field.getGenericType().getTypeName().equals(genericType)
        && !field.isSynthetic() && field.getDeclaredAnnotations().length == 0,
        "field metadata " + name);
  }

  private static void checkMethod(Method method, Class<?>[] parameters) {
    check(method.getModifiers() == Modifier.PUBLIC && method.getReturnType() == void.class
        && Arrays.equals(method.getParameterTypes(), parameters)
        && method.getExceptionTypes().length == 0 && method.getTypeParameters().length == 0
        && !method.isBridge() && !method.isSynthetic() && !method.isDefault()
        && !method.isVarArgs(), "method metadata " + method.getName());
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type, InvocationHandler handler) {
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] { type }, handler);
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == char.class) return '\0';
    if (type == byte.class) return (byte) 0;
    if (type == short.class) return (short) 0;
    if (type == int.class) return 0;
    if (type == long.class) return 0L;
    if (type == float.class) return 0.0F;
    if (type == double.class) return 0.0D;
    throw new AssertionError("unknown primitive " + type);
  }

  private static class DefaultHandler implements InvocationHandler {
    public Object invoke(Object proxy, Method method, Object[] arguments) {
      if (method.getName().equals("hashCode")) return System.identityHashCode(proxy);
      if (method.getName().equals("equals")) return proxy == arguments[0];
      return defaultValue(method.getReturnType());
    }
  }

  private static final class PlayerHandler extends DefaultHandler {
    final List<Long> thresholds = new ArrayList<>();

    public Object invoke(Object proxy, Method method, Object[] arguments) {
      if (method.getName().equals("checkCleanup")) {
        thresholds.add((Long) arguments[0]);
        return null;
      }
      return super.invoke(proxy, method, arguments);
    }
  }

  private static final class SchedulerHandler extends DefaultHandler {
    final List<TaskHandler> tasks = new ArrayList<>();
    Runnable expectedRunnable;

    public Object invoke(Object proxy, Method method, Object[] arguments) {
      if (method.getName().equals("scheduleAtFixedRate")) {
        check(arguments[0] == expectedRunnable && arguments[1].equals(10_000L)
            && arguments[2].equals(10_000L) && arguments[3] == TimeUnit.MILLISECONDS,
            "fixed-rate arguments");
        TaskHandler task = new TaskHandler();
        tasks.add(task);
        return GateAudioPlayerLifecycleManager.proxy(ScheduledFuture.class, task);
      }
      return super.invoke(proxy, method, arguments);
    }
  }

  private static final class TaskHandler extends DefaultHandler {
    final List<Boolean> cancelValues = new ArrayList<>();

    public Object invoke(Object proxy, Method method, Object[] arguments) {
      if (method.getName().equals("cancel")) {
        cancelValues.add((Boolean) arguments[0]);
        return true;
      }
      return super.invoke(proxy, method, arguments);
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const AUDIO_PLAYER_INTERFACE_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.filter.PcmFilterFactory;
import com.sedmelluq.discord.lavaplayer.player.AudioPlayer;
import com.sedmelluq.discord.lavaplayer.player.event.AudioEventListener;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameProvider;
import com.sedmelluq.discord.lavaplayer.track.playback.MutableAudioFrame;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.TimeUnit;

public final class GateAudioPlayerInterface {
  public static void main(String[] args) throws Exception {
    AudioTrack track = proxy(AudioTrack.class);
    PcmFilterFactory filterFactory = proxy(PcmFilterFactory.class);
    AudioEventListener listener = proxy(AudioEventListener.class);
    AudioFrame frame = proxy(AudioFrame.class);
    PlayerHandler calls = new PlayerHandler(track, filterFactory, listener, frame);
    AudioPlayer player = proxy(AudioPlayer.class, calls);

    check(player.getPlayingTrack() == track, "playing track return");
    player.playTrack(track);
    player.playTrack(null);
    check(player.startTrack(track, false), "start false return");
    check(!player.startTrack(null, true), "start true return");
    player.stopTrack();
    check(player.getVolume() == Integer.MIN_VALUE, "volume return width");
    player.setVolume(Integer.MIN_VALUE);
    player.setVolume(Integer.MAX_VALUE);
    player.setFilterFactory(filterFactory);
    player.setFilterFactory(null);
    player.setFrameBufferDuration(Integer.MAX_VALUE);
    player.setFrameBufferDuration(null);
    check(player.isPaused(), "paused return");
    player.setPaused(false);
    player.setPaused(true);
    player.destroy();
    player.addListener(listener);
    player.addListener(null);
    player.removeListener(listener);
    player.removeListener(null);
    player.checkCleanup(Long.MIN_VALUE);
    player.checkCleanup(Long.MAX_VALUE);

    check(player.provide() == frame, "inherited provide return");
    check(player.provide(Long.MIN_VALUE, TimeUnit.NANOSECONDS) == frame,
        "inherited timed provide return");
    check(player.provide((MutableAudioFrame) null), "inherited mutable provide return");
    check(!player.provide(null, Long.MAX_VALUE, TimeUnit.DAYS),
        "inherited timed mutable provide return");

    check(calls.names.equals(Arrays.asList(
        "get-track", "play", "play-null", "start-false", "start-null-true", "stop",
        "get-volume", "set-volume-min", "set-volume-max", "set-filter", "set-filter-null",
        "set-buffer-max", "set-buffer-null", "is-paused", "set-paused-false",
        "set-paused-true", "destroy", "add-listener", "add-listener-null",
        "remove-listener", "remove-listener-null", "cleanup-min", "cleanup-max",
        "provide", "provide-timed", "provide-mutable", "provide-mutable-timed")),
        "dispatch order");

    checkReflection();
    System.out.println(
        "dispatch=track,start,volume,filter,buffer,pause,listener,cleanup,inherited-frame;"
        + "values=identity,nulls,int-width,long-width;"
        + "reflection=interface,0-fields,14-methods,0-constructors");
  }

  private static void checkReflection() throws Exception {
    Class<AudioPlayer> type = AudioPlayer.class;
    int modifiers = type.getModifiers();
    check(Modifier.isPublic(modifiers) && Modifier.isInterface(modifiers)
        && Modifier.isAbstract(modifiers) && !Modifier.isFinal(modifiers)
        && type.getSuperclass() == null
        && Arrays.equals(type.getInterfaces(), new Class<?>[] { AudioFrameProvider.class })
        && type.getTypeParameters().length == 0 && type.getDeclaredAnnotations().length == 0,
        "interface structure");
    check(type.getDeclaredFields().length == 0 && type.getDeclaredMethods().length == 14
        && type.getDeclaredConstructors().length == 0 && type.getMethods().length == 18,
        "member counts");
    checkMethod(type.getDeclaredMethod("getPlayingTrack"), AudioTrack.class, new Class<?>[0]);
    checkMethod(type.getDeclaredMethod("playTrack", AudioTrack.class), void.class,
        new Class<?>[] { AudioTrack.class });
    checkMethod(type.getDeclaredMethod("startTrack", AudioTrack.class, boolean.class),
        boolean.class, new Class<?>[] { AudioTrack.class, boolean.class });
    checkMethod(type.getDeclaredMethod("stopTrack"), void.class, new Class<?>[0]);
    checkMethod(type.getDeclaredMethod("getVolume"), int.class, new Class<?>[0]);
    checkMethod(type.getDeclaredMethod("setVolume", int.class), void.class,
        new Class<?>[] { int.class });
    checkMethod(type.getDeclaredMethod("setFilterFactory", PcmFilterFactory.class), void.class,
        new Class<?>[] { PcmFilterFactory.class });
    checkMethod(type.getDeclaredMethod("setFrameBufferDuration", Integer.class), void.class,
        new Class<?>[] { Integer.class });
    checkMethod(type.getDeclaredMethod("isPaused"), boolean.class, new Class<?>[0]);
    checkMethod(type.getDeclaredMethod("setPaused", boolean.class), void.class,
        new Class<?>[] { boolean.class });
    checkMethod(type.getDeclaredMethod("destroy"), void.class, new Class<?>[0]);
    checkMethod(type.getDeclaredMethod("addListener", AudioEventListener.class), void.class,
        new Class<?>[] { AudioEventListener.class });
    checkMethod(type.getDeclaredMethod("removeListener", AudioEventListener.class), void.class,
        new Class<?>[] { AudioEventListener.class });
    checkMethod(type.getDeclaredMethod("checkCleanup", long.class), void.class,
        new Class<?>[] { long.class });

    checkInherited(type.getMethod("provide"), new Class<?>[0]);
    Method timed = type.getMethod("provide", long.class, TimeUnit.class);
    checkInherited(timed, new Class<?>[] { long.class, TimeUnit.class });
    check(Arrays.equals(timed.getExceptionTypes(),
        new Class<?>[] { java.util.concurrent.TimeoutException.class, InterruptedException.class }),
        "timed inherited exceptions");
    checkInherited(type.getMethod("provide", MutableAudioFrame.class),
        new Class<?>[] { MutableAudioFrame.class });
    Method mutableTimed = type.getMethod(
        "provide", MutableAudioFrame.class, long.class, TimeUnit.class);
    checkInherited(mutableTimed,
        new Class<?>[] { MutableAudioFrame.class, long.class, TimeUnit.class });
    check(Arrays.equals(mutableTimed.getExceptionTypes(),
        new Class<?>[] { java.util.concurrent.TimeoutException.class, InterruptedException.class }),
        "mutable timed inherited exceptions");
  }

  private static void checkMethod(Method method, Class<?> returnType, Class<?>[] parameters) {
    check(Modifier.isPublic(method.getModifiers()) && Modifier.isAbstract(method.getModifiers())
        && !Modifier.isStatic(method.getModifiers()) && !method.isDefault()
        && !method.isBridge() && !method.isSynthetic() && !method.isVarArgs()
        && method.getReturnType() == returnType
        && Arrays.equals(method.getParameterTypes(), parameters)
        && method.getExceptionTypes().length == 0 && method.getTypeParameters().length == 0
        && method.getDeclaredAnnotations().length == 0,
        "method metadata " + method.getName());
  }

  private static void checkInherited(Method method, Class<?>[] parameters) {
    check(method.getDeclaringClass() == AudioFrameProvider.class
        && Arrays.equals(method.getParameterTypes(), parameters),
        "inherited method " + method);
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type) {
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] { type },
        (instance, method, arguments) -> null);
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type, InvocationHandler handler) {
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] { type }, handler);
  }

  private static final class PlayerHandler implements InvocationHandler {
    final List<String> names = new java.util.ArrayList<>();
    final AudioTrack track;
    final PcmFilterFactory filterFactory;
    final AudioEventListener listener;
    final AudioFrame frame;

    PlayerHandler(AudioTrack track, PcmFilterFactory filterFactory,
        AudioEventListener listener, AudioFrame frame) {
      this.track = track;
      this.filterFactory = filterFactory;
      this.listener = listener;
      this.frame = frame;
    }

    public Object invoke(Object proxy, Method method, Object[] arguments) {
      switch (method.getName()) {
        case "getPlayingTrack":
          names.add("get-track");
          return track;
        case "playTrack":
          check(arguments[0] == track || arguments[0] == null, "play identity");
          names.add(arguments[0] == null ? "play-null" : "play");
          return null;
        case "startTrack":
          check(arguments[0] == track || arguments[0] == null, "start identity");
          boolean noInterrupt = (Boolean) arguments[1];
          names.add((arguments[0] == null ? "start-null-" : "start-") + noInterrupt);
          return !noInterrupt;
        case "stopTrack": names.add("stop"); return null;
        case "getVolume": names.add("get-volume"); return Integer.MIN_VALUE;
        case "setVolume":
          int volume = (Integer) arguments[0];
          names.add(volume == Integer.MIN_VALUE ? "set-volume-min" : "set-volume-max");
          return null;
        case "setFilterFactory":
          check(arguments[0] == filterFactory || arguments[0] == null, "filter identity");
          names.add(arguments[0] == null ? "set-filter-null" : "set-filter");
          return null;
        case "setFrameBufferDuration":
          check(arguments[0] == null || arguments[0].equals(Integer.MAX_VALUE), "buffer duration");
          names.add(arguments[0] == null ? "set-buffer-null" : "set-buffer-max");
          return null;
        case "isPaused": names.add("is-paused"); return true;
        case "setPaused":
          names.add((Boolean) arguments[0] ? "set-paused-true" : "set-paused-false");
          return null;
        case "destroy": names.add("destroy"); return null;
        case "addListener":
          check(arguments[0] == listener || arguments[0] == null, "add listener identity");
          names.add(arguments[0] == null ? "add-listener-null" : "add-listener");
          return null;
        case "removeListener":
          check(arguments[0] == listener || arguments[0] == null, "remove listener identity");
          names.add(arguments[0] == null ? "remove-listener-null" : "remove-listener");
          return null;
        case "checkCleanup":
          long threshold = (Long) arguments[0];
          names.add(threshold == Long.MIN_VALUE ? "cleanup-min" : "cleanup-max");
          return null;
        case "provide":
          return provide(method, arguments);
        default:
          throw new AssertionError("unexpected method " + method);
      }
    }

    private Object provide(Method method, Object[] arguments) {
      Class<?>[] parameters = method.getParameterTypes();
      if (parameters.length == 0) {
        names.add("provide");
        return frame;
      }
      if (parameters[0] == long.class) {
        check(arguments[0].equals(Long.MIN_VALUE) && arguments[1] == TimeUnit.NANOSECONDS,
            "timed provide arguments");
        names.add("provide-timed");
        return frame;
      }
      check(arguments[0] == null, "mutable frame null");
      if (parameters.length == 1) {
        names.add("provide-mutable");
        return true;
      }
      check(arguments[1].equals(Long.MAX_VALUE) && arguments[2] == TimeUnit.DAYS,
          "mutable timed provide arguments");
      names.add("provide-mutable-timed");
      return false;
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const AUDIO_PLAYER_MANAGER_INTERFACE_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.player.AudioConfiguration;
import com.sedmelluq.discord.lavaplayer.player.AudioLoadResultHandler;
import com.sedmelluq.discord.lavaplayer.player.AudioPlayer;
import com.sedmelluq.discord.lavaplayer.player.AudioPlayerManager;
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.tools.io.MessageInput;
import com.sedmelluq.discord.lavaplayer.tools.io.MessageOutput;
import com.sedmelluq.discord.lavaplayer.track.AudioItem;
import com.sedmelluq.discord.lavaplayer.track.AudioReference;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.DecodedTrackHolder;
import java.lang.invoke.MethodHandles;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.lang.reflect.Type;
import java.lang.reflect.TypeVariable;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.Future;
import java.util.function.Consumer;
import java.util.function.Function;

public final class GateAudioPlayerManagerInterface {
  public static void main(String[] args) throws Exception {
    AudioSourceManager firstSource = proxy(AudioSourceManager.class);
    AudioSourceManager secondSource = proxy(AudioSourceManager.class);
    AudioLoadResultHandler resultHandler = proxy(AudioLoadResultHandler.class);
    AudioTrack track = proxy(AudioTrack.class);
    AudioItem item = proxy(AudioItem.class);
    AudioPlayer player = proxy(AudioPlayer.class);
    AudioConfiguration configuration = new AudioConfiguration();
    DecodedTrackHolder decoded = new DecodedTrackHolder(track);
    Future<Void> asyncFuture = new CompletableFuture<>();
    Future<Void> orderedFuture = new CompletableFuture<>();
    List<AudioSourceManager> sources = Arrays.asList(firstSource, secondSource);
    MessageInput input = null;
    MessageOutput output = null;
    Object orderingKey = new Object();
    Function<Object, Object> requestConfigurator = value -> value;
    Consumer<Object> builderConfigurator = value -> { };

    ManagerHandler calls = new ManagerHandler(
        firstSource, secondSource, resultHandler, track, item, player, configuration, decoded,
        asyncFuture, orderedFuture, sources, input, output, orderingKey,
        requestConfigurator, builderConfigurator);
    AudioPlayerManager manager = proxy(AudioPlayerManager.class, calls);

    manager.registerSourceManagers(firstSource, null, secondSource);
    manager.registerSourceManagers();
    int registrations = calls.names.size();
    try {
      manager.registerSourceManagers((AudioSourceManager[]) null);
      throw new AssertionError("null source array accepted");
    } catch (NullPointerException expected) {
      check(calls.names.size() == registrations, "null array dispatched");
    }

    check(manager.loadItem("async-id", resultHandler) == asyncFuture,
        "async default return identity");
    manager.loadItemSync("sync-id", resultHandler);
    check(manager.loadItemSync("item-id") == item, "sync item default return identity");
    check(manager.loadItemOrdered(orderingKey, "ordered-id", resultHandler) == orderedFuture,
        "ordered default return identity");
    manager.loadItemSync((String) null, resultHandler);

    AudioReference direct = new AudioReference("direct-id", "direct-title");
    manager.shutdown();
    manager.enableGcMonitoring();
    manager.registerSourceManager(firstSource);
    check(manager.source(AudioSourceManager.class) == firstSource, "source return identity");
    check(manager.getSourceManagers() == sources, "source list return identity");
    check(manager.loadItem(direct, resultHandler) == asyncFuture, "direct async identity");
    manager.loadItemSync(direct, resultHandler);
    check(manager.loadItemSync(direct) == item, "direct item identity");
    check(manager.loadItemOrdered(orderingKey, direct, resultHandler) == orderedFuture,
        "direct ordered identity");
    manager.encodeTrack(output, track);
    check(manager.decodeTrack(input) == decoded, "decoded holder identity");
    check(manager.getConfiguration() == configuration, "configuration identity");
    check(manager.isUsingSeekGhosting(), "seek ghosting return");
    manager.setUseSeekGhosting(false);
    manager.setUseSeekGhosting(true);
    check(manager.getFrameBufferDuration() == Integer.MIN_VALUE, "duration return width");
    manager.setFrameBufferDuration(Integer.MIN_VALUE);
    manager.setFrameBufferDuration(Integer.MAX_VALUE);
    manager.setTrackStuckThreshold(Long.MIN_VALUE);
    manager.setTrackStuckThreshold(Long.MAX_VALUE);
    manager.setPlayerCleanupThreshold(Long.MIN_VALUE);
    manager.setPlayerCleanupThreshold(Long.MAX_VALUE);
    manager.setItemLoaderThreadPoolSize(Integer.MIN_VALUE);
    manager.setItemLoaderThreadPoolSize(Integer.MAX_VALUE);
    check(manager.createPlayer() == player, "player return identity");
    Method requestMethod = AudioPlayerManager.class.getDeclaredMethod(
        "setHttpRequestConfigurator", Function.class);
    Method builderMethod = AudioPlayerManager.class.getDeclaredMethod(
        "setHttpBuilderConfigurator", Consumer.class);
    requestMethod.invoke(manager, requestConfigurator);
    requestMethod.invoke(manager, new Object[] { null });
    builderMethod.invoke(manager, builderConfigurator);
    builderMethod.invoke(manager, new Object[] { null });

    check(calls.names.equals(Arrays.asList(
        "register-first", "register-null", "register-second",
        "load-async:async-id:null", "load-sync-handler:sync-id:null",
        "load-sync-item:item-id:null", "load-ordered:ordered-id:null",
        "load-sync-handler:null:null", "shutdown", "enable-gc", "register-first",
        "source", "get-sources", "load-async:direct-id:direct-title",
        "load-sync-handler:direct-id:direct-title",
        "load-sync-item:direct-id:direct-title",
        "load-ordered:direct-id:direct-title", "encode", "decode", "configuration",
        "get-seek", "set-seek-false", "set-seek-true", "get-duration",
        "set-duration-min", "set-duration-max", "track-stuck-min", "track-stuck-max",
        "cleanup-min", "cleanup-max", "loader-min", "loader-max", "create-player",
        "request", "request-null", "builder", "builder-null")), "dispatch order");

    checkReflection();
    System.out.println(
        "defaults=register-order,string-reference,identity-return,null-array;"
        + "dispatch=loads,serialization,configuration,thresholds,http;"
        + "reflection=interface,0-fields,27-methods,0-constructors");
  }

  private static void checkReflection() throws Exception {
    Class<AudioPlayerManager> type = AudioPlayerManager.class;
    int modifiers = type.getModifiers();
    check(Modifier.isPublic(modifiers) && Modifier.isInterface(modifiers)
        && Modifier.isAbstract(modifiers) && !Modifier.isFinal(modifiers)
        && type.getSuperclass() == null && type.getInterfaces().length == 0
        && type.getTypeParameters().length == 0 && type.getDeclaredAnnotations().length == 0,
        "interface structure");
    check(type.getDeclaredFields().length == 0 && type.getDeclaredMethods().length == 27
        && type.getDeclaredConstructors().length == 0, "member counts");

    checkAbstract(type.getDeclaredMethod("shutdown"), void.class, new Class<?>[0]);
    checkAbstract(type.getDeclaredMethod("enableGcMonitoring"), void.class, new Class<?>[0]);
    checkAbstract(type.getDeclaredMethod("registerSourceManager", AudioSourceManager.class),
        void.class, new Class<?>[] { AudioSourceManager.class });
    checkDefault(type.getDeclaredMethod("registerSourceManagers", AudioSourceManager[].class),
        void.class, new Class<?>[] { AudioSourceManager[].class }, true);

    Method source = type.getDeclaredMethod("source", Class.class);
    checkAbstract(source, AudioSourceManager.class, new Class<?>[] { Class.class });
    TypeVariable<Method>[] sourceVariables = source.getTypeParameters();
    check(sourceVariables.length == 1 && sourceVariables[0].getName().equals("T")
        && Arrays.equals(sourceVariables[0].getBounds(), new Type[] { AudioSourceManager.class })
        && source.getGenericReturnType() == sourceVariables[0]
        && source.getGenericParameterTypes()[0].getTypeName().equals("java.lang.Class<T>"),
        "source generic metadata");
    Method getSources = type.getDeclaredMethod("getSourceManagers");
    checkAbstract(getSources, List.class, new Class<?>[0]);
    check(getSources.getGenericReturnType().getTypeName().equals(
        "java.util.List<com.sedmelluq.discord.lavaplayer.source.AudioSourceManager>"),
        "source list generic metadata");

    Method loadString = type.getDeclaredMethod(
        "loadItem", String.class, AudioLoadResultHandler.class);
    checkDefault(loadString, Future.class,
        new Class<?>[] { String.class, AudioLoadResultHandler.class }, false);
    checkFutureVoid(loadString);
    Method loadReference = type.getDeclaredMethod(
        "loadItem", AudioReference.class, AudioLoadResultHandler.class);
    checkAbstract(loadReference, Future.class,
        new Class<?>[] { AudioReference.class, AudioLoadResultHandler.class });
    checkFutureVoid(loadReference);
    checkDefault(type.getDeclaredMethod(
        "loadItemSync", String.class, AudioLoadResultHandler.class), void.class,
        new Class<?>[] { String.class, AudioLoadResultHandler.class }, false);
    checkAbstract(type.getDeclaredMethod(
        "loadItemSync", AudioReference.class, AudioLoadResultHandler.class), void.class,
        new Class<?>[] { AudioReference.class, AudioLoadResultHandler.class });
    checkDefault(type.getDeclaredMethod("loadItemSync", String.class), AudioItem.class,
        new Class<?>[] { String.class }, false);
    checkAbstract(type.getDeclaredMethod("loadItemSync", AudioReference.class), AudioItem.class,
        new Class<?>[] { AudioReference.class });
    Method orderedString = type.getDeclaredMethod(
        "loadItemOrdered", Object.class, String.class, AudioLoadResultHandler.class);
    checkDefault(orderedString, Future.class,
        new Class<?>[] { Object.class, String.class, AudioLoadResultHandler.class }, false);
    checkFutureVoid(orderedString);
    Method orderedReference = type.getDeclaredMethod(
        "loadItemOrdered", Object.class, AudioReference.class, AudioLoadResultHandler.class);
    checkAbstract(orderedReference, Future.class,
        new Class<?>[] { Object.class, AudioReference.class, AudioLoadResultHandler.class });
    checkFutureVoid(orderedReference);

    Method encode = type.getDeclaredMethod("encodeTrack", MessageOutput.class, AudioTrack.class);
    checkAbstract(encode, void.class, new Class<?>[] { MessageOutput.class, AudioTrack.class });
    check(Arrays.equals(encode.getExceptionTypes(), new Class<?>[] { java.io.IOException.class }),
        "encode exceptions");
    Method decode = type.getDeclaredMethod("decodeTrack", MessageInput.class);
    checkAbstract(decode, DecodedTrackHolder.class, new Class<?>[] { MessageInput.class });
    check(Arrays.equals(decode.getExceptionTypes(), new Class<?>[] { java.io.IOException.class }),
        "decode exceptions");
    checkAbstract(type.getDeclaredMethod("getConfiguration"), AudioConfiguration.class,
        new Class<?>[0]);
    checkAbstract(type.getDeclaredMethod("isUsingSeekGhosting"), boolean.class, new Class<?>[0]);
    checkAbstract(type.getDeclaredMethod("setUseSeekGhosting", boolean.class), void.class,
        new Class<?>[] { boolean.class });
    checkAbstract(type.getDeclaredMethod("getFrameBufferDuration"), int.class, new Class<?>[0]);
    checkAbstract(type.getDeclaredMethod("setFrameBufferDuration", int.class), void.class,
        new Class<?>[] { int.class });
    checkAbstract(type.getDeclaredMethod("setTrackStuckThreshold", long.class), void.class,
        new Class<?>[] { long.class });
    checkAbstract(type.getDeclaredMethod("setPlayerCleanupThreshold", long.class), void.class,
        new Class<?>[] { long.class });
    checkAbstract(type.getDeclaredMethod("setItemLoaderThreadPoolSize", int.class), void.class,
        new Class<?>[] { int.class });
    checkAbstract(type.getDeclaredMethod("createPlayer"), AudioPlayer.class, new Class<?>[0]);
    checkAbstract(type.getDeclaredMethod("setHttpRequestConfigurator", Function.class), void.class,
        new Class<?>[] { Function.class });
    checkAbstract(type.getDeclaredMethod("setHttpBuilderConfigurator", Consumer.class), void.class,
        new Class<?>[] { Consumer.class });
  }

  private static void checkFutureVoid(Method method) {
    ParameterizedType type = (ParameterizedType) method.getGenericReturnType();
    check(type.getRawType() == Future.class
        && Arrays.equals(type.getActualTypeArguments(), new Type[] { Void.class }),
        "future generic metadata " + method);
  }

  private static void checkAbstract(Method method, Class<?> returnType, Class<?>[] parameters) {
    check(Modifier.isPublic(method.getModifiers()) && Modifier.isAbstract(method.getModifiers())
        && !Modifier.isStatic(method.getModifiers()) && !method.isDefault()
        && !method.isBridge() && !method.isSynthetic() && !method.isVarArgs()
        && method.getReturnType() == returnType
        && Arrays.equals(method.getParameterTypes(), parameters)
        && method.getDeclaredAnnotations().length == 0,
        "abstract metadata " + method);
  }

  private static void checkDefault(
      Method method, Class<?> returnType, Class<?>[] parameters, boolean varargs) {
    check(Modifier.isPublic(method.getModifiers()) && !Modifier.isAbstract(method.getModifiers())
        && !Modifier.isStatic(method.getModifiers()) && method.isDefault()
        && !method.isBridge() && !method.isSynthetic() && method.isVarArgs() == varargs
        && method.getReturnType() == returnType
        && Arrays.equals(method.getParameterTypes(), parameters)
        && method.getExceptionTypes().length == 0 && method.getTypeParameters().length == 0
        && method.getDeclaredAnnotations().length == 0,
        "default metadata " + method);
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type) {
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] { type },
        (instance, method, arguments) -> null);
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type, InvocationHandler handler) {
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] { type }, handler);
  }

  private static final class ManagerHandler implements InvocationHandler {
    final List<String> names = new java.util.ArrayList<>();
    final AudioSourceManager firstSource;
    final AudioSourceManager secondSource;
    final AudioLoadResultHandler resultHandler;
    final AudioTrack track;
    final AudioItem item;
    final AudioPlayer player;
    final AudioConfiguration configuration;
    final DecodedTrackHolder decoded;
    final Future<Void> asyncFuture;
    final Future<Void> orderedFuture;
    final List<AudioSourceManager> sources;
    final MessageInput input;
    final MessageOutput output;
    final Object orderingKey;
    final Function<Object, Object> requestConfigurator;
    final Consumer<Object> builderConfigurator;

    ManagerHandler(AudioSourceManager firstSource, AudioSourceManager secondSource,
        AudioLoadResultHandler resultHandler, AudioTrack track, AudioItem item, AudioPlayer player,
        AudioConfiguration configuration, DecodedTrackHolder decoded, Future<Void> asyncFuture,
        Future<Void> orderedFuture, List<AudioSourceManager> sources, MessageInput input,
        MessageOutput output, Object orderingKey, Function<Object, Object> requestConfigurator,
        Consumer<Object> builderConfigurator) {
      this.firstSource = firstSource;
      this.secondSource = secondSource;
      this.resultHandler = resultHandler;
      this.track = track;
      this.item = item;
      this.player = player;
      this.configuration = configuration;
      this.decoded = decoded;
      this.asyncFuture = asyncFuture;
      this.orderedFuture = orderedFuture;
      this.sources = sources;
      this.input = input;
      this.output = output;
      this.orderingKey = orderingKey;
      this.requestConfigurator = requestConfigurator;
      this.builderConfigurator = builderConfigurator;
    }

    public Object invoke(Object proxy, Method method, Object[] arguments) throws Throwable {
      if (method.isDefault()) {
        MethodHandles.Lookup lookup = MethodHandles.privateLookupIn(
            method.getDeclaringClass(), MethodHandles.lookup());
        Object[] values = arguments == null ? new Object[0] : arguments;
        return lookup.unreflectSpecial(method, method.getDeclaringClass())
            .bindTo(proxy).invokeWithArguments(values);
      }
      switch (method.getName()) {
        case "shutdown": names.add("shutdown"); return null;
        case "enableGcMonitoring": names.add("enable-gc"); return null;
        case "registerSourceManager":
          Object source = arguments[0];
          check(source == firstSource || source == secondSource || source == null,
              "registered source identity");
          names.add(source == firstSource ? "register-first"
              : source == secondSource ? "register-second" : "register-null");
          return null;
        case "source":
          check(arguments[0] == AudioSourceManager.class, "source class identity");
          names.add("source");
          return firstSource;
        case "getSourceManagers": names.add("get-sources"); return sources;
        case "loadItem":
          check(arguments[1] == resultHandler, "async handler identity");
          names.add(referenceName("load-async", (AudioReference) arguments[0]));
          return asyncFuture;
        case "loadItemSync":
          if (arguments.length == 2) {
            check(arguments[1] == resultHandler, "sync handler identity");
            names.add(referenceName("load-sync-handler", (AudioReference) arguments[0]));
            return null;
          }
          names.add(referenceName("load-sync-item", (AudioReference) arguments[0]));
          return item;
        case "loadItemOrdered":
          check(arguments[0] == orderingKey && arguments[2] == resultHandler,
              "ordered argument identity");
          names.add(referenceName("load-ordered", (AudioReference) arguments[1]));
          return orderedFuture;
        case "encodeTrack":
          check(arguments[0] == output && arguments[1] == track, "encode identity");
          names.add("encode"); return null;
        case "decodeTrack":
          check(arguments[0] == input, "decode identity");
          names.add("decode"); return decoded;
        case "getConfiguration": names.add("configuration"); return configuration;
        case "isUsingSeekGhosting": names.add("get-seek"); return true;
        case "setUseSeekGhosting":
          names.add((Boolean) arguments[0] ? "set-seek-true" : "set-seek-false");
          return null;
        case "getFrameBufferDuration": names.add("get-duration"); return Integer.MIN_VALUE;
        case "setFrameBufferDuration":
          names.add(((Integer) arguments[0]) == Integer.MIN_VALUE
              ? "set-duration-min" : "set-duration-max");
          return null;
        case "setTrackStuckThreshold":
          names.add(((Long) arguments[0]) == Long.MIN_VALUE
              ? "track-stuck-min" : "track-stuck-max");
          return null;
        case "setPlayerCleanupThreshold":
          names.add(((Long) arguments[0]) == Long.MIN_VALUE ? "cleanup-min" : "cleanup-max");
          return null;
        case "setItemLoaderThreadPoolSize":
          names.add(((Integer) arguments[0]) == Integer.MIN_VALUE ? "loader-min" : "loader-max");
          return null;
        case "createPlayer": names.add("create-player"); return player;
        case "setHttpRequestConfigurator":
          check(arguments[0] == requestConfigurator || arguments[0] == null,
              "request configurator identity");
          names.add(arguments[0] == null ? "request-null" : "request");
          return null;
        case "setHttpBuilderConfigurator":
          check(arguments[0] == builderConfigurator || arguments[0] == null,
              "builder configurator identity");
          names.add(arguments[0] == null ? "builder-null" : "builder");
          return null;
        default: throw new AssertionError("unexpected manager method " + method);
      }
    }

    private static String referenceName(String prefix, AudioReference reference) {
      check(reference != null, prefix + " reference null");
      return prefix + ":" + reference.identifier + ":" + reference.title;
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const DEFAULT_AUDIO_PLAYER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.filter.PcmFilterFactory;
import com.sedmelluq.discord.lavaplayer.player.AudioConfiguration;
import com.sedmelluq.discord.lavaplayer.player.AudioPlayerOptions;
import com.sedmelluq.discord.lavaplayer.player.DefaultAudioPlayer;
import com.sedmelluq.discord.lavaplayer.player.DefaultAudioPlayerManager;
import com.sedmelluq.discord.lavaplayer.player.event.AudioEvent;
import com.sedmelluq.discord.lavaplayer.player.event.AudioEventListener;
import com.sedmelluq.discord.lavaplayer.player.event.PlayerPauseEvent;
import com.sedmelluq.discord.lavaplayer.player.event.PlayerResumeEvent;
import com.sedmelluq.discord.lavaplayer.player.event.TrackEndEvent;
import com.sedmelluq.discord.lavaplayer.player.event.TrackExceptionEvent;
import com.sedmelluq.discord.lavaplayer.player.event.TrackStartEvent;
import com.sedmelluq.discord.lavaplayer.player.event.TrackStuckEvent;
import com.sedmelluq.discord.lavaplayer.tools.FriendlyException;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.InternalAudioTrack;
import com.sedmelluq.discord.lavaplayer.track.TrackStateListener;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioTrackExecutor;
import com.sedmelluq.discord.lavaplayer.track.playback.MutableAudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.TerminatorAudioFrame;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;

public final class GateDefaultAudioPlayer {
  public static void main(String[] args) throws Exception {
    TestManager manager = allocate(TestManager.class);
    manager.configuration = new AudioConfiguration();
    manager.thresholdNanos = -1L;
    DefaultAudioPlayer player = new DefaultAudioPlayer(manager);
    check(player.getPlayingTrack() == null && player.getVolume() == 100 && !player.isPaused(),
        "constructor defaults");

    List<String> events = new ArrayList<>();
    AudioEventListener recorder = event -> events.add(eventName(player, event));
    AudioEventListener throwing = event -> { throw new IllegalStateException("listener"); };
    AudioEventListener duplicate = event -> events.add("duplicate");
    player.addListener(throwing);
    player.addListener(recorder);
    player.addListener(duplicate);
    player.addListener(duplicate);
    player.removeListener(duplicate);
    player.setPaused(true);
    player.setPaused(true);
    player.setPaused(false);
    player.setPaused(false);
    check(events.equals(Arrays.asList("pause", "resume")), "pause event transitions");
    check(!player.isPaused(), "resume state");

    PcmFilterFactory filter = proxy(PcmFilterFactory.class, null);
    player.setVolume(Integer.MIN_VALUE);
    check(player.getVolume() == 0, "volume lower clamp");
    player.setVolume(Integer.MAX_VALUE);
    check(player.getVolume() == 1000, "volume upper clamp");
    player.setVolume(321);
    player.setFilterFactory(filter);
    player.setFrameBufferDuration(-1000);

    TrackHandler firstCalls = new TrackHandler("first");
    TrackHandler secondCalls = new TrackHandler("second");
    InternalAudioTrack first = track(firstCalls);
    InternalAudioTrack second = track(secondCalls);
    try {
      player.startTrack(proxy(AudioTrack.class, null), false);
      throw new AssertionError("non-internal track accepted");
    } catch (ClassCastException expected) {
      check(player.getPlayingTrack() == null, "invalid track mutated state");
    }

    check(player.startTrack(first, false), "first start");
    check(player.getPlayingTrack() == first && manager.executions == 1
        && manager.lastTrack == first && manager.lastOptions.volumeLevel.get() == 321
        && manager.lastOptions.filterFactory.get() == filter
        && manager.lastOptions.frameBufferDuration.get().equals(200), "first execution state");
    player.setFrameBufferDuration(null);
    check(!player.startTrack(second, true) && player.getPlayingTrack() == first
        && manager.executions == 1, "no-interrupt rejection");
    check(player.startTrack(second, false) && firstCalls.stops == 1
        && manager.executions == 2 && manager.lastOptions.frameBufferDuration.get() == null,
        "replacement");

    AudioFrame shadowFrame = frame(false);
    firstCalls.immediate = shadowFrame;
    secondCalls.immediate = null;
    check(player.provide() == shadowFrame, "shadow frame");
    AudioFrame timedFrame = frame(false);
    secondCalls.timed = timedFrame;
    check(player.provide(Long.MAX_VALUE, TimeUnit.DAYS) == timedFrame
        && secondCalls.lastTimeout == Long.MAX_VALUE && secondCalls.lastUnit == TimeUnit.DAYS,
        "timed frame");
    MutableAudioFrame mutable = new MutableAudioFrame();
    secondCalls.mutableImmediate = true;
    check(player.provide(mutable), "mutable frame");
    secondCalls.mutableTimed = true;
    check(player.provide(mutable, Long.MIN_VALUE, TimeUnit.NANOSECONDS),
        "negative-timeout mutable frame");

    player.stopTrack();
    check(player.getPlayingTrack() == null && secondCalls.stops == 1, "stop");
    player.playTrack(first);
    player.destroy();
    check(firstCalls.stops == 2, "destroy");
    player.playTrack(second);
    player.checkCleanup(Long.MIN_VALUE);
    check(secondCalls.stops == 2 && player.getPlayingTrack() == null, "cleanup");

    FriendlyException failure = allocate(FriendlyException.class);
    player.onTrackException(first, failure);
    player.onTrackStuck(first, Long.MAX_VALUE);
    player.playTrack(first);
    firstCalls.immediate = null;
    check(player.provide() == null, "stuck provide");
    check(player.provide() == null, "stuck event once");
    player.stopTrack();

    check(events.equals(Arrays.asList(
        "pause", "resume", "start:first", "end:first:REPLACED", "start:second",
        "stuck:second:0", "end:second:STOPPED", "start:first", "end:first:STOPPED", "start:second",
        "end:second:CLEANUP", "exception:first", "stuck:first:" + Long.MAX_VALUE,
        "start:first", "stuck:first:0", "end:first:STOPPED")), "event order: " + events);

    List<String> terminalReasons = new ArrayList<>();
    DefaultAudioPlayer terminalPlayer = new DefaultAudioPlayer(manager);
    terminalPlayer.addListener(event -> {
      if (event instanceof TrackEndEvent) {
        terminalReasons.add(((TrackEndEvent) event).endReason.name());
      }
    });
    TrackHandler finishedCalls = new TrackHandler("finished");
    finishedCalls.immediate = TerminatorAudioFrame.INSTANCE;
    InternalAudioTrack finished = track(finishedCalls);
    terminalPlayer.playTrack(finished);
    check(terminalPlayer.provide() == null && terminalPlayer.getPlayingTrack() == null
        && finishedCalls.stops == 1, "finished terminator");
    TrackHandler failedCalls = new TrackHandler("failed");
    failedCalls.failedBeforeLoad = true;
    failedCalls.immediate = TerminatorAudioFrame.INSTANCE;
    InternalAudioTrack failed = track(failedCalls);
    terminalPlayer.playTrack(failed);
    check(terminalPlayer.provide() == null && terminalPlayer.getPlayingTrack() == null
        && failedCalls.stops == 1 && terminalReasons.equals(Arrays.asList("FINISHED", "LOAD_FAILED")),
        "failed terminator: " + terminalReasons);

    DefaultAudioPlayer nullManager = new DefaultAudioPlayer(null);
    check(nullManager.getVolume() == 100 && nullManager.getPlayingTrack() == null,
        "nullable manager construction");
    check(!nullManager.startTrack(null, false), "null track stop");
    checkReflection();
    System.out.println(
        "state=defaults,clamps,pause,replace,stop,destroy,cleanup;"
        + "frames=shadow,timed,mutable,stuck;events=ordered,isolated,identity;"
        + "reflection=class,0-fields,21-methods,1-constructor");
  }

  private static String eventName(DefaultAudioPlayer player, AudioEvent event) {
    check(event.player == player, "event player identity");
    if (event instanceof PlayerPauseEvent) return "pause";
    if (event instanceof PlayerResumeEvent) return "resume";
    if (event instanceof TrackStartEvent) {
      return "start:" + ((TrackHandler) Proxy.getInvocationHandler(((TrackStartEvent) event).track)).name;
    }
    if (event instanceof TrackEndEvent) {
      TrackEndEvent value = (TrackEndEvent) event;
      return "end:" + ((TrackHandler) Proxy.getInvocationHandler(value.track)).name
          + ":" + value.endReason;
    }
    if (event instanceof TrackExceptionEvent) {
      TrackExceptionEvent value = (TrackExceptionEvent) event;
      check(value.exception != null, "exception identity");
      return "exception:" + ((TrackHandler) Proxy.getInvocationHandler(value.track)).name;
    }
    if (event instanceof TrackStuckEvent) {
      TrackStuckEvent value = (TrackStuckEvent) event;
      check(value.stackTrace == null, "stuck stack trace");
      return "stuck:" + ((TrackHandler) Proxy.getInvocationHandler(value.track)).name
          + ":" + value.thresholdMs;
    }
    throw new AssertionError("unexpected event " + event);
  }

  private static void checkReflection() throws Exception {
    Class<DefaultAudioPlayer> type = DefaultAudioPlayer.class;
    int modifiers = type.getModifiers();
    check(Modifier.isPublic(modifiers) && !Modifier.isAbstract(modifiers)
        && !Modifier.isFinal(modifiers) && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {
            com.sedmelluq.discord.lavaplayer.player.AudioPlayer.class,
            TrackStateListener.class }) && type.getFields().length == 0
        && Arrays.stream(type.getDeclaredMethods()).filter(method -> Modifier.isPublic(
            method.getModifiers())).count() == 20 && type.getDeclaredConstructors().length == 1,
        "class structure: " + Modifier.toString(modifiers) + ",fields=" + type.getFields().length
            + ",methods=" + Arrays.stream(type.getDeclaredMethods()).filter(method ->
                Modifier.isPublic(method.getModifiers())).count() + ",constructors="
            + type.getDeclaredConstructors().length + ",interfaces="
            + Arrays.toString(type.getInterfaces()));
    Method timed = type.getDeclaredMethod("provide", long.class, TimeUnit.class);
    Method mutableTimed = type.getDeclaredMethod(
        "provide", MutableAudioFrame.class, long.class, TimeUnit.class);
    check(Arrays.equals(timed.getExceptionTypes(), new Class<?>[] {
        TimeoutException.class, InterruptedException.class })
        && Arrays.equals(mutableTimed.getExceptionTypes(), new Class<?>[] {
            TimeoutException.class, InterruptedException.class }), "checked exceptions");
    for (Method method : type.getDeclaredMethods()) {
      if (!Modifier.isPublic(method.getModifiers())) continue;
      check(Modifier.isPublic(method.getModifiers()) && !Modifier.isStatic(method.getModifiers())
          && !Modifier.isAbstract(method.getModifiers()), "method modifiers " + method);
    }
  }

  private static InternalAudioTrack track(TrackHandler handler) {
    handler.executor = proxy(AudioTrackExecutor.class,
        (instance, method, arguments) -> method.getName().equals("failedBeforeLoad")
            ? handler.failedBeforeLoad
            : defaultValue(method.getReturnType()));
    return proxy(InternalAudioTrack.class, handler);
  }

  private static AudioFrame frame(boolean terminator) {
    return proxy(AudioFrame.class, (instance, method, arguments) ->
        method.getName().equals("isTerminator") ? terminator : defaultValue(method.getReturnType()));
  }

  private static final class TrackHandler implements java.lang.reflect.InvocationHandler {
    final String name;
    AudioFrame immediate;
    AudioFrame timed;
    boolean mutableImmediate;
    boolean mutableTimed;
    long lastTimeout;
    TimeUnit lastUnit;
    int stops;
    AudioTrackExecutor executor;
    boolean failedBeforeLoad;

    TrackHandler(String name) { this.name = name; }

    public Object invoke(Object instance, Method method, Object[] arguments) {
      if (method.getName().equals("stop")) { stops++; return null; }
      if (method.getName().equals("getActiveExecutor")) return executor;
      if (method.getName().equals("provide")) {
        int count = arguments == null ? 0 : arguments.length;
        if (count == 0) return immediate;
        if (arguments[0] instanceof MutableAudioFrame) {
          if (count == 1) return mutableImmediate;
          lastTimeout = (Long) arguments[1]; lastUnit = (TimeUnit) arguments[2];
          return mutableTimed;
        }
        lastTimeout = (Long) arguments[0]; lastUnit = (TimeUnit) arguments[1];
        return timed;
      }
      if (method.getName().equals("toString")) return name;
      return defaultValue(method.getReturnType());
    }
  }

  public static class TestManager extends DefaultAudioPlayerManager {
    AudioConfiguration configuration;
    long thresholdNanos;
    int executions;
    InternalAudioTrack lastTrack;
    AudioPlayerOptions lastOptions;

    public AudioConfiguration getConfiguration() { return configuration; }
    public long getTrackStuckThresholdNanos() { return thresholdNanos; }
    public void executeTrack(TrackStateListener listener, InternalAudioTrack track,
        AudioConfiguration configuration, AudioPlayerOptions options) {
      check(listener instanceof DefaultAudioPlayer && configuration == this.configuration,
          "execute identities");
      executions++; lastTrack = track; lastOptions = options;
    }
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type, java.lang.reflect.InvocationHandler handler) {
    java.lang.reflect.InvocationHandler actual = handler == null
        ? (instance, method, arguments) -> defaultValue(method.getReturnType()) : handler;
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] { type }, actual);
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == byte.class) return (byte) 0;
    if (type == short.class) return (short) 0;
    if (type == int.class) return 0;
    if (type == long.class) return 0L;
    if (type == float.class) return 0.0f;
    if (type == double.class) return 0.0d;
    if (type == char.class) return (char) 0;
    return null;
  }

  private static <T> T allocate(Class<T> type) throws Exception {
    Class<?> unsafeType = Class.forName("sun.misc.Unsafe");
    Field singleton = unsafeType.getDeclaredField("theUnsafe");
    singleton.setAccessible(true);
    Object unsafe = singleton.get(null);
    return type.cast(unsafeType.getMethod("allocateInstance", Class.class).invoke(unsafe, type));
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const DEFAULT_AUDIO_PLAYER_MANAGER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.player.AudioConfiguration;
import com.sedmelluq.discord.lavaplayer.player.AudioPlayer;
import com.sedmelluq.discord.lavaplayer.player.AudioPlayerOptions;
import com.sedmelluq.discord.lavaplayer.player.DefaultAudioPlayer;
import com.sedmelluq.discord.lavaplayer.player.DefaultAudioPlayerManager;
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpConfigurable;
import com.sedmelluq.discord.lavaplayer.tools.io.MessageInput;
import com.sedmelluq.discord.lavaplayer.tools.io.MessageOutput;
import com.sedmelluq.discord.lavaplayer.track.AudioItem;
import com.sedmelluq.discord.lavaplayer.track.AudioReference;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import com.sedmelluq.discord.lavaplayer.track.DecodedTrackHolder;
import com.sedmelluq.discord.lavaplayer.track.InternalAudioTrack;
import com.sedmelluq.discord.lavaplayer.track.TrackStateListener;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioTrackExecutor;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.DataInput;
import java.io.DataOutput;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;
import java.util.Base64;
import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;
import java.util.function.Consumer;
import java.util.function.Function;

@SuppressWarnings({"rawtypes", "unchecked"})
public final class GateDefaultAudioPlayerManager {
  public static void main(String[] args) throws Exception {
    if (args.length != 0) System.load(args[0]);
    ExposedManager manager = new ExposedManager();

    AudioConfiguration configuration = manager.getConfiguration();
    check(configuration != null && manager.getConfiguration() == configuration,
        "configuration identity");
    check(manager.isUsingSeekGhosting() && manager.getFrameBufferDuration() == 5000
        && manager.getTrackStuckThresholdNanos() == TimeUnit.MILLISECONDS.toNanos(10000),
        "constructor defaults");
    manager.setUseSeekGhosting(false);
    manager.setFrameBufferDuration(Integer.MIN_VALUE);
    manager.setTrackStuckThreshold(Long.MAX_VALUE);
    check(!manager.isUsingSeekGhosting() && manager.getFrameBufferDuration() == 200
        && manager.getTrackStuckThresholdNanos()
            == TimeUnit.MILLISECONDS.toNanos(Long.MAX_VALUE), "mutable settings");
    manager.setFrameBufferDuration(321);
    manager.setTrackStuckThreshold(Long.MIN_VALUE);
    manager.setPlayerCleanupThreshold(Long.MIN_VALUE);
    check(manager.getFrameBufferDuration() == 321
        && manager.getTrackStuckThresholdNanos()
            == TimeUnit.MILLISECONDS.toNanos(Long.MIN_VALUE), "setting edges");

    AtomicInteger requestCalls = new AtomicInteger();
    AtomicInteger builderCalls = new AtomicInteger();
    AtomicInteger shutdownCalls = new AtomicInteger();
    AtomicInteger encodeCalls = new AtomicInteger();
    AtomicInteger decodeCalls = new AtomicInteger();
    AtomicLong decodedPosition = new AtomicLong(Long.MIN_VALUE);
    Function requestConfigurator = value -> value;
    Consumer builderConfigurator = value -> {};
    SourceHandler firstCalls = new SourceHandler(
        "oracle", requestConfigurator, builderConfigurator, requestCalls, builderCalls,
        shutdownCalls, encodeCalls, decodeCalls, decodedPosition);
    AudioSourceManager first = source(firstCalls);
    manager.setHttpRequestConfigurator(requestConfigurator);
    manager.setHttpBuilderConfigurator(builderConfigurator);
    manager.registerSourceManager(first);
    check(requestCalls.get() == 1 && builderCalls.get() == 1, "registration configures HTTP");
    manager.setHttpRequestConfigurator(requestConfigurator);
    manager.setHttpBuilderConfigurator(builderConfigurator);
    check(requestCalls.get() == 2 && builderCalls.get() == 2, "existing HTTP reconfigured");
    manager.setHttpRequestConfigurator(null);
    manager.setHttpBuilderConfigurator(null);

    AtomicInteger secondRequests = new AtomicInteger();
    AtomicInteger secondBuilders = new AtomicInteger();
    AudioSourceManager second = source(new SourceHandler(
        "second", requestConfigurator, builderConfigurator, secondRequests, secondBuilders,
        shutdownCalls, encodeCalls, decodeCalls, decodedPosition));
    manager.registerSourceManager(second);
    List<AudioSourceManager> sources = manager.getSourceManagers();
    check(sources.size() == 2 && sources.get(0) == first && sources.get(1) == second
        && manager.source(AudioSourceManager.class) == first
        && manager.source(first.getClass()) == first
        && secondRequests.get() == 0 && secondBuilders.get() == 0, "source registry");
    try {
      sources.clear();
      throw new AssertionError("source view mutable");
    } catch (UnsupportedOperationException expected) {}

    AudioTrackInfo info = new AudioTrackInfo(
        "title", "author", 123456789L, "identifier", false,
        "https://example.invalid/audio", null, "USABC1234567");
    AudioTrack encoded = track(info, first, 987654321L, null);
    firstCalls.loadedTrack = encoded;
    check(manager.loadItemSync(new AudioReference("identifier", "title")) == encoded,
        "source track load");
    byte[] details = manager.encodeTrackDetails(encoded);
    check(encodeCalls.get() == 1, "details encoded");
    AudioTrack detailsDecoded = manager.decodeTrackDetails(info, details);
    check(detailsDecoded != null && decodeCalls.get() == 1, "details decoded");

    ByteArrayOutputStream bytes = new ByteArrayOutputStream();
    MessageOutput output = new MessageOutput(bytes);
    manager.encodeTrack(output, encoded);
    output.finish();
    MessageInput input = new MessageInput(new ByteArrayInputStream(bytes.toByteArray()));
    DecodedTrackHolder holder = manager.decodeTrack(input);
    check(holder != null && holder.decodedTrack != null && decodedPosition.get() == 987654321L
        && manager.decodeTrack(input) == null, "track envelope round trip");

    CountDownLatch executed = new CountDownLatch(1);
    AtomicInteger assignments = new AtomicInteger();
    AudioTrackExecutor executor = proxy(AudioTrackExecutor.class,
        (instance, method, arguments) -> {
          if (method.getName().equals("execute")) { executed.countDown(); return null; }
          return defaultValue(method.getReturnType());
        });
    InternalAudioTrack executable = (InternalAudioTrack) track(
        info, first, 0L, (instance, method, arguments) -> {
          if (method.getName().equals("createLocalExecutor")) return executor;
          if (method.getName().equals("assignExecutor")) {
            check(arguments[0] == executor && Boolean.TRUE.equals(arguments[1]),
                "executor assignment values");
            assignments.incrementAndGet();
            return null;
          }
          return defaultValue(method.getReturnType());
        });
    TrackStateListener listener = proxy(TrackStateListener.class, null);
    manager.executeTrack(listener, executable, configuration, new AudioPlayerOptions());
    check(executed.await(5, TimeUnit.SECONDS) && assignments.get() == 1,
        "custom executor dispatch");

    ExecutorService playback = manager.getExecutor();
    check(playback != null && playback == manager.getExecutor() && !playback.isShutdown(),
        "executor identity");
    manager.setItemLoaderThreadPoolSize(2);
    manager.setItemLoaderThreadPoolSize(1);
    try {
      manager.setItemLoaderThreadPoolSize(0);
      throw new AssertionError("zero loader pool accepted");
    } catch (IllegalArgumentException expected) {}
    manager.enableGcMonitoring();

    AudioPlayer created = manager.createPlayer();
    AudioPlayer constructed = manager.exposeConstructPlayer();
    check(created.getClass() == DefaultAudioPlayer.class
        && constructed.getClass() == DefaultAudioPlayer.class
        && created != constructed, "player construction");
    checkReflection();

    String envelope = Base64.getEncoder().encodeToString(bytes.toByteArray());
    manager.shutdown();
    check(shutdownCalls.get() == 2 && playback.isShutdown(),
        "shutdown cascade:" + shutdownCalls.get() + ":" + playback.isShutdown());
    System.out.println(
        "state=defaults,identity,clamps,thresholds;source=ordered,http,readonly;"
        + "serialization=details,envelope:" + envelope + ";"
        + "execution=custom,async;player=default;shutdown=cascade;reflection=exact");
  }

  private static void checkReflection() throws Exception {
    Class<DefaultAudioPlayerManager> type = DefaultAudioPlayerManager.class;
    check(Modifier.isPublic(type.getModifiers()) && !Modifier.isFinal(type.getModifiers())
        && !Modifier.isAbstract(type.getModifiers()) && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {
            com.sedmelluq.discord.lavaplayer.player.AudioPlayerManager.class })
        && type.getFields().length == 0 && type.getDeclaredConstructors().length == 1
        && Arrays.stream(type.getDeclaredMethods()).filter(method -> method.getModifiers()
            != 0 && (Modifier.isPublic(method.getModifiers())
                || Modifier.isProtected(method.getModifiers()))).count() == 28, "class structure");
    Method construct = type.getDeclaredMethod("constructPlayer");
    check(Modifier.isProtected(construct.getModifiers()) && !Modifier.isFinal(construct.getModifiers())
        && construct.getReturnType() == AudioPlayer.class, "constructPlayer metadata");
    check(Arrays.equals(type.getDeclaredMethod("encodeTrack", MessageOutput.class, AudioTrack.class)
        .getExceptionTypes(), new Class<?>[] { java.io.IOException.class })
        && Arrays.equals(type.getDeclaredMethod("decodeTrack", MessageInput.class)
        .getExceptionTypes(), new Class<?>[] { java.io.IOException.class }), "checked exceptions");
  }

  public static final class ExposedManager extends DefaultAudioPlayerManager {
    AudioPlayer exposeConstructPlayer() { return constructPlayer(); }
  }

  private static final class SourceHandler implements java.lang.reflect.InvocationHandler {
    final String name;
    final Function requestConfigurator;
    final Consumer builderConfigurator;
    final AtomicInteger requestCalls;
    final AtomicInteger builderCalls;
    final AtomicInteger shutdownCalls;
    final AtomicInteger encodeCalls;
    final AtomicInteger decodeCalls;
    final AtomicLong decodedPosition;
    AudioTrack loadedTrack;

    SourceHandler(String name, Function requestConfigurator,
        Consumer builderConfigurator, AtomicInteger requestCalls,
        AtomicInteger builderCalls, AtomicInteger shutdownCalls, AtomicInteger encodeCalls,
        AtomicInteger decodeCalls, AtomicLong decodedPosition) {
      this.name = name;
      this.requestConfigurator = requestConfigurator;
      this.builderConfigurator = builderConfigurator;
      this.requestCalls = requestCalls;
      this.builderCalls = builderCalls;
      this.shutdownCalls = shutdownCalls;
      this.encodeCalls = encodeCalls;
      this.decodeCalls = decodeCalls;
      this.decodedPosition = decodedPosition;
    }

    public Object invoke(Object instance, Method method, Object[] arguments) throws Exception {
      switch (method.getName()) {
        case "getSourceName": return name;
        case "configureRequests":
          check(arguments[0] == requestConfigurator, "request configurator identity");
          requestCalls.incrementAndGet(); return null;
        case "configureBuilder":
          check(arguments[0] == builderConfigurator, "builder configurator identity");
          builderCalls.incrementAndGet(); return null;
        case "encodeTrack":
          encodeCalls.incrementAndGet(); ((DataOutput) arguments[1]).writeInt(0x1234ABCD); return null;
        case "isTrackEncodable": return true;
        case "decodeTrack":
          check(((DataInput) arguments[1]).readInt() == 0x1234ABCD, "source payload");
          decodeCalls.incrementAndGet();
          return track((AudioTrackInfo) arguments[0], (AudioSourceManager) instance, 0L,
              (track, trackMethod, trackArguments) -> {
                if (trackMethod.getName().equals("setPosition")) {
                  decodedPosition.set((Long) trackArguments[0]); return null;
                }
                return defaultValue(trackMethod.getReturnType());
              });
        case "loadItem": return loadedTrack;
        case "shutdown": shutdownCalls.incrementAndGet(); return null;
        case "toString": return name;
        default: return defaultValue(method.getReturnType());
      }
    }
  }

  private static AudioSourceManager source(SourceHandler handler) {
    return (AudioSourceManager) Proxy.newProxyInstance(
        AudioSourceManager.class.getClassLoader(),
        new Class<?>[] { AudioSourceManager.class, HttpConfigurable.class }, handler);
  }

  private static AudioTrack track(AudioTrackInfo info, AudioSourceManager source, long position,
      java.lang.reflect.InvocationHandler override) {
    return proxy(InternalAudioTrack.class, (instance, method, arguments) -> {
      if (override != null) {
        Object result = override.invoke(instance, method, arguments);
        if (result != null || method.getReturnType() == void.class) return result;
      }
      switch (method.getName()) {
        case "getInfo": return info;
        case "getSourceManager": return source;
        case "getPosition": return position;
        case "makeClone": return instance;
        case "toString": return "track";
        default: return defaultValue(method.getReturnType());
      }
    });
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type, java.lang.reflect.InvocationHandler handler) {
    java.lang.reflect.InvocationHandler actual = handler == null
        ? (instance, method, arguments) -> defaultValue(method.getReturnType()) : handler;
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] { type }, actual);
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == byte.class) return (byte) 0;
    if (type == short.class) return (short) 0;
    if (type == int.class) return 0;
    if (type == long.class) return 0L;
    if (type == float.class) return 0.0f;
    if (type == double.class) return 0.0d;
    if (type == char.class) return (char) 0;
    return null;
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const LOCAL_AUDIO_TRACK_EXECUTOR_CALLBACK_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor;
import com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor.ReadExecutor;
import com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor.SeekExecutor;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

public final class GateLocalAudioTrackExecutorCallbacks {
  public static void main(String[] args) throws Exception {
    List<String> calls = new ArrayList<>();
    Exception readFailure = new Exception("read-sentinel");
    Exception seekFailure = new Exception("seek-sentinel");
    int[] reads = { 0 };
    int[] seeks = { 0 };

    ReadExecutor read = () -> {
      calls.add(reads[0]++ == 0 ? "read-ok" : "read-fail");
      if (reads[0] == 2) throw readFailure;
    };
    SeekExecutor seek = position -> {
      if (seeks[0] == 0) {
        check(position == Long.MIN_VALUE, "minimum seek width");
        calls.add("seek-min");
      } else if (seeks[0] == 1) {
        check(position == Long.MAX_VALUE, "maximum seek width");
        calls.add("seek-max");
      } else {
        calls.add("seek-fail");
        throw seekFailure;
      }
      seeks[0]++;
    };

    read.performRead();
    try {
      read.performRead();
      throw new AssertionError("read exception was swallowed");
    } catch (Exception error) {
      check(error == readFailure, "read exception identity");
    }
    seek.performSeek(Long.MIN_VALUE);
    seek.performSeek(Long.MAX_VALUE);
    try {
      seek.performSeek(0L);
      throw new AssertionError("seek exception was swallowed");
    } catch (Exception error) {
      check(error == seekFailure, "seek exception identity");
    }
    check(calls.equals(Arrays.asList(
        "read-ok", "read-fail", "seek-min", "seek-max", "seek-fail")),
        "dispatch order");

    checkCallback(ReadExecutor.class, "performRead", new Class<?>[0]);
    checkCallback(SeekExecutor.class, "performSeek", new Class<?>[] { long.class });
    System.out.println(
        "dispatch=read-ok,read-fail,seek-min,seek-max,seek-fail;"
        + "exceptions=identity;nesting=LocalAudioTrackExecutor,public-static;"
        + "reflection=2-interfaces,0-fields,1-method-each,throws-Exception");
  }

  private static void checkCallback(Class<?> type, String name, Class<?>[] parameters)
      throws Exception {
    int modifiers = type.getModifiers();
    check(Modifier.isPublic(modifiers) && Modifier.isStatic(modifiers)
        && Modifier.isInterface(modifiers) && Modifier.isAbstract(modifiers)
        && !Modifier.isFinal(modifiers) && type.getSuperclass() == null
        && type.getInterfaces().length == 0 && type.getTypeParameters().length == 0
        && type.getDeclaredAnnotations().length == 0,
        type.getSimpleName() + " structure");
    check(type.getDeclaringClass() == LocalAudioTrackExecutor.class
        && type.getEnclosingClass() == LocalAudioTrackExecutor.class,
        type.getSimpleName() + " nesting");
    check(type.getDeclaredFields().length == 0 && type.getDeclaredMethods().length == 1
        && type.getDeclaredConstructors().length == 0,
        type.getSimpleName() + " member counts");

    Method method = type.getDeclaredMethod(name, parameters);
    int methodModifiers = method.getModifiers();
    check(Modifier.isPublic(methodModifiers) && Modifier.isAbstract(methodModifiers)
        && !Modifier.isStatic(methodModifiers) && !Modifier.isFinal(methodModifiers)
        && !method.isDefault() && !method.isBridge() && !method.isSynthetic()
        && method.getReturnType() == void.class
        && Arrays.equals(method.getParameterTypes(), parameters)
        && Arrays.equals(method.getExceptionTypes(), new Class<?>[] { Exception.class })
        && method.getTypeParameters().length == 0
        && method.getDeclaredAnnotations().length == 0,
        name + " metadata");
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const LOCAL_AUDIO_TRACK_EXECUTOR_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.format.AudioDataFormat;
import com.sedmelluq.discord.lavaplayer.player.AudioConfiguration;
import com.sedmelluq.discord.lavaplayer.player.AudioPlayerOptions;
import com.sedmelluq.discord.lavaplayer.tools.FriendlyException;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackState;
import com.sedmelluq.discord.lavaplayer.track.InternalAudioTrack;
import com.sedmelluq.discord.lavaplayer.track.TrackMarker;
import com.sedmelluq.discord.lavaplayer.track.TrackMarkerHandler.MarkerState;
import com.sedmelluq.discord.lavaplayer.track.TrackStateListener;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameBuffer;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioProcessingContext;
import com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor;
import com.sedmelluq.discord.lavaplayer.track.playback.MutableAudioFrame;
import java.lang.reflect.Constructor;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.concurrent.atomic.AtomicBoolean;

public final class GateLocalAudioTrackExecutor {
  public static void main(String[] args) throws Exception {
    constructorAndPosition();
    markersAndFrames();
    processingLoops();
    executionLifecycle();
    forwardingFailures();
    reflection();
    System.out.println(
        "constructor=context,buffer,factory,disposed;position=seekable,clamp,ghosting;"
        + "markers=late,removed,overwritten,reached,bypassed,ended,stopped;"
        + "processing=read,internal-seek,external-seek,wait,decode-failure;"
        + "frames=immediate,timed,mutable,timed-mutable-bug,terminator;"
        + "execution=loading,finished,failure,stop,stack;reflection=class,1-constructor,19-methods");
  }

  private static void constructorAndPosition() throws Exception {
    Fixture fixed = new Fixture(false, false, null);
    check(fixed.executor.getAudioBuffer() == fixed.buffer, "buffer identity");
    AudioProcessingContext context = fixed.executor.getProcessingContext();
    check(context.configuration == fixed.configuration && context.frameBuffer == fixed.buffer
        && context.playerOptions == fixed.options && context.outputFormat == fixed.format,
        "processing context identity");
    check(fixed.factoryDuration == 731 && fixed.factoryFormat == fixed.format
        && fixed.disposed != null && !fixed.disposed.get(), "factory arguments");
    check(fixed.executor.getState() == AudioTrackState.INACTIVE
        && fixed.executor.getPosition() == 0L && fixed.executor.getStackTrace() == null,
        "initial state");
    fixed.executor.setPosition(44L);
    check(fixed.executor.getPosition() == 0L && fixed.clears == 0,
        "non-seekable position ignored");

    Fixture seekable = new Fixture(true, false, null);
    seekable.executor.setPosition(-5L);
    check(seekable.executor.getPosition() == 0L && seekable.clears == 1,
        "negative seek clamp");
    seekable.executor.setPosition(Long.MAX_VALUE);
    check(seekable.executor.getPosition() == Long.MAX_VALUE && seekable.clears == 2,
        "full-width queued seek");

    Fixture ghost = new Fixture(true, true, null);
    ghost.executor.setPosition(88L);
    check(ghost.executor.getPosition() == 88L && ghost.clears == 0,
        "ghost seek preserves buffer");
  }

  private static void markersAndFrames() throws Exception {
    Fixture fixture = new Fixture(true, false, null);
    List<MarkerState> states = new ArrayList<>();
    TrackMarker reached = new TrackMarker(40L, states::add);
    fixture.executor.addMarker(reached);
    fixture.nextFrame = frame(41L, false);
    check(fixture.executor.provide() == fixture.nextFrame
        && fixture.executor.getPosition() == 41L
        && states.equals(Arrays.asList(MarkerState.REACHED)), "reached marker");

    fixture.executor.addMarker(new TrackMarker(40L, states::add));
    check(states.get(states.size() - 1) == MarkerState.LATE, "late marker");
    TrackMarker removed = new TrackMarker(90L, states::add);
    fixture.executor.addMarker(removed);
    fixture.executor.removeMarker(removed);
    check(states.get(states.size() - 1) == MarkerState.REMOVED, "removed marker");
    fixture.executor.addMarker(new TrackMarker(100L, states::add));
    fixture.executor.setMarker(new TrackMarker(110L, states::add));
    check(states.get(states.size() - 1) == MarkerState.OVERWRITTEN, "overwritten marker");
    fixture.executor.setMarker(null);
    check(states.get(states.size() - 1) == MarkerState.REMOVED, "set null marker");

    fixture.nextFrame = frame(77L, false);
    check(fixture.executor.provide(9L, TimeUnit.MICROSECONDS) == fixture.nextFrame
        && fixture.timedTimeout == 9L && fixture.timedUnit == TimeUnit.MICROSECONDS,
        "timed frame forwarding");
    MutableAudioFrame mutable = new MutableAudioFrame();
    mutable.setTimecode(81L);
    fixture.mutableResult = true;
    check(fixture.executor.provide(mutable) && fixture.executor.getPosition() == 81L,
        "mutable frame forwarding");
    fixture.mutableResult = false;
    check(!fixture.executor.provide(mutable), "mutable false forwarding");
    fixture.timedMutableResult = false;
    check(fixture.executor.provide(mutable, 13L, TimeUnit.NANOSECONDS),
        "timed mutable compatibility return");
    long before = fixture.executor.getPosition();
    fixture.nextFrame = frame(999L, true);
    fixture.executor.provide();
    check(fixture.executor.getPosition() == before, "terminator does not advance");
  }

  private static void processingLoops() throws Exception {
    Fixture internal = new Fixture(true, true, null);
    List<String> calls = new ArrayList<>();
    internal.executor.addMarker(new TrackMarker(25L, state -> calls.add(state.name())));
    internal.executor.setPosition(30L);
    internal.executor.executeProcessingLoop(() -> calls.add("read"),
        position -> calls.add("seek-" + position), false);
    check(calls.equals(Arrays.asList("BYPASSED", "seek-30", "read"))
        && internal.clearOnInsert && internal.executor.getState() == AudioTrackState.PLAYING,
        "internal seek processing");

    Fixture external = new Fixture(true, false, null);
    external.executor.setPosition(52L);
    int[] reads = { 0 };
    external.executor.executeProcessingLoop(() -> reads[0]++, null, false);
    check(reads[0] == 0, "external seek defers read");
    List<Long> seeks = new ArrayList<>();
    external.executor.executeProcessingLoop(() -> reads[0]++, seeks::add, false);
    check(reads[0] == 2 && seeks.equals(Arrays.asList(52L)), "external seek resumes loop");

    Fixture waiting = new Fixture(false, false, null);
    waiting.executor.executeProcessingLoop(() -> { }, null);
    check(waiting.terminates == 1 && waiting.waits == 1, "default wait on end");

    Fixture stopping = new Fixture(false, false, null);
    List<MarkerState> stopped = new ArrayList<>();
    stopping.executor.addMarker(new TrackMarker(100L, stopped::add));
    stopping.executor.stop();
    Thread.currentThread().interrupt();
    stopping.executor.executeProcessingLoop(() -> {
      throw new AssertionError("stopped loop performed a read");
    }, null, false);
    check(stopped.equals(Arrays.asList(MarkerState.STOPPED))
        && !Thread.currentThread().isInterrupted(), "stopped processing marker");

    Fixture decode = new Fixture(false, false, null);
    Exception failure = new Exception("decode-sentinel");
    try {
      decode.executor.executeProcessingLoop(() -> { throw failure; }, null, false);
      throw new AssertionError("decode failure was swallowed");
    } catch (FriendlyException error) {
      check(error.severity == FriendlyException.Severity.FAULT
          && error.getCause() == failure
          && error.getMessage().equals("Something went wrong when decoding the track."),
          "decode failure wrapping");
    }
  }

  private static void executionLifecycle() throws Exception {
    List<MarkerState> markerStates = new ArrayList<>();
    Fixture success = new Fixture(false, false, executor -> {
      check(executor.getState() == AudioTrackState.LOADING, "loading during process");
      check(executor.getStackTrace() != null, "active stack trace");
    });
    success.executor.addMarker(new TrackMarker(100L, markerStates::add));
    success.executor.execute(success.listener);
    check(success.processes == 1 && success.executor.getState() == AudioTrackState.FINISHED
        && success.executor.getStackTrace() == null
        && markerStates.equals(Arrays.asList(MarkerState.ENDED)), "successful execution");

    RuntimeException cause = new RuntimeException("play-sentinel");
    Fixture failed = new Fixture(false, false, executor -> { throw cause; });
    failed.executor.execute(failed.listener);
    check(failed.failure != null && failed.failure.severity == FriendlyException.Severity.FAULT
        && failed.failure.getCause() == cause
        && failed.failure.getMessage().equals("Something broke when playing the track.")
        && failed.executor.failedBeforeLoad() && failed.terminates == 1
        && failed.executor.getState() == AudioTrackState.FINISHED, "failed execution");
    failed.received = true;
    check(!failed.executor.failedBeforeLoad(), "failure after frames");

    Fixture stopped = new Fixture(false, false, null);
    stopped.executor.addMarker(new TrackMarker(10L, markerStates::add));
    stopped.executor.stop();
    check(stopped.disposed.get(), "stop disposes");
    stopped.executor.execute(stopped.listener);
    check(stopped.processes == 0 && stopped.executor.getState() == AudioTrackState.INACTIVE,
        "disposed executor does not start");
  }

  private static void forwardingFailures() throws Exception {
    Fixture fixture = new Fixture(false, false, null);
    InterruptedException wait = new InterruptedException("wait-sentinel");
    fixture.waitFailure = wait;
    try {
      fixture.executor.waitOnEnd();
      throw new AssertionError("wait interruption swallowed");
    } catch (InterruptedException error) {
      check(error == wait && fixture.terminates == 1, "wait exception identity");
    }
    TimeoutException timeout = new TimeoutException("timeout-sentinel");
    fixture.timedFailure = timeout;
    try {
      fixture.executor.provide(1L, TimeUnit.SECONDS);
      throw new AssertionError("timeout swallowed");
    } catch (TimeoutException error) {
      check(error == timeout, "timeout identity");
    }
  }

  private static void reflection() throws Exception {
    Class<LocalAudioTrackExecutor> type = LocalAudioTrackExecutor.class;
    check(Modifier.isPublic(type.getModifiers()) && !Modifier.isFinal(type.getModifiers())
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {
          com.sedmelluq.discord.lavaplayer.track.playback.AudioTrackExecutor.class
        }), "class structure");
    Constructor<?> constructor = type.getDeclaredConstructor(InternalAudioTrack.class,
        AudioConfiguration.class, AudioPlayerOptions.class, boolean.class, int.class);
    check(Modifier.isPublic(constructor.getModifiers())
        && constructor.getExceptionTypes().length == 0, "constructor metadata");
    int publicDeclared = 0;
    for (Method method : type.getDeclaredMethods()) {
      if (Modifier.isPublic(method.getModifiers())) publicDeclared++;
    }
    check(publicDeclared == 19, "public method count");
    check(Arrays.equals(type.getDeclaredMethod("waitOnEnd").getExceptionTypes(),
        new Class<?>[] { InterruptedException.class }), "wait exception metadata");
    check(Arrays.equals(type.getDeclaredMethod("provide", long.class, TimeUnit.class)
        .getExceptionTypes(), new Class<?>[] { TimeoutException.class, InterruptedException.class }),
        "timed provide exception metadata");
  }

  private interface Processor {
    void run(LocalAudioTrackExecutor executor) throws Exception;
  }

  private static final class Fixture {
    final AudioConfiguration configuration = new AudioConfiguration();
    final AudioPlayerOptions options = new AudioPlayerOptions();
    final AudioDataFormat format = configuration.getOutputFormat();
    final boolean seekable;
    final Processor processor;
    final AudioFrameBuffer buffer;
    final InternalAudioTrack track;
    final LocalAudioTrackExecutor executor;
    final TrackStateListener listener;
    AtomicBoolean disposed;
    int factoryDuration;
    AudioDataFormat factoryFormat;
    int clears;
    int terminates;
    int waits;
    int processes;
    boolean clearOnInsert;
    boolean received;
    boolean mutableResult;
    boolean timedMutableResult;
    long timedTimeout;
    TimeUnit timedUnit;
    AudioFrame nextFrame;
    InterruptedException waitFailure;
    TimeoutException timedFailure;
    FriendlyException failure;

    Fixture(boolean seekable, boolean ghosting, Processor processor) {
      this.seekable = seekable;
      this.processor = processor;
      this.buffer = proxy(AudioFrameBuffer.class, (instance, method, arguments) -> {
        switch (method.getName()) {
          case "provide":
            if (arguments == null) return nextFrame;
            if (arguments.length == 1) return mutableResult;
            if (arguments.length == 2) {
              timedTimeout = (Long) arguments[0]; timedUnit = (TimeUnit) arguments[1];
              if (timedFailure != null) throw timedFailure;
              return nextFrame;
            }
            timedTimeout = (Long) arguments[1]; timedUnit = (TimeUnit) arguments[2];
            if (timedFailure != null) throw timedFailure;
            return timedMutableResult;
          case "clear": clears++; clearOnInsert = false; return null;
          case "setClearOnInsert": clearOnInsert = true; return null;
          case "hasClearOnInsert": return clearOnInsert;
          case "setTerminateOnEmpty": terminates++; return null;
          case "waitForTermination":
            waits++; if (waitFailure != null) throw waitFailure; return null;
          case "hasReceivedFrames": return received;
          default: return defaultValue(method.getReturnType());
        }
      });
      configuration.setFrameBufferFactory((duration, dataFormat, disposedOf) -> {
        factoryDuration = duration; factoryFormat = dataFormat; disposed = disposedOf;
        return buffer;
      });
      AudioTrackInfo info = new AudioTrackInfo(
          "title", "author", 1000L, "id", false, "uri", "art", "isrc");
      this.track = proxy(InternalAudioTrack.class, (instance, method, arguments) -> {
        switch (method.getName()) {
          case "isSeekable": return this.seekable;
          case "getInfo": return info;
          case "getIdentifier": return "id";
          case "process":
            processes++;
            check(arguments[0] instanceof LocalAudioTrackExecutor,
                "process executor type");
            if (this.processor != null) {
              this.processor.run((LocalAudioTrackExecutor) arguments[0]);
            }
            return null;
          default: return defaultValue(method.getReturnType());
        }
      });
      this.listener = new TrackStateListener() {
        public void onTrackException(com.sedmelluq.discord.lavaplayer.track.AudioTrack item,
            FriendlyException error) {
          check(item == track, "failure track identity"); failure = error;
        }
        public void onTrackStuck(com.sedmelluq.discord.lavaplayer.track.AudioTrack item,
            long thresholdMs) { }
      };
      this.executor = new LocalAudioTrackExecutor(
          track, configuration, options, ghosting, 731);
    }
  }

  private static AudioFrame frame(long timecode, boolean terminator) {
    return proxy(AudioFrame.class, (instance, method, arguments) -> {
      if (method.getName().equals("getTimecode")) return timecode;
      if (method.getName().equals("isTerminator")) return terminator;
      return defaultValue(method.getReturnType());
    });
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type, java.lang.reflect.InvocationHandler handler) {
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] { type }, handler);
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == byte.class) return (byte) 0;
    if (type == short.class) return (short) 0;
    if (type == int.class) return 0;
    if (type == long.class) return 0L;
    if (type == float.class) return 0.0f;
    if (type == double.class) return 0.0d;
    if (type == char.class) return (char) 0;
    return null;
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const BASE_AUDIO_TRACK_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.player.AudioPlayerManager;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackState;
import com.sedmelluq.discord.lavaplayer.track.BaseAudioTrack;
import com.sedmelluq.discord.lavaplayer.track.TrackMarker;
import com.sedmelluq.discord.lavaplayer.track.TrackMarkerHandler.MarkerState;
import com.sedmelluq.discord.lavaplayer.track.TrackStateListener;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameBuffer;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioTrackExecutor;
import com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor;
import com.sedmelluq.discord.lavaplayer.track.playback.MutableAudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.PrimordialAudioTrackExecutor;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;

public final class GateBaseAudioTrack {
  public static void main(String[] args) throws Exception {
    constructorAndMetadata();
    primordialState();
    assignmentAndForwarding();
    stopAndFailureEdges();
    durationAndUserData();
    cloningAndDefaults();
    concurrentAssignment();
    reflection();
    System.out.println(
        "constructor=identity,null,primordial;metadata=identifier,seekable,duration;"
        + "primordial=position,markers,empty-provider;assignment=once,apply,skip,null,poison;"
        + "forwarding=state,position,markers,frames,exceptions;stop=handoff,failure-order;"
        + "userdata=volatile,typed,null-class;clone=shallow,user-data,defaults;"
        + "concurrency=single-winner;reflection=abstract,6-fields,1-constructor,23-public-methods");
  }

  private static void constructorAndMetadata() {
    AudioTrackInfo info = info(false, 123L, "identifier");
    TestTrack track = new TestTrack(info);
    check(track.getInfo() == info && track.exposedInfo() == info, "track info identity");
    check(track.getIdentifier() == info.identifier && track.isSeekable(), "metadata forwarding");
    check(track.getDuration() == 123L && track.exposedDuration() == 0L, "initial duration");
    check(track.getSourceManager() == null && track.createLocalExecutor(null) == null,
        "default factories");
    AudioTrackExecutor initial = track.getActiveExecutor();
    check(initial instanceof PrimordialAudioTrackExecutor
        && initial == track.getActiveExecutor() && initial.getState() == AudioTrackState.INACTIVE,
        "primordial executor identity");

    TestTrack stream = new TestTrack(info(true, Long.MAX_VALUE, "stream"));
    check(!stream.isSeekable() && stream.getDuration() == Long.MAX_VALUE, "stream metadata");
    TestTrack nullable = new TestTrack(null);
    check(nullable.getInfo() == null, "null info accepted");
    try {
      nullable.getIdentifier();
      throw new AssertionError("null identifier did not fail");
    } catch (NullPointerException expected) { }
  }

  private static void primordialState() throws Exception {
    TestTrack track = new TestTrack(info(false, 100, "primordial"));
    AudioTrackExecutor initial = track.getActiveExecutor();
    check(initial.getAudioBuffer() == null && !initial.failedBeforeLoad()
        && initial.provide() == null && initial.provide(3, null) == null,
        "empty primordial provider");
    MutableAudioFrame mutable = new MutableAudioFrame();
    check(!initial.provide(mutable) && !initial.provide(mutable, 4, null),
        "empty primordial mutable provider");
    try {
      initial.execute(null);
      throw new AssertionError("primordial execute did not fail");
    } catch (UnsupportedOperationException error) {
      check(error.getMessage() == null, "primordial execute message");
    }

    List<String> events = new ArrayList<>();
    TrackMarker late = marker("late", 5, events);
    track.setPosition(5);
    track.addMarker(late);
    check(events.equals(Arrays.asList("late:LATE")), "primordial late marker");
    TrackMarker first = marker("first", 20, events);
    TrackMarker second = marker("second", 30, events);
    track.addMarker(first);
    track.addMarker(second);
    track.removeMarker(first);
    track.setMarker(marker("third", 40, events));
    check(events.equals(Arrays.asList("late:LATE", "first:REMOVED", "second:OVERWRITTEN")),
        "primordial marker states");
    track.setPosition(40);
    check(events.get(events.size() - 1).equals("third:BYPASSED") && track.getPosition() == 40,
        "primordial seek marker");
  }

  private static void assignmentAndForwarding() throws Exception {
    TestTrack track = new TestTrack(info(false, 100, "assigned"));
    List<String> markers = new ArrayList<>();
    TrackMarker first = marker("first", 60, markers);
    TrackMarker second = marker("second", 70, markers);
    track.setPosition(40);
    track.addMarker(first);
    track.addMarker(second);
    ExecutorFixture fixture = new ExecutorFixture();
    fixture.position = 51;
    fixture.state = AudioTrackState.PLAYING;
    fixture.frame = frame(12);
    fixture.mutableResult = true;
    fixture.timedMutableResult = false;
    track.assignExecutor(fixture.executor, true);
    check(track.getActiveExecutor() == fixture.executor
        && fixture.calls.equals(Arrays.asList("setPosition:40", "addMarker:first", "addMarker:second")),
        "primordial state application");
    fixture.calls.clear();
    fixture.position = 51;
    check(track.getState() == AudioTrackState.PLAYING && track.getPosition() == 51,
        "state and position forwarding");
    track.setPosition(Long.MIN_VALUE);
    track.setMarker(first);
    track.addMarker(second);
    track.removeMarker(first);
    check(track.provide() == fixture.frame && track.provide(9, TimeUnit.NANOSECONDS) == fixture.frame,
        "frame forwarding");
    MutableAudioFrame mutable = new MutableAudioFrame();
    check(track.provide(mutable) && !track.provide(mutable, 11, TimeUnit.MICROSECONDS),
        "mutable forwarding");
    check(fixture.calls.equals(Arrays.asList("getState", "getPosition", "setPosition:" + Long.MIN_VALUE,
        "setMarker:first", "addMarker:second", "removeMarker:first", "provide",
        "provideTimed:9:NANOSECONDS", "provideMutable", "provideMutableTimed:11:MICROSECONDS")),
        "forwarding order and arguments");

    TimeoutException timeout = new TimeoutException("timeout-sentinel");
    fixture.failure = timeout;
    try {
      track.provide(1, TimeUnit.SECONDS);
      throw new AssertionError("timed failure swallowed");
    } catch (TimeoutException error) {
      check(error == timeout, "timed failure identity");
    }
    try {
      track.assignExecutor(fixture.executor, false);
      throw new AssertionError("second assignment accepted");
    } catch (IllegalStateException error) {
      check(error.getMessage().equals(
          "Cannot play the same instance of a track twice, use track.makeClone()."),
          "second assignment message");
    }

    TestTrack skipped = new TestTrack(info(false, 100, "skip"));
    skipped.setPosition(55);
    skipped.addMarker(marker("retained", 90, markers));
    ExecutorFixture skippedFixture = new ExecutorFixture();
    skipped.assignExecutor(skippedFixture.executor, false);
    check(skippedFixture.calls.isEmpty(), "apply false skips primordial state");

    TestTrack nullTrack = new TestTrack(info(false, 100, "null"));
    nullTrack.assignExecutor(null, false);
    check(nullTrack.getActiveExecutor() instanceof PrimordialAudioTrackExecutor,
        "null assignment leaves primordial active");
    try {
      nullTrack.assignExecutor(fixture.executor, false);
      throw new AssertionError("null assignment did not consume instance");
    } catch (IllegalStateException expected) { }

    TestTrack poisoned = new TestTrack(info(false, 100, "poison"));
    poisoned.setPosition(1);
    try {
      poisoned.assignExecutor(null, true);
      throw new AssertionError("null state application did not fail");
    } catch (NullPointerException expected) { }
    try {
      poisoned.assignExecutor(fixture.executor, false);
      throw new AssertionError("failed assignment did not consume instance");
    } catch (IllegalStateException expected) { }
  }

  private static void stopAndFailureEdges() {
    TestTrack track = new TestTrack(info(false, 100, "stop"));
    ExecutorFixture fixture = new ExecutorFixture();
    fixture.position = 88;
    track.assignExecutor(fixture.executor, false);
    track.stop();
    check(fixture.stops == 1 && track.getActiveExecutor() instanceof PrimordialAudioTrackExecutor
        && track.getPosition() == 88, "stop position handoff");
    track.stop();
    check(fixture.stops == 1, "repeated stop no-op");
    try {
      track.assignExecutor(fixture.executor, false);
      throw new AssertionError("stop allowed reassignment");
    } catch (IllegalStateException expected) { }

    RuntimeException positionFailure = new RuntimeException("position-sentinel");
    TestTrack failing = new TestTrack(info(false, 100, "failing"));
    ExecutorFixture badPosition = new ExecutorFixture();
    badPosition.positionFailure = positionFailure;
    failing.assignExecutor(badPosition.executor, false);
    try {
      failing.stop();
      throw new AssertionError("position failure swallowed");
    } catch (RuntimeException error) {
      check(error == positionFailure && badPosition.stops == 0
          && failing.getActiveExecutor() instanceof PrimordialAudioTrackExecutor,
          "active cleared before position failure");
    }

    RuntimeException stopFailure = new RuntimeException("stop-sentinel");
    TestTrack badStopTrack = new TestTrack(info(false, 100, "bad-stop"));
    ExecutorFixture badStop = new ExecutorFixture();
    badStop.position = 77;
    badStop.stopFailure = stopFailure;
    badStopTrack.assignExecutor(badStop.executor, false);
    try {
      badStopTrack.stop();
      throw new AssertionError("stop failure swallowed");
    } catch (RuntimeException error) {
      check(error == stopFailure && badStopTrack.getPosition() == 77,
          "position copied before stop failure");
    }
  }

  private static void durationAndUserData() {
    TestTrack track = new TestTrack(info(false, 123, "data"));
    track.setAccurateDuration(456);
    check(track.getDuration() == 456, "accurate duration");
    track.setAccurateDuration(-7);
    check(track.getDuration() == -7, "negative accurate duration");
    track.setAccurateDuration(0);
    check(track.getDuration() == 123, "zero duration fallback");
    check(track.getUserData() == null && track.getUserData(String.class) == null,
        "default user data");
    String data = new String("payload");
    track.setUserData(data);
    check(track.getUserData() == data && track.getUserData(String.class) == data
        && track.getUserData(Object.class) == data && track.getUserData(Integer.class) == null,
        "typed user data");
    try {
      track.getUserData(null);
      throw new AssertionError("null class with data did not fail");
    } catch (NullPointerException expected) { }
    track.setUserData(null);
    check(track.getUserData(null) == null, "null class short circuit");
  }

  private static void cloningAndDefaults() {
    AudioTrackInfo info = info(false, 100, "clone");
    TestTrack track = new TestTrack(info);
    Object data = new Object();
    track.setUserData(data);
    AudioTrack clone = track.makeClone();
    check(clone != track && clone.getInfo() == info && clone.getUserData() == data
        && clone.getState() == AudioTrackState.INACTIVE, "shallow clone and user data");
    try {
      new NoCloneTrack(info).makeClone();
      throw new AssertionError("default shallow clone did not fail");
    } catch (UnsupportedOperationException error) {
      check(error.getMessage() == null, "default clone failure message");
    }
    try {
      new NullCloneTrack(info).makeClone();
      throw new AssertionError("null shallow clone did not fail");
    } catch (NullPointerException expected) { }
  }

  private static void concurrentAssignment() throws Exception {
    TestTrack track = new TestTrack(info(false, 100, "concurrent"));
    ExecutorFixture fixture = new ExecutorFixture();
    AtomicInteger successes = new AtomicInteger();
    AtomicInteger failures = new AtomicInteger();
    Thread[] threads = new Thread[8];
    for (int index = 0; index < threads.length; index++) {
      threads[index] = new Thread(() -> {
        try {
          track.assignExecutor(fixture.executor, false);
          successes.incrementAndGet();
        } catch (IllegalStateException expected) {
          failures.incrementAndGet();
        }
      });
      threads[index].start();
    }
    for (Thread thread : threads) thread.join();
    check(successes.get() == 1 && failures.get() == 7
        && track.getActiveExecutor() == fixture.executor, "single assignment winner");
  }

  private static void reflection() throws Exception {
    Class<BaseAudioTrack> type = BaseAudioTrack.class;
    check(Modifier.isPublic(type.getModifiers()) && Modifier.isAbstract(type.getModifiers())
        && !Modifier.isFinal(type.getModifiers()) && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {
          com.sedmelluq.discord.lavaplayer.track.InternalAudioTrack.class
        }), "class metadata");
    Field[] fields = type.getDeclaredFields();
    check(fields.length == 6, "declared field count");
    checkField(type, "trackInfo", AudioTrackInfo.class, Modifier.PROTECTED | Modifier.FINAL);
    checkField(type, "accurateDuration", AtomicLong.class, Modifier.PROTECTED | Modifier.FINAL);
    checkField(type, "initialExecutor", PrimordialAudioTrackExecutor.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "executorAssigned", java.util.concurrent.atomic.AtomicBoolean.class,
        Modifier.PRIVATE | Modifier.FINAL);
    Field active = checkField(type, "activeExecutor",
        java.util.concurrent.atomic.AtomicReference.class, Modifier.PRIVATE | Modifier.FINAL);
    ParameterizedType activeType = (ParameterizedType) active.getGenericType();
    check(activeType.getActualTypeArguments()[0] == AudioTrackExecutor.class,
        "active executor generic type");
    checkField(type, "userData", Object.class, Modifier.PRIVATE | Modifier.VOLATILE);
    Constructor<?>[] constructors = type.getDeclaredConstructors();
    check(constructors.length == 1 && Modifier.isPublic(constructors[0].getModifiers())
        && Arrays.equals(constructors[0].getParameterTypes(), new Class<?>[] { AudioTrackInfo.class }),
        "constructor metadata");
    int publicDeclared = 0;
    for (Method method : type.getDeclaredMethods()) {
      if (Modifier.isPublic(method.getModifiers())) publicDeclared++;
    }
    check(publicDeclared == 23, "public method count");
    Method shallow = type.getDeclaredMethod("makeShallowClone");
    check(Modifier.isProtected(shallow.getModifiers()) && !Modifier.isAbstract(shallow.getModifiers()),
        "shallow clone metadata");
    Method typed = type.getDeclaredMethod("getUserData", Class.class);
    check(typed.getTypeParameters().length == 1
        && typed.getGenericReturnType() == typed.getTypeParameters()[0], "typed user metadata");
    check(Arrays.equals(type.getDeclaredMethod("provide", long.class, TimeUnit.class)
        .getExceptionTypes(), new Class<?>[] { TimeoutException.class, InterruptedException.class }),
        "timed exception metadata");
  }

  private static Field checkField(Class<?> type, String name, Class<?> fieldType, int modifiers)
      throws Exception {
    Field field = type.getDeclaredField(name);
    check(field.getType() == fieldType && field.getModifiers() == modifiers,
        name + " field metadata");
    return field;
  }

  private static AudioTrackInfo info(boolean stream, long length, String identifier) {
    return new AudioTrackInfo(
        "title", "author", length, identifier, stream, "uri", "art", "isrc");
  }

  private static TrackMarker marker(String name, long timecode, List<String> events) {
    return new TrackMarker(timecode, state -> events.add(name + ":" + state.name()));
  }

  private static AudioFrame frame(long timecode) {
    return (AudioFrame) Proxy.newProxyInstance(AudioFrame.class.getClassLoader(),
        new Class<?>[] { AudioFrame.class }, (instance, method, arguments) -> {
          if (method.getName().equals("getTimecode")) return timecode;
          return defaultValue(method.getReturnType());
        });
  }

  private static class TestTrack extends BaseAudioTrack {
    TestTrack(AudioTrackInfo info) { super(info); }
    protected AudioTrack makeShallowClone() { return new TestTrack(trackInfo); }
    public void process(LocalAudioTrackExecutor executor) { }
    AudioTrackInfo exposedInfo() { return trackInfo; }
    long exposedDuration() { return accurateDuration.get(); }
    void setAccurateDuration(long value) { accurateDuration.set(value); }
  }

  private static final class NoCloneTrack extends BaseAudioTrack {
    NoCloneTrack(AudioTrackInfo info) { super(info); }
    public void process(LocalAudioTrackExecutor executor) { }
  }

  private static final class NullCloneTrack extends BaseAudioTrack {
    NullCloneTrack(AudioTrackInfo info) { super(info); }
    protected AudioTrack makeShallowClone() { return null; }
    public void process(LocalAudioTrackExecutor executor) { }
  }

  private static final class ExecutorFixture {
    final List<String> calls = new ArrayList<>();
    final AudioTrackExecutor executor;
    long position;
    AudioTrackState state = AudioTrackState.INACTIVE;
    AudioFrame frame;
    boolean mutableResult;
    boolean timedMutableResult;
    int stops;
    Throwable failure;
    RuntimeException positionFailure;
    RuntimeException stopFailure;

    ExecutorFixture() {
      executor = (AudioTrackExecutor) Proxy.newProxyInstance(
          AudioTrackExecutor.class.getClassLoader(), new Class<?>[] { AudioTrackExecutor.class },
          (instance, method, arguments) -> {
            String name = method.getName();
            if (name.equals("getState")) { calls.add("getState"); return state; }
            if (name.equals("getPosition")) {
              calls.add("getPosition");
              if (positionFailure != null) throw positionFailure;
              return position;
            }
            if (name.equals("setPosition")) {
              position = (Long) arguments[0]; calls.add("setPosition:" + position); return null;
            }
            if (name.equals("setMarker") || name.equals("addMarker") || name.equals("removeMarker")) {
              TrackMarker marker = (TrackMarker) arguments[0];
              String markerName = marker == null ? "null" : marker.timecode == 60 ? "first" : "second";
              calls.add(name + ":" + markerName); return null;
            }
            if (name.equals("stop")) {
              stops++; if (stopFailure != null) throw stopFailure; return null;
            }
            if (name.equals("provide")) {
              if (failure != null) throw failure;
              if (arguments == null) { calls.add("provide"); return frame; }
              if (arguments.length == 1) { calls.add("provideMutable"); return mutableResult; }
              if (arguments.length == 2) {
                calls.add("provideTimed:" + arguments[0] + ":" + arguments[1]); return frame;
              }
              calls.add("provideMutableTimed:" + arguments[1] + ":" + arguments[2]);
              return timedMutableResult;
            }
            return defaultValue(method.getReturnType());
          });
    }
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == long.class) return 0L;
    if (type == int.class) return 0;
    if (type == short.class) return (short) 0;
    if (type == byte.class) return (byte) 0;
    if (type == float.class) return 0.0f;
    if (type == double.class) return 0.0d;
    if (type == char.class) return (char) 0;
    return null;
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const PRIMORDIAL_AUDIO_TRACK_EXECUTOR_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackState;
import com.sedmelluq.discord.lavaplayer.track.TrackMarker;
import com.sedmelluq.discord.lavaplayer.track.TrackMarkerHandler.MarkerState;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioTrackExecutor;
import com.sedmelluq.discord.lavaplayer.track.playback.MutableAudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.PrimordialAudioTrackExecutor;
import java.lang.reflect.Constructor;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.concurrent.atomic.AtomicInteger;

public final class GatePrimordialAudioTrackExecutor {
  public static void main(String[] args) throws Exception {
    defaultsAndStop();
    positionsAndMarkers();
    stateApplication();
    applicationFailures();
    concurrency();
    reflection();
    System.out.println(
        "defaults=buffer,state,position,failed,providers,execute;stop=log,null-info;"
        + "markers=late,overwrite,remove,seek,duplicates;"
        + "apply=position,ordered,clear,retry,failure-order,null-target;"
        + "concurrency=volatile,copy-on-write;reflection=class,1-constructor,15-methods,exceptions");
  }

  private static void defaultsAndStop() throws Exception {
    PrimordialAudioTrackExecutor executor = new PrimordialAudioTrackExecutor(info("normal"));
    check(executor.getAudioBuffer() == null && executor.getState() == AudioTrackState.INACTIVE
        && executor.getPosition() == 0L && !executor.failedBeforeLoad(), "initial state");
    check(executor.provide() == null && executor.provide(Long.MIN_VALUE, null) == null
        && !executor.provide((MutableAudioFrame) null)
        && !executor.provide(null, Long.MAX_VALUE, null), "empty providers");
    try {
      executor.execute(null);
      throw new AssertionError("execute succeeded");
    } catch (UnsupportedOperationException error) {
      check(error.getMessage() == null && error.getCause() == null, "execute failure shape");
    }
    executor.setPosition(71L);
    executor.addMarker(marker("stop", 90L, new ArrayList<>()));
    executor.stop();
    check(executor.getPosition() == 71L, "stop mutated state");
    try {
      new PrimordialAudioTrackExecutor(null).stop();
      throw new AssertionError("null info stop succeeded");
    } catch (NullPointerException expected) { }
  }

  private static void positionsAndMarkers() {
    PrimordialAudioTrackExecutor executor = new PrimordialAudioTrackExecutor(info("markers"));
    List<String> events = new ArrayList<>();
    executor.setPosition(Long.MIN_VALUE);
    check(executor.getPosition() == Long.MIN_VALUE, "minimum position");
    executor.setPosition(20L);
    executor.addMarker(null);
    executor.addMarker(marker("late", 20L, events));
    check(events.equals(Arrays.asList("late:LATE")), "late boundary");
    TrackMarker first = marker("first", 30L, events);
    TrackMarker duplicate = marker("duplicate", 40L, events);
    executor.addMarker(first);
    executor.addMarker(duplicate);
    executor.addMarker(duplicate);
    executor.removeMarker(duplicate);
    check(events.get(events.size() - 1).equals("duplicate:REMOVED"), "single removal");
    executor.setMarker(marker("replacement", 50L, events));
    check(events.subList(2, 4).equals(Arrays.asList(
        "first:OVERWRITTEN", "duplicate:OVERWRITTEN")), "overwrite order");
    executor.setPosition(50L);
    check(events.get(events.size() - 1).equals("replacement:BYPASSED"), "seek bypass");
    executor.setPosition(Long.MAX_VALUE);
    check(executor.getPosition() == Long.MAX_VALUE, "maximum position");
  }

  private static void stateApplication() {
    PrimordialAudioTrackExecutor source = new PrimordialAudioTrackExecutor(info("apply"));
    List<String> events = new ArrayList<>();
    source.setPosition(23L);
    TrackMarker first = marker("first", 30L, events);
    TrackMarker second = marker("second", 40L, events);
    source.addMarker(first);
    source.addMarker(second);
    source.addMarker(second);
    List<String> calls = new ArrayList<>();
    source.applyStateToExecutor(target(calls, null, null));
    check(calls.equals(Arrays.asList("position:23", "marker:30", "marker:40", "marker:40")),
        "state application order");
    calls.clear();
    source.applyStateToExecutor(target(calls, null, null));
    check(calls.equals(Arrays.asList("position:23")), "markers cleared after apply");

    PrimordialAudioTrackExecutor empty = new PrimordialAudioTrackExecutor(info("empty"));
    empty.applyStateToExecutor(null);
  }

  private static void applicationFailures() {
    RuntimeException markerFailure = new RuntimeException("marker-sentinel");
    PrimordialAudioTrackExecutor markers = new PrimordialAudioTrackExecutor(info("failure"));
    markers.addMarker(marker("one", 10L, new ArrayList<>()));
    markers.addMarker(marker("two", 20L, new ArrayList<>()));
    List<String> failedCalls = new ArrayList<>();
    try {
      markers.applyStateToExecutor(target(failedCalls, null, markerFailure));
      throw new AssertionError("marker failure swallowed");
    } catch (RuntimeException error) {
      check(error == markerFailure
          && failedCalls.equals(Arrays.asList("marker:10", "marker:20")),
          "marker failure identity and order");
    }
    List<String> retryCalls = new ArrayList<>();
    markers.applyStateToExecutor(target(retryCalls, null, null));
    check(retryCalls.equals(Arrays.asList("marker:10", "marker:20")),
        "marker failure retained state");

    RuntimeException positionFailure = new RuntimeException("position-sentinel");
    PrimordialAudioTrackExecutor positioned = new PrimordialAudioTrackExecutor(info("position"));
    positioned.setPosition(-7L);
    positioned.addMarker(marker("retained", 10L, new ArrayList<>()));
    try {
      positioned.applyStateToExecutor(target(new ArrayList<>(), positionFailure, null));
      throw new AssertionError("position failure swallowed");
    } catch (RuntimeException error) {
      check(error == positionFailure, "position failure identity");
    }
    List<String> recovered = new ArrayList<>();
    positioned.applyStateToExecutor(target(recovered, null, null));
    check(recovered.equals(Arrays.asList("position:-7", "marker:10")),
        "position failure retained markers");

    PrimordialAudioTrackExecutor nullTarget = new PrimordialAudioTrackExecutor(info("null"));
    nullTarget.addMarker(marker("retained", 12L, new ArrayList<>()));
    try {
      nullTarget.applyStateToExecutor(null);
      throw new AssertionError("null marker target succeeded");
    } catch (NullPointerException expected) { }
    List<String> afterNull = new ArrayList<>();
    nullTarget.applyStateToExecutor(target(afterNull, null, null));
    check(afterNull.equals(Arrays.asList("marker:12")), "null target retained marker");
  }

  private static void concurrency() throws Exception {
    PrimordialAudioTrackExecutor executor = new PrimordialAudioTrackExecutor(info("concurrent"));
    Thread writer = new Thread(() -> {
      for (long value = 1; value <= 10000; value++) executor.setPosition(value);
    });
    writer.start();
    writer.join();
    check(executor.getPosition() == 10000L, "position publication");
    executor.setPosition(0L);
    Thread[] adders = new Thread[8];
    for (int index = 0; index < adders.length; index++) {
      final int markerIndex = index;
      adders[index] = new Thread(() -> executor.addMarker(
          marker("concurrent-" + markerIndex, 100L + markerIndex, new ArrayList<>())));
      adders[index].start();
    }
    for (Thread adder : adders) adder.join();
    AtomicInteger applied = new AtomicInteger();
    AudioTrackExecutor target = (AudioTrackExecutor) Proxy.newProxyInstance(
        AudioTrackExecutor.class.getClassLoader(), new Class<?>[] { AudioTrackExecutor.class },
        (instance, method, arguments) -> {
          if (method.getName().equals("addMarker")) applied.incrementAndGet();
          return defaultValue(method.getReturnType());
        });
    executor.applyStateToExecutor(target);
    check(applied.get() == 8, "concurrent marker adds");
  }

  private static void reflection() throws Exception {
    Class<PrimordialAudioTrackExecutor> type = PrimordialAudioTrackExecutor.class;
    check(Modifier.isPublic(type.getModifiers()) && !Modifier.isFinal(type.getModifiers())
        && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] { AudioTrackExecutor.class }),
        "class metadata");
    check(type.getFields().length == 0, "exported fields");
    Constructor<?>[] constructors = type.getDeclaredConstructors();
    check(constructors.length == 1 && Modifier.isPublic(constructors[0].getModifiers())
        && Arrays.equals(constructors[0].getParameterTypes(), new Class<?>[] { AudioTrackInfo.class })
        && constructors[0].getExceptionTypes().length == 0, "constructor metadata");
    int publicMethods = 0;
    for (Method method : type.getDeclaredMethods()) {
      check(Modifier.isPublic(method.getModifiers()) && !Modifier.isStatic(method.getModifiers())
          && !Modifier.isAbstract(method.getModifiers()), "method modifiers");
      publicMethods++;
    }
    check(publicMethods == 15, "declared method count");
    check(type.getDeclaredMethod("provide", long.class, TimeUnit.class)
        .getExceptionTypes().length == 0, "narrow timed frame exceptions");
    check(Arrays.equals(type.getDeclaredMethod("provide", MutableAudioFrame.class, long.class,
        TimeUnit.class).getExceptionTypes(),
        new Class<?>[] { TimeoutException.class, InterruptedException.class }),
        "timed mutable exceptions");
  }

  private static AudioTrackExecutor target(List<String> calls, RuntimeException positionFailure,
      RuntimeException markerFailure) {
    AtomicInteger markers = new AtomicInteger();
    return (AudioTrackExecutor) Proxy.newProxyInstance(AudioTrackExecutor.class.getClassLoader(),
        new Class<?>[] { AudioTrackExecutor.class }, (instance, method, arguments) -> {
          if (method.getName().equals("setPosition")) {
            calls.add("position:" + arguments[0]);
            if (positionFailure != null) throw positionFailure;
          } else if (method.getName().equals("addMarker")) {
            TrackMarker marker = (TrackMarker) arguments[0];
            calls.add("marker:" + marker.timecode);
            if (markerFailure != null && markers.incrementAndGet() == 2) throw markerFailure;
          }
          return defaultValue(method.getReturnType());
        });
  }

  private static AudioTrackInfo info(String identifier) {
    return new AudioTrackInfo("title", "author", 1000L, identifier, false,
        "uri", "art", "isrc");
  }

  private static TrackMarker marker(String name, long timecode, List<String> events) {
    return new TrackMarker(timecode, state -> events.add(name + ":" + state.name()));
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == long.class) return 0L;
    if (type == int.class) return 0;
    if (type == short.class) return (short) 0;
    if (type == byte.class) return (byte) 0;
    if (type == float.class) return 0.0f;
    if (type == double.class) return 0.0d;
    if (type == char.class) return (char) 0;
    return null;
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const DELEGATED_AUDIO_TRACK_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import com.sedmelluq.discord.lavaplayer.track.DelegatedAudioTrack;
import com.sedmelluq.discord.lavaplayer.track.InternalAudioTrack;
import com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicLong;

public final class GateDelegatedAudioTrack {
  public static void main(String[] args) throws Exception {
    constructorAndFallback();
    installationAndForwarding();
    failuresAndReset();
    monitorSemantics();
    reflection();
    System.out.println(
        "constructor=identity,null;fallback=duration,accurate,position;"
        + "delegate=publish,assign-before-process,position,duration,failures,reset;"
        + "monitor=fallback-blocks,delegate-fast-path,process-synchronized;"
        + "reflection=abstract,private-field,1-constructor,4-exported-methods,exception");
  }

  private static void constructorAndFallback() {
    AudioTrackInfo info = info(123L, "fallback");
    TestTrack track = new TestTrack(info);
    check(track.getInfo() == info && track.getDuration() == 123L && track.getPosition() == 0L,
        "constructor and fallback identity");
    track.setAccurateDuration(456L);
    check(track.getDuration() == 456L, "accurate duration fallback");
    track.setAccurateDuration(0L);
    track.setPosition(Long.MIN_VALUE);
    check(track.getPosition() == Long.MIN_VALUE, "primordial position fallback");
    TestTrack nullInfo = new TestTrack(null);
    check(nullInfo.getInfo() == null && nullInfo.getPosition() == 0L, "null info construction");
    try {
      nullInfo.getDuration();
      throw new AssertionError("null info duration succeeded");
    } catch (NullPointerException expected) { }
  }

  private static void installationAndForwarding() throws Exception {
    TestTrack track = new TestTrack(info(100L, "delegate"));
    track.setPosition(42L);
    DelegateFixture fixture = new DelegateFixture(track, 9L, 700L);
    track.install(fixture.delegate, null);
    check(fixture.calls.equals(Arrays.asList(
        "assign:null:false", "published:9", "process:null")),
        "delegate publication and call order");
    check(track.getPosition() == 9L && track.getDuration() == 700L,
        "delegate getters");
    track.setPosition(Long.MAX_VALUE);
    check(fixture.position == Long.MAX_VALUE
        && fixture.calls.get(fixture.calls.size() - 1).equals("setPosition:" + Long.MAX_VALUE),
        "delegate full-width position");

    RuntimeException positionFailure = new RuntimeException("set-sentinel");
    fixture.positionFailure = positionFailure;
    try {
      track.setPosition(1L);
      throw new AssertionError("position failure swallowed");
    } catch (RuntimeException error) {
      check(error == positionFailure, "position failure identity");
    }
    RuntimeException durationFailure = new RuntimeException("duration-sentinel");
    fixture.durationFailure = durationFailure;
    try {
      track.getDuration();
      throw new AssertionError("duration failure swallowed");
    } catch (RuntimeException error) {
      check(error == durationFailure, "duration failure identity");
    }
  }

  private static void failuresAndReset() throws Exception {
    TestTrack assignTrack = new TestTrack(info(200L, "assign-failure"));
    RuntimeException assignFailure = new RuntimeException("assign-sentinel");
    DelegateFixture assign = new DelegateFixture(assignTrack, 31L, 810L);
    assign.assignFailure = assignFailure;
    try {
      assignTrack.install(assign.delegate, null);
      throw new AssertionError("assign failure swallowed");
    } catch (RuntimeException error) {
      check(error == assignFailure && assignTrack.getPosition() == 31L
          && !assign.calls.contains("process:null"), "assign failure retains delegate");
    }

    TestTrack processTrack = new TestTrack(info(300L, "process-failure"));
    Exception processFailure = new Exception("process-sentinel");
    DelegateFixture process = new DelegateFixture(processTrack, 41L, 910L);
    process.processFailure = processFailure;
    try {
      processTrack.install(process.delegate, null);
      throw new AssertionError("process failure swallowed");
    } catch (Exception error) {
      check(error == processFailure && processTrack.getDuration() == 910L,
          "process exception identity and retained delegate");
    }

    TestTrack reset = new TestTrack(info(444L, "reset"));
    reset.setPosition(52L);
    DelegateFixture installed = new DelegateFixture(reset, 61L, 1010L);
    reset.install(installed.delegate, null);
    try {
      reset.install(null, null);
      throw new AssertionError("null delegate succeeded");
    } catch (NullPointerException expected) { }
    check(reset.getPosition() == 52L && reset.getDuration() == 444L,
        "null delegate restores base fallback");
  }

  private static void monitorSemantics() throws Exception {
    TestTrack fallback = new TestTrack(info(555L, "monitor-fallback"));
    MonitorHolder holder = new MonitorHolder(fallback);
    holder.start();
    AtomicLong fallbackValue = new AtomicLong(Long.MIN_VALUE);
    Thread fallbackReader = new Thread(() -> fallbackValue.set(fallback.getDuration()));
    fallbackReader.start();
    awaitBlocked(fallbackReader, "fallback reader did not acquire track monitor");
    holder.release();
    fallbackReader.join(2000L);
    check(!fallbackReader.isAlive() && fallbackValue.get() == 555L,
        "fallback monitor release");

    TestTrack delegated = new TestTrack(info(1L, "monitor-delegate"));
    DelegateFixture fixture = new DelegateFixture(delegated, 2L, 777L);
    delegated.install(fixture.delegate, null);
    MonitorHolder fastHolder = new MonitorHolder(delegated);
    fastHolder.start();
    AtomicLong fastValue = new AtomicLong(Long.MIN_VALUE);
    Thread fastReader = new Thread(() -> fastValue.set(delegated.getDuration()));
    fastReader.start();
    fastReader.join(2000L);
    check(!fastReader.isAlive() && fastValue.get() == 777L,
        "delegate fast path took track monitor");
    fastHolder.release();

    TestTrack process = new TestTrack(info(1L, "monitor-process"));
    DelegateFixture processFixture = new DelegateFixture(process, 3L, 4L);
    MonitorHolder processHolder = new MonitorHolder(process);
    processHolder.start();
    List<Throwable> failures = new ArrayList<>();
    Thread installer = new Thread(() -> {
      try {
        process.install(processFixture.delegate, null);
      } catch (Throwable error) {
        failures.add(error);
      }
    });
    installer.start();
    awaitBlocked(installer, "processDelegate was not synchronized on track");
    processHolder.release();
    installer.join(2000L);
    check(!installer.isAlive() && failures.isEmpty(), "synchronized install completion");
  }

  private static void reflection() throws Exception {
    Class<DelegatedAudioTrack> type = DelegatedAudioTrack.class;
    check(Modifier.isPublic(type.getModifiers()) && Modifier.isAbstract(type.getModifiers())
        && !Modifier.isFinal(type.getModifiers()) && type.getSuperclass()
        == com.sedmelluq.discord.lavaplayer.track.BaseAudioTrack.class
        && type.getInterfaces().length == 0, "class metadata");
    Field delegate = type.getDeclaredField("delegate");
    check(type.getDeclaredFields().length == 1 && delegate.getType() == InternalAudioTrack.class
        && delegate.getModifiers() == Modifier.PRIVATE, "delegate field metadata");
    Constructor<?>[] constructors = type.getDeclaredConstructors();
    check(constructors.length == 1 && Modifier.isPublic(constructors[0].getModifiers())
        && Arrays.equals(constructors[0].getParameterTypes(), new Class<?>[] { AudioTrackInfo.class })
        && constructors[0].getExceptionTypes().length == 0, "constructor metadata");
    int exportedMethods = 0;
    for (Method method : type.getDeclaredMethods()) {
      if (Modifier.isPublic(method.getModifiers()) || Modifier.isProtected(method.getModifiers())) {
        exportedMethods++;
      }
    }
    check(exportedMethods == 4, "exported method count");
    Method process = type.getDeclaredMethod("processDelegate", InternalAudioTrack.class,
        LocalAudioTrackExecutor.class);
    check(Modifier.isProtected(process.getModifiers())
        && Modifier.isSynchronized(process.getModifiers()) && !Modifier.isAbstract(process.getModifiers())
        && Arrays.equals(process.getExceptionTypes(), new Class<?>[] { Exception.class }),
        "process metadata");
    for (String name : Arrays.asList("setPosition", "getDuration", "getPosition")) {
      Method method = name.equals("setPosition")
          ? type.getDeclaredMethod(name, long.class) : type.getDeclaredMethod(name);
      check(Modifier.isPublic(method.getModifiers())
          && !Modifier.isSynchronized(method.getModifiers()), name + " metadata");
    }
  }

  private static void awaitBlocked(Thread thread, String message) throws Exception {
    long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(2L);
    while (thread.getState() != Thread.State.BLOCKED && System.nanoTime() < deadline) {
      Thread.yield();
    }
    check(thread.getState() == Thread.State.BLOCKED, message);
  }

  private static AudioTrackInfo info(long length, String identifier) {
    return new AudioTrackInfo("title", "author", length, identifier, false,
        "uri", "art", "isrc");
  }

  private static final class TestTrack extends DelegatedAudioTrack {
    TestTrack(AudioTrackInfo info) { super(info); }
    public void process(LocalAudioTrackExecutor executor) { }
    protected AudioTrack makeShallowClone() { return new TestTrack(trackInfo); }
    void install(InternalAudioTrack delegate, LocalAudioTrackExecutor executor) throws Exception {
      processDelegate(delegate, executor);
    }
    void setAccurateDuration(long value) { accurateDuration.set(value); }
  }

  private static final class DelegateFixture {
    final TestTrack owner;
    final InternalAudioTrack delegate;
    final List<String> calls = new ArrayList<>();
    long position;
    long duration;
    RuntimeException assignFailure;
    Exception processFailure;
    RuntimeException positionFailure;
    RuntimeException durationFailure;

    DelegateFixture(TestTrack owner, long position, long duration) {
      this.owner = owner;
      this.position = position;
      this.duration = duration;
      this.delegate = (InternalAudioTrack) Proxy.newProxyInstance(
          InternalAudioTrack.class.getClassLoader(), new Class<?>[] { InternalAudioTrack.class },
          (instance, method, arguments) -> {
            switch (method.getName()) {
              case "assignExecutor":
                calls.add("assign:" + arguments[0] + ":" + arguments[1]);
                calls.add("published:" + owner.getPosition());
                if (assignFailure != null) throw assignFailure;
                return null;
              case "process":
                calls.add("process:" + arguments[0]);
                if (processFailure != null) throw processFailure;
                return null;
              case "setPosition":
                this.position = (Long) arguments[0];
                calls.add("setPosition:" + this.position);
                if (positionFailure != null) throw positionFailure;
                return null;
              case "getPosition":
                return this.position;
              case "getDuration":
                if (durationFailure != null) throw durationFailure;
                return duration;
              default:
                return defaultValue(method.getReturnType());
            }
          });
    }
  }

  private static final class MonitorHolder {
    final Object monitor;
    final CountDownLatch locked = new CountDownLatch(1);
    final CountDownLatch release = new CountDownLatch(1);
    final Thread thread;

    MonitorHolder(Object monitor) {
      this.monitor = monitor;
      this.thread = new Thread(() -> {
        synchronized (this.monitor) {
          locked.countDown();
          try {
            release.await();
          } catch (InterruptedException error) {
            throw new RuntimeException(error);
          }
        }
      });
    }

    void start() throws Exception {
      thread.start();
      check(locked.await(2L, TimeUnit.SECONDS), "monitor holder did not lock");
    }

    void release() throws Exception {
      release.countDown();
      thread.join(2000L);
      check(!thread.isAlive(), "monitor holder did not exit");
    }
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == long.class) return 0L;
    if (type == int.class) return 0;
    if (type == short.class) return (short) 0;
    if (type == byte.class) return (byte) 0;
    if (type == float.class) return 0.0f;
    if (type == double.class) return 0.0d;
    if (type == char.class) return (char) 0;
    return null;
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const AUDIO_TRACK_INFO_BUILDER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.tools.io.SeekableInputStream;
import com.sedmelluq.discord.lavaplayer.track.AudioReference;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import com.sedmelluq.discord.lavaplayer.track.info.AudioTrackInfoBuilder;
import com.sedmelluq.discord.lavaplayer.track.info.AudioTrackInfoProvider;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

public final class GateAudioTrackInfoBuilder {
  public static void main(String[] args) throws Exception {
    emptyAndSetters();
    buildsAndInference();
    applyOrderingAndFailures();
    createAndProviders();
    reflection();
    System.out.println(
        "empty=nulls,distinct;setters=fluent,null-retain,stream-reset;"
        + "build=unknown,finite,explicit-stream,snapshot;"
        + "apply=null,ordered,partial-failure,no-stream;"
        + "create=defaults,reference,providers,null-list,failure;"
        + "reflection=class,10-fields,1-private-constructor,19-methods");
  }

  private static void emptyAndSetters() {
    AudioTrackInfoBuilder first = AudioTrackInfoBuilder.empty();
    AudioTrackInfoBuilder second = AudioTrackInfoBuilder.empty();
    check(first != second && first.getTitle() == null && first.getAuthor() == null
        && first.getLength() == null && first.getIdentifier() == null && first.getUri() == null
        && first.getArtworkUrl() == null && first.getISRC() == null, "empty builder state");
    check(first.setTitle("title") == first && first.setAuthor("author") == first
        && first.setLength(-7L) == first && first.setIdentifier("id") == first
        && first.setUri("uri") == first && first.setArtworkUrl("art") == first
        && first.setISRC("isrc") == first && first.setIsStream(Boolean.TRUE) == first,
        "fluent setters");
    first.setTitle(null).setAuthor(null).setLength(null).setIdentifier(null).setUri(null)
        .setArtworkUrl(null).setISRC(null);
    check(first.getTitle().equals("title") && first.getAuthor().equals("author")
        && first.getLength().equals(-7L) && first.getIdentifier().equals("id")
        && first.getUri().equals("uri") && first.getArtworkUrl().equals("art")
        && first.getISRC().equals("isrc"), "null setters retain values");
    first.setIsStream(null);
    check(!first.build().isStream, "null stream setter resets inference");
  }

  private static void buildsAndInference() {
    AudioTrackInfo unknown = AudioTrackInfoBuilder.empty().build();
    check(unknown.title == null && unknown.author == null && unknown.length == Long.MAX_VALUE
        && unknown.identifier == null && unknown.isStream && unknown.uri == null
        && unknown.artworkUrl == null && unknown.isrc == null, "empty build defaults");
    AudioTrackInfo finite = AudioTrackInfoBuilder.empty().setLength(0L).build();
    check(finite.length == 0L && !finite.isStream, "finite inference");
    AudioTrackInfo negative = AudioTrackInfoBuilder.empty().setLength(Long.MIN_VALUE).build();
    check(!negative.isStream && negative.length == Long.MIN_VALUE, "negative length");
    AudioTrackInfo forcedStream = AudioTrackInfoBuilder.empty().setLength(12L)
        .setIsStream(Boolean.TRUE).build();
    AudioTrackInfo forcedFinite = AudioTrackInfoBuilder.empty().setLength(Long.MAX_VALUE)
        .setIsStream(Boolean.FALSE).build();
    check(forcedStream.isStream && !forcedFinite.isStream, "explicit stream overrides");

    AudioTrackInfoBuilder builder = AudioTrackInfoBuilder.empty().setTitle("before").setLength(1L);
    AudioTrackInfo before = builder.build();
    AudioTrackInfo again = builder.build();
    builder.setTitle("after").setLength(2L);
    check(before != again && before.title.equals("before") && before.length == 1L
        && builder.build().title.equals("after"), "fresh immutable snapshots");
  }

  private static void applyOrderingAndFailures() {
    AudioTrackInfoBuilder builder = AudioTrackInfoBuilder.empty().setTitle("old-title")
        .setAuthor("old-author").setLength(1L).setIdentifier("old-id").setUri("old-uri")
        .setArtworkUrl("old-art").setISRC("old-isrc").setIsStream(Boolean.TRUE);
    check(builder.apply(null) == builder, "null provider identity");
    List<String> calls = new ArrayList<>();
    AudioTrackInfoProvider provider = provider(calls,
        "new-title", null, 5L, "new-id", "new-uri", null, "new-isrc", null, null);
    check(builder.apply(provider) == builder
        && calls.equals(Arrays.asList("title", "author", "length", "identifier", "uri",
            "artwork", "isrc")), "provider getter order");
    check(builder.getTitle().equals("new-title") && builder.getAuthor().equals("old-author")
        && builder.getLength().equals(5L) && builder.getIdentifier().equals("new-id")
        && builder.getUri().equals("new-uri") && builder.getArtworkUrl().equals("old-art")
        && builder.getISRC().equals("new-isrc") && builder.build().isStream,
        "provider values and stream omission");

    RuntimeException failure = new RuntimeException("uri-sentinel");
    List<String> failedCalls = new ArrayList<>();
    AudioTrackInfoProvider failing = provider(failedCalls,
        "partial-title", "partial-author", 9L, "partial-id", "unused-uri", "unused-art",
        "unused-isrc", "uri", failure);
    try {
      builder.apply(failing);
      throw new AssertionError("provider failure swallowed");
    } catch (RuntimeException error) {
      check(error == failure && failedCalls.equals(Arrays.asList(
          "title", "author", "length", "identifier", "uri")), "partial failure order");
    }
    check(builder.getTitle().equals("partial-title") && builder.getAuthor().equals("partial-author")
        && builder.getLength().equals(9L) && builder.getIdentifier().equals("partial-id")
        && builder.getUri().equals("new-uri") && builder.getArtworkUrl().equals("old-art")
        && builder.getISRC().equals("new-isrc"), "partial application retained prefix");
  }

  private static void createAndProviders() {
    AudioTrackInfo defaults = AudioTrackInfoBuilder.create(null, null).build();
    check(defaults.title.equals("Unknown title") && defaults.author.equals("Unknown artist")
        && defaults.length == Long.MAX_VALUE && defaults.isStream, "create defaults");
    AudioReference reference = new AudioReference("reference-id", "reference-title");
    AudioTrackInfo referenced = AudioTrackInfoBuilder.create(reference, null).build();
    check(referenced.title.equals("reference-title")
        && referenced.author.equals("Unknown artist") && referenced.identifier.equals("reference-id")
        && referenced.length == Long.MAX_VALUE, "reference overlay");

    AudioTrackInfoProvider first = provider(new ArrayList<>(), "stream-title", null, 33L,
        null, "stream-uri", null, null, null, null);
    AudioTrackInfoProvider second = provider(new ArrayList<>(), null, "stream-author", null,
        "stream-id", null, "stream-art", "stream-isrc", null, null);
    TestStream stream = new TestStream(Arrays.asList(first, null, second));
    AudioTrackInfo combined = AudioTrackInfoBuilder.create(reference, stream).build();
    check(combined.title.equals("stream-title") && combined.author.equals("stream-author")
        && combined.length == 33L && combined.identifier.equals("stream-id")
        && combined.uri.equals("stream-uri") && combined.artworkUrl.equals("stream-art")
        && combined.isrc.equals("stream-isrc") && !combined.isStream,
        "ordered stream provider overlays");

    try {
      AudioTrackInfoBuilder.create(null,
          new TestStream((List<AudioTrackInfoProvider>) null));
      throw new AssertionError("null provider list succeeded");
    } catch (NullPointerException expected) { }
    RuntimeException failure = new RuntimeException("providers-sentinel");
    try {
      AudioTrackInfoBuilder.create(null, new TestStream(failure));
      throw new AssertionError("provider list failure swallowed");
    } catch (RuntimeException error) {
      check(error == failure, "provider list failure identity");
    }
  }

  private static void reflection() throws Exception {
    Class<AudioTrackInfoBuilder> type = AudioTrackInfoBuilder.class;
    check(Modifier.isPublic(type.getModifiers()) && !Modifier.isFinal(type.getModifiers())
        && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] { AudioTrackInfoProvider.class }),
        "class metadata");
    check(type.getFields().length == 0 && type.getDeclaredFields().length == 10,
        "field counts");
    checkField(type, "UNKNOWN_TITLE", String.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "UNKNOWN_ARTIST", String.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    for (String name : Arrays.asList("title", "author", "identifier", "uri", "artworkUrl", "isrc")) {
      checkField(type, name, String.class, Modifier.PRIVATE);
    }
    checkField(type, "length", Long.class, Modifier.PRIVATE);
    checkField(type, "isStream", Boolean.class, Modifier.PRIVATE);
    Constructor<?>[] constructors = type.getDeclaredConstructors();
    check(constructors.length == 1 && Modifier.isPrivate(constructors[0].getModifiers())
        && constructors[0].getParameterCount() == 0, "private constructor");
    Method[] methods = type.getDeclaredMethods();
    check(methods.length == 19, "declared method count");
    for (Method method : methods) check(Modifier.isPublic(method.getModifiers()), "method visibility");
    check(Modifier.isStatic(type.getDeclaredMethod("empty").getModifiers())
        && Modifier.isStatic(type.getDeclaredMethod("create", AudioReference.class,
            SeekableInputStream.class).getModifiers()), "static factories");
  }

  private static void checkField(Class<?> type, String name, Class<?> fieldType, int modifiers)
      throws Exception {
    Field field = type.getDeclaredField(name);
    check(field.getType() == fieldType && field.getModifiers() == modifiers,
        name + " field metadata");
  }

  private static AudioTrackInfoProvider provider(List<String> calls, String title, String author,
      Long length, String identifier, String uri, String artwork, String isrc,
      String failingGetter, RuntimeException failure) {
    return (AudioTrackInfoProvider) Proxy.newProxyInstance(AudioTrackInfoProvider.class.getClassLoader(),
        new Class<?>[] { AudioTrackInfoProvider.class }, (instance, method, arguments) -> {
          String key;
          switch (method.getName()) {
            case "getTitle": key = "title"; break;
            case "getAuthor": key = "author"; break;
            case "getLength": key = "length"; break;
            case "getIdentifier": key = "identifier"; break;
            case "getUri": key = "uri"; break;
            case "getArtworkUrl": key = "artwork"; break;
            case "getISRC": key = "isrc"; break;
            default: return null;
          }
          calls.add(key);
          if (key.equals(failingGetter)) throw failure;
          switch (key) {
            case "title": return title;
            case "author": return author;
            case "length": return length;
            case "identifier": return identifier;
            case "uri": return uri;
            case "artwork": return artwork;
            default: return isrc;
          }
        });
  }

  private static final class TestStream extends SeekableInputStream {
    final List<AudioTrackInfoProvider> providers;
    final RuntimeException failure;
    TestStream(List<AudioTrackInfoProvider> providers) {
      super(0L, 0L); this.providers = providers; this.failure = null;
    }
    TestStream(RuntimeException failure) {
      super(0L, 0L); this.providers = null; this.failure = failure;
    }
    public int read() { return -1; }
    public long getPosition() { return 0L; }
    protected void seekHard(long position) throws IOException { }
    public boolean canSeekHard() { return true; }
    public List<AudioTrackInfoProvider> getTrackInfoProviders() {
      if (failure != null) throw failure;
      return providers;
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const ABSTRACT_AUDIO_FRAME_BUFFER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.format.AudioDataFormat;
import com.sedmelluq.discord.lavaplayer.format.transcoder.AudioChunkDecoder;
import com.sedmelluq.discord.lavaplayer.format.transcoder.AudioChunkEncoder;
import com.sedmelluq.discord.lavaplayer.player.AudioConfiguration;
import com.sedmelluq.discord.lavaplayer.track.playback.AbstractAudioFrameBuffer;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameBuffer;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameRebuilder;
import com.sedmelluq.discord.lavaplayer.track.playback.MutableAudioFrame;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.concurrent.atomic.AtomicBoolean;

public final class GateAbstractAudioFrameBuffer {
  public static void main(String[] args) throws Exception {
    constructionAndFlags();
    terminationAndClearOrdering();
    failureRetentionAndMonitorRelease();
    waitLoopInterruptionAndMonitorBlocking();
    reflection();
    System.out.println(
        "constructor=format,null,unique-monitor,zero-flags;"
        + "flags=clear-cancels-terminate,lock,received;"
        + "terminate=ordered-clear,signal,terminated-skip;"
        + "failures=clear-prefix,signal-prefix,monitor-release;"
        + "wait=loop,notify,interrupt,monitor-blocking;"
        + "reflection=public-abstract,7-fields,1-protected-constructor,7-methods");
  }

  private static void constructionAndFlags() {
    AudioDataFormat format = new TestFormat();
    ProbeBuffer first = new ProbeBuffer(format);
    ProbeBuffer second = new ProbeBuffer(null);
    check(first.formatValue() == format && second.formatValue() == null
        && first.monitor() != second.monitor()
        && first.monitor().getClass() == Object.class, "constructor identities");
    check(!first.lockedValue() && !first.receivedValue() && !first.terminatedValue()
        && !first.terminateOnEmptyValue() && !first.hasClearOnInsert()
        && !first.hasReceivedFrames(), "constructor flags");
    first.lockBuffer();
    first.setReceived(true);
    check(first.lockedValue() && first.hasReceivedFrames(), "volatile flag access");
    first.setReceived(false);
    check(!first.hasReceivedFrames(), "received frame reset visibility");
  }

  private static void terminationAndClearOrdering() {
    ProbeBuffer buffer = new ProbeBuffer(null);
    buffer.setTerminateOnEmpty();
    check(buffer.terminateOnEmptyValue() && buffer.signals == 1 && buffer.clears == 0
        && buffer.signalHeldMonitor, "initial termination request");

    buffer.setClearOnInsert();
    check(buffer.hasClearOnInsert() && !buffer.terminateOnEmptyValue(),
        "clear request cancels termination");
    buffer.setTerminateOnEmpty();
    check(!buffer.hasClearOnInsert() && buffer.terminateOnEmptyValue()
        && buffer.clears == 1 && buffer.signals == 2
        && buffer.clearHeldMonitor && buffer.signalHeldMonitor,
        "clear then termination ordering");

    buffer.reset(false, false, true);
    int previousSignals = buffer.signals;
    buffer.setClearOnInsert();
    buffer.setTerminateOnEmpty();
    check(buffer.clears == 2 && !buffer.hasClearOnInsert()
        && !buffer.terminateOnEmptyValue() && buffer.signals == previousSignals,
        "already terminated still consumes clear without signal");
  }

  private static void failureRetentionAndMonitorRelease() throws Exception {
    ProbeBuffer clearFailure = new ProbeBuffer(null);
    RuntimeException clearSentinel = new RuntimeException("clear-sentinel");
    clearFailure.setClearOnInsert();
    clearFailure.clearFailure = clearSentinel;
    expectIdentity(clearSentinel, clearFailure::setTerminateOnEmpty);
    check(clearFailure.hasClearOnInsert() && !clearFailure.terminateOnEmptyValue()
        && clearFailure.clears == 1 && clearFailure.signals == 0
        && clearFailure.clearHeldMonitor, "clear failure retained prefix");
    assertMonitorAvailable(clearFailure.monitor(), "clear failure monitor release");

    ProbeBuffer signalFailure = new ProbeBuffer(null);
    RuntimeException signalSentinel = new RuntimeException("signal-sentinel");
    signalFailure.signalFailure = signalSentinel;
    expectIdentity(signalSentinel, signalFailure::setTerminateOnEmpty);
    check(signalFailure.terminateOnEmptyValue() && signalFailure.signals == 1
        && signalFailure.signalHeldMonitor, "signal failure retained prefix");
    assertMonitorAvailable(signalFailure.monitor(), "signal failure monitor release");
  }

  private static void waitLoopInterruptionAndMonitorBlocking() throws Exception {
    ProbeBuffer waiting = new ProbeBuffer(null);
    AtomicBoolean returned = new AtomicBoolean();
    AtomicBoolean failed = new AtomicBoolean();
    Thread waiter = daemon(() -> {
      try {
        waiting.waitForTermination();
        returned.set(true);
      } catch (InterruptedException error) {
        failed.set(true);
      }
    });
    waiter.start();
    awaitState(waiter, Thread.State.WAITING, "initial termination wait");
    waiting.pokeAndObserveBlocked(waiter);
    awaitState(waiter, Thread.State.WAITING, "spurious notification loop");
    check(!returned.get() && !failed.get(), "spurious notification did not return");
    waiting.finish();
    join(waiter, "terminated waiter");
    check(returned.get() && !failed.get(), "termination releases waiter");
    waiting.waitForTermination();

    ProbeBuffer interrupted = new ProbeBuffer(null);
    AtomicBoolean interruptedThrown = new AtomicBoolean();
    AtomicBoolean interruptStatus = new AtomicBoolean(true);
    Thread interruptedWaiter = daemon(() -> {
      try {
        interrupted.waitForTermination();
      } catch (InterruptedException error) {
        interruptedThrown.set(true);
        interruptStatus.set(Thread.currentThread().isInterrupted());
      }
    });
    interruptedWaiter.start();
    awaitState(interruptedWaiter, Thread.State.WAITING, "interrupt wait");
    interruptedWaiter.interrupt();
    join(interruptedWaiter, "interrupted waiter");
    check(interruptedThrown.get() && !interruptStatus.get(), "interrupt propagation");
    assertMonitorAvailable(interrupted.monitor(), "interrupted wait monitor release");

    ProbeBuffer blocked = new ProbeBuffer(null);
    AtomicBoolean entered = new AtomicBoolean();
    AtomicBoolean completed = new AtomicBoolean();
    Thread setter;
    synchronized (blocked.monitor()) {
      setter = daemon(() -> {
        entered.set(true);
        blocked.setClearOnInsert();
        completed.set(true);
      });
      setter.start();
      awaitTrue(entered, "setter start");
      awaitState(setter, Thread.State.BLOCKED, "setter monitor block");
      check(!completed.get(), "setter completed while monitor held");
    }
    join(setter, "unblocked setter");
    check(completed.get() && blocked.hasClearOnInsert(), "setter after monitor release");
  }

  private static void reflection() throws Exception {
    Class<AbstractAudioFrameBuffer> type = AbstractAudioFrameBuffer.class;
    check(Modifier.isPublic(type.getModifiers()) && Modifier.isAbstract(type.getModifiers())
        && !Modifier.isFinal(type.getModifiers()) && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] { AudioFrameBuffer.class }),
        "class metadata");
    check(type.getDeclaredFields().length == 7 && type.getFields().length == 0,
        "field counts");
    checkField(type, "format", AudioDataFormat.class, Modifier.PROTECTED | Modifier.FINAL);
    checkField(type, "synchronizer", Object.class, Modifier.PROTECTED | Modifier.FINAL);
    checkField(type, "locked", boolean.class, Modifier.PROTECTED | Modifier.VOLATILE);
    checkField(type, "receivedFrames", boolean.class, Modifier.PROTECTED | Modifier.VOLATILE);
    checkField(type, "terminated", boolean.class, Modifier.PROTECTED);
    checkField(type, "terminateOnEmpty", boolean.class, Modifier.PROTECTED);
    checkField(type, "clearOnInsert", boolean.class, Modifier.PROTECTED);

    Constructor<?>[] constructors = type.getDeclaredConstructors();
    check(constructors.length == 1 && constructors[0].getModifiers() == Modifier.PROTECTED
        && Arrays.equals(constructors[0].getParameterTypes(),
            new Class<?>[] { AudioDataFormat.class }), "constructor metadata");
    Method[] methods = type.getDeclaredMethods();
    check(methods.length == 7, "declared method count");
    Method wait = type.getDeclaredMethod("waitForTermination");
    check(wait.getModifiers() == Modifier.PUBLIC
        && Arrays.equals(wait.getExceptionTypes(), new Class<?>[] { InterruptedException.class }),
        "wait metadata");
    for (String name : Arrays.asList("setTerminateOnEmpty", "setClearOnInsert", "lockBuffer")) {
      Method method = type.getDeclaredMethod(name);
      check(method.getModifiers() == Modifier.PUBLIC && method.getReturnType() == void.class
          && method.getExceptionTypes().length == 0, name + " metadata");
    }
    for (String name : Arrays.asList("hasClearOnInsert", "hasReceivedFrames")) {
      Method method = type.getDeclaredMethod(name);
      check(method.getModifiers() == Modifier.PUBLIC && method.getReturnType() == boolean.class
          && method.getExceptionTypes().length == 0, name + " metadata");
    }
    Method signal = type.getDeclaredMethod("signalWaiters");
    check(signal.getModifiers() == (Modifier.PROTECTED | Modifier.ABSTRACT)
        && signal.getReturnType() == void.class && signal.getExceptionTypes().length == 0,
        "signal metadata");
  }

  private static void checkField(Class<?> type, String name, Class<?> fieldType, int modifiers)
      throws Exception {
    Field field = type.getDeclaredField(name);
    check(field.getType() == fieldType && field.getModifiers() == modifiers,
        name + " field metadata");
  }

  private static void expectIdentity(RuntimeException expected, Operation operation) {
    try {
      operation.run();
      throw new AssertionError("failure was swallowed");
    } catch (RuntimeException error) {
      check(error == expected, "failure identity");
    }
  }

  private static void assertMonitorAvailable(Object monitor, String message) throws Exception {
    AtomicBoolean acquired = new AtomicBoolean();
    Thread thread = daemon(() -> {
      synchronized (monitor) {
        acquired.set(true);
      }
    });
    thread.start();
    join(thread, message);
    check(acquired.get(), message);
  }

  private static Thread daemon(Runnable operation) {
    Thread thread = new Thread(operation);
    thread.setDaemon(true);
    return thread;
  }

  private static void awaitTrue(AtomicBoolean value, String message) throws Exception {
    long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(2L);
    while (!value.get() && System.nanoTime() < deadline) Thread.sleep(1L);
    check(value.get(), message);
  }

  private static void awaitState(Thread thread, Thread.State state, String message)
      throws Exception {
    long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(2L);
    while (thread.getState() != state && System.nanoTime() < deadline) Thread.sleep(1L);
    check(thread.getState() == state, message + ": " + thread.getState());
  }

  private static void join(Thread thread, String message) throws Exception {
    thread.join(2_000L);
    check(!thread.isAlive(), message);
  }

  private interface Operation { void run(); }

  private static final class ProbeBuffer extends AbstractAudioFrameBuffer {
    int clears;
    int signals;
    boolean clearHeldMonitor;
    boolean signalHeldMonitor;
    RuntimeException clearFailure;
    RuntimeException signalFailure;

    ProbeBuffer(AudioDataFormat format) { super(format); }
    AudioDataFormat formatValue() { return format; }
    Object monitor() { return synchronizer; }
    boolean lockedValue() { return locked; }
    boolean receivedValue() { return receivedFrames; }
    boolean terminatedValue() { return terminated; }
    boolean terminateOnEmptyValue() { return terminateOnEmpty; }
    void setReceived(boolean value) { receivedFrames = value; }
    void reset(boolean terminate, boolean clear, boolean ended) {
      terminateOnEmpty = terminate;
      clearOnInsert = clear;
      terminated = ended;
    }
    void pokeAndObserveBlocked(Thread waiter) throws Exception {
      synchronized (synchronizer) {
        synchronizer.notifyAll();
        awaitState(waiter, Thread.State.BLOCKED, "notified waiter monitor reacquisition");
      }
    }
    void finish() {
      synchronized (synchronizer) {
        terminated = true;
        synchronizer.notifyAll();
      }
    }

    public int getRemainingCapacity() { return 0; }
    public int getFullCapacity() { return 0; }
    public AudioFrame provide() { return null; }
    public AudioFrame provide(long timeout, TimeUnit unit) throws TimeoutException,
        InterruptedException { return null; }
    public boolean provide(MutableAudioFrame frame) { return false; }
    public boolean provide(MutableAudioFrame frame, long timeout, TimeUnit unit)
        throws TimeoutException, InterruptedException { return false; }
    public void consume(AudioFrame frame) throws InterruptedException { }
    public void clear() {
      clears++;
      clearHeldMonitor = Thread.holdsLock(synchronizer);
      if (clearFailure != null) throw clearFailure;
    }
    public void rebuild(AudioFrameRebuilder rebuilder) { }
    public Long getLastInputTimecode() { return null; }
    protected void signalWaiters() {
      signals++;
      signalHeldMonitor = Thread.holdsLock(synchronizer);
      if (signalFailure != null) throw signalFailure;
    }
  }

  private static final class TestFormat extends AudioDataFormat {
    TestFormat() { super(1, 1, 1); }
    public String codecName() { return "test"; }
    public byte[] silenceBytes() { return new byte[0]; }
    public int expectedChunkSize() { return 0; }
    public int maximumChunkSize() { return 0; }
    public AudioChunkDecoder createDecoder() { return null; }
    public AudioChunkEncoder createEncoder(AudioConfiguration configuration) { return null; }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const ALLOCATING_AUDIO_FRAME_BUFFER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.format.AudioDataFormat;
import com.sedmelluq.discord.lavaplayer.format.transcoder.AudioChunkDecoder;
import com.sedmelluq.discord.lavaplayer.format.transcoder.AudioChunkEncoder;
import com.sedmelluq.discord.lavaplayer.player.AudioConfiguration;
import com.sedmelluq.discord.lavaplayer.track.playback.AbstractAudioFrameBuffer;
import com.sedmelluq.discord.lavaplayer.track.playback.AllocatingAudioFrameBuffer;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameRebuilder;
import com.sedmelluq.discord.lavaplayer.track.playback.ImmutableAudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.MutableAudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.TerminatorAudioFrame;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.nio.ByteBuffer;
import java.util.Arrays;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicReference;

public final class GateAllocatingAudioFrameBuffer {
  private static final TestFormat FORMAT = new TestFormat();

  public static void main(String[] args) throws Exception {
    constructorCapacityAndBasicConsumption();
    mutableFreezeSilenceAndTimedProvide();
    terminationWaitAndBackpressure();
    rebuildClearAndLastTimecode();
    reflection();
    System.out.println(
        "constructor=capacity,format,stopping,private-layout;"
        + "consume=order,stop,lock,null,clear,freeze,backpressure,interrupt;"
        + "provide=identity,silence,timed,mutable,terminator;"
        + "termination=full-queue,pending,wakeup,multiple;"
        + "rebuild=order,partial-failure,null,clear,last-timecode,monitor;"
        + "reflection=public-class,4-private-fields,1-constructor,14-methods");
  }

  private static void constructorCapacityAndBasicConsumption() throws Exception {
    AtomicBoolean stopping = new AtomicBoolean();
    ProbeBuffer buffer = new ProbeBuffer(39, FORMAT, stopping);
    check(buffer.getFullCapacity() == 2 && buffer.getRemainingCapacity() == 2
        && buffer.formatValue() == FORMAT && field(buffer, "stopping") == stopping,
        "constructor and capacity");
    expect(IllegalArgumentException.class, () -> new ProbeBuffer(-20, FORMAT, null));

    AudioFrame first = frame(1L, 10, 1);
    AudioFrame second = frame(2L, 20, 2);
    buffer.consume(first);
    buffer.consume(second);
    check(buffer.hasReceivedFrames() && buffer.getRemainingCapacity() == 0
        && buffer.provide() == first && buffer.provide() == second
        && buffer.provide() == null, "queue order and identity");

    ProbeBuffer stopped = new ProbeBuffer(0, FORMAT, new AtomicBoolean(true));
    expect(InterruptedException.class, () -> stopped.consume(first));
    check(!stopped.hasReceivedFrames() && stopped.getRemainingCapacity() == 1,
        "stopping preflight");
    ProbeBuffer locked = new ProbeBuffer(0, FORMAT, null);
    locked.lockBuffer();
    locked.consume(first);
    check(!locked.hasReceivedFrames() && locked.getRemainingCapacity() == 1,
        "locked discard");

    ProbeBuffer nullFrame = new ProbeBuffer(0, FORMAT, null);
    expect(NullPointerException.class, () -> nullFrame.consume(null));
    check(nullFrame.hasReceivedFrames() && nullFrame.getRemainingCapacity() == 1,
        "null failure after received flag");
  }

  private static void mutableFreezeSilenceAndTimedProvide() throws Exception {
    ProbeBuffer buffer = new ProbeBuffer(60, FORMAT, null);
    MutableAudioFrame mutable = new MutableAudioFrame(ByteBuffer.allocate(16));
    mutable.setTimecode(11L);
    mutable.setVolume(55);
    mutable.setFormat(FORMAT);
    mutable.store(new byte[] { 3, 4, 5 }, 0, 3);
    buffer.consume(mutable);
    mutable.setTimecode(99L);
    mutable.store(new byte[] { 8 }, 0, 1);
    AudioFrame frozen = buffer.provide();
    check(frozen != mutable && frozen.getTimecode() == 11L && frozen.getVolume() == 55
        && Arrays.equals(frozen.getData(), new byte[] { 3, 4, 5 })
        && frozen.getFormat() == FORMAT, "mutable freeze snapshot");

    AudioFrame silentInput = frame(21L, 0, 7, 7, 7);
    buffer.consume(silentInput);
    AudioFrame silent = buffer.provide();
    check(silent != silentInput && silent.getTimecode() == 21L && silent.getVolume() == 0
        && silent.getFormat() == FORMAT
        && Arrays.equals(silent.getData(), FORMAT.silenceBytes()), "silence substitution");

    MutableAudioFrame target = new MutableAudioFrame(ByteBuffer.allocate(16));
    buffer.consume(frame(31L, 77, 4, 5));
    check(buffer.provide(target) && target.getTimecode() == 31L && target.getVolume() == 77
        && Arrays.equals(target.getData(), new byte[] { 4, 5 })
        && target.getFormat() == FORMAT && !target.isTerminator(), "mutable copy");
    buffer.consume(frame(32L, 10, 6));
    check(!buffer.provide((MutableAudioFrame) null) && buffer.provide() == null,
        "null mutable target still consumes");
    check(!buffer.provide(target) && !buffer.provide(target, 0L, null)
        && buffer.provide(0L, null) == null && buffer.provide(-1L, null) == null,
        "empty and non-positive timed provide");
    expect(NullPointerException.class, () -> buffer.provide(1L, null));

    AtomicReference<AudioFrame> delivered = new AtomicReference<>();
    AtomicReference<Throwable> failure = new AtomicReference<>();
    Thread timed = daemon(() -> {
      try {
        delivered.set(buffer.provide(2L, TimeUnit.SECONDS));
      } catch (Throwable error) {
        failure.set(error);
      }
    });
    timed.start();
    awaitState(timed, Thread.State.TIMED_WAITING, "timed provide wait");
    AudioFrame later = frame(41L, 9, 9);
    buffer.consume(later);
    join(timed, "timed delivery");
    check(failure.get() == null && delivered.get() == later, "timed delivery identity");

    Thread interrupted = daemon(() -> {
      try {
        buffer.provide(2L, TimeUnit.SECONDS);
      } catch (Throwable error) {
        failure.set(error);
      }
    });
    failure.set(null);
    interrupted.start();
    awaitState(interrupted, Thread.State.TIMED_WAITING, "timed interrupt wait");
    interrupted.interrupt();
    join(interrupted, "timed interrupt");
    check(failure.get() instanceof InterruptedException, "timed interrupt propagation");
  }

  private static void terminationWaitAndBackpressure() throws Exception {
    ProbeBuffer full = new ProbeBuffer(0, FORMAT, null);
    AudioFrame ordinary = frame(51L, 10, 1);
    full.consume(ordinary);
    full.setTerminateOnEmpty();
    check(full.provide() == ordinary && !full.terminatedValue(),
        "full queue defers terminator");
    check(full.provide() == TerminatorAudioFrame.INSTANCE && full.terminatedValue()
        && !full.terminateOnEmptyValue(), "pending terminator after drain");

    ProbeBuffer waiting = new ProbeBuffer(20, FORMAT, null);
    AtomicBoolean waitReturned = new AtomicBoolean();
    Thread waiter = daemon(() -> {
      try {
        waiting.waitForTermination();
        waitReturned.set(true);
      } catch (InterruptedException error) {
        throw new AssertionError(error);
      }
    });
    waiter.start();
    awaitState(waiter, Thread.State.WAITING, "termination waiter");
    waiting.setTerminateOnEmpty();
    check(waiting.provide() == TerminatorAudioFrame.INSTANCE, "queued terminator");
    join(waiter, "termination notification");
    check(waitReturned.get() && waiting.terminatedValue(), "termination waiter released");

    ProbeBuffer multiple = new ProbeBuffer(40, FORMAT, null);
    multiple.setTerminateOnEmpty();
    multiple.setTerminateOnEmpty();
    check(multiple.provide() == TerminatorAudioFrame.INSTANCE
        && multiple.provide() == TerminatorAudioFrame.INSTANCE
        && multiple.provide() == null, "multiple queued terminators");

    ProbeBuffer blocked = new ProbeBuffer(0, FORMAT, null);
    AudioFrame first = frame(61L, 10, 1);
    AudioFrame second = frame(62L, 10, 2);
    blocked.consume(first);
    AtomicReference<Throwable> consumeFailure = new AtomicReference<>();
    Thread producer = daemon(() -> {
      try {
        blocked.consume(second);
      } catch (Throwable error) {
        consumeFailure.set(error);
      }
    });
    producer.start();
    awaitState(producer, Thread.State.WAITING, "blocked producer");
    check(blocked.provide() == first, "backpressure release frame");
    join(producer, "released producer");
    check(consumeFailure.get() == null && blocked.provide() == second,
        "backpressure completion");

    blocked.consume(first);
    Thread interruptedProducer = daemon(() -> {
      try {
        blocked.consume(second);
      } catch (Throwable error) {
        consumeFailure.set(error);
      }
    });
    consumeFailure.set(null);
    interruptedProducer.start();
    awaitState(interruptedProducer, Thread.State.WAITING, "interruptible producer");
    interruptedProducer.interrupt();
    join(interruptedProducer, "interrupted producer");
    check(consumeFailure.get() instanceof InterruptedException && blocked.provide() == first
        && blocked.provide() == null, "blocked consume interruption");
  }

  private static void rebuildClearAndLastTimecode() throws Exception {
    ProbeBuffer buffer = new ProbeBuffer(80, FORMAT, null);
    buffer.consume(frame(1L, 10, 1));
    buffer.consume(frame(2L, 10, 2));
    buffer.consume(frame(3L, 10, 3));
    check(buffer.getLastInputTimecode().equals(3L), "last queued timecode");
    buffer.rebuild(frame -> frameBytes(frame.getTimecode() + 100L, frame.getVolume(),
        frame.getData()));
    check(buffer.provide().getTimecode() == 101L && buffer.provide().getTimecode() == 102L
        && buffer.provide().getTimecode() == 103L, "rebuild order");

    buffer.consume(frame(10L, 10, 1));
    buffer.consume(frame(20L, 10, 2));
    RuntimeException sentinel = new RuntimeException("rebuild-sentinel");
    AtomicBoolean first = new AtomicBoolean(true);
    expectIdentity(sentinel, () -> buffer.rebuild(frame -> {
      if (first.getAndSet(false)) return frame(frame.getTimecode() + 1L, 10, 8);
      throw sentinel;
    }));
    check(buffer.getRemainingCapacity() == buffer.getFullCapacity() - 1
        && buffer.provide().getTimecode() == 11L && buffer.provide() == null,
        "rebuild partial failure retains prefix only");

    buffer.consume(frame(30L, 10, 3));
    expect(NullPointerException.class, () -> buffer.rebuild(null));
    check(buffer.provide() == null, "null rebuilder drains first");
    buffer.consume(frame(40L, 10, 4));
    buffer.clear();
    check(buffer.getRemainingCapacity() == buffer.getFullCapacity()
        && buffer.getLastInputTimecode() == null, "clear empties queue");

    buffer.consume(frame(50L, 10, 5));
    buffer.setClearOnInsert();
    check(buffer.getLastInputTimecode() == null, "clear-on-insert masks last timecode");
    buffer.consume(frame(60L, 10, 6));
    check(!buffer.hasClearOnInsert() && buffer.getLastInputTimecode().equals(60L),
        "clear-on-insert consumed by next frame");

    AtomicReference<Long> observed = new AtomicReference<>();
    Thread reader;
    synchronized (buffer.monitor()) {
      reader = daemon(() -> observed.set(buffer.getLastInputTimecode()));
      reader.start();
      awaitState(reader, Thread.State.BLOCKED, "last timecode monitor block");
    }
    join(reader, "last timecode monitor release");
    check(observed.get().equals(60L), "last timecode synchronized read");
  }

  private static void reflection() throws Exception {
    Class<AllocatingAudioFrameBuffer> type = AllocatingAudioFrameBuffer.class;
    check(Modifier.isPublic(type.getModifiers()) && !Modifier.isAbstract(type.getModifiers())
        && !Modifier.isFinal(type.getModifiers())
        && type.getSuperclass() == AbstractAudioFrameBuffer.class
        && type.getInterfaces().length == 0, "class metadata");
    check(type.getDeclaredFields().length == 4 && type.getFields().length == 0,
        "private field count");
    checkField(type, "log", Class.forName("org.slf4j.Logger"),
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "fullCapacity", int.class, Modifier.PRIVATE | Modifier.FINAL);
    Field queue = checkField(type, "audioFrames", java.util.concurrent.ArrayBlockingQueue.class,
        Modifier.PRIVATE | Modifier.FINAL);
    check(queue.getGenericType() instanceof ParameterizedType
        && ((ParameterizedType) queue.getGenericType()).getActualTypeArguments()[0]
            == AudioFrame.class, "queue generic metadata");
    checkField(type, "stopping", AtomicBoolean.class, Modifier.PRIVATE | Modifier.FINAL);
    Field log = type.getDeclaredField("log");
    log.setAccessible(true);
    check(log.get(null) != null, "logger initialization");

    Constructor<?>[] constructors = type.getDeclaredConstructors();
    check(constructors.length == 1 && constructors[0].getModifiers() == Modifier.PUBLIC
        && Arrays.equals(constructors[0].getParameterTypes(), new Class<?>[] {
            int.class, AudioDataFormat.class, AtomicBoolean.class }), "constructor metadata");
    check(type.getDeclaredMethods().length == 14, "declared method count");
    check(type.getDeclaredMethod("passToMutable", AudioFrame.class, MutableAudioFrame.class)
        .getModifiers() == Modifier.PRIVATE, "private mutable helper");
    check(type.getDeclaredMethod("fetchPendingTerminator").getModifiers() == Modifier.PRIVATE,
        "private terminator helper");
    check(type.getDeclaredMethod("filterFrame", AudioFrame.class).getModifiers()
        == Modifier.PRIVATE, "private filter helper");
    Method signal = type.getDeclaredMethod("signalWaiters");
    check(signal.getModifiers() == Modifier.PROTECTED && signal.getReturnType() == void.class,
        "signal metadata");
    Method timed = type.getDeclaredMethod("provide", long.class, TimeUnit.class);
    Method timedMutable = type.getDeclaredMethod(
        "provide", MutableAudioFrame.class, long.class, TimeUnit.class);
    check(Arrays.equals(timed.getExceptionTypes(), new Class<?>[] {
        TimeoutException.class, InterruptedException.class })
        && Arrays.equals(timedMutable.getExceptionTypes(), new Class<?>[] {
            TimeoutException.class, InterruptedException.class }), "timed exceptions");
    check(Arrays.equals(type.getDeclaredMethod("consume", AudioFrame.class).getExceptionTypes(),
        new Class<?>[] { InterruptedException.class }), "consume exception");
  }

  private static Field checkField(Class<?> type, String name, Class<?> fieldType, int modifiers)
      throws Exception {
    Field field = type.getDeclaredField(name);
    check(field.getType() == fieldType && field.getModifiers() == modifiers,
        name + " field metadata");
    return field;
  }

  private static Object field(Object target, String name) throws Exception {
    Field field = AllocatingAudioFrameBuffer.class.getDeclaredField(name);
    field.setAccessible(true);
    return field.get(target);
  }

  private static AudioFrame frame(long timecode, int volume, int... bytes) {
    byte[] data = new byte[bytes.length];
    for (int index = 0; index < bytes.length; index++) data[index] = (byte) bytes[index];
    return new ImmutableAudioFrame(timecode, data, volume, FORMAT);
  }

  private static AudioFrame frameBytes(long timecode, int volume, byte[] data) {
    return new ImmutableAudioFrame(timecode, data, volume, FORMAT);
  }

  private static Thread daemon(Runnable operation) {
    Thread thread = new Thread(operation);
    thread.setDaemon(true);
    return thread;
  }

  private static void awaitState(Thread thread, Thread.State state, String message)
      throws Exception {
    long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(2L);
    while (thread.getState() != state && System.nanoTime() < deadline) Thread.sleep(1L);
    check(thread.getState() == state, message + ": " + thread.getState());
  }

  private static void join(Thread thread, String message) throws Exception {
    thread.join(2_000L);
    check(!thread.isAlive(), message);
  }

  private static void expect(Class<? extends Throwable> type, Operation operation) {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
    }
  }

  private static void expectIdentity(RuntimeException expected, Operation operation) {
    try {
      operation.run();
      throw new AssertionError("failure was swallowed");
    } catch (RuntimeException error) {
      check(error == expected, "failure identity");
    } catch (Exception error) {
      throw new AssertionError("wrong exception", error);
    }
  }

  private interface Operation { void run() throws Exception; }

  private static final class ProbeBuffer extends AllocatingAudioFrameBuffer {
    ProbeBuffer(int duration, AudioDataFormat format, AtomicBoolean stopping) {
      super(duration, format, stopping);
    }
    AudioDataFormat formatValue() { return format; }
    Object monitor() { return synchronizer; }
    boolean terminatedValue() { return terminated; }
    boolean terminateOnEmptyValue() { return terminateOnEmpty; }
  }

  private static final class TestFormat extends AudioDataFormat {
    private final byte[] silence = new byte[] { 9, 8, 7 };
    TestFormat() { super(2, 48_000, 960); }
    public String codecName() { return "test"; }
    public byte[] silenceBytes() { return silence; }
    public int expectedChunkSize() { return 3; }
    public int maximumChunkSize() { return 16; }
    public AudioChunkDecoder createDecoder() { return null; }
    public AudioChunkEncoder createEncoder(AudioConfiguration configuration) { return null; }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const NON_ALLOCATING_AUDIO_FRAME_BUFFER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.format.AudioDataFormat;
import com.sedmelluq.discord.lavaplayer.format.transcoder.AudioChunkDecoder;
import com.sedmelluq.discord.lavaplayer.format.transcoder.AudioChunkEncoder;
import com.sedmelluq.discord.lavaplayer.player.AudioConfiguration;
import com.sedmelluq.discord.lavaplayer.track.playback.AbstractAudioFrameBuffer;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.ImmutableAudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.MutableAudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.NonAllocatingAudioFrameBuffer;
import com.sedmelluq.discord.lavaplayer.track.playback.ReferenceMutableAudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.TerminatorAudioFrame;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.nio.ByteBuffer;
import java.util.Arrays;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicReference;

public final class GateNonAllocatingAudioFrameBuffer {
  private static final TestFormat FORMAT = new TestFormat();

  public static void main(String[] args) throws Exception {
    constructorAndRingCapacity();
    consumptionAndCopies();
    timedTerminationAndBackpressure();
    stateAndReflection();
    System.out.println(
        "constructor=preallocation,capacity,layout;"
        + "ring=order,wrap,fragmentation,oversize,clear;"
        + "consume=stop,lock,null,copy,silence,backpressure,interrupt;"
        + "provide=bridge,mutable,timed,timeout,terminator;"
        + "state=last-timecode,rebuild,monitor;reflection=9-fields,20-methods");
  }

  private static void constructorAndRingCapacity() throws Exception {
    AtomicBoolean stopping = new AtomicBoolean();
    ProbeBuffer buffer = new ProbeBuffer(59, FORMAT, stopping);
    check(buffer.getFullCapacity() == 2 && buffer.getRemainingCapacity() == 2
        && buffer.formatValue() == FORMAT && field(buffer, "stopping") == stopping,
        "constructor capacity");
    check(((ReferenceMutableAudioFrame[]) field(buffer, "frames")).length == 3
        && ((byte[]) field(buffer, "frameBuffer")).length == 9
        && field(buffer, "silentFrame") != null && field(buffer, "bridgeFrame") == null,
        "preallocated layout");

    buffer.consume(frame(1L, 10, 1, 2, 3));
    buffer.consume(frame(2L, 20, 4, 5, 6));
    check(buffer.getRemainingCapacity() == 0 && buffer.provide().getTimecode() == 1L,
        "initial byte capacity");
    buffer.consume(frame(3L, 30, 7, 8, 9));
    check(buffer.provide().getTimecode() == 2L && buffer.provide().getTimecode() == 3L
        && buffer.provide() == null && buffer.getRemainingCapacity() == 2, "ring wrap order");

    ProbeBuffer oversized = new ProbeBuffer(0, FORMAT, null);
    expect(IllegalArgumentException.class, () -> oversized.consume(frame(4L, 10, 1, 2, 3, 4)));
    check(oversized.hasReceivedFrames() && oversized.provide() == null,
        "oversized frame failure");
    expect(NegativeArraySizeException.class, () -> new ProbeBuffer(-40, FORMAT, null));
  }

  private static void consumptionAndCopies() throws Exception {
    ProbeBuffer stopped = new ProbeBuffer(0, FORMAT, new AtomicBoolean(true));
    expect(InterruptedException.class, () -> stopped.consume(frame(1L, 1, 1)));
    check(!stopped.hasReceivedFrames(), "stopping preflight");
    ProbeBuffer locked = new ProbeBuffer(0, FORMAT, null);
    locked.lockBuffer();
    locked.consume(frame(1L, 1, 1));
    check(!locked.hasReceivedFrames() && locked.provide() == null, "locked discard");
    ProbeBuffer nullFrame = new ProbeBuffer(0, FORMAT, null);
    expect(NullPointerException.class, () -> nullFrame.consume(null));
    check(nullFrame.hasReceivedFrames(), "null after received flag");

    ProbeBuffer buffer = new ProbeBuffer(80, FORMAT, null);
    MutableAudioFrame source = new MutableAudioFrame(ByteBuffer.allocate(8));
    source.setTimecode(11L);
    source.setVolume(55);
    source.setFormat(FORMAT);
    source.store(new byte[] { 3, 4, 5 }, 0, 3);
    buffer.consume(source);
    source.setTimecode(99L);
    source.store(new byte[] { 8 }, 0, 1);
    AudioFrame copy = buffer.provide();
    check(copy != source && copy.getTimecode() == 11L && copy.getVolume() == 55
        && Arrays.equals(copy.getData(), new byte[] { 3, 4, 5 })
        && copy.getFormat() == FORMAT, "ring snapshot");

    buffer.consume(frame(21L, 0, 7, 7, 7));
    AudioFrame silence = buffer.provide();
    check(silence.getTimecode() == 21L && silence.getVolume() == 0
        && Arrays.equals(silence.getData(), FORMAT.silenceBytes()), "silence substitution");

    MutableAudioFrame target = new MutableAudioFrame(ByteBuffer.allocate(8));
    buffer.consume(frame(31L, 77, 4, 5));
    check(buffer.provide(target) && target.getTimecode() == 31L && target.getVolume() == 77
        && Arrays.equals(target.getData(), new byte[] { 4, 5 }) && !target.isTerminator(),
        "mutable copy");
    buffer.consume(frame(32L, 10, 6));
    expect(NullPointerException.class, () -> buffer.provide((MutableAudioFrame) null));
    check(buffer.provide().getTimecode() == 32L, "null target retains frame");
  }

  private static void timedTerminationAndBackpressure() throws Exception {
    ProbeBuffer buffer = new ProbeBuffer(20, FORMAT, null);
    MutableAudioFrame target = new MutableAudioFrame(ByteBuffer.allocate(8));
    buffer.consume(frame(40L, 10, 1));
    check(buffer.provide(target, 1L, TimeUnit.MILLISECONDS) && target.getTimecode() == 40L,
        "timed queued frame");
    expect(NullPointerException.class, () -> buffer.provide(target, 1L, null));
    expect(TimeoutException.class, () -> buffer.provide(target, 1L, TimeUnit.MILLISECONDS));

    buffer.setTerminateOnEmpty();
    check(buffer.provide(1L, TimeUnit.MILLISECONDS) == TerminatorAudioFrame.INSTANCE
        && buffer.terminatedValue(), "pending terminator");

    ProbeBuffer blocked = new ProbeBuffer(0, FORMAT, null);
    AudioFrame first = frame(51L, 10, 1, 2, 3);
    AudioFrame second = frame(52L, 10, 4, 5, 6);
    blocked.consume(first);
    AtomicReference<Throwable> failure = new AtomicReference<>();
    Thread producer = daemon(() -> {
      try { blocked.consume(second); } catch (Throwable error) { failure.set(error); }
    });
    producer.start();
    awaitState(producer, Thread.State.WAITING, "blocked producer");
    check(blocked.provide().getTimecode() == 51L, "backpressure first");
    join(producer, "released producer");
    check(failure.get() == null && blocked.provide().getTimecode() == 52L,
        "backpressure second");

    blocked.consume(first);
    failure.set(null);
    Thread interrupted = daemon(() -> {
      try { blocked.consume(second); } catch (Throwable error) { failure.set(error); }
    });
    interrupted.start();
    awaitState(interrupted, Thread.State.WAITING, "interruptible producer");
    interrupted.interrupt();
    join(interrupted, "interrupted producer");
    check(failure.get() instanceof InterruptedException && blocked.provide().getTimecode() == 51L,
        "producer interruption");
  }

  private static void stateAndReflection() throws Exception {
    ProbeBuffer buffer = new ProbeBuffer(80, FORMAT, null);
    buffer.consume(frame(1L, 10, 1));
    buffer.consume(frame(2L, 10, 2));
    check(buffer.getLastInputTimecode().equals(2L), "last timecode");
    buffer.rebuild(frame -> { throw new AssertionError("unsupported rebuild invoked"); });
    check(buffer.getLastInputTimecode().equals(2L), "rebuild no-op");
    buffer.setClearOnInsert();
    check(buffer.getLastInputTimecode() == null, "clear-on-insert masks timecode");
    buffer.consume(frame(3L, 10, 3));
    check(buffer.getLastInputTimecode().equals(3L) && buffer.provide().getTimecode() == 3L,
        "clear-on-insert replacement");
    buffer.clear();
    check(buffer.getRemainingCapacity() == buffer.getFullCapacity(), "clear capacity");

    Class<NonAllocatingAudioFrameBuffer> type = NonAllocatingAudioFrameBuffer.class;
    check(Modifier.isPublic(type.getModifiers()) && !Modifier.isFinal(type.getModifiers())
        && type.getSuperclass() == AbstractAudioFrameBuffer.class
        && type.getDeclaredFields().length == 9 && type.getDeclaredMethods().length == 20,
        "class metadata");
    checkField(type, "log", Class.forName("org.slf4j.Logger"),
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "worstCaseFrameCount", int.class, Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "frames", ReferenceMutableAudioFrame[].class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "silentFrame", ReferenceMutableAudioFrame.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "stopping", AtomicBoolean.class, Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "bridgeFrame", MutableAudioFrame.class, Modifier.PRIVATE);
    checkField(type, "frameBuffer", byte[].class, Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "firstFrame", int.class, Modifier.PRIVATE);
    checkField(type, "frameCount", int.class, Modifier.PRIVATE);
    check(field(null, "log") != null, "logger initialization");
    Constructor<?>[] constructors = type.getDeclaredConstructors();
    check(constructors.length == 1 && Modifier.isPublic(constructors[0].getModifiers())
        && Arrays.equals(constructors[0].getParameterTypes(), new Class<?>[] {
            int.class, AudioDataFormat.class, AtomicBoolean.class }), "constructor metadata");
    check(Modifier.isPrivate(type.getDeclaredMethod("attemptStore", AudioFrame.class).getModifiers())
        && Modifier.isPrivate(type.getDeclaredMethod("wrappedFrameIndex", int.class).getModifiers())
        && Modifier.isStatic(type.getDeclaredMethod(
            "createFrames", int.class, AudioDataFormat.class).getModifiers()), "helper metadata");
    Method timed = type.getDeclaredMethod("provide", MutableAudioFrame.class,
        long.class, TimeUnit.class);
    check(Arrays.equals(timed.getExceptionTypes(), new Class<?>[] {
        TimeoutException.class, InterruptedException.class }), "timed exceptions");

    AtomicReference<Long> observed = new AtomicReference<>();
    Thread reader;
    buffer.consume(frame(9L, 10, 9));
    synchronized (buffer.monitor()) {
      reader = daemon(() -> observed.set(buffer.getLastInputTimecode()));
      reader.start();
      awaitState(reader, Thread.State.BLOCKED, "monitor blocking");
    }
    join(reader, "monitor release");
    check(observed.get().equals(9L), "monitor synchronized read");
  }

  private static Field checkField(Class<?> type, String name, Class<?> fieldType, int modifiers)
      throws Exception {
    Field field = type.getDeclaredField(name);
    check(field.getType() == fieldType && field.getModifiers() == modifiers,
        name + " field metadata");
    return field;
  }

  private static Object field(Object target, String name) throws Exception {
    Field field = NonAllocatingAudioFrameBuffer.class.getDeclaredField(name);
    field.setAccessible(true);
    return field.get(target);
  }

  private static AudioFrame frame(long timecode, int volume, int... bytes) {
    byte[] data = new byte[bytes.length];
    for (int index = 0; index < bytes.length; index++) data[index] = (byte) bytes[index];
    return new ImmutableAudioFrame(timecode, data, volume, FORMAT);
  }

  private static Thread daemon(Runnable operation) {
    Thread thread = new Thread(operation);
    thread.setDaemon(true);
    return thread;
  }

  private static void awaitState(Thread thread, Thread.State state, String message)
      throws Exception {
    long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(2L);
    while (thread.getState() != state && System.nanoTime() < deadline) Thread.sleep(1L);
    check(thread.getState() == state, message + ": " + thread.getState());
  }

  private static void join(Thread thread, String message) throws Exception {
    thread.join(2_000L);
    check(!thread.isAlive(), message);
  }

  private static void expect(Class<? extends Throwable> type, Operation operation) {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
    }
  }

  private interface Operation { void run() throws Exception; }

  private static final class ProbeBuffer extends NonAllocatingAudioFrameBuffer {
    ProbeBuffer(int duration, AudioDataFormat format, AtomicBoolean stopping) {
      super(duration, format, stopping);
    }
    AudioDataFormat formatValue() { return format; }
    Object monitor() { return synchronizer; }
    boolean terminatedValue() { return terminated; }
  }

  private static final class TestFormat extends AudioDataFormat {
    private final byte[] silence = new byte[] { 9, 8, 7 };
    TestFormat() { super(2, 48_000, 960); }
    public String codecName() { return "test"; }
    public byte[] silenceBytes() { return silence; }
    public int expectedChunkSize() { return 3; }
    public int maximumChunkSize() { return 4; }
    public AudioChunkDecoder createDecoder() { return null; }
    public AudioChunkEncoder createEncoder(AudioConfiguration configuration) { return null; }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const AUDIO_SOURCE_MANAGER_INTERFACE_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.player.AudioPlayerManager;
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.track.AudioItem;
import com.sedmelluq.discord.lavaplayer.track.AudioReference;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.DataInput;
import java.io.DataInputStream;
import java.io.DataOutput;
import java.io.DataOutputStream;
import java.io.IOException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;

public final class GateAudioSourceManagerInterface {
  public static void main(String[] args) throws Exception {
    callerImplementation();
    checkedFailures();
    reflection();
    System.out.println(
        "implementation=name,load,encodable,encode,decode,shutdown,identity;"
        + "exceptions=encode-io,decode-io;"
        + "reflection=public-abstract-interface,0-fields,0-constructors,6-methods");
  }

  private static void callerImplementation() throws Exception {
    AudioPlayerManager manager = proxy(AudioPlayerManager.class);
    AudioTrack track = proxy(AudioTrack.class);
    AudioReference reference = new AudioReference("identifier", "container");
    AudioTrackInfo info = new AudioTrackInfo(
        "title", "author", 123L, "identifier", false, "https://example.invalid/item");
    RecordingSource source = new RecordingSource(track);

    check(source.getSourceName().equals("recording")
        && source.loadItem(manager, reference) == track
        && source.loadManager == manager && source.loadReference == reference
        && source.loads == 1, "name and load dispatch");
    check(source.isTrackEncodable(track) && !source.isTrackEncodable(null)
        && source.encodableCalls == 2, "encodable dispatch");

    ByteArrayOutputStream bytes = new ByteArrayOutputStream();
    source.encodeTrack(track, new DataOutputStream(bytes));
    check(source.encodedTrack == track && source.encodes == 1, "encode identity");
    AudioTrack decoded = source.decodeTrack(
        info, new DataInputStream(new ByteArrayInputStream(bytes.toByteArray())));
    check(decoded == track && source.decodedInfo == info && source.decodes == 1
        && source.decodedCode == 0x12345678 && source.decodedText.equals("source-details"),
        "encode/decode round trip");

    source.shutdown();
    source.shutdown();
    check(source.shutdowns == 2, "shutdown dispatch");
  }

  private static void checkedFailures() throws Exception {
    RecordingSource source = new RecordingSource(proxy(AudioTrack.class));
    IOException encodeFailure = new IOException("encode-sentinel");
    source.encodeFailure = encodeFailure;
    expectIdentity(encodeFailure, () -> source.encodeTrack(
        source.loadedTrack, new DataOutputStream(new ByteArrayOutputStream())));
    check(source.encodes == 0 && source.encodedTrack == null, "encode failure prefix");

    IOException decodeFailure = new IOException("decode-sentinel");
    source.decodeFailure = decodeFailure;
    AudioTrackInfo info = new AudioTrackInfo("t", "a", 1L, "i", false, null);
    expectIdentity(decodeFailure, () -> source.decodeTrack(
        info, new DataInputStream(new ByteArrayInputStream(new byte[0]))));
    check(source.decodes == 0 && source.decodedInfo == null, "decode failure prefix");
  }

  private static void reflection() throws Exception {
    Class<AudioSourceManager> type = AudioSourceManager.class;
    check(type.isInterface() && Modifier.isPublic(type.getModifiers())
        && Modifier.isAbstract(type.getModifiers()) && !Modifier.isFinal(type.getModifiers())
        && type.getSuperclass() == null && type.getInterfaces().length == 0,
        "interface metadata");
    check(type.getDeclaredFields().length == 0 && type.getDeclaredConstructors().length == 0
        && type.getDeclaredMethods().length == 6, "member counts");

    checkMethod(type.getDeclaredMethod("getSourceName"), String.class, new Class<?>[0]);
    checkMethod(type.getDeclaredMethod(
        "loadItem", AudioPlayerManager.class, AudioReference.class),
        AudioItem.class, new Class<?>[0]);
    checkMethod(type.getDeclaredMethod("isTrackEncodable", AudioTrack.class),
        boolean.class, new Class<?>[0]);
    checkMethod(type.getDeclaredMethod("encodeTrack", AudioTrack.class, DataOutput.class),
        void.class, new Class<?>[] { IOException.class });
    checkMethod(type.getDeclaredMethod("decodeTrack", AudioTrackInfo.class, DataInput.class),
        AudioTrack.class, new Class<?>[] { IOException.class });
    checkMethod(type.getDeclaredMethod("shutdown"), void.class, new Class<?>[0]);
  }

  private static void checkMethod(Method method, Class<?> returnType, Class<?>[] exceptions) {
    check(method.getModifiers() == (Modifier.PUBLIC | Modifier.ABSTRACT)
        && method.getReturnType() == returnType
        && Arrays.equals(method.getExceptionTypes(), exceptions)
        && !method.isDefault() && !method.isBridge() && !method.isSynthetic()
        && !method.isVarArgs(), method.getName() + " metadata");
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type) {
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] { type },
        (instance, method, arguments) -> {
          if (method.getName().equals("toString")) return type.getSimpleName() + "Proxy";
          if (method.getName().equals("hashCode")) return System.identityHashCode(instance);
          if (method.getName().equals("equals")) return instance == arguments[0];
          Class<?> result = method.getReturnType();
          if (result == boolean.class) return false;
          if (result == int.class) return 0;
          if (result == long.class) return 0L;
          return null;
        });
  }

  private static void expectIdentity(IOException expected, IoOperation operation) {
    try {
      operation.run();
      throw new AssertionError("failure was swallowed");
    } catch (IOException error) {
      check(error == expected, "IOException identity");
    }
  }

  private interface IoOperation { void run() throws IOException; }

  private static final class RecordingSource implements AudioSourceManager {
    final AudioTrack loadedTrack;
    AudioPlayerManager loadManager;
    AudioReference loadReference;
    AudioTrack encodedTrack;
    AudioTrackInfo decodedInfo;
    IOException encodeFailure;
    IOException decodeFailure;
    int loads;
    int encodableCalls;
    int encodes;
    int decodes;
    int shutdowns;
    int decodedCode;
    String decodedText;

    RecordingSource(AudioTrack loadedTrack) { this.loadedTrack = loadedTrack; }

    public String getSourceName() { return "recording"; }

    public AudioItem loadItem(AudioPlayerManager manager, AudioReference reference) {
      loads++;
      loadManager = manager;
      loadReference = reference;
      return loadedTrack;
    }

    public boolean isTrackEncodable(AudioTrack track) {
      encodableCalls++;
      return track == loadedTrack;
    }

    public void encodeTrack(AudioTrack track, DataOutput output) throws IOException {
      if (encodeFailure != null) throw encodeFailure;
      encodedTrack = track;
      encodes++;
      output.writeInt(0x12345678);
      output.writeUTF("source-details");
    }

    public AudioTrack decodeTrack(AudioTrackInfo info, DataInput input) throws IOException {
      if (decodeFailure != null) throw decodeFailure;
      decodedInfo = info;
      decodes++;
      decodedCode = input.readInt();
      decodedText = input.readUTF();
      return loadedTrack;
    }

    public void shutdown() { shutdowns++; }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const AUDIO_SOURCE_MANAGERS_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.container.MediaContainerRegistry;
import com.sedmelluq.discord.lavaplayer.player.AudioPlayerManager;
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManagers;
import com.sedmelluq.discord.lavaplayer.source.bandcamp.BandcampAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.beam.BeamAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.getyarn.GetyarnAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.http.HttpAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.local.LocalAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.nico.NicoAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.twitch.TwitchStreamAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.vimeo.VimeoAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeAudioSourceManager;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;

public final class GateAudioSourceManagers {
  private static final List<Class<?>> REMOTE_ORDER = Arrays.asList(
      YoutubeAudioSourceManager.class,
      YandexMusicAudioSourceManager.class,
      SoundCloudAudioSourceManager.class,
      BandcampAudioSourceManager.class,
      VimeoAudioSourceManager.class,
      TwitchStreamAudioSourceManager.class,
      BeamAudioSourceManager.class,
      GetyarnAudioSourceManager.class,
      NicoAudioSourceManager.class,
      HttpAudioSourceManager.class);

  public static void main(String[] args) throws Exception {
    constructorAndReflection();
    remoteOverloads();
    exclusionsAndFailures();
    localOverloads();
    System.out.println(
        "remote=order,defaults,custom-registry,constructor-options;"
        + "excluded=exact,empty,all,null,duplicate,failure-prefix;"
        + "local=default,custom,null-registry;"
        + "reflection=public-class,0-fields,1-constructor,6-static-methods,2-varargs");
  }

  private static void constructorAndReflection() throws Exception {
    Class<AudioSourceManagers> type = AudioSourceManagers.class;
    check(Modifier.isPublic(type.getModifiers()) && !Modifier.isAbstract(type.getModifiers())
        && !Modifier.isFinal(type.getModifiers()) && type.getSuperclass() == Object.class
        && type.getInterfaces().length == 0, "class metadata");
    check(type.getDeclaredFields().length == 0 && type.getDeclaredConstructors().length == 1
        && type.getDeclaredMethods().length == 6, "member counts");
    Constructor<AudioSourceManagers> constructor = type.getDeclaredConstructor();
    check(constructor.getModifiers() == Modifier.PUBLIC
        && constructor.newInstance().getClass() == type, "public constructor");

    int varargs = 0;
    for (Method method : type.getDeclaredMethods()) {
      int expectedModifiers = Modifier.PUBLIC | Modifier.STATIC;
      if (method.isVarArgs()) expectedModifiers |= 0x80;
      check(method.getModifiers() == expectedModifiers
          && method.getReturnType() == void.class && !method.isBridge()
          && !method.isSynthetic(), method + " metadata");
      if (method.isVarArgs()) {
        varargs++;
        check(method.isAnnotationPresent(SafeVarargs.class)
            && method.getParameterTypes()[method.getParameterCount() - 1] == Class[].class,
            method + " varargs metadata");
      } else {
        check(!method.isAnnotationPresent(SafeVarargs.class), method + " annotation absence");
      }
    }
    check(varargs == 2, "varargs count");
  }

  private static void remoteOverloads() throws Exception {
    RecordingManager defaults = new RecordingManager(-1, null);
    try {
      AudioSourceManagers.registerRemoteSources(defaults.proxy);
      checkOrder(defaults.sources, REMOTE_ORDER, "default order");
      checkRegistry(defaults.sources.get(9), MediaContainerRegistry.DEFAULT_REGISTRY);
      checkSearchFlags(defaults.sources);
    } finally {
      defaults.shutdown();
    }

    MediaContainerRegistry custom = new MediaContainerRegistry(Collections.emptyList());
    RecordingManager explicit = new RecordingManager(-1, null);
    try {
      AudioSourceManagers.registerRemoteSources(explicit.proxy, custom);
      checkOrder(explicit.sources, REMOTE_ORDER, "explicit order");
      checkRegistry(explicit.sources.get(9), custom);
      checkSearchFlags(explicit.sources);
    } finally {
      explicit.shutdown();
    }

    RecordingManager empty = new RecordingManager(-1, null);
    try {
      AudioSourceManagers.registerRemoteSources(empty.proxy, custom, emptyClasses());
      checkOrder(empty.sources, REMOTE_ORDER, "empty exclusion order");
      checkRegistry(empty.sources.get(9), custom);
    } finally {
      empty.shutdown();
    }
  }

  private static void exclusionsAndFailures() throws Exception {
    RecordingManager selected = new RecordingManager(-1, null);
    try {
      AudioSourceManagers.registerRemoteSources(selected.proxy,
          YoutubeAudioSourceManager.class,
          SoundCloudAudioSourceManager.class,
          HttpAudioSourceManager.class);
      checkOrder(selected.sources, Arrays.asList(
          YandexMusicAudioSourceManager.class,
          BandcampAudioSourceManager.class,
          VimeoAudioSourceManager.class,
          TwitchStreamAudioSourceManager.class,
          BeamAudioSourceManager.class,
          GetyarnAudioSourceManager.class,
          NicoAudioSourceManager.class), "selected exclusions");
    } finally {
      selected.shutdown();
    }

    @SuppressWarnings("unchecked")
    Class<? extends AudioSourceManager>[] all = REMOTE_ORDER.toArray(new Class[0]);
    AudioSourceManagers.registerRemoteSources(null,
        new MediaContainerRegistry(Collections.emptyList()), all);

    RecordingManager invalid = new RecordingManager(-1, null);
    expect(NullPointerException.class,
        () -> AudioSourceManagers.registerRemoteSources(invalid.proxy,
            (Class<? extends AudioSourceManager>[]) null));
    check(invalid.sources.isEmpty(), "null array failure prefix");
    expect(NullPointerException.class,
        () -> AudioSourceManagers.registerRemoteSources(invalid.proxy,
            YoutubeAudioSourceManager.class, null));
    check(invalid.sources.isEmpty(), "null member failure prefix");
    expect(IllegalArgumentException.class,
        () -> AudioSourceManagers.registerRemoteSources(invalid.proxy,
            YoutubeAudioSourceManager.class, YoutubeAudioSourceManager.class));
    check(invalid.sources.isEmpty(), "duplicate failure prefix");

    RuntimeException sentinel = new RuntimeException("register-sentinel");
    RecordingManager failing = new RecordingManager(3, sentinel);
    try {
      expectIdentity(sentinel, () -> AudioSourceManagers.registerRemoteSources(failing.proxy));
      checkOrder(failing.sources, REMOTE_ORDER.subList(0, 4), "callback failure prefix");
    } finally {
      failing.shutdown();
    }
  }

  private static void localOverloads() throws Exception {
    RecordingManager defaults = new RecordingManager(-1, null);
    try {
      AudioSourceManagers.registerLocalSource(defaults.proxy);
      checkOrder(defaults.sources, Arrays.asList(LocalAudioSourceManager.class), "local default");
      checkRegistry(defaults.sources.get(0), MediaContainerRegistry.DEFAULT_REGISTRY);
    } finally {
      defaults.shutdown();
    }

    MediaContainerRegistry custom = new MediaContainerRegistry(Collections.emptyList());
    RecordingManager explicit = new RecordingManager(-1, null);
    try {
      AudioSourceManagers.registerLocalSource(explicit.proxy, custom);
      checkOrder(explicit.sources, Arrays.asList(LocalAudioSourceManager.class), "local custom");
      checkRegistry(explicit.sources.get(0), custom);
    } finally {
      explicit.shutdown();
    }

    RecordingManager nullable = new RecordingManager(-1, null);
    try {
      AudioSourceManagers.registerLocalSource(nullable.proxy, null);
      checkOrder(nullable.sources, Arrays.asList(LocalAudioSourceManager.class), "local null");
      checkRegistry(nullable.sources.get(0), null);
    } finally {
      nullable.shutdown();
    }
  }

  private static void checkSearchFlags(List<AudioSourceManager> sources) throws Exception {
    check(readField(sources.get(0), "allowSearch") == Boolean.TRUE,
        "youtube search enabled");
    check(readField(sources.get(1), "allowSearch") == Boolean.TRUE,
        "yandex search enabled");
  }

  private static void checkRegistry(AudioSourceManager source, MediaContainerRegistry expected)
      throws Exception {
    check(readField(source, "containerRegistry") == expected, "registry identity");
  }

  private static Object readField(Object instance, String name) throws Exception {
    for (Class<?> type = instance.getClass(); type != null; type = type.getSuperclass()) {
      try {
        Field field = type.getDeclaredField(name);
        field.setAccessible(true);
        return field.get(instance);
      } catch (NoSuchFieldException ignored) {
      }
    }
    throw new AssertionError("missing field " + name);
  }

  private static void checkOrder(
      List<AudioSourceManager> actual, List<Class<?>> expected, String message) {
    check(actual.size() == expected.size(), message + " size");
    for (int index = 0; index < expected.size(); index++) {
      check(actual.get(index).getClass() == expected.get(index), message + " at " + index);
    }
  }

  @SuppressWarnings("unchecked")
  private static Class<? extends AudioSourceManager>[] emptyClasses() {
    return (Class<? extends AudioSourceManager>[]) new Class<?>[0];
  }

  private static void expect(Class<? extends Throwable> type, Operation operation) {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
    }
  }

  private static void expectIdentity(Throwable expected, Operation operation) {
    try {
      operation.run();
      throw new AssertionError("failure was swallowed");
    } catch (Throwable error) {
      check(error == expected, "failure identity");
    }
  }

  private interface Operation { void run() throws Exception; }

  private static final class RecordingManager {
    final List<AudioSourceManager> sources = new ArrayList<>();
    final int failAt;
    final RuntimeException failure;
    final AudioPlayerManager proxy;

    RecordingManager(int failAt, RuntimeException failure) {
      this.failAt = failAt;
      this.failure = failure;
      proxy = (AudioPlayerManager) Proxy.newProxyInstance(
          AudioPlayerManager.class.getClassLoader(),
          new Class<?>[] { AudioPlayerManager.class },
          (instance, method, arguments) -> {
            if (method.getName().equals("registerSourceManager")) {
              sources.add((AudioSourceManager) arguments[0]);
              if (sources.size() - 1 == this.failAt) throw this.failure;
              return null;
            }
            if (method.getName().equals("toString")) return "RecordingAudioPlayerManager";
            if (method.getName().equals("hashCode")) return System.identityHashCode(instance);
            if (method.getName().equals("equals")) return instance == arguments[0];
            Class<?> result = method.getReturnType();
            if (result == boolean.class) return false;
            if (result == int.class) return 0;
            if (result == long.class) return 0L;
            return null;
          });
    }

    void shutdown() {
      for (AudioSourceManager source : sources) {
        try {
          source.shutdown();
        } catch (Throwable ignored) {
        }
      }
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const PROBING_AUDIO_SOURCE_MANAGER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.container.MediaContainerDescriptor;
import com.sedmelluq.discord.lavaplayer.container.MediaContainerDetectionResult;
import com.sedmelluq.discord.lavaplayer.container.MediaContainerHints;
import com.sedmelluq.discord.lavaplayer.container.MediaContainerProbe;
import com.sedmelluq.discord.lavaplayer.container.MediaContainerRegistry;
import com.sedmelluq.discord.lavaplayer.player.AudioPlayerManager;
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.ProbingAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.tools.FriendlyException;
import com.sedmelluq.discord.lavaplayer.tools.FriendlyException.Severity;
import com.sedmelluq.discord.lavaplayer.tools.io.SeekableInputStream;
import com.sedmelluq.discord.lavaplayer.track.AudioItem;
import com.sedmelluq.discord.lavaplayer.track.AudioReference;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.DataInput;
import java.io.DataInputStream;
import java.io.DataOutput;
import java.io.DataOutputStream;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;
import java.util.Collections;

public final class GateProbingAudioSourceManager {
  public static void main(String[] args) throws Exception {
    constructorAndReflection();
    loadResults();
    factoryEncoding();
    factoryDecoding();
    System.out.println(
        "load=null,reference,unknown,unsupported,supported,identity;"
        + "encode=name,null-name,null-empty-nested-parameters,modified-utf,io-prefix;"
        + "decode=known,empty,nested,unknown,first-probe,io-prefix;"
        + "reflection=public-abstract,2-fields,1-protected-constructor,4-protected-methods");
  }

  private static void constructorAndReflection() throws Exception {
    MediaContainerRegistry registry = new MediaContainerRegistry(Collections.emptyList());
    TestManager manager = new TestManager(registry);
    check(manager.registry() == registry, "registry identity");
    check(new TestManager(null).registry() == null, "null registry retained");

    Class<ProbingAudioSourceManager> type = ProbingAudioSourceManager.class;
    check(Modifier.isPublic(type.getModifiers()) && Modifier.isAbstract(type.getModifiers())
        && !Modifier.isFinal(type.getModifiers()) && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] { AudioSourceManager.class }),
        "class metadata");
    check(type.getDeclaredFields().length == 2 && type.getDeclaredConstructors().length == 1
        && type.getDeclaredMethods().length == 4, "member counts");

    Field separator = type.getDeclaredField("PARAMETERS_SEPARATOR");
    separator.setAccessible(true);
    check(separator.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL)
        && separator.getType() == char.class && separator.getChar(null) == '|',
        "separator metadata");
    Field containerRegistry = type.getDeclaredField("containerRegistry");
    check(containerRegistry.getModifiers() == (Modifier.PROTECTED | Modifier.FINAL)
        && containerRegistry.getType() == MediaContainerRegistry.class,
        "registry field metadata");

    Constructor<?> constructor = type.getDeclaredConstructor(MediaContainerRegistry.class);
    check(constructor.getModifiers() == Modifier.PROTECTED && !constructor.isVarArgs()
        && constructor.getExceptionTypes().length == 0, "constructor metadata");
    checkMethod(type.getDeclaredMethod(
        "handleLoadResult", MediaContainerDetectionResult.class), AudioItem.class, false, false);
    checkMethod(type.getDeclaredMethod(
        "createTrack", AudioTrackInfo.class, MediaContainerDescriptor.class),
        AudioTrack.class, true, false);
    checkMethod(type.getDeclaredMethod(
        "encodeTrackFactory", MediaContainerDescriptor.class, DataOutput.class),
        void.class, false, true);
    checkMethod(type.getDeclaredMethod("decodeTrackFactory", DataInput.class),
        MediaContainerDescriptor.class, false, true);
  }

  private static void checkMethod(
      Method method, Class<?> returnType, boolean isAbstract, boolean throwsIo) {
    int modifiers = Modifier.PROTECTED | (isAbstract ? Modifier.ABSTRACT : 0);
    check(method.getModifiers() == modifiers && method.getReturnType() == returnType
        && !method.isBridge() && !method.isSynthetic() && !method.isVarArgs()
        && Arrays.equals(method.getExceptionTypes(),
            throwsIo ? new Class<?>[] { IOException.class } : new Class<?>[0]),
        method + " metadata");
  }

  private static void loadResults() {
    MediaContainerProbe probe = probe("known", new int[1]);
    MediaContainerRegistry registry = new MediaContainerRegistry(Arrays.asList(probe));
    AudioTrack track = proxy(AudioTrack.class);
    TestManager manager = new TestManager(registry);
    manager.trackResult = track;

    check(manager.handle(null) == null && manager.creates == 0, "null result");
    AudioReference reference = new AudioReference("ref-id", "ref-container");
    check(manager.handle(MediaContainerDetectionResult.refer(probe, reference)) == reference
        && manager.creates == 0, "reference identity");

    FriendlyException unknown = expectFriendly(
        () -> manager.handle(MediaContainerDetectionResult.unknownFormat()));
    check(unknown.getMessage().equals("Unknown file format.")
        && unknown.severity == Severity.COMMON && unknown.getCause() == null
        && manager.creates == 0, "unknown format failure");

    String reason = new String("unsupported-sentinel");
    FriendlyException unsupported = expectFriendly(
        () -> manager.handle(MediaContainerDetectionResult.unsupportedFormat(probe, reason)));
    check(unsupported.getMessage() == reason && unsupported.severity == Severity.COMMON
        && unsupported.getCause() == null && manager.creates == 0,
        "unsupported failure");

    AudioTrackInfo info = new AudioTrackInfo(
        "title", "author", 42L, "identifier", false, "https://example.invalid");
    check(manager.handle(MediaContainerDetectionResult.supportedFormat(probe, "settings", info))
        == track && manager.creates == 1 && manager.createdInfo == info
        && manager.createdDescriptor.probe == probe
        && manager.createdDescriptor.parameters.equals("settings"), "supported dispatch");

    RuntimeException failure = new RuntimeException("create-sentinel");
    manager.createFailure = failure;
    expectIdentity(failure, () -> manager.handle(
        MediaContainerDetectionResult.supportedFormat(probe, null, info)));
    check(manager.creates == 2 && manager.createdInfo == info
        && manager.createdDescriptor.parameters == null, "create failure prefix");
  }

  private static void factoryEncoding() throws Exception {
    MediaContainerRegistry registry = new MediaContainerRegistry(Collections.emptyList());
    TestManager manager = new TestManager(registry);
    checkEncoded(manager, "known", null, "known");
    checkEncoded(manager, "known", "", "known|");
    checkEncoded(manager, "known", "a|b", "known|a|b");
    checkEncoded(manager, null, "value", "null|value");
    checkEncoded(manager, "žluťoučký", "水|😀", "žluťoučký|水|😀");

    int[] calls = new int[1];
    MediaContainerDescriptor descriptor = new MediaContainerDescriptor(probe("known", calls), "p");
    IOException failure = new IOException("write-sentinel");
    DataOutput output = (DataOutput) Proxy.newProxyInstance(
        DataOutput.class.getClassLoader(), new Class<?>[] { DataOutput.class },
        (instance, method, arguments) -> {
          if (method.getName().equals("writeUTF")) throw failure;
          return null;
        });
    expectIdentity(failure, () -> manager.encode(descriptor, output));
    check(calls[0] == 1, "write failure prefix");

    expect(NullPointerException.class, () -> manager.encode(null, output));
    expect(NullPointerException.class,
        () -> manager.encode(new MediaContainerDescriptor(null, null), output));
    int[] nullOutputCalls = new int[1];
    expect(NullPointerException.class, () -> manager.encode(
        new MediaContainerDescriptor(probe("known", nullOutputCalls), "p"), null));
    check(nullOutputCalls[0] == 1, "null output failure prefix");
  }

  private static void checkEncoded(
      TestManager manager, String name, String parameters, String expected) throws Exception {
    int[] calls = new int[1];
    ByteArrayOutputStream bytes = new ByteArrayOutputStream();
    manager.encode(new MediaContainerDescriptor(probe(name, calls), parameters),
        new DataOutputStream(bytes));
    String actual = new DataInputStream(new ByteArrayInputStream(bytes.toByteArray())).readUTF();
    check(actual.equals(expected) && calls[0] == 1, "encoded " + expected);
  }

  private static void factoryDecoding() throws Exception {
    int[] firstCalls = new int[1];
    MediaContainerProbe first = probe("known", firstCalls);
    MediaContainerProbe duplicate = probe("known", new int[1]);
    MediaContainerProbe empty = probe("", new int[1]);
    TestManager manager = new TestManager(
        new MediaContainerRegistry(Arrays.asList(first, duplicate, empty)));

    MediaContainerDescriptor plain = manager.decode(input("known"));
    check(plain.probe == first && plain.parameters == null, "plain decode");
    MediaContainerDescriptor emptyParameters = manager.decode(input("known|"));
    check(emptyParameters.probe == first && emptyParameters.parameters.equals(""),
        "empty parameters");
    MediaContainerDescriptor nested = manager.decode(input("known|a|b"));
    check(nested.probe == first && nested.parameters.equals("a|b"), "first separator");
    MediaContainerDescriptor emptyName = manager.decode(input("|value"));
    check(emptyName.probe == empty && emptyName.parameters.equals("value"), "empty name");
    check(manager.decode(input("missing|value")) == null, "unknown probe");
    check(firstCalls[0] >= 4, "registry lookup dispatch");

    IOException failure = new IOException("read-sentinel");
    DataInput failing = (DataInput) Proxy.newProxyInstance(
        DataInput.class.getClassLoader(), new Class<?>[] { DataInput.class },
        (instance, method, arguments) -> {
          if (method.getName().equals("readUTF")) throw failure;
          return null;
        });
    expectIdentity(failure, () -> manager.decode(failing));
    expect(NullPointerException.class, () -> manager.decode(null));
    expect(NullPointerException.class, () -> new TestManager(null).decode(input("known")));
  }

  private static DataInput input(String value) throws IOException {
    ByteArrayOutputStream bytes = new ByteArrayOutputStream();
    new DataOutputStream(bytes).writeUTF(value);
    return new DataInputStream(new ByteArrayInputStream(bytes.toByteArray()));
  }

  private static MediaContainerProbe probe(String name, int[] calls) {
    return (MediaContainerProbe) Proxy.newProxyInstance(
        MediaContainerProbe.class.getClassLoader(), new Class<?>[] { MediaContainerProbe.class },
        (instance, method, arguments) -> {
          if (method.getName().equals("getName")) {
            calls[0]++;
            return name;
          }
          if (method.getName().equals("toString")) return "Probe(" + name + ")";
          if (method.getName().equals("hashCode")) return System.identityHashCode(instance);
          if (method.getName().equals("equals")) return instance == arguments[0];
          if (method.getReturnType() == boolean.class) return false;
          return null;
        });
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type) {
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] { type },
        (instance, method, arguments) -> {
          if (method.getName().equals("toString")) return type.getSimpleName() + "Proxy";
          if (method.getName().equals("hashCode")) return System.identityHashCode(instance);
          if (method.getName().equals("equals")) return instance == arguments[0];
          Class<?> result = method.getReturnType();
          if (result == boolean.class) return false;
          if (result == int.class) return 0;
          if (result == long.class) return 0L;
          return null;
        });
  }

  private static FriendlyException expectFriendly(Operation operation) {
    try {
      operation.run();
      throw new AssertionError("expected FriendlyException");
    } catch (FriendlyException error) {
      return error;
    } catch (Throwable error) {
      throw new AssertionError("wrong exception", error);
    }
  }

  private static void expect(Class<? extends Throwable> type, Operation operation) {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
    }
  }

  private static void expectIdentity(Throwable expected, Operation operation) {
    try {
      operation.run();
      throw new AssertionError("failure was swallowed");
    } catch (Throwable error) {
      check(error == expected, "failure identity");
    }
  }

  private interface Operation { void run() throws Exception; }

  private static final class TestManager extends ProbingAudioSourceManager {
    AudioTrack trackResult;
    AudioTrackInfo createdInfo;
    MediaContainerDescriptor createdDescriptor;
    RuntimeException createFailure;
    int creates;

    TestManager(MediaContainerRegistry registry) { super(registry); }
    MediaContainerRegistry registry() { return containerRegistry; }
    AudioItem handle(MediaContainerDetectionResult result) { return handleLoadResult(result); }
    void encode(MediaContainerDescriptor descriptor, DataOutput output) throws IOException {
      encodeTrackFactory(descriptor, output);
    }
    MediaContainerDescriptor decode(DataInput input) throws IOException {
      return decodeTrackFactory(input);
    }

    protected AudioTrack createTrack(AudioTrackInfo info, MediaContainerDescriptor descriptor) {
      creates++;
      createdInfo = info;
      createdDescriptor = descriptor;
      if (createFailure != null) throw createFailure;
      return trackResult;
    }

    public String getSourceName() { return "test"; }
    public AudioItem loadItem(AudioPlayerManager manager, AudioReference reference) { return null; }
    public boolean isTrackEncodable(AudioTrack track) { return false; }
    public void encodeTrack(AudioTrack track, DataOutput output) { }
    public AudioTrack decodeTrack(AudioTrackInfo info, DataInput input) { return null; }
    public void shutdown() { }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const LOCAL_AUDIO_SOURCE_MANAGER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.container.MediaContainerDescriptor;
import com.sedmelluq.discord.lavaplayer.container.MediaContainerDetectionResult;
import com.sedmelluq.discord.lavaplayer.container.MediaContainerHints;
import com.sedmelluq.discord.lavaplayer.container.MediaContainerProbe;
import com.sedmelluq.discord.lavaplayer.container.MediaContainerRegistry;
import com.sedmelluq.discord.lavaplayer.source.ProbingAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.local.LocalAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.local.LocalAudioTrack;
import com.sedmelluq.discord.lavaplayer.tools.io.SeekableInputStream;
import com.sedmelluq.discord.lavaplayer.track.AudioItem;
import com.sedmelluq.discord.lavaplayer.track.AudioReference;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.DataInput;
import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.Collections;

public final class GateLocalAudioSourceManager {
  public static void main(String[] args) throws Exception {
    constructorsAndReflection();
    loading();
    trackCreationAndSerialization();
    lifecycle();
    System.out.println(
        "constructors=default,custom,null-registry;"
        + "load=missing,directory,eligible,extension,closed,nulls;"
        + "track=create,encodable,encode,decode,unknown,failures;"
        + "lifecycle=name,shutdown;reflection=public-concrete,2-constructors,7-exported-methods");
  }

  private static void constructorsAndReflection() throws Exception {
    ExposedManager defaults = new ExposedManager();
    check(defaults.registry() == MediaContainerRegistry.DEFAULT_REGISTRY,
        "default registry identity");
    MediaContainerRegistry custom = new MediaContainerRegistry(Collections.emptyList());
    check(new ExposedManager(custom).registry() == custom, "custom registry identity");
    check(new ExposedManager(null).registry() == null, "null registry retained");

    Class<LocalAudioSourceManager> type = LocalAudioSourceManager.class;
    check(Modifier.isPublic(type.getModifiers()) && !Modifier.isAbstract(type.getModifiers())
        && !Modifier.isFinal(type.getModifiers())
        && type.getSuperclass() == ProbingAudioSourceManager.class
        && type.getInterfaces().length == 0, "class metadata");
    check(type.getDeclaredFields().length == 0, "field count");
    Constructor<?>[] constructors = type.getDeclaredConstructors();
    check(constructors.length == 2, "constructor count");
    for (Constructor<?> constructor : constructors) {
      check(constructor.getModifiers() == Modifier.PUBLIC && !constructor.isVarArgs()
          && constructor.getExceptionTypes().length == 0, "constructor metadata");
    }

    int exported = 0;
    for (Method method : type.getDeclaredMethods()) {
      if (Modifier.isPublic(method.getModifiers()) || Modifier.isProtected(method.getModifiers())) {
        exported++;
      }
    }
    check(exported == 7, "exported method count");
    checkMethod(type.getDeclaredMethod("getSourceName"), String.class, Modifier.PUBLIC, false);
    checkMethod(type.getDeclaredMethod("loadItem",
        com.sedmelluq.discord.lavaplayer.player.AudioPlayerManager.class, AudioReference.class),
        AudioItem.class, Modifier.PUBLIC, false);
    checkMethod(type.getDeclaredMethod("createTrack",
        AudioTrackInfo.class, MediaContainerDescriptor.class),
        AudioTrack.class, Modifier.PROTECTED, false);
    checkMethod(type.getDeclaredMethod("isTrackEncodable", AudioTrack.class),
        boolean.class, Modifier.PUBLIC, false);
    checkMethod(type.getDeclaredMethod("encodeTrack", AudioTrack.class, java.io.DataOutput.class),
        void.class, Modifier.PUBLIC, true);
    checkMethod(type.getDeclaredMethod("decodeTrack", AudioTrackInfo.class, DataInput.class),
        AudioTrack.class, Modifier.PUBLIC, true);
    checkMethod(type.getDeclaredMethod("shutdown"), void.class, Modifier.PUBLIC, false);
  }

  private static void loading() throws Exception {
    AudioTrackInfo info = new AudioTrackInfo(
        "local-title", "local-author", 17L, "local-id", false, "file:///local");
    ProbeState state = new ProbeState(info);
    MediaContainerProbe probe = state.proxy();
    ExposedManager manager = new ExposedManager(
        new MediaContainerRegistry(Collections.singletonList(probe)));

    Path directory = Files.createTempDirectory("mantle-local-source-");
    Path missing = directory.resolve("missing.bin");
    check(manager.loadItem(null, new AudioReference(missing.toString(), null)) == null,
        "missing path");
    check(manager.loadItem(null, new AudioReference(directory.toString(), null)) == null,
        "directory path");

    Path file = directory.resolve("sample.part.MP3");
    Files.write(file, new byte[] { 42, 43, 44 });
    AudioReference reference = new AudioReference(file.toString(), "container");
    AudioItem loaded = manager.loadItem(null, reference);
    check(loaded instanceof LocalAudioTrack, "eligible path result");
    LocalAudioTrack track = (LocalAudioTrack) loaded;
    check(track.getInfo() == info && track.getSourceManager() == manager,
        "loaded track identities");
    MediaContainerDescriptor descriptor = track.getContainerTrackFactory();
    check(descriptor.probe == probe && descriptor.parameters.equals("load-settings"),
        "loaded descriptor");
    check(state.reference == reference && state.firstByte == 42 && state.matches == 1
        && state.probes == 1 && state.hints.mimeType == null
        && state.hints.fileExtension.equals("MP3"), "detection dispatch and extension");
    expect(IOException.class, () -> state.stream.seek(3));

    ProbeState failingState = new ProbeState(info);
    AssertionError probeFailure = new AssertionError("probe-sentinel");
    failingState.failure = probeFailure;
    ExposedManager failingManager = new ExposedManager(new MediaContainerRegistry(
        Collections.singletonList(failingState.proxy())));
    expectIdentity(probeFailure, () -> failingManager.loadItem(null, reference));
    expect(IOException.class, () -> failingState.stream.seek(3));

    expect(NullPointerException.class, () -> manager.loadItem(null, null));
    expect(NullPointerException.class,
        () -> manager.loadItem(null, new AudioReference(null, null)));
    Files.delete(file);
    Files.delete(directory);
  }

  private static void trackCreationAndSerialization() throws Exception {
    MediaContainerProbe probe = new ProbeState(null).proxy();
    MediaContainerRegistry registry = new MediaContainerRegistry(Collections.singletonList(probe));
    ExposedManager manager = new ExposedManager(registry);
    AudioTrackInfo info = new AudioTrackInfo(
        "title", "author", 23L, "identifier", false, "file:///identifier");
    MediaContainerDescriptor descriptor = new MediaContainerDescriptor(probe, "a|b");
    LocalAudioTrack track = (LocalAudioTrack) manager.create(info, descriptor);
    check(track.getInfo() == info && track.getContainerTrackFactory() == descriptor
        && track.getSourceManager() == manager, "protected create track");
    check(manager.isTrackEncodable(null) && manager.isTrackEncodable(track)
        && manager.isTrackEncodable(proxy(AudioTrack.class)), "encodable result");

    ByteArrayOutputStream encoded = new ByteArrayOutputStream();
    manager.encodeTrack(track, new DataOutputStream(encoded));
    check(new DataInputStream(new ByteArrayInputStream(encoded.toByteArray())).readUTF()
        .equals("probe|a|b"), "encode descriptor");
    expect(ClassCastException.class,
        () -> manager.encodeTrack(proxy(AudioTrack.class), new DataOutputStream(encoded)));
    expect(NullPointerException.class,
        () -> manager.encodeTrack(null, new DataOutputStream(encoded)));

    LocalAudioTrack decoded = (LocalAudioTrack) manager.decodeTrack(info, input("probe|decoded"));
    check(decoded.getInfo() == info && decoded.getSourceManager() == manager
        && decoded.getContainerTrackFactory().probe == probe
        && decoded.getContainerTrackFactory().parameters.equals("decoded"), "decode descriptor");
    check(manager.decodeTrack(info, input("missing")) == null, "unknown descriptor");
    IOException failure = new IOException("read-sentinel");
    DataInput failing = (DataInput) Proxy.newProxyInstance(
        DataInput.class.getClassLoader(), new Class<?>[] { DataInput.class },
        (instance, method, arguments) -> {
          if (method.getName().equals("readUTF")) throw failure;
          return null;
        });
    expectIdentity(failure, () -> manager.decodeTrack(info, failing));
    expect(NullPointerException.class, () -> manager.decodeTrack(info, null));
  }

  private static void lifecycle() {
    LocalAudioSourceManager manager = new LocalAudioSourceManager();
    check(manager.getSourceName() == "local", "source name");
    manager.shutdown();
    manager.shutdown();
  }

  private static DataInput input(String value) throws IOException {
    ByteArrayOutputStream bytes = new ByteArrayOutputStream();
    new DataOutputStream(bytes).writeUTF(value);
    return new DataInputStream(new ByteArrayInputStream(bytes.toByteArray()));
  }

  private static void checkMethod(
      Method method, Class<?> returnType, int modifiers, boolean throwsIo) {
    check(method.getModifiers() == modifiers && method.getReturnType() == returnType
        && !method.isBridge() && !method.isSynthetic() && !method.isVarArgs()
        && Arrays.equals(method.getExceptionTypes(),
            throwsIo ? new Class<?>[] { IOException.class } : new Class<?>[0]),
        method + " metadata");
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type) {
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] { type },
        (instance, method, arguments) -> {
          if (method.getName().equals("toString")) return type.getSimpleName() + "Proxy";
          if (method.getName().equals("hashCode")) return System.identityHashCode(instance);
          if (method.getName().equals("equals")) return instance == arguments[0];
          Class<?> result = method.getReturnType();
          if (result == boolean.class) return false;
          if (result == int.class) return 0;
          if (result == long.class) return 0L;
          return null;
        });
  }

  private static final class ProbeState {
    final AudioTrackInfo info;
    AudioReference reference;
    SeekableInputStream stream;
    MediaContainerHints hints;
    int firstByte;
    int matches;
    int probes;
    AssertionError failure;

    ProbeState(AudioTrackInfo info) { this.info = info; }

    MediaContainerProbe proxy() {
      return (MediaContainerProbe) Proxy.newProxyInstance(
          MediaContainerProbe.class.getClassLoader(), new Class<?>[] { MediaContainerProbe.class },
          (instance, method, arguments) -> {
            switch (method.getName()) {
              case "getName": return "probe";
              case "matchesHints":
                matches++;
                hints = (MediaContainerHints) arguments[0];
                return true;
              case "probe":
                probes++;
                reference = (AudioReference) arguments[0];
                stream = (SeekableInputStream) arguments[1];
                firstByte = stream.read();
                if (failure != null) throw failure;
                return MediaContainerDetectionResult.supportedFormat(
                    (MediaContainerProbe) instance, "load-settings", info);
              case "toString": return "ProbeState";
              case "hashCode": return System.identityHashCode(instance);
              case "equals": return instance == arguments[0];
              default: return null;
            }
          });
    }
  }

  private static final class ExposedManager extends LocalAudioSourceManager {
    ExposedManager() { super(); }
    ExposedManager(MediaContainerRegistry registry) { super(registry); }
    MediaContainerRegistry registry() { return containerRegistry; }
    AudioTrack create(AudioTrackInfo info, MediaContainerDescriptor descriptor) {
      return super.createTrack(info, descriptor);
    }
  }

  private static void expect(Class<? extends Throwable> type, Operation operation) {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
    }
  }

  private static void expectIdentity(Throwable expected, Operation operation) {
    try {
      operation.run();
      throw new AssertionError("failure was swallowed");
    } catch (Throwable error) {
      check(error == expected, "failure identity");
    }
  }

  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const HEARTBEATING_HTTP_STREAM_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.nico.HeartbeatingHttpStream;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.tools.io.PersistentHttpStream;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.net.URI;
import java.util.Arrays;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.ScheduledFuture;

public final class GateHeartbeatingHttpStream {
  public static void main(String[] args) throws Exception {
    check(args.length == 1, "expected disposition argument");
    boolean reference = args[0].equals("reference");
    check(reference || args[0].equals("candidate"), "unknown disposition");
    reflectionContract();
    ExposedStream stream = constructionAndClose(reference);
    legacyDisposition(stream, reference);
    System.out.println(
        "common=public-concrete,6-fields,1-constructor,3-exported-methods,"
        + "capture,setup-dispatch,cancel,close;legacy="
        + (reference ? "reference-scheduler,network-attempt"
            : "retained-shell,no-scheduler,unsupported"));
  }

  private static void reflectionContract() throws Exception {
    Class<HeartbeatingHttpStream> type = HeartbeatingHttpStream.class;
    check(type.getModifiers() == Modifier.PUBLIC
        && type.getSuperclass() == PersistentHttpStream.class
        && type.getInterfaces().length == 0, "class metadata");
    check(type.getDeclaredFields().length == 6, "field count");
    checkFieldName(type.getDeclaredField("log"), "org.slf4j.Logger",
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL, null);
    checkField(type.getDeclaredField("executor"), ScheduledExecutorService.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL, null);
    checkField(type.getDeclaredField("heartbeatUrl"), String.class, Modifier.PRIVATE, null);
    checkField(type.getDeclaredField("heartbeatInterval"), int.class, Modifier.PRIVATE, null);
    checkField(type.getDeclaredField("heartbeatPayload"), String.class, Modifier.PRIVATE, null);
    checkField(type.getDeclaredField("heartbeatFuture"), ScheduledFuture.class, Modifier.PRIVATE,
        "java.util.concurrent.ScheduledFuture<?>");

    Constructor<?> constructor = type.getDeclaredConstructor(
        HttpInterface.class, URI.class, Long.class, String.class, int.class, String.class);
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC
        && constructor.getExceptionTypes().length == 0 && !constructor.isVarArgs(),
        "constructor metadata");
    checkMethod(type.getDeclaredMethod("setupHeartbeat"), Modifier.PROTECTED, false);
    checkMethod(type.getDeclaredMethod("sendHeartbeat"), Modifier.PROTECTED, true);
    checkMethod(type.getDeclaredMethod("close"), Modifier.PUBLIC, true);
    long exported = Arrays.stream(type.getDeclaredMethods())
        .filter(method -> Modifier.isPublic(method.getModifiers())
            || Modifier.isProtected(method.getModifiers()))
        .count();
    check(exported == 3L, "exported method count");
  }

  private static ExposedStream constructionAndClose(boolean reference) throws Exception {
    URI content = URI.create("https://content.invalid/audio");
    String heartbeat = "https://heartbeat.invalid/session";
    String payload = "{\"session\":\"redacted\"}";
    ExposedStream stream = new ExposedStream(null, content, 123L, heartbeat, 0, payload);
    check(stream.setups == 1 && stream.sameInterface(null) && stream.sameContentUrl(content)
        && stream.getContentLength() == 123L && stream.getPosition() == 0L,
        "constructor and setup dispatch");
    check(field("heartbeatUrl").get(stream) == heartbeat
        && field("heartbeatInterval").getInt(stream) == 0
        && field("heartbeatPayload").get(stream) == payload
        && field("heartbeatFuture").get(stream) == null, "legacy state capture");

    Field log = field("log");
    Field executor = field("executor");
    check(log.get(null) != null, "logger initialized");
    check((executor.get(null) != null) == reference, "scheduler disposition");

    int[] cancellations = new int[1];
    boolean[] interrupt = new boolean[1];
    ScheduledFuture<?> future = (ScheduledFuture<?>) Proxy.newProxyInstance(
        ScheduledFuture.class.getClassLoader(), new Class<?>[] { ScheduledFuture.class },
        (instance, method, arguments) -> {
          if (method.getName().equals("cancel")) {
            cancellations[0]++;
            interrupt[0] = (Boolean) arguments[0];
            return true;
          }
          return defaultValue(method.getReturnType());
        });
    field("heartbeatFuture").set(stream, future);
    stream.close();
    check(cancellations[0] == 1 && !interrupt[0], "future cancellation");
    return stream;
  }

  private static void legacyDisposition(ExposedStream stream, boolean reference) throws Exception {
    if (reference) {
      expect(IllegalArgumentException.class, stream::defaultSetup);
      expect(NullPointerException.class, stream::heartbeat);
    } else {
      stream.defaultSetup();
      IOException error = expect(IOException.class, stream::heartbeat);
      check(error.getMessage().equals(
          "Legacy NicoNico DMC heartbeat protocol is unsupported."), "unsupported message");
      check(field("heartbeatFuture").get(stream) != null, "no scheduler replacement");
    }
  }

  private static Field field(String name) throws Exception {
    Field field = HeartbeatingHttpStream.class.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }

  private static void checkField(
      Field field, Class<?> type, int modifiers, String genericType) {
    check(field.getModifiers() == modifiers && field.getType() == type && !field.isSynthetic()
        && (genericType == null || field.getGenericType().getTypeName().equals(genericType)),
        field + " metadata");
  }

  private static void checkFieldName(
      Field field, String type, int modifiers, String genericType) {
    check(field.getModifiers() == modifiers && field.getType().getName().equals(type)
        && !field.isSynthetic()
        && (genericType == null || field.getGenericType().getTypeName().equals(genericType)),
        field + " metadata");
  }

  private static void checkMethod(Method method, int modifiers, boolean throwsIo) {
    check(method.getModifiers() == modifiers && method.getReturnType() == void.class
        && !method.isBridge() && !method.isSynthetic() && !method.isVarArgs()
        && Arrays.equals(method.getExceptionTypes(), throwsIo
            ? new Class<?>[] { IOException.class } : new Class<?>[0]), method + " metadata");
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == byte.class) return (byte) 0;
    if (type == short.class) return (short) 0;
    if (type == int.class) return 0;
    if (type == long.class) return 0L;
    if (type == float.class) return 0.0f;
    if (type == double.class) return 0.0d;
    if (type == char.class) return (char) 0;
    return null;
  }

  private static <T extends Throwable> T expect(
      Class<T> type, Operation operation) throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private static final class ExposedStream extends HeartbeatingHttpStream {
    int setups;

    ExposedStream(HttpInterface httpInterface, URI contentUrl, Long contentLength,
                  String heartbeatUrl, int heartbeatInterval, String heartbeatPayload) {
      super(httpInterface, contentUrl, contentLength, heartbeatUrl, heartbeatInterval,
          heartbeatPayload);
    }

    @Override
    protected void setupHeartbeat() { setups++; }
    void defaultSetup() { super.setupHeartbeat(); }
    void heartbeat() throws IOException { super.sendHeartbeat(); }
    boolean sameInterface(HttpInterface value) { return httpInterface == value; }
    boolean sameContentUrl(URI value) { return contentUrl == value; }
  }

  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const NICO_AUDIO_SOURCE_MANAGER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.nico.NicoAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.nico.NicoAudioTrack;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpConfigurable;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterfaceManager;
import com.sedmelluq.discord.lavaplayer.track.AudioReference;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.io.DataInput;
import java.io.DataOutput;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.regex.Pattern;

public final class GateNicoAudioSourceManager {
  public static void main(String[] args) throws Exception {
    check(args.length >= 1 && args.length <= 2, "expected disposition and optional native path");
    boolean reference = args[0].equals("reference");
    check(reference || args[0].equals("candidate"), "unknown disposition");
    check(reference == (args.length == 1), "candidate requires native path");
    reflectionContract();
    commonBehavior();
    if (!reference) currentDisposition(args[1]);
    System.out.println(
        "common=public-concrete,4-fields,2-constructors,9-exported-methods,"
        + "source-name,route-filter,empty-details,decode,shutdown,http-config;service="
        + (reference ? "legacy-xml-login" : "current-native,no-legacy-login"));
  }

  private static void reflectionContract() throws Exception {
    Class<NicoAudioSourceManager> type = NicoAudioSourceManager.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(),
            new Class<?>[] { AudioSourceManager.class, HttpConfigurable.class }),
        "class metadata");
    check(type.getDeclaredFields().length == 4, "field count");
    checkField(type.getDeclaredField("TRACK_URL_REGEX"), String.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type.getDeclaredField("trackUrlPattern"), Pattern.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type.getDeclaredField("httpInterfaceManager"), HttpInterfaceManager.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField(type.getDeclaredField("loggedIn"), AtomicBoolean.class,
        Modifier.PRIVATE | Modifier.FINAL);

    Constructor<?> defaultConstructor = type.getDeclaredConstructor();
    Constructor<?> credentialConstructor = type.getDeclaredConstructor(String.class, String.class);
    check(type.getDeclaredConstructors().length == 2
        && defaultConstructor.getModifiers() == Modifier.PUBLIC
        && credentialConstructor.getModifiers() == Modifier.PUBLIC,
        "constructor metadata");
    check(type.getDeclaredMethods().length == 13, "method count");
    long exported = Arrays.stream(type.getDeclaredMethods())
        .filter(method -> Modifier.isPublic(method.getModifiers())
            || Modifier.isProtected(method.getModifiers()))
        .count();
    check(exported == 9L, "exported method count");
    checkMethod(type.getDeclaredMethod("getSourceName"), String.class, Modifier.PUBLIC);
    checkMethod(type.getDeclaredMethod("loadItem",
        com.sedmelluq.discord.lavaplayer.player.AudioPlayerManager.class,
        AudioReference.class),
        com.sedmelluq.discord.lavaplayer.track.AudioItem.class, Modifier.PUBLIC);
    checkMethod(type.getDeclaredMethod("isTrackEncodable", AudioTrack.class),
        boolean.class, Modifier.PUBLIC);
    checkMethod(type.getDeclaredMethod("encodeTrack", AudioTrack.class, DataOutput.class),
        void.class, Modifier.PUBLIC, java.io.IOException.class);
    checkMethod(type.getDeclaredMethod("decodeTrack", AudioTrackInfo.class, DataInput.class),
        AudioTrack.class, Modifier.PUBLIC, java.io.IOException.class);
    checkMethod(type.getDeclaredMethod("shutdown"), void.class, Modifier.PUBLIC);
    checkMethod(type.getDeclaredMethod("getHttpInterface"), HttpInterface.class, Modifier.PUBLIC);
    checkMethod(type.getDeclaredMethod("configureRequests", java.util.function.Function.class),
        void.class, Modifier.PUBLIC);
    checkMethod(type.getDeclaredMethod("configureBuilder", java.util.function.Consumer.class),
        void.class, Modifier.PUBLIC);
  }

  private static void commonBehavior() throws Exception {
    NicoAudioSourceManager manager = new NicoAudioSourceManager();
    NicoAudioSourceManager nullCredentials = new NicoAudioSourceManager(null, null);
    check(manager.getSourceName().equals("niconico")
        && nullCredentials.getSourceName().equals("niconico"), "source name");
    check(field("httpInterfaceManager").get(manager) != null
        && field("loggedIn").get(manager) instanceof AtomicBoolean
        && !((AtomicBoolean) field("loggedIn").get(manager)).get(), "constructor state");

    for (String rejected : new String[] {
        "https://example.invalid/watch/sm9", "https://www.nicovideo.jp/shorts/sm9",
        "https://www.nicovideo.jp/watch/not-a-video", "prefix sm9" }) {
      check(manager.loadItem(null, new AudioReference(rejected, null)) == null,
          "route rejection: " + rejected);
    }
    expect(NullPointerException.class, () -> manager.loadItem(null, null));
    expect(NullPointerException.class,
        () -> manager.loadItem(null, new AudioReference(null, null)));

    check(manager.isTrackEncodable(null), "encodability");
    manager.encodeTrack(null, null);
    AudioTrackInfo info = new AudioTrackInfo(
        "title", "author", 1234L, "sm9", false,
        "https://www.nicovideo.jp/watch/sm9", "art", null);
    AudioTrack decoded = manager.decodeTrack(info, null);
    check(decoded instanceof NicoAudioTrack && decoded.getInfo() == info
        && decoded.getSourceManager() == manager, "empty-detail decode");
    manager.shutdown();
    manager.shutdown();

    HttpInterface http = manager.getHttpInterface();
    check(http != null, "HTTP interface");
    http.close();
    Method requests = NicoAudioSourceManager.class.getDeclaredMethod(
        "configureRequests", java.util.function.Function.class);
    Method builder = NicoAudioSourceManager.class.getDeclaredMethod(
        "configureBuilder", java.util.function.Consumer.class);
    expectInvocation(NullPointerException.class,
        () -> requests.invoke(manager, new Object[] { null }));
    expectInvocation(NullPointerException.class,
        () -> builder.invoke(manager, new Object[] { null }));
  }

  private static void currentDisposition(String nativeLibrary) throws Exception {
    Class.forName("dev.mantle.internal.NativeLoader")
        .getMethod("load", String.class).invoke(null, nativeLibrary);
    NicoAudioSourceManager manager = new NicoAudioSourceManager("legacy@example.invalid", "secret");
    check(!((AtomicBoolean) field("loggedIn").get(manager)).get(), "legacy login disabled");
    Method login = NicoAudioSourceManager.class.getDeclaredMethod(
        "logIn", String.class, String.class);
    login.setAccessible(true);
    UnsupportedOperationException error = expectInvocation(
        UnsupportedOperationException.class, () -> login.invoke(manager, "legacy", "secret"));
    check(error.getMessage().equals(
        "Legacy NicoNico email/password login is unsupported."), "legacy login message");
    Class<?> nativeType = Class.forName("dev.mantle.internal.MantleNative");
    Method load = nativeType.getDeclaredMethod(
        "loadNicoItem", NicoAudioSourceManager.class, AudioReference.class);
    check(Modifier.isPublic(load.getModifiers()) && Modifier.isStatic(load.getModifiers())
        && Modifier.isNative(load.getModifiers()), "current native route");
    check(manager.loadItem(null, new AudioReference(
        "https://www.nicovideo.jp/watch/XX123", null)) == null,
        "recognized legacy route crosses strict native router");
  }

  private static Field field(String name) throws Exception {
    Field field = NicoAudioSourceManager.class.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }

  private static void checkField(Field field, Class<?> type, int modifiers) {
    check(field.getType() == type && field.getModifiers() == modifiers && !field.isSynthetic(),
        field + " metadata");
  }

  private static void checkMethod(Method method, Class<?> returnType, int modifiers,
                                  Class<?>... exceptions) {
    check(method.getReturnType() == returnType && method.getModifiers() == modifiers
        && !method.isBridge() && !method.isSynthetic() && !method.isVarArgs()
        && Arrays.equals(method.getExceptionTypes(), exceptions), method + " metadata");
  }

  private static <T extends Throwable> T expect(
      Class<T> type, Operation operation) throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private static <T extends Throwable> T expectInvocation(
      Class<T> type, Operation operation) throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (java.lang.reflect.InvocationTargetException error) {
      Throwable cause = error.getCause();
      if (!type.isInstance(cause)) throw new AssertionError("wrong exception", cause);
      return type.cast(cause);
    }
  }

  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const NICO_AUDIO_TRACK_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.nico.NicoAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.nico.NicoAudioTrack;
import com.sedmelluq.discord.lavaplayer.tools.JsonBrowser;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import com.sedmelluq.discord.lavaplayer.track.DelegatedAudioTrack;
import com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;

public final class GateNicoAudioTrack {
  public static void main(String[] args) throws Exception {
    check(args.length >= 1 && args.length <= 2, "expected disposition and optional native path");
    boolean reference = args[0].equals("reference");
    check(reference || args[0].equals("candidate"), "unknown disposition");
    check(reference == (args.length == 1), "candidate requires native path");
    reflectionContract();
    commonBehavior();
    if (!reference) currentDisposition(args[1]);
    System.out.println(
        "common=public-concrete,6-fields,1-constructor,3-exported-methods,"
        + "capture,source-identity,shallow-clone;service="
        + (reference ? "legacy-dmc-mpeg" : "current-native-cmaf-opus,no-legacy-dmc"));
  }

  private static void reflectionContract() throws Exception {
    Class<NicoAudioTrack> type = NicoAudioTrack.class;
    check(type.getModifiers() == Modifier.PUBLIC
        && type.getSuperclass() == DelegatedAudioTrack.class
        && type.getInterfaces().length == 0, "class metadata");
    check(type.getDeclaredFields().length == 6, "field count");
    checkField(type, "log", Class.forName("org.slf4j.Logger"),
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "actionTrackId", String.class, Modifier.PRIVATE | Modifier.STATIC);
    checkField(type, "sourceManager", NicoAudioSourceManager.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "heartbeatUrl", String.class, Modifier.PRIVATE);
    checkField(type, "heartbeatIntervalMs", int.class, Modifier.PRIVATE);
    checkField(type, "initialHeartbeatPayload", String.class, Modifier.PRIVATE);

    Constructor<?> constructor = type.getDeclaredConstructor(
        AudioTrackInfo.class, NicoAudioSourceManager.class);
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC, "constructor metadata");
    check(type.getDeclaredMethods().length == 7, "method count");
    long exported = Arrays.stream(type.getDeclaredMethods())
        .filter(method -> Modifier.isPublic(method.getModifiers())
            || Modifier.isProtected(method.getModifiers()))
        .count();
    check(exported == 3L, "exported method count");
    checkMethod(type.getDeclaredMethod("process", LocalAudioTrackExecutor.class),
        void.class, Modifier.PUBLIC, Exception.class);
    checkMethod(type.getDeclaredMethod("loadVideoApi", HttpInterface.class),
        JsonBrowser.class, Modifier.PRIVATE, IOException.class);
    checkMethod(type.getDeclaredMethod("loadVideoMainPage", HttpInterface.class),
        JsonBrowser.class, Modifier.PRIVATE, IOException.class);
    checkMethod(type.getDeclaredMethod("loadPlaybackUrl", HttpInterface.class),
        String.class, Modifier.PRIVATE, IOException.class);
    checkMethod(type.getDeclaredMethod("processJSON", JsonBrowser.class),
        Class.forName("org.json.JSONObject"), Modifier.PRIVATE);
    checkMethod(type.getDeclaredMethod("makeShallowClone"),
        AudioTrack.class, Modifier.PROTECTED);
    checkMethod(type.getDeclaredMethod("getSourceManager"),
        AudioSourceManager.class, Modifier.PUBLIC);
  }

  private static void commonBehavior() throws Exception {
    AudioTrackInfo info = new AudioTrackInfo(
        "title", "author", 1234L, "sm9", false,
        "https://www.nicovideo.jp/watch/sm9", "art", null);
    NicoAudioSourceManager source = new NicoAudioSourceManager();
    ExposedTrack track = new ExposedTrack(info, source);
    check(track.getInfo() == info && track.getSourceManager() == source, "captured identity");
    check(field("sourceManager").get(track) == source
        && field("heartbeatUrl").get(track) == null
        && field("heartbeatIntervalMs").getInt(track) == 0
        && field("initialHeartbeatPayload").get(track) == null, "constructor state");
    check(field("log").get(null) != null
        && field("actionTrackId").get(null).equals("S1G2fKdzOl_1702504390263"),
        "static state");
    AudioTrack clone = track.shallowClone();
    check(clone instanceof NicoAudioTrack && clone != track && clone.getInfo() == info
        && clone.getSourceManager() == source, "shallow clone identity");
    check(field("heartbeatUrl").get(clone) == null
        && field("heartbeatIntervalMs").getInt(clone) == 0
        && field("initialHeartbeatPayload").get(clone) == null, "shallow clone state");
    source.shutdown();
  }

  private static void currentDisposition(String nativeLibrary) throws Exception {
    Class.forName("dev.mantle.internal.NativeLoader")
        .getMethod("load", String.class).invoke(null, nativeLibrary);
    Class<?> nativeType = Class.forName("dev.mantle.internal.MantleNative");
    Method process = nativeType.getDeclaredMethod(
        "processNicoTrack", NicoAudioTrack.class, LocalAudioTrackExecutor.class);
    check(Modifier.isPublic(process.getModifiers()) && Modifier.isStatic(process.getModifiers())
        && Modifier.isNative(process.getModifiers()), "current native route");
    AudioTrackInfo invalid = new AudioTrackInfo(
        "title", "author", 1234L, "XX123", false,
        "https://www.nicovideo.jp/watch/XX123", null, null);
    NicoAudioSourceManager source = new NicoAudioSourceManager();
    NicoAudioTrack track = new NicoAudioTrack(invalid, source);
    expect(RuntimeException.class, () -> track.process(null));
    check(field("heartbeatUrl").get(track) == null
        && field("heartbeatIntervalMs").getInt(track) == 0
        && field("initialHeartbeatPayload").get(track) == null,
        "legacy DMC state remains unused");
    source.shutdown();
  }

  private static Field field(String name) throws Exception {
    Field field = NicoAudioTrack.class.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }

  private static void checkField(Class<?> type, String name, Class<?> fieldType, int modifiers)
      throws Exception {
    Field field = type.getDeclaredField(name);
    check(field.getType() == fieldType && field.getModifiers() == modifiers
        && !field.isSynthetic(), field + " metadata");
  }

  private static void checkMethod(Method method, Class<?> returnType, int modifiers,
                                  Class<?>... exceptions) {
    check(method.getReturnType() == returnType && method.getModifiers() == modifiers
        && !method.isBridge() && !method.isSynthetic() && !method.isVarArgs()
        && Arrays.equals(method.getExceptionTypes(), exceptions), method + " metadata");
  }

  private static <T extends Throwable> T expect(
      Class<T> type, Operation operation) throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private static final class ExposedTrack extends NicoAudioTrack {
    ExposedTrack(AudioTrackInfo info, NicoAudioSourceManager source) { super(info, source); }
    AudioTrack shallowClone() { return super.makeShallowClone(); }
  }

  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const SOUND_CLOUD_AUDIO_TRACK_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudAudioTrack;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudM3uInfo;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import com.sedmelluq.discord.lavaplayer.track.DelegatedAudioTrack;
import com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;

public final class GateSoundCloudAudioTrack {
  public static void main(String[] args) throws Exception {
    check(args.length >= 1 && args.length <= 2, "expected disposition and optional native path");
    boolean reference = args[0].equals("reference");
    check(reference || args[0].equals("candidate"), "unknown disposition");
    check(reference == (args.length == 1), "candidate requires native path");
    reflectionContract();
    commonBehavior();
    if (!reference) currentDisposition(args[1]);
    System.out.println(
        "common=public-concrete,2-fields,1-constructor,3-exported-methods,"
        + "capture,source-identity,shallow-clone;service="
        + (reference ? "legacy-web-client-http" : "current-native-explicit-credentials,no-client-scrape"));
  }

  private static void reflectionContract() throws Exception {
    Class<SoundCloudAudioTrack> type = SoundCloudAudioTrack.class;
    check(type.getModifiers() == Modifier.PUBLIC
        && type.getSuperclass() == DelegatedAudioTrack.class
        && type.getInterfaces().length == 0, "class metadata");
    check(type.getDeclaredFields().length == 2, "field count");
    checkField(type, "log", Class.forName("org.slf4j.Logger"),
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "sourceManager", SoundCloudAudioSourceManager.class,
        Modifier.PRIVATE | Modifier.FINAL);
    Constructor<?> constructor = type.getDeclaredConstructor(
        AudioTrackInfo.class, SoundCloudAudioSourceManager.class);
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC, "constructor metadata");
    check(type.getDeclaredMethods().length == 5, "method count");
    long exported = Arrays.stream(type.getDeclaredMethods())
        .filter(method -> Modifier.isPublic(method.getModifiers())
            || Modifier.isProtected(method.getModifiers()))
        .count();
    check(exported == 3L, "exported method count");
    checkMethod(type.getDeclaredMethod("process", LocalAudioTrackExecutor.class),
        void.class, Modifier.PUBLIC, Exception.class);
    checkMethod(type.getDeclaredMethod("playFromIdentifier", HttpInterface.class, String.class,
        boolean.class, LocalAudioTrackExecutor.class), void.class, Modifier.PRIVATE, Exception.class);
    checkMethod(type.getDeclaredMethod("loadFromMp3Url", LocalAudioTrackExecutor.class,
        HttpInterface.class, String.class), void.class, Modifier.PRIVATE, Exception.class);
    checkMethod(type.getDeclaredMethod("makeShallowClone"),
        AudioTrack.class, Modifier.PROTECTED);
    checkMethod(type.getDeclaredMethod("getSourceManager"),
        AudioSourceManager.class, Modifier.PUBLIC);
  }

  private static void commonBehavior() throws Exception {
    AudioTrackInfo info = new AudioTrackInfo(
        "title", "author", 1234L, "O:123", false,
        "https://soundcloud.com/fixture/song", "art", null);
    SoundCloudAudioSourceManager source = SoundCloudAudioSourceManager.createDefault();
    ExposedTrack track = new ExposedTrack(info, source);
    check(track.getInfo() == info && track.getSourceManager() == source, "captured identity");
    check(field("sourceManager").get(track) == source && field("log").get(null) != null,
        "constructor and static state");
    AudioTrack clone = track.shallowClone();
    check(clone instanceof SoundCloudAudioTrack && clone != track && clone.getInfo() == info
        && clone.getSourceManager() == source, "shallow clone identity");
    source.shutdown();
  }

  private static void currentDisposition(String nativeLibrary) throws Exception {
    Class.forName("dev.mantle.internal.NativeLoader")
        .getMethod("load", String.class).invoke(null, nativeLibrary);
    Class<?> nativeType = Class.forName("dev.mantle.internal.MantleNative");
    Method process = nativeType.getDeclaredMethod(
        "processSoundCloudTrack", SoundCloudAudioTrack.class, LocalAudioTrackExecutor.class);
    check(Modifier.isPublic(process.getModifiers()) && Modifier.isStatic(process.getModifiers())
        && Modifier.isNative(process.getModifiers()), "current native route");
    System.clearProperty("dev.mantle.soundcloud.clientId");
    System.clearProperty("dev.mantle.soundcloud.oauthToken");
    AudioTrackInfo info = new AudioTrackInfo(
        "title", "author", 1234L, "O:123", false,
        "https://soundcloud.com/fixture/song", null, null);
    SoundCloudAudioSourceManager source = SoundCloudAudioSourceManager.createDefault();
    RuntimeException error = expect(RuntimeException.class,
        () -> new SoundCloudAudioTrack(info, source).process(null));
    check(error.getMessage().contains("dev.mantle.soundcloud.clientId"),
        "missing explicit credential failure");
    source.shutdown();
  }

  private static Field field(String name) throws Exception {
    Field field = SoundCloudAudioTrack.class.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }

  private static void checkField(Class<?> type, String name, Class<?> fieldType, int modifiers)
      throws Exception {
    Field field = type.getDeclaredField(name);
    check(field.getType() == fieldType && field.getModifiers() == modifiers
        && !field.isSynthetic(), field + " metadata");
  }

  private static void checkMethod(Method method, Class<?> returnType, int modifiers,
                                  Class<?>... exceptions) {
    check(method.getReturnType() == returnType && method.getModifiers() == modifiers
        && Arrays.equals(method.getExceptionTypes(), exceptions)
        && !method.isBridge() && !method.isSynthetic(), method + " metadata");
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private interface Operation { void run() throws Exception; }

  private static final class ExposedTrack extends SoundCloudAudioTrack {
    ExposedTrack(AudioTrackInfo info, SoundCloudAudioSourceManager source) {
      super(info, source);
    }
    AudioTrack shallowClone() { return makeShallowClone(); }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const SOUND_CLOUD_CLIENT_ID_TRACKER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudClientIdTracker;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterfaceManager;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import org.apache.http.client.protocol.HttpClientContext;

public final class GateSoundCloudClientIdTracker {
  private static final String PROPERTY = "dev.mantle.soundcloud.clientId";

  public static void main(String[] args) throws Exception {
    check(args.length == 1 && (args[0].equals("reference") || args[0].equals("candidate")),
        "expected disposition");
    boolean reference = args[0].equals("reference");
    reflectionContract();
    commonBehavior();
    if (reference) referenceDisposition(); else currentDisposition();
    System.out.println(
        "common=public-concrete,11-fields,1-constructor,3-exported-methods,"
        + "dependency-capture,context-marker,private-shell;service="
        + (reference ? "legacy-web-client-scrape" :
            "bounded-explicit-property,no-http,no-client-scrape"));
  }

  private static void reflectionContract() throws Exception {
    Class<SoundCloudClientIdTracker> type = SoundCloudClientIdTracker.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && type.getInterfaces().length == 0, "class metadata");
    check(type.getDeclaredFields().length == 11, "field count");
    checkField(type, "log", Class.forName("org.slf4j.Logger"),
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "ID_FETCH_CONTEXT_ATTRIBUTE", String.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "CLIENT_ID_REFRESH_INTERVAL", long.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "PAGE_APP_SCRIPT_REGEX", String.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "APP_SCRIPT_CLIENT_ID_REGEX", String.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "pageAppScriptPattern", Pattern.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "appScriptClientIdPattern", Pattern.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "clientIdLock", Object.class, Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "httpInterfaceManager", HttpInterfaceManager.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "clientId", String.class, Modifier.PRIVATE);
    checkField(type, "lastClientIdUpdate", long.class, Modifier.PRIVATE);

    Constructor<?> constructor = type.getDeclaredConstructor(HttpInterfaceManager.class);
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC, "constructor metadata");
    check(type.getDeclaredMethods().length == 7, "method count");
    long exported = Arrays.stream(type.getDeclaredMethods())
        .filter(method -> Modifier.isPublic(method.getModifiers())
            || Modifier.isProtected(method.getModifiers()))
        .count();
    check(exported == 3L, "exported method count");
    checkMethod(type.getDeclaredMethod("updateClientId"), void.class, Modifier.PUBLIC);
    checkMethod(type.getDeclaredMethod("getClientId"), String.class, Modifier.PUBLIC);
    checkMethod(type.getDeclaredMethod("isIdFetchContext", HttpClientContext.class),
        boolean.class, Modifier.PUBLIC);
    checkMethod(type.getDeclaredMethod("findClientIdFromSite"), String.class,
        Modifier.PRIVATE, IOException.class);
    checkMethod(type.getDeclaredMethod("findApplicationScriptUrl",
        Class.forName("com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface")),
        String.class, Modifier.PRIVATE, IOException.class);
    checkMethod(type.getDeclaredMethod("getLastMatchWithinLimit", Matcher.class, int.class),
        String.class, Modifier.PRIVATE);
    checkMethod(type.getDeclaredMethod("findClientIdFromApplicationScript",
        Class.forName("com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface"), String.class),
        String.class, Modifier.PRIVATE, IOException.class);

    check(stringField("ID_FETCH_CONTEXT_ATTRIBUTE").equals("sc-raw"), "context constant");
    check(longField("CLIENT_ID_REFRESH_INTERVAL") == 3_600_000L, "refresh constant");
    check(stringField("PAGE_APP_SCRIPT_REGEX").equals(
        "https://[A-Za-z0-9-.]+/assets/[a-f0-9-]+\\.js"), "page regex constant");
    check(stringField("APP_SCRIPT_CLIENT_ID_REGEX").equals(
        "[^_]client_id:\"([a-zA-Z0-9-_]+)\""), "client regex constant");
    check(((Pattern) field("pageAppScriptPattern").get(null)).pattern().equals(
        stringField("PAGE_APP_SCRIPT_REGEX")), "page pattern");
    check(((Pattern) field("appScriptClientIdPattern").get(null)).pattern().equals(
        stringField("APP_SCRIPT_CLIENT_ID_REGEX")), "client pattern");
  }

  private static void commonBehavior() throws Exception {
    AtomicInteger acquisitions = new AtomicInteger();
    HttpInterfaceManager manager = manager(acquisitions);
    SoundCloudClientIdTracker tracker = new SoundCloudClientIdTracker(manager);
    check(field("httpInterfaceManager").get(tracker) == manager
        && field("clientIdLock").get(tracker) != null
        && field("clientId").get(tracker) == null
        && field("lastClientIdUpdate").getLong(tracker) == 0L, "constructor state");
    HttpClientContext context = HttpClientContext.create();
    check(!tracker.isIdFetchContext(context), "missing context marker");
    context.setAttribute("sc-raw", Boolean.FALSE);
    check(!tracker.isIdFetchContext(context), "false context marker");
    context.setAttribute("sc-raw", Boolean.TRUE);
    check(tracker.isIdFetchContext(context), "true identity context marker");
    expect(NullPointerException.class, () -> tracker.isIdFetchContext(null));
    check(acquisitions.get() == 0, "common behavior stays offline");
  }

  private static void referenceDisposition() throws Exception {
    AtomicInteger acquisitions = new AtomicInteger();
    SoundCloudClientIdTracker tracker = new SoundCloudClientIdTracker(manager(acquisitions));
    field("clientId").set(tracker, "frozen-id");
    check(tracker.getClientId().equals("frozen-id") && acquisitions.get() == 0,
        "cached reference ID");
  }

  private static void currentDisposition() throws Exception {
    AtomicInteger acquisitions = new AtomicInteger();
    SoundCloudClientIdTracker tracker = new SoundCloudClientIdTracker(manager(acquisitions));
    String previous = System.getProperty(PROPERTY);
    try {
      System.setProperty(PROPERTY, "caller-id_1");
      check(tracker.getClientId().equals("caller-id_1"), "lazy explicit ID");
      check(field("lastClientIdUpdate").getLong(tracker) > 0L, "successful update timestamp");
      System.setProperty(PROPERTY, "caller-id_2");
      tracker.updateClientId();
      check(tracker.getClientId().equals("caller-id_2"), "explicit refresh");

      System.clearProperty(PROPERTY);
      SoundCloudClientIdTracker missing = new SoundCloudClientIdTracker(manager(acquisitions));
      IllegalStateException absent = expect(IllegalStateException.class, missing::getClientId);
      check(absent.getMessage().contains(PROPERTY), "missing credential message");
      check(field("clientId").get(missing) == null
          && field("lastClientIdUpdate").getLong(missing) == 0L, "missing state unchanged");

      for (String invalid : new String[] {"", "space id", "é", "x".repeat(257)}) {
        System.setProperty(PROPERTY, invalid);
        SoundCloudClientIdTracker rejected =
            new SoundCloudClientIdTracker(manager(acquisitions));
        IllegalArgumentException error =
            expect(IllegalArgumentException.class, rejected::updateClientId);
        check(error.getMessage().equals("Invalid explicit SoundCloud client ID")
            && field("clientId").get(rejected) == null,
            "invalid credential redaction and state");
      }

      Method legacy = SoundCloudClientIdTracker.class.getDeclaredMethod("findClientIdFromSite");
      legacy.setAccessible(true);
      UnsupportedOperationException disabled = expectInvocation(
          UnsupportedOperationException.class, () -> legacy.invoke(tracker));
      check(disabled.getMessage().contains("unsupported"), "legacy scraper failure");
      check(acquisitions.get() == 0, "candidate never acquires HTTP");
    } finally {
      if (previous == null) System.clearProperty(PROPERTY);
      else System.setProperty(PROPERTY, previous);
    }
  }

  private static HttpInterfaceManager manager(AtomicInteger acquisitions) {
    return (HttpInterfaceManager) java.lang.reflect.Proxy.newProxyInstance(
        GateSoundCloudClientIdTracker.class.getClassLoader(),
        new Class<?>[] {HttpInterfaceManager.class}, (proxy, method, args) -> {
          if (method.getName().equals("getInterface")) acquisitions.incrementAndGet();
          if (method.getName().equals("toString")) return "manager-proxy";
          if (method.getReturnType() == boolean.class) return false;
          if (method.getReturnType() == int.class) return 0;
          if (method.getReturnType() == long.class) return 0L;
          return null;
        });
  }

  private static Field field(String name) throws Exception {
    Field field = SoundCloudClientIdTracker.class.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }

  private static String stringField(String name) throws Exception {
    return (String) field(name).get(null);
  }

  private static long longField(String name) throws Exception {
    return field(name).getLong(null);
  }

  private static void checkField(Class<?> type, String name, Class<?> fieldType, int modifiers)
      throws Exception {
    Field field = type.getDeclaredField(name);
    check(field.getType() == fieldType && field.getModifiers() == modifiers
        && !field.isSynthetic(), field + " metadata");
  }

  private static void checkMethod(Method method, Class<?> returnType, int modifiers,
                                  Class<?>... exceptions) {
    check(method.getReturnType() == returnType && method.getModifiers() == modifiers
        && Arrays.equals(method.getExceptionTypes(), exceptions)
        && !method.isBridge() && !method.isSynthetic(), method + " metadata");
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private static <T extends Throwable> T expectInvocation(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (InvocationTargetException error) {
      if (!type.isInstance(error.getCause())) throw new AssertionError("wrong exception", error);
      return type.cast(error.getCause());
    }
  }

  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const SOUND_CLOUD_DATA_LOADER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudDataLoader;
import com.sedmelluq.discord.lavaplayer.tools.JsonBrowser;
import com.sedmelluq.discord.lavaplayer.tools.http.HttpContextFilter;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import java.io.IOException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;
import org.apache.http.client.protocol.HttpClientContext;

public final class GateSoundCloudDataLoader {
  public static void main(String[] args) throws Exception {
    dispatchContract();
    failureContract();
    reflectionContract();
    System.out.println(
        "public-abstract-interface,0-fields,0-constructors,1-method;"
        + "dispatch,argument-identity,return-identity,nulls,checked-io,reflection");
  }

  private static void dispatchContract() throws Exception {
    HttpInterface http = new HttpInterface(
        null, HttpClientContext.create(), false, proxy(HttpContextFilter.class));
    String url = new String("https://api-v2.soundcloud.com/resolve?url=fixture");
    JsonBrowser result = JsonBrowser.parse("{\"kind\":\"track\"}");
    Object[] observed = new Object[2];
    int[] calls = new int[1];
    SoundCloudDataLoader loader = (actualHttp, actualUrl) -> {
      observed[0] = actualHttp;
      observed[1] = actualUrl;
      calls[0]++;
      return result;
    };
    check(loader.load(http, url) == result && observed[0] == http && observed[1] == url
        && calls[0] == 1, "ordinary dispatch identity");
    check(loader.load(null, null) == result && observed[0] == null && observed[1] == null
        && calls[0] == 2, "null dispatch identity");

    SoundCloudDataLoader nullLoader = (actualHttp, actualUrl) -> null;
    check(nullLoader.load(http, url) == null, "null return");
  }

  private static void failureContract() throws Exception {
    IOException failure = new IOException("load-sentinel");
    SoundCloudDataLoader loader = (http, url) -> { throw failure; };
    try {
      loader.load(null, "fixture");
      throw new AssertionError("expected IOException");
    } catch (IOException error) {
      check(error == failure, "checked failure identity");
    }
  }

  private static void reflectionContract() throws Exception {
    Class<SoundCloudDataLoader> type = SoundCloudDataLoader.class;
    check(type.isInterface() && Modifier.isPublic(type.getModifiers())
        && Modifier.isAbstract(type.getModifiers()) && !Modifier.isFinal(type.getModifiers())
        && type.getSuperclass() == null && type.getInterfaces().length == 0
        && type.getAnnotations().length == 0, "interface metadata");
    check(type.getDeclaredFields().length == 0 && type.getDeclaredConstructors().length == 0
        && type.getDeclaredMethods().length == 1, "member counts");
    Method method = type.getDeclaredMethod("load", HttpInterface.class, String.class);
    check(method.getReturnType() == JsonBrowser.class
        && method.getModifiers() == (Modifier.PUBLIC | Modifier.ABSTRACT)
        && Arrays.equals(method.getExceptionTypes(), new Class<?>[] {IOException.class})
        && method.getTypeParameters().length == 0
        && method.getGenericReturnType() == JsonBrowser.class
        && Arrays.equals(method.getGenericParameterTypes(),
            new Object[] {HttpInterface.class, String.class})
        && !method.isDefault() && !method.isBridge() && !method.isSynthetic()
        && !method.isVarArgs(), "method metadata");
  }

  private static <T> T proxy(Class<T> type) {
    return type.cast(Proxy.newProxyInstance(
        GateSoundCloudDataLoader.class.getClassLoader(), new Class<?>[] {type},
        (proxy, method, args) -> {
          if (method.getName().equals("toString")) return "proxy";
          if (method.getReturnType() == boolean.class) return false;
          if (method.getReturnType() == int.class) return 0;
          if (method.getReturnType() == long.class) return 0L;
          return null;
        }));
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const SOUND_CLOUD_PLAYLIST_LOADER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudPlaylistLoader;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterfaceManager;
import com.sedmelluq.discord.lavaplayer.track.AudioPlaylist;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Proxy;
import java.util.Arrays;
import java.util.function.Function;

public final class GateSoundCloudPlaylistLoader {
  public static void main(String[] args) throws Exception {
    dispatchContract();
    nullContract();
    failureContract();
    reflectionContract();
    System.out.println(
        "public-abstract-interface,0-fields,0-constructors,1-method;"
        + "dispatch,argument-identity,return-identity,nulls,unchecked,"
        + "generic-function-parameter,reflection");
  }

  private static void dispatchContract() {
    String identifier = new String("fixture");
    HttpInterfaceManager manager = proxy(HttpInterfaceManager.class);
    AudioPlaylist playlist = proxy(AudioPlaylist.class);
    Function<AudioTrackInfo, AudioTrack> factory = info -> proxy(AudioTrack.class);
    Object[] observed = new Object[3];
    int[] calls = new int[1];
    SoundCloudPlaylistLoader loader = (actualIdentifier, actualManager, actualFactory) -> {
      observed[0] = actualIdentifier;
      observed[1] = actualManager;
      observed[2] = actualFactory;
      calls[0]++;
      return playlist;
    };
    check(loader.load(identifier, manager, factory) == playlist
        && observed[0] == identifier && observed[1] == manager && observed[2] == factory
        && calls[0] == 1, "ordinary dispatch identity");
  }

  private static void nullContract() {
    Object[] observed = new Object[3];
    SoundCloudPlaylistLoader loader = (identifier, manager, factory) -> {
      observed[0] = identifier;
      observed[1] = manager;
      observed[2] = factory;
      return null;
    };
    check(loader.load(null, null, null) == null
        && observed[0] == null && observed[1] == null && observed[2] == null,
        "null dispatch and return");
  }

  private static void failureContract() {
    RuntimeException failure = new RuntimeException("playlist-loader-sentinel");
    SoundCloudPlaylistLoader loader = (identifier, manager, factory) -> { throw failure; };
    try {
      loader.load(null, null, null);
      throw new AssertionError("expected failure");
    } catch (RuntimeException error) {
      check(error == failure, "unchecked failure identity");
    }
  }

  private static void reflectionContract() throws Exception {
    Class<SoundCloudPlaylistLoader> type = SoundCloudPlaylistLoader.class;
    check(type.isInterface() && Modifier.isPublic(type.getModifiers())
        && Modifier.isAbstract(type.getModifiers()) && !Modifier.isFinal(type.getModifiers())
        && type.getSuperclass() == null && type.getInterfaces().length == 0
        && type.getTypeParameters().length == 0 && type.getAnnotations().length == 0,
        "interface metadata");
    check(type.getDeclaredFields().length == 0 && type.getDeclaredConstructors().length == 0
        && type.getDeclaredMethods().length == 1, "member counts");
    Method method = type.getDeclaredMethod(
        "load", String.class, HttpInterfaceManager.class, Function.class);
    check(method.getReturnType() == AudioPlaylist.class
        && method.getModifiers() == (Modifier.PUBLIC | Modifier.ABSTRACT)
        && Arrays.equals(method.getParameterTypes(),
            new Class<?>[] {String.class, HttpInterfaceManager.class, Function.class})
        && method.getExceptionTypes().length == 0 && method.getTypeParameters().length == 0
        && method.getGenericReturnType() == AudioPlaylist.class
        && method.getGenericParameterTypes()[0] == String.class
        && method.getGenericParameterTypes()[1] == HttpInterfaceManager.class
        && method.getGenericParameterTypes()[2] instanceof ParameterizedType
        && !method.isDefault() && !method.isBridge() && !method.isSynthetic()
        && !method.isVarArgs(), "method metadata");
    ParameterizedType factoryType =
        (ParameterizedType) method.getGenericParameterTypes()[2];
    check(factoryType.getRawType() == Function.class
        && Arrays.equals(factoryType.getActualTypeArguments(),
            new Object[] {AudioTrackInfo.class, AudioTrack.class}),
        "generic factory metadata");
  }

  private static <T> T proxy(Class<T> type) {
    return type.cast(Proxy.newProxyInstance(
        GateSoundCloudPlaylistLoader.class.getClassLoader(), new Class<?>[] {type},
        (proxy, method, args) -> {
          if (method.getName().equals("toString")) return "proxy";
          if (method.getReturnType() == boolean.class) return false;
          if (method.getReturnType() == int.class) return 0;
          if (method.getReturnType() == long.class) return 0L;
          return null;
        }));
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const SOUND_CLOUD_SEGMENT_DECODER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.format.AudioDataFormat;
import com.sedmelluq.discord.lavaplayer.player.AudioConfiguration;
import com.sedmelluq.discord.lavaplayer.player.AudioPlayerOptions;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudSegmentDecoder;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameBuffer;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioProcessingContext;
import java.io.IOException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;

public final class GateSoundCloudSegmentDecoder {
  public static void main(String[] args) throws Exception {
    dispatchContract();
    failureContract();
    reflectionContract();
    System.out.println(
        "public-abstract-interface,autocloseable,0-fields,0-constructors,3-methods;"
        + "ordered-dispatch,boolean,longs,context-identity,nulls,checked-failures,reflection");
  }

  private static void dispatchContract() throws Exception {
    AudioConfiguration configuration = new AudioConfiguration();
    AudioFrameBuffer buffer = proxy(AudioFrameBuffer.class);
    AudioPlayerOptions options = new AudioPlayerOptions();
    AudioDataFormat format = configuration.getOutputFormat();
    AudioProcessingContext context =
        new AudioProcessingContext(configuration, buffer, options, format);
    RecordingDecoder decoder = new RecordingDecoder();
    decoder.prepareStream(true);
    decoder.prepareStream(false);
    decoder.resetStream();
    decoder.playStream(context, Long.MIN_VALUE, Long.MAX_VALUE);
    check(decoder.context == context && decoder.startPosition == Long.MIN_VALUE
        && decoder.desiredPosition == Long.MAX_VALUE, "play argument identity");
    decoder.playStream(null, -1L, 0L);
    check(decoder.context == null && decoder.startPosition == -1L
        && decoder.desiredPosition == 0L, "null play context");
    decoder.close();
    check(decoder.events.toString().equals("prepare:true;prepare:false;reset;play;play;close;"),
        "ordered dispatch");
  }

  private static void failureContract() throws Exception {
    IOException prepareFailure = new IOException("prepare-sentinel");
    RecordingDecoder prepareDecoder = new RecordingDecoder();
    prepareDecoder.failure = prepareFailure;
    check(capture(() -> prepareDecoder.prepareStream(true)) == prepareFailure,
        "prepare IOException identity");

    IOException resetFailure = new IOException("reset-sentinel");
    RecordingDecoder resetDecoder = new RecordingDecoder();
    resetDecoder.failure = resetFailure;
    check(capture(resetDecoder::resetStream) == resetFailure, "reset IOException identity");

    InterruptedException interrupted = new InterruptedException("play-interrupted");
    RecordingDecoder interruptedDecoder = new RecordingDecoder();
    interruptedDecoder.failure = interrupted;
    check(capture(() -> interruptedDecoder.playStream(null, 0L, 0L)) == interrupted,
        "play interruption identity");

    IOException playFailure = new IOException("play-io-sentinel");
    RecordingDecoder ioDecoder = new RecordingDecoder();
    ioDecoder.failure = playFailure;
    check(capture(() -> ioDecoder.playStream(null, 0L, 0L)) == playFailure,
        "play IOException identity");

    Exception closeFailure = new Exception("close-sentinel");
    RecordingDecoder closeDecoder = new RecordingDecoder();
    closeDecoder.failure = closeFailure;
    check(capture(closeDecoder::close) == closeFailure, "inherited close failure identity");
  }

  private static void reflectionContract() throws Exception {
    Class<SoundCloudSegmentDecoder> type = SoundCloudSegmentDecoder.class;
    check(type.isInterface() && Modifier.isPublic(type.getModifiers())
        && Modifier.isAbstract(type.getModifiers()) && !Modifier.isFinal(type.getModifiers())
        && type.getSuperclass() == null
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {AutoCloseable.class})
        && type.getTypeParameters().length == 0 && type.getAnnotations().length == 0,
        "interface metadata");
    check(type.getDeclaredFields().length == 0 && type.getDeclaredConstructors().length == 0
        && type.getDeclaredMethods().length == 3 && type.getMethods().length == 4,
        "member counts");
    checkMethod(type, "prepareStream", new Class<?>[] {boolean.class},
        new Class<?>[] {IOException.class});
    checkMethod(type, "resetStream", new Class<?>[0], new Class<?>[] {IOException.class});
    checkMethod(type, "playStream",
        new Class<?>[] {AudioProcessingContext.class, long.class, long.class},
        new Class<?>[] {InterruptedException.class, IOException.class});
    Method close = type.getMethod("close");
    check(close.getDeclaringClass() == AutoCloseable.class
        && Arrays.equals(close.getExceptionTypes(), new Class<?>[] {Exception.class}),
        "inherited close metadata");
  }

  private static void checkMethod(Class<?> owner, String name, Class<?>[] parameters,
                                  Class<?>[] exceptions) throws Exception {
    Method method = owner.getDeclaredMethod(name, parameters);
    check(method.getReturnType() == void.class
        && method.getModifiers() == (Modifier.PUBLIC | Modifier.ABSTRACT)
        && Arrays.equals(method.getParameterTypes(), parameters)
        && Arrays.equals(method.getExceptionTypes(), exceptions)
        && method.getTypeParameters().length == 0 && !method.isDefault()
        && !method.isBridge() && !method.isSynthetic() && !method.isVarArgs(),
        name + " metadata");
  }

  private static Throwable capture(Operation operation) {
    try {
      operation.run();
      throw new AssertionError("expected failure");
    } catch (Throwable error) {
      return error;
    }
  }

  private static <T> T proxy(Class<T> type) {
    return type.cast(Proxy.newProxyInstance(
        GateSoundCloudSegmentDecoder.class.getClassLoader(), new Class<?>[] {type},
        (proxy, method, args) -> {
          if (method.getReturnType() == boolean.class) return false;
          if (method.getReturnType() == int.class) return 0;
          if (method.getReturnType() == long.class) return 0L;
          return null;
        }));
  }

  private static final class RecordingDecoder implements SoundCloudSegmentDecoder {
    private final StringBuilder events = new StringBuilder();
    private Throwable failure;
    private AudioProcessingContext context;
    private long startPosition;
    private long desiredPosition;

    @Override
    public void prepareStream(boolean beginning) throws IOException {
      if (failure != null) throw (IOException) failure;
      events.append("prepare:").append(beginning).append(';');
    }

    @Override
    public void resetStream() throws IOException {
      if (failure != null) throw (IOException) failure;
      events.append("reset;");
    }

    @Override
    public void playStream(AudioProcessingContext actualContext, long actualStartPosition,
                           long actualDesiredPosition) throws InterruptedException, IOException {
      if (failure instanceof InterruptedException) throw (InterruptedException) failure;
      if (failure != null) throw (IOException) failure;
      context = actualContext;
      startPosition = actualStartPosition;
      desiredPosition = actualDesiredPosition;
      events.append("play;");
    }

    @Override
    public void close() throws Exception {
      if (failure != null) throw (Exception) failure;
      events.append("close;");
    }
  }

  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const SOUND_CLOUD_SEGMENT_DECODER_FACTORY_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudSegmentDecoder;
import com.sedmelluq.discord.lavaplayer.tools.io.SeekableInputStream;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Proxy;
import java.util.Arrays;
import java.util.function.Supplier;

public final class GateSoundCloudSegmentDecoderFactory {
  public static void main(String[] args) throws Exception {
    dispatchContract();
    failureContract();
    reflectionContract();
    System.out.println(
        "public-static-abstract-interface,0-fields,0-constructors,1-method;"
        + "dispatch,argument-identity,return-identity,nulls,unchecked,no-supplier-invocation,"
        + "generic-supplier,reflection");
  }

  private static void dispatchContract() {
    int[] supplierCalls = new int[1];
    Supplier<SeekableInputStream> supplier = () -> {
      supplierCalls[0]++;
      return null;
    };
    SoundCloudSegmentDecoder decoder = proxy(SoundCloudSegmentDecoder.class);
    Object[] observed = new Object[1];
    int[] factoryCalls = new int[1];
    SoundCloudSegmentDecoder.Factory factory = actualSupplier -> {
      observed[0] = actualSupplier;
      factoryCalls[0]++;
      return decoder;
    };
    check(factory.create(supplier) == decoder && observed[0] == supplier
        && factoryCalls[0] == 1 && supplierCalls[0] == 0,
        "ordinary dispatch identity");
    check(factory.create(null) == decoder && observed[0] == null
        && factoryCalls[0] == 2 && supplierCalls[0] == 0,
        "null argument identity");

    SoundCloudSegmentDecoder.Factory nullFactory = actualSupplier -> null;
    check(nullFactory.create(supplier) == null && supplierCalls[0] == 0, "null return");
  }

  private static void failureContract() {
    RuntimeException failure = new RuntimeException("factory-sentinel");
    SoundCloudSegmentDecoder.Factory factory = supplier -> { throw failure; };
    try {
      factory.create(null);
      throw new AssertionError("expected failure");
    } catch (RuntimeException error) {
      check(error == failure, "unchecked failure identity");
    }
  }

  private static void reflectionContract() throws Exception {
    Class<SoundCloudSegmentDecoder.Factory> type = SoundCloudSegmentDecoder.Factory.class;
    int modifiers = type.getModifiers();
    check(type.isInterface() && modifiers == (Modifier.PUBLIC | Modifier.STATIC
            | Modifier.INTERFACE | Modifier.ABSTRACT)
        && type.getSuperclass() == null && type.getInterfaces().length == 0
        && type.getTypeParameters().length == 0 && type.getAnnotations().length == 0,
        "interface metadata");
    check(type.isMemberClass() && type.getDeclaringClass() == SoundCloudSegmentDecoder.class
        && type.getEnclosingClass() == SoundCloudSegmentDecoder.class
        && type.getSimpleName().equals("Factory"), "nest metadata");
    check(type.getDeclaredFields().length == 0 && type.getDeclaredConstructors().length == 0
        && type.getDeclaredMethods().length == 1 && type.getDeclaredClasses().length == 0,
        "member counts");
    Method method = type.getDeclaredMethod("create", Supplier.class);
    check(method.getReturnType() == SoundCloudSegmentDecoder.class
        && method.getGenericReturnType() == SoundCloudSegmentDecoder.class
        && method.getModifiers() == (Modifier.PUBLIC | Modifier.ABSTRACT)
        && Arrays.equals(method.getParameterTypes(), new Class<?>[] {Supplier.class})
        && method.getExceptionTypes().length == 0 && method.getTypeParameters().length == 0
        && method.getGenericParameterTypes()[0] instanceof ParameterizedType
        && !method.isDefault() && !method.isBridge() && !method.isSynthetic()
        && !method.isVarArgs(), "method metadata");
    ParameterizedType supplierType =
        (ParameterizedType) method.getGenericParameterTypes()[0];
    check(supplierType.getRawType() == Supplier.class
        && Arrays.equals(supplierType.getActualTypeArguments(),
            new Object[] {SeekableInputStream.class}), "generic supplier metadata");
  }

  private static <T> T proxy(Class<T> type) {
    return type.cast(Proxy.newProxyInstance(
        GateSoundCloudSegmentDecoderFactory.class.getClassLoader(), new Class<?>[] {type},
        (proxy, method, args) -> {
          if (method.getReturnType() == boolean.class) return false;
          if (method.getReturnType() == int.class) return 0;
          if (method.getReturnType() == long.class) return 0L;
          return null;
        }));
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const SOUND_CLOUD_TRACK_FORMAT_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudTrackFormat;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;

public final class GateSoundCloudTrackFormat {
  private static final String[] METHODS = {
      "getTrackId", "getProtocol", "getMimeType", "getLookupUrl"
  };

  public static void main(String[] args) throws Exception {
    dispatchContract();
    nullContract();
    failureContract();
    reflectionContract();
    System.out.println(
        "public-abstract-interface,0-fields,0-constructors,4-methods;"
        + "ordered-dispatch,return-identity,nulls,unchecked,reflection");
  }

  private static void dispatchContract() {
    String trackId = new String("track-id");
    String protocol = new String("hls");
    String mimeType = new String("audio/mpeg");
    String lookupUrl = new String("https://media/fixture");
    RecordingHandler handler = new RecordingHandler(
        new Object[] {trackId, protocol, mimeType, lookupUrl}, null);
    SoundCloudTrackFormat format = handler.proxy();
    check(format.getTrackId() == trackId, "track ID identity");
    check(format.getProtocol() == protocol, "protocol identity");
    check(format.getMimeType() == mimeType, "MIME type identity");
    check(format.getLookupUrl() == lookupUrl, "lookup URL identity");
    check(handler.events.toString().equals(
        "getTrackId;getProtocol;getMimeType;getLookupUrl;"), "ordered dispatch");
  }

  private static void nullContract() {
    RecordingHandler handler = new RecordingHandler(new Object[4], null);
    SoundCloudTrackFormat format = handler.proxy();
    check(format.getTrackId() == null && format.getProtocol() == null
        && format.getMimeType() == null && format.getLookupUrl() == null,
        "null returns");
    check(handler.events.toString().equals(
        "getTrackId;getProtocol;getMimeType;getLookupUrl;"), "null dispatch order");
  }

  private static void failureContract() {
    for (int index = 0; index < METHODS.length; index++) {
      RuntimeException failure = new RuntimeException(METHODS[index] + "-sentinel");
      SoundCloudTrackFormat format = new RecordingHandler(new Object[4], failure).proxy();
      final int methodIndex = index;
      check(capture(() -> invoke(format, methodIndex)) == failure,
          METHODS[index] + " failure identity");
    }
  }

  private static void reflectionContract() throws Exception {
    Class<SoundCloudTrackFormat> type = SoundCloudTrackFormat.class;
    check(type.isInterface() && Modifier.isPublic(type.getModifiers())
        && Modifier.isAbstract(type.getModifiers()) && !Modifier.isFinal(type.getModifiers())
        && type.getSuperclass() == null && type.getInterfaces().length == 0
        && type.getTypeParameters().length == 0 && type.getAnnotations().length == 0,
        "interface metadata");
    check(type.getDeclaredFields().length == 0 && type.getDeclaredConstructors().length == 0
        && type.getDeclaredMethods().length == 4 && type.getDeclaredClasses().length == 0,
        "member counts");
    for (String name : METHODS) {
      Method method = type.getDeclaredMethod(name);
      check(method.getReturnType() == String.class
          && method.getGenericReturnType() == String.class
          && method.getModifiers() == (Modifier.PUBLIC | Modifier.ABSTRACT)
          && method.getParameterTypes().length == 0 && method.getExceptionTypes().length == 0
          && method.getTypeParameters().length == 0 && !method.isDefault()
          && !method.isBridge() && !method.isSynthetic() && !method.isVarArgs(),
          name + " metadata");
    }
  }

  private static String invoke(SoundCloudTrackFormat format, int index) {
    switch (index) {
      case 0: return format.getTrackId();
      case 1: return format.getProtocol();
      case 2: return format.getMimeType();
      case 3: return format.getLookupUrl();
      default: throw new AssertionError("invalid method index");
    }
  }

  private static Throwable capture(Operation operation) {
    try {
      operation.run();
      throw new AssertionError("expected failure");
    } catch (Throwable error) {
      return error;
    }
  }

  private static final class RecordingHandler implements InvocationHandler {
    private final Object[] returns;
    private final RuntimeException failure;
    private final StringBuilder events = new StringBuilder();

    RecordingHandler(Object[] values, RuntimeException valueFailure) {
      returns = values;
      failure = valueFailure;
    }

    SoundCloudTrackFormat proxy() {
      return (SoundCloudTrackFormat) Proxy.newProxyInstance(
          SoundCloudTrackFormat.class.getClassLoader(),
          new Class<?>[] {SoundCloudTrackFormat.class}, this);
    }

    @Override
    public Object invoke(Object proxy, Method method, Object[] arguments) {
      if (failure != null) throw failure;
      events.append(method.getName()).append(';');
      for (int index = 0; index < METHODS.length; index++) {
        if (METHODS[index].equals(method.getName())) return returns[index];
      }
      throw new AssertionError("unexpected method: " + method);
    }
  }

  private interface Operation { void run(); }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const SOUND_CLOUD_DATA_READER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudDataReader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudTrackFormat;
import com.sedmelluq.discord.lavaplayer.tools.JsonBrowser;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Proxy;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;

public final class GateSoundCloudDataReader {
  public static void main(String[] args) throws Exception {
    dispatchContract();
    nullContract();
    failureContract();
    reflectionContract();
    System.out.println(
        "public-abstract-interface,0-fields,0-constructors,9-methods;"
        + "dispatch,argument-identity,return-identity,boolean,nulls,unchecked,"
        + "generic-signatures,reflection");
  }

  private static void dispatchContract() throws Exception {
    JsonBrowser track = JsonBrowser.parse("{\"kind\":\"track\"}");
    JsonBrowser foundTrack = JsonBrowser.parse("{\"id\":1}");
    JsonBrowser playlist = JsonBrowser.parse("{\"kind\":\"playlist\"}");
    String identifier = new String("identifier");
    String trackId = new String("track-id");
    String playlistName = new String("playlist-name");
    String playlistId = new String("playlist-id");
    AudioTrackInfo info = new AudioTrackInfo("title", "author", 1L, "id", false, "uri");
    List<SoundCloudTrackFormat> formats = Collections.singletonList(proxy(SoundCloudTrackFormat.class));
    List<JsonBrowser> tracks = Arrays.asList(track, foundTrack);
    RecordingReader state = new RecordingReader(
        foundTrack, trackId, true, info, formats, playlist, playlistName, playlistId, tracks);
    SoundCloudDataReader reader = state.proxy();

    check(reader.findTrackData(track) == foundTrack, "findTrackData return identity");
    state.checkCall("findTrackData", track);
    check(reader.readTrackId(track) == trackId, "readTrackId return identity");
    state.checkCall("readTrackId", track);
    check(reader.isTrackBlocked(track), "isTrackBlocked value");
    state.checkCall("isTrackBlocked", track);
    check(reader.readTrackInfo(track, identifier) == info, "readTrackInfo return identity");
    state.checkCall("readTrackInfo", track, identifier);
    check(reader.readTrackFormats(track) == formats, "readTrackFormats return identity");
    state.checkCall("readTrackFormats", track);
    check(reader.findPlaylistData(track, identifier) == playlist,
        "findPlaylistData return identity");
    state.checkCall("findPlaylistData", track, identifier);
    check(reader.readPlaylistName(playlist) == playlistName,
        "readPlaylistName return identity");
    state.checkCall("readPlaylistName", playlist);
    check(reader.readPlaylistIdentifier(playlist) == playlistId,
        "readPlaylistIdentifier return identity");
    state.checkCall("readPlaylistIdentifier", playlist);
    check(reader.readPlaylistTracks(playlist) == tracks,
        "readPlaylistTracks return identity");
    state.checkCall("readPlaylistTracks", playlist);
    check(state.calls == 9, "dispatch count");
  }

  private static void nullContract() {
    RecordingReader state = new RecordingReader(
        null, null, false, null, null, null, null, null, null);
    SoundCloudDataReader reader = state.proxy();
    check(reader.findTrackData(null) == null, "null track data");
    check(reader.readTrackId(null) == null, "null track ID");
    check(!reader.isTrackBlocked(null), "false blocked value");
    check(reader.readTrackInfo(null, null) == null, "null track info");
    check(reader.readTrackFormats(null) == null, "null formats");
    check(reader.findPlaylistData(null, null) == null, "null playlist data");
    check(reader.readPlaylistName(null) == null, "null playlist name");
    check(reader.readPlaylistIdentifier(null) == null, "null playlist ID");
    check(reader.readPlaylistTracks(null) == null, "null playlist tracks");
    check(state.calls == 9, "null dispatch count");
  }

  private static void failureContract() {
    RuntimeException failure = new RuntimeException("reader-sentinel");
    SoundCloudDataReader reader = (SoundCloudDataReader) Proxy.newProxyInstance(
        SoundCloudDataReader.class.getClassLoader(), new Class<?>[] {SoundCloudDataReader.class},
        (proxy, method, args) -> { throw failure; });
    try {
      reader.readTrackId(null);
      throw new AssertionError("expected failure");
    } catch (RuntimeException error) {
      check(error == failure, "unchecked failure identity");
    }
  }

  private static void reflectionContract() throws Exception {
    Class<SoundCloudDataReader> type = SoundCloudDataReader.class;
    check(type.isInterface() && Modifier.isPublic(type.getModifiers())
        && Modifier.isAbstract(type.getModifiers()) && !Modifier.isFinal(type.getModifiers())
        && type.getSuperclass() == null && type.getInterfaces().length == 0
        && type.getTypeParameters().length == 0 && type.getAnnotations().length == 0,
        "interface metadata");
    check(type.getDeclaredFields().length == 0 && type.getDeclaredConstructors().length == 0
        && type.getDeclaredMethods().length == 9, "member counts");
    checkMethod(type, "findTrackData", JsonBrowser.class,
        new Class<?>[] {JsonBrowser.class}, null);
    checkMethod(type, "readTrackId", String.class,
        new Class<?>[] {JsonBrowser.class}, null);
    checkMethod(type, "isTrackBlocked", boolean.class,
        new Class<?>[] {JsonBrowser.class}, null);
    checkMethod(type, "readTrackInfo", AudioTrackInfo.class,
        new Class<?>[] {JsonBrowser.class, String.class}, null);
    checkMethod(type, "readTrackFormats", List.class,
        new Class<?>[] {JsonBrowser.class}, SoundCloudTrackFormat.class);
    checkMethod(type, "findPlaylistData", JsonBrowser.class,
        new Class<?>[] {JsonBrowser.class, String.class}, null);
    checkMethod(type, "readPlaylistName", String.class,
        new Class<?>[] {JsonBrowser.class}, null);
    checkMethod(type, "readPlaylistIdentifier", String.class,
        new Class<?>[] {JsonBrowser.class}, null);
    checkMethod(type, "readPlaylistTracks", List.class,
        new Class<?>[] {JsonBrowser.class}, JsonBrowser.class);
  }

  private static void checkMethod(Class<?> owner, String name, Class<?> returnType,
                                  Class<?>[] parameters, Class<?> genericElement) throws Exception {
    Method method = owner.getDeclaredMethod(name, parameters);
    check(method.getReturnType() == returnType
        && method.getModifiers() == (Modifier.PUBLIC | Modifier.ABSTRACT)
        && Arrays.equals(method.getParameterTypes(), parameters)
        && method.getExceptionTypes().length == 0 && method.getTypeParameters().length == 0
        && !method.isDefault() && !method.isBridge() && !method.isSynthetic()
        && !method.isVarArgs(), name + " metadata");
    if (genericElement == null) {
      check(method.getGenericReturnType() == returnType, name + " raw return metadata");
    } else {
      check(method.getGenericReturnType() instanceof ParameterizedType,
          name + " parameterized return");
      ParameterizedType genericReturn = (ParameterizedType) method.getGenericReturnType();
      check(genericReturn.getRawType() == List.class
          && Arrays.equals(genericReturn.getActualTypeArguments(), new Object[] {genericElement}),
          name + " generic return metadata");
    }
  }

  private static final class RecordingReader implements InvocationHandler {
    private final Object[] returns;
    private String methodName;
    private Object[] arguments;
    private int calls;

    RecordingReader(JsonBrowser foundTrack, String trackId, boolean blocked, AudioTrackInfo info,
                    List<SoundCloudTrackFormat> formats, JsonBrowser playlist,
                    String playlistName, String playlistId, List<JsonBrowser> tracks) {
      returns = new Object[] {
          foundTrack, trackId, blocked, info, formats, playlist, playlistName, playlistId, tracks
      };
    }

    SoundCloudDataReader proxy() {
      return (SoundCloudDataReader) Proxy.newProxyInstance(
          SoundCloudDataReader.class.getClassLoader(), new Class<?>[] {SoundCloudDataReader.class},
          this);
    }

    @Override
    public Object invoke(Object proxy, Method method, Object[] args) {
      methodName = method.getName();
      arguments = args;
      calls++;
      switch (methodName) {
        case "findTrackData": return returns[0];
        case "readTrackId": return returns[1];
        case "isTrackBlocked": return returns[2];
        case "readTrackInfo": return returns[3];
        case "readTrackFormats": return returns[4];
        case "findPlaylistData": return returns[5];
        case "readPlaylistName": return returns[6];
        case "readPlaylistIdentifier": return returns[7];
        case "readPlaylistTracks": return returns[8];
        default: throw new AssertionError("unexpected method: " + method);
      }
    }

    void checkCall(String expectedMethod, Object... expectedArguments) {
      check(expectedMethod.equals(methodName) && arguments.length == expectedArguments.length,
          expectedMethod + " dispatch");
      for (int index = 0; index < arguments.length; index++) {
        check(arguments[index] == expectedArguments[index], expectedMethod + " argument identity");
      }
    }
  }

  private static <T> T proxy(Class<T> type) {
    return type.cast(Proxy.newProxyInstance(
        GateSoundCloudDataReader.class.getClassLoader(), new Class<?>[] {type},
        (proxy, method, args) -> null));
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const SOUND_CLOUD_FORMAT_HANDLER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudFormatHandler;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudM3uInfo;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudTrackFormat;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Proxy;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;

public final class GateSoundCloudFormatHandler {
  public static void main(String[] args) throws Exception {
    dispatchContract();
    nullContract();
    failureContract();
    reflectionContract();
    System.out.println(
        "public-abstract-interface,0-fields,0-constructors,4-methods;"
        + "dispatch,argument-identity,return-identity,nulls,unchecked,"
        + "generic-list-parameter,reflection");
  }

  private static void dispatchContract() {
    SoundCloudTrackFormat format = proxy(SoundCloudTrackFormat.class);
    List<SoundCloudTrackFormat> formats = Collections.singletonList(format);
    String formatIdentifier = new String("O:format");
    String m3uIdentifier = new String("O:m3u");
    String mp3Identifier = new String("M:mp3");
    String mp3Url = new String("https://media/mp3");
    SoundCloudM3uInfo m3uInfo = new SoundCloudM3uInfo("https://media/m3u", null);
    RecordingHandler state = new RecordingHandler(format, formatIdentifier, m3uInfo, mp3Url);
    SoundCloudFormatHandler handler = state.proxy();

    check(handler.chooseBestFormat(formats) == format, "choose return identity");
    state.checkCall("chooseBestFormat", formats);
    check(handler.buildFormatIdentifier(format) == formatIdentifier,
        "identifier return identity");
    state.checkCall("buildFormatIdentifier", format);
    check(handler.getM3uInfo(m3uIdentifier) == m3uInfo, "M3U return identity");
    state.checkCall("getM3uInfo", m3uIdentifier);
    check(handler.getMp3LookupUrl(mp3Identifier) == mp3Url, "MP3 return identity");
    state.checkCall("getMp3LookupUrl", mp3Identifier);
    check(state.calls == 4, "dispatch count");
  }

  private static void nullContract() {
    RecordingHandler state = new RecordingHandler(null, null, null, null);
    SoundCloudFormatHandler handler = state.proxy();
    check(handler.chooseBestFormat(null) == null, "null format list");
    state.checkCall("chooseBestFormat", (Object) null);
    check(handler.buildFormatIdentifier(null) == null, "null format");
    state.checkCall("buildFormatIdentifier", (Object) null);
    check(handler.getM3uInfo(null) == null, "null M3U identifier");
    state.checkCall("getM3uInfo", (Object) null);
    check(handler.getMp3LookupUrl(null) == null, "null MP3 identifier");
    state.checkCall("getMp3LookupUrl", (Object) null);
    check(state.calls == 4, "null dispatch count");
  }

  private static void failureContract() {
    RuntimeException failure = new RuntimeException("format-handler-sentinel");
    SoundCloudFormatHandler handler = (SoundCloudFormatHandler) Proxy.newProxyInstance(
        SoundCloudFormatHandler.class.getClassLoader(),
        new Class<?>[] {SoundCloudFormatHandler.class},
        (proxy, method, args) -> { throw failure; });
    try {
      handler.getMp3LookupUrl(null);
      throw new AssertionError("expected failure");
    } catch (RuntimeException error) {
      check(error == failure, "unchecked failure identity");
    }
  }

  private static void reflectionContract() throws Exception {
    Class<SoundCloudFormatHandler> type = SoundCloudFormatHandler.class;
    check(type.isInterface() && Modifier.isPublic(type.getModifiers())
        && Modifier.isAbstract(type.getModifiers()) && !Modifier.isFinal(type.getModifiers())
        && type.getSuperclass() == null && type.getInterfaces().length == 0
        && type.getTypeParameters().length == 0 && type.getAnnotations().length == 0,
        "interface metadata");
    check(type.getDeclaredFields().length == 0 && type.getDeclaredConstructors().length == 0
        && type.getDeclaredMethods().length == 4, "member counts");
    Method choose = checkMethod(type, "chooseBestFormat", SoundCloudTrackFormat.class,
        new Class<?>[] {List.class});
    check(choose.getGenericParameterTypes()[0] instanceof ParameterizedType,
        "choose parameterized list");
    ParameterizedType formatList = (ParameterizedType) choose.getGenericParameterTypes()[0];
    check(formatList.getRawType() == List.class
        && Arrays.equals(formatList.getActualTypeArguments(),
            new Object[] {SoundCloudTrackFormat.class}), "choose generic list parameter");
    checkMethod(type, "buildFormatIdentifier", String.class,
        new Class<?>[] {SoundCloudTrackFormat.class});
    checkMethod(type, "getM3uInfo", SoundCloudM3uInfo.class,
        new Class<?>[] {String.class});
    checkMethod(type, "getMp3LookupUrl", String.class,
        new Class<?>[] {String.class});
  }

  private static Method checkMethod(Class<?> owner, String name, Class<?> returnType,
                                    Class<?>[] parameters) throws Exception {
    Method method = owner.getDeclaredMethod(name, parameters);
    check(method.getReturnType() == returnType && method.getGenericReturnType() == returnType
        && method.getModifiers() == (Modifier.PUBLIC | Modifier.ABSTRACT)
        && Arrays.equals(method.getParameterTypes(), parameters)
        && method.getExceptionTypes().length == 0 && method.getTypeParameters().length == 0
        && !method.isDefault() && !method.isBridge() && !method.isSynthetic()
        && !method.isVarArgs(), name + " metadata");
    return method;
  }

  private static final class RecordingHandler implements InvocationHandler {
    private final Object[] returns;
    private String methodName;
    private Object[] arguments;
    private int calls;

    RecordingHandler(SoundCloudTrackFormat format, String identifier,
                     SoundCloudM3uInfo m3uInfo, String mp3Url) {
      returns = new Object[] {format, identifier, m3uInfo, mp3Url};
    }

    SoundCloudFormatHandler proxy() {
      return (SoundCloudFormatHandler) Proxy.newProxyInstance(
          SoundCloudFormatHandler.class.getClassLoader(),
          new Class<?>[] {SoundCloudFormatHandler.class}, this);
    }

    @Override
    public Object invoke(Object proxy, Method method, Object[] args) {
      methodName = method.getName();
      arguments = args;
      calls++;
      switch (methodName) {
        case "chooseBestFormat": return returns[0];
        case "buildFormatIdentifier": return returns[1];
        case "getM3uInfo": return returns[2];
        case "getMp3LookupUrl": return returns[3];
        default: throw new AssertionError("unexpected method: " + method);
      }
    }

    void checkCall(String expectedMethod, Object... expectedArguments) {
      check(expectedMethod.equals(methodName) && arguments.length == expectedArguments.length,
          expectedMethod + " dispatch");
      for (int index = 0; index < arguments.length; index++) {
        check(arguments[index] == expectedArguments[index], expectedMethod + " argument identity");
      }
    }
  }

  private static <T> T proxy(Class<T> type) {
    return type.cast(Proxy.newProxyInstance(
        GateSoundCloudFormatHandler.class.getClassLoader(), new Class<?>[] {type},
        (proxy, method, args) -> null));
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const SOUND_CLOUD_HELPER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudHelper;
import com.sedmelluq.discord.lavaplayer.tools.http.HttpContextFilter;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.track.AudioReference;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;
import org.apache.http.client.methods.HttpHead;
import org.apache.http.client.methods.HttpUriRequest;
import org.apache.http.client.protocol.HttpClientContext;

public final class GateSoundCloudHelper {
  private static final String PLAYBACK_DISABLED =
      "SoundCloud playback URL resolution is handled by Mantle's bounded native source.";
  private static final String MOBILE_DISABLED =
      "Legacy SoundCloud mobile redirects are unsupported.";
  private static final String SHORT_DISABLED =
      "SoundCloud short-link resolution requires Mantle's bounded native source.";

  public static void main(String[] args) throws Exception {
    check(args.length == 1 && (args[0].equals("reference") || args[0].equals("candidate")),
        "expected disposition");
    reflectionContract();
    commonContract();
    if (args[0].equals("reference")) {
      referenceServiceContract();
      System.out.println("common=public-concrete,0-fields,1-constructor,4-static-methods,"
          + "non-mobile,checked-io,reflection;"
          + "service=legacy-http-playback,mobile-get,short-head");
    } else {
      candidateServiceContract();
      System.out.println("common=public-concrete,0-fields,1-constructor,4-static-methods,"
          + "non-mobile,checked-io,reflection;"
          + "service=bounded-native-source,no-http,legacy-mobile-disabled,short-link-disabled");
    }
  }

  private static void reflectionContract() throws Exception {
    Class<SoundCloudHelper> type = SoundCloudHelper.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && type.getInterfaces().length == 0 && type.getAnnotations().length == 0,
        "class metadata");
    check(type.getDeclaredFields().length == 0 && type.getDeclaredMethods().length == 4,
        "member counts");
    Constructor<?> constructor = type.getDeclaredConstructor();
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC, "constructor metadata");
    checkMethod(type, "nonMobileUrl", String.class, new Class<?>[] {String.class});
    checkMethod(type, "loadPlaybackUrl", String.class,
        new Class<?>[] {HttpInterface.class, String.class}, IOException.class);
    checkMethod(type, "redirectMobileLink", AudioReference.class,
        new Class<?>[] {HttpInterface.class, AudioReference.class});
    checkMethod(type, "resolveShortTrackUrl", AudioReference.class,
        new Class<?>[] {HttpInterface.class, AudioReference.class});
  }

  private static void commonContract() throws Exception {
    check(new SoundCloudHelper().getClass() == SoundCloudHelper.class, "construction");
    check(SoundCloudHelper.nonMobileUrl("https://m.soundcloud.com/user/track?x=1")
        .equals("https://soundcloud.com/user/track?x=1"), "mobile normalization");
    String ordinary = new String("https://www.soundcloud.com/user/track");
    check(SoundCloudHelper.nonMobileUrl(ordinary) == ordinary, "ordinary identity");
    String insecure = new String("http://m.soundcloud.com/user/track");
    check(SoundCloudHelper.nonMobileUrl(insecure) == insecure, "scheme-sensitive identity");
    expect(NullPointerException.class, () -> SoundCloudHelper.nonMobileUrl(null));
  }

  private static void referenceServiceContract() throws Exception {
    RecordingHttpInterface playback = new RecordingHttpInterface();
    expect(IOException.class, () -> SoundCloudHelper.loadPlaybackUrl(
        playback, "https://media.example/playback"));
    playback.checkRequest("GET", "https://media.example/playback");

    RecordingHttpInterface mobile = new RecordingHttpInterface();
    expect(RuntimeException.class, () -> SoundCloudHelper.redirectMobileLink(
        mobile, new AudioReference("https://soundcloud.app.goo.gl/fixture", "container")));
    mobile.checkRequest("GET", "https://soundcloud.app.goo.gl/fixture");

    RecordingHttpInterface shortLink = new RecordingHttpInterface();
    expect(RuntimeException.class, () -> SoundCloudHelper.resolveShortTrackUrl(
        shortLink, new AudioReference("https://on.soundcloud.com/fixture", "container")));
    shortLink.checkRequest("HEAD", "https://on.soundcloud.com/fixture");
    check(shortLink.request instanceof HttpHead
        && ((HttpHead) shortLink.request).getConfig() != null
        && !((HttpHead) shortLink.request).getConfig().isRedirectsEnabled(),
        "short redirect policy");
  }

  private static void candidateServiceContract() throws Exception {
    RecordingHttpInterface playback = new RecordingHttpInterface();
    UnsupportedOperationException playbackFailure = expect(UnsupportedOperationException.class,
        () -> SoundCloudHelper.loadPlaybackUrl(playback, "https://media.example/playback"));
    check(PLAYBACK_DISABLED.equals(playbackFailure.getMessage()) && playback.executes == 0,
        "bounded playback policy");

    RecordingHttpInterface mobile = new RecordingHttpInterface();
    UnsupportedOperationException mobileFailure = expect(UnsupportedOperationException.class,
        () -> SoundCloudHelper.redirectMobileLink(
            mobile, new AudioReference("https://soundcloud.app.goo.gl/fixture", "container")));
    check(MOBILE_DISABLED.equals(mobileFailure.getMessage()) && mobile.executes == 0,
        "legacy mobile policy");

    RecordingHttpInterface shortLink = new RecordingHttpInterface();
    UnsupportedOperationException shortFailure = expect(UnsupportedOperationException.class,
        () -> SoundCloudHelper.resolveShortTrackUrl(
            shortLink, new AudioReference("https://on.soundcloud.com/fixture", "container")));
    check(SHORT_DISABLED.equals(shortFailure.getMessage()) && shortLink.executes == 0,
        "short-link policy");
  }

  private static void checkMethod(Class<?> owner, String name, Class<?> returnType,
                                  Class<?>[] parameters, Class<?>... exceptions) throws Exception {
    Method method = owner.getDeclaredMethod(name, parameters);
    check(method.getReturnType() == returnType && method.getGenericReturnType() == returnType
        && method.getModifiers() == (Modifier.PUBLIC | Modifier.STATIC)
        && Arrays.equals(method.getParameterTypes(), parameters)
        && Arrays.equals(method.getExceptionTypes(), exceptions)
        && method.getTypeParameters().length == 0 && !method.isBridge()
        && !method.isSynthetic() && !method.isVarArgs(), name + " metadata");
  }

  private static final class RecordingHttpInterface extends HttpInterface {
    private final IOException failure = new IOException("network-sentinel");
    private HttpUriRequest request;
    private int executes;

    RecordingHttpInterface() {
      super(null, HttpClientContext.create(), false, proxy(HttpContextFilter.class));
    }

    @Override
    public org.apache.http.client.methods.CloseableHttpResponse execute(HttpUriRequest request)
        throws IOException {
      this.request = request;
      executes++;
      throw failure;
    }

    void checkRequest(String method, String uri) {
      check(executes == 1 && request != null && method.equals(request.getMethod())
          && uri.equals(request.getURI().toString()), method + " request");
    }
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private static <T> T proxy(Class<T> type) {
    return type.cast(Proxy.newProxyInstance(
        GateSoundCloudHelper.class.getClassLoader(), new Class<?>[] {type},
        (proxy, method, args) -> {
          if (method.getReturnType() == boolean.class) return false;
          if (method.getReturnType() == int.class) return 0;
          return null;
        }));
  }

  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const SOUND_CLOUD_HTTP_CONTEXT_FILTER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudClientIdTracker;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudHttpContextFilter;
import com.sedmelluq.discord.lavaplayer.tools.http.HttpContextFilter;
import com.sedmelluq.discord.lavaplayer.tools.http.HttpContextRetryCounter;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterfaceManager;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.net.URI;
import java.util.Arrays;
import java.util.concurrent.atomic.AtomicInteger;
import org.apache.http.HttpResponse;
import org.apache.http.ProtocolVersion;
import org.apache.http.client.methods.HttpGet;
import org.apache.http.client.methods.HttpUriRequest;
import org.apache.http.client.protocol.HttpClientContext;
import org.apache.http.message.BasicHttpResponse;

public final class GateSoundCloudHttpContextFilter {
  private static final String USER_AGENT =
      "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) "
      + "Chrome/76.0.3809.100 Safari/537.36";
  private static final String INJECTION_DISABLED =
      "Legacy SoundCloud HTTP credential injection is unsupported; "
      + "use Mantle's bounded native source.";

  public static void main(String[] args) throws Exception {
    check(args.length == 1 && (args[0].equals("reference") || args[0].equals("candidate")),
        "expected disposition");
    reflectionContract();
    commonContract();
    if (args[0].equals("reference")) {
      referenceServiceContract();
      System.out.println("common=public-concrete,2-fields,1-constructor,5-callbacks,"
          + "no-op-lifecycle,false-exception,user-agent,retry-counter,cdn-pass-through,reflection;"
          + "service=legacy-global-client-id-injection,substring-cdn-bypass,401-refresh");
    } else {
      candidateServiceContract();
      System.out.println("common=public-concrete,2-fields,1-constructor,5-callbacks,"
          + "no-op-lifecycle,false-exception,user-agent,retry-counter,cdn-pass-through,reflection;"
          + "service=bounded-native-control-plane,strict-cdn-pass-through,"
          + "no-client-id-injection,no-refresh");
    }
  }

  private static void reflectionContract() throws Exception {
    Class<SoundCloudHttpContextFilter> type = SoundCloudHttpContextFilter.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {HttpContextFilter.class})
        && type.getAnnotations().length == 0, "class metadata");
    check(type.getDeclaredFields().length == 2 && type.getDeclaredMethods().length == 5,
        "member counts");
    checkField(type, "retryCounter", HttpContextRetryCounter.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "clientIdTracker", SoundCloudClientIdTracker.class,
        Modifier.PRIVATE | Modifier.FINAL);
    Constructor<?> constructor = type.getDeclaredConstructor(SoundCloudClientIdTracker.class);
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC, "constructor metadata");
    checkMethod(type, "onContextOpen", void.class,
        new Class<?>[] {HttpClientContext.class});
    checkMethod(type, "onContextClose", void.class,
        new Class<?>[] {HttpClientContext.class});
    checkMethod(type, "onRequest", void.class,
        new Class<?>[] {HttpClientContext.class, HttpUriRequest.class, boolean.class});
    checkMethod(type, "onRequestResponse", boolean.class,
        new Class<?>[] {HttpClientContext.class, HttpUriRequest.class, HttpResponse.class});
    checkMethod(type, "onRequestException", boolean.class,
        new Class<?>[] {HttpClientContext.class, HttpUriRequest.class, Throwable.class});
  }

  private static void commonContract() throws Exception {
    Fixture fixture = fixture();
    check(field("clientIdTracker").get(fixture.filter) == fixture.tracker,
        "constructor dependency identity");
    fixture.filter.onContextOpen(null);
    fixture.filter.onContextClose(null);
    check(!fixture.filter.onRequestException(null, null, new Throwable("fixture")),
        "exception callback");

    HttpClientContext rawContext = HttpClientContext.create();
    rawContext.setAttribute("sc-raw", true);
    HttpGet raw = new HttpGet("https://example.com/raw");
    fixture.filter.onRequest(rawContext, raw, false);
    check(USER_AGENT.equals(raw.getFirstHeader("user-agent").getValue())
        && raw.getURI().equals(URI.create("https://example.com/raw")), "raw context bypass");

    HttpClientContext cdnContext = HttpClientContext.create();
    HttpGet cdn = new HttpGet("https://cf-media.sndcdn.com/fixture?token=1");
    fixture.filter.onRequest(cdnContext, cdn, false);
    check(USER_AGENT.equals(cdn.getFirstHeader("user-agent").getValue())
        && cdn.getURI().equals(URI.create("https://cf-media.sndcdn.com/fixture?token=1")),
        "CDN pass-through");
    fixture.filter.onRequest(cdnContext, cdn, true);
    check(!fixture.filter.onRequestResponse(cdnContext, cdn, response(401)),
        "bounded retry count");
  }

  private static void referenceServiceContract() throws Exception {
    Fixture fixture = fixture();
    setTrackerState(fixture.tracker, "frozen-id", System.currentTimeMillis());

    HttpClientContext apiContext = HttpClientContext.create();
    HttpGet api = new HttpGet("https://api-v2.soundcloud.com/resolve?url=fixture&client_id=old");
    fixture.filter.onRequest(apiContext, api, false);
    check(api.getURI().toString().contains("client_id=frozen-id")
        && !api.getURI().toString().contains("client_id=old"), "legacy API injection");
    check(fixture.filter.onRequestResponse(apiContext, api, response(401)),
        "legacy 401 refresh");

    HttpGet offOrigin = new HttpGet("https://example.com/control?x=1");
    fixture.filter.onRequest(HttpClientContext.create(), offOrigin, false);
    check(offOrigin.getURI().toString().contains("client_id=frozen-id"),
        "legacy off-origin injection");

    HttpGet deceptive = new HttpGet("https://evil-sndcdn.com/fixture");
    fixture.filter.onRequest(HttpClientContext.create(), deceptive, false);
    check(deceptive.getURI().equals(URI.create("https://evil-sndcdn.com/fixture")),
        "legacy substring CDN bypass");
    check(fixture.acquisitions.get() == 0, "reference fixture stays offline");
  }

  private static void candidateServiceContract() throws Exception {
    AtomicInteger acquisitions = new AtomicInteger();
    CountingTracker tracker = new CountingTracker(manager(acquisitions));
    SoundCloudHttpContextFilter filter = new SoundCloudHttpContextFilter(tracker);

    HttpGet cdn = new HttpGet("https://cf-media.sndcdn.com/fixture");
    filter.onRequest(HttpClientContext.create(), cdn, false);
    check(cdn.getURI().equals(URI.create("https://cf-media.sndcdn.com/fixture")),
        "strict CDN pass-through");

    for (String uri : new String[] {
        "https://api-v2.soundcloud.com/resolve?url=fixture",
        "https://example.com/control?x=1",
        "https://evil-sndcdn.com/fixture",
        "http://cf-media.sndcdn.com/fixture",
        "https://user@cf-media.sndcdn.com/fixture",
        "https://cf-media.sndcdn.com:8443/fixture"
    }) {
      HttpGet request = new HttpGet(uri);
      UnsupportedOperationException error = expect(UnsupportedOperationException.class,
          () -> filter.onRequest(HttpClientContext.create(), request, false));
      check(INJECTION_DISABLED.equals(error.getMessage())
          && request.getURI().equals(URI.create(uri))
          && (request.getURI().getQuery() == null
              || !request.getURI().getQuery().contains("client_id")),
          "bounded origin policy " + uri);
    }
    check(!filter.onRequestResponse(
        HttpClientContext.create(), new HttpGet("https://api-v2.soundcloud.com/resolve"),
        response(401)), "candidate never refreshes on 401");
    check(tracker.reads == 0 && tracker.updates == 0 && acquisitions.get() == 0,
        "candidate never accesses credentials or HTTP");
  }

  private static Fixture fixture() {
    AtomicInteger acquisitions = new AtomicInteger();
    SoundCloudClientIdTracker tracker = new SoundCloudClientIdTracker(manager(acquisitions));
    return new Fixture(tracker, new SoundCloudHttpContextFilter(tracker), acquisitions);
  }

  private static HttpInterfaceManager manager(AtomicInteger acquisitions) {
    return (HttpInterfaceManager) java.lang.reflect.Proxy.newProxyInstance(
        GateSoundCloudHttpContextFilter.class.getClassLoader(),
        new Class<?>[] {HttpInterfaceManager.class}, (proxy, method, args) -> {
          if (method.getName().equals("getInterface")) acquisitions.incrementAndGet();
          if (method.getName().equals("toString")) return "manager-proxy";
          if (method.getReturnType() == boolean.class) return false;
          if (method.getReturnType() == int.class) return 0;
          if (method.getReturnType() == long.class) return 0L;
          return null;
        });
  }

  private static HttpResponse response(int status) {
    return new BasicHttpResponse(new ProtocolVersion("HTTP", 1, 1), status, "fixture");
  }

  private static void setTrackerState(SoundCloudClientIdTracker tracker, String clientId,
                                      long updateTime) throws Exception {
    field("clientId").set(tracker, clientId);
    field("lastClientIdUpdate").setLong(tracker, updateTime);
  }

  private static void checkMethod(Class<?> owner, String name, Class<?> returnType,
                                  Class<?>[] parameters) throws Exception {
    Method method = owner.getDeclaredMethod(name, parameters);
    check(method.getReturnType() == returnType && method.getGenericReturnType() == returnType
        && method.getModifiers() == Modifier.PUBLIC
        && Arrays.equals(method.getParameterTypes(), parameters)
        && method.getExceptionTypes().length == 0 && method.getTypeParameters().length == 0
        && !method.isBridge() && !method.isSynthetic() && !method.isVarArgs(), name + " metadata");
  }

  private static void checkField(Class<?> owner, String name, Class<?> type, int modifiers)
      throws Exception {
    Field field = owner.getDeclaredField(name);
    check(field.getType() == type && field.getGenericType() == type
        && field.getModifiers() == modifiers && !field.isSynthetic(), name + " metadata");
  }

  private static Field field(String name) throws Exception {
    Class<?> owner = name.equals("clientIdTracker") || name.equals("retryCounter")
        ? SoundCloudHttpContextFilter.class : SoundCloudClientIdTracker.class;
    Field field = owner.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private static final class Fixture {
    final SoundCloudClientIdTracker tracker;
    final SoundCloudHttpContextFilter filter;
    final AtomicInteger acquisitions;

    Fixture(SoundCloudClientIdTracker tracker, SoundCloudHttpContextFilter filter,
            AtomicInteger acquisitions) {
      this.tracker = tracker;
      this.filter = filter;
      this.acquisitions = acquisitions;
    }
  }

  private static final class CountingTracker extends SoundCloudClientIdTracker {
    int reads;
    int updates;

    CountingTracker(HttpInterfaceManager manager) {
      super(manager);
    }

    @Override
    public String getClientId() {
      reads++;
      return "unexpected-read";
    }

    @Override
    public void updateClientId() {
      updates++;
    }
  }

  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const SOUND_CLOUD_M3U_AUDIO_TRACK_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudM3uAudioTrack;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudM3uInfo;
import com.sedmelluq.discord.lavaplayer.tools.http.HttpContextFilter;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import com.sedmelluq.discord.lavaplayer.track.DelegatedAudioTrack;
import com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;
import org.apache.http.client.methods.HttpUriRequest;
import org.apache.http.client.protocol.HttpClientContext;

public final class GateSoundCloudM3uAudioTrack {
  private static final String HLS_DISABLED =
      "Legacy SoundCloud HLS segment playback is unsupported; "
      + "use Mantle's bounded progressive native source.";

  public static void main(String[] args) throws Exception {
    check(args.length == 1 && (args[0].equals("reference") || args[0].equals("candidate")),
        "expected disposition");
    reflectionContract();
    commonContract();
    if (args[0].equals("reference")) {
      referenceServiceContract();
      System.out.println("common=public-concrete,4-fields,1-constructor,1-exported-method,"
          + "capture,static-state,checked-exception,reflection;"
          + "service=legacy-hls-playback-get");
    } else {
      candidateServiceContract();
      System.out.println("common=public-concrete,4-fields,1-constructor,1-exported-method,"
          + "capture,static-state,checked-exception,reflection;"
          + "service=bounded-progressive-only,no-http,hls-explicitly-unsupported");
    }
  }

  private static void reflectionContract() throws Exception {
    Class<SoundCloudM3uAudioTrack> type = SoundCloudM3uAudioTrack.class;
    check(type.getModifiers() == Modifier.PUBLIC
        && type.getSuperclass() == DelegatedAudioTrack.class
        && type.getInterfaces().length == 0 && type.getAnnotations().length == 0,
        "class metadata");
    check(type.getDeclaredFields().length == 4 && type.getDeclaredMethods().length == 9,
        "private shell counts");
    checkField(type, "log", Class.forName("org.slf4j.Logger"),
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "SEGMENT_UPDATE_INTERVAL", long.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "httpInterface", HttpInterface.class, Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "m3uInfo", SoundCloudM3uInfo.class, Modifier.PRIVATE | Modifier.FINAL);

    Constructor<?> constructor = type.getDeclaredConstructor(
        AudioTrackInfo.class, HttpInterface.class, SoundCloudM3uInfo.class);
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC, "constructor metadata");
    Method process = type.getDeclaredMethod("process", LocalAudioTrackExecutor.class);
    check(process.getModifiers() == Modifier.PUBLIC && process.getReturnType() == void.class
        && Arrays.equals(process.getExceptionTypes(), new Class<?>[] {Exception.class})
        && !process.isBridge() && !process.isSynthetic() && !process.isVarArgs(),
        "process metadata");
    long exported = Arrays.stream(type.getDeclaredMethods())
        .filter(method -> Modifier.isPublic(method.getModifiers())
            || Modifier.isProtected(method.getModifiers()))
        .count();
    check(exported == 1L, "exported method count");
  }

  private static void commonContract() throws Exception {
    AudioTrackInfo info = new AudioTrackInfo(
        "title", "author", 1234L, "O:fixture", false,
        "https://soundcloud.com/fixture/song", "art", null);
    RecordingHttpInterface http = new RecordingHttpInterface();
    SoundCloudM3uInfo m3uInfo = new SoundCloudM3uInfo(
        "https://api-v2.soundcloud.com/media/fixture", null);
    SoundCloudM3uAudioTrack track = new SoundCloudM3uAudioTrack(info, http, m3uInfo);
    check(track.getInfo() == info && field("httpInterface").get(track) == http
        && field("m3uInfo").get(track) == m3uInfo, "constructor capture");
    check(field("log").get(null) != null
        && field("SEGMENT_UPDATE_INTERVAL").getLong(null) == 600_000L,
        "static state");
  }

  private static void referenceServiceContract() throws Exception {
    RecordingHttpInterface http = new RecordingHttpInterface();
    SoundCloudM3uAudioTrack track = track(http);
    expect(IOException.class, () -> track.process(null));
    http.checkRequest("GET", "https://api-v2.soundcloud.com/media/fixture");
  }

  private static void candidateServiceContract() throws Exception {
    RecordingHttpInterface http = new RecordingHttpInterface();
    SoundCloudM3uAudioTrack track = track(http);
    UnsupportedOperationException error = expect(
        UnsupportedOperationException.class, () -> track.process(null));
    check(HLS_DISABLED.equals(error.getMessage()) && http.executes == 0,
        "bounded progressive-only policy");
  }

  private static SoundCloudM3uAudioTrack track(RecordingHttpInterface http) {
    AudioTrackInfo info = new AudioTrackInfo(
        "title", "author", 1234L, "O:fixture", false,
        "https://soundcloud.com/fixture/song", null, null);
    SoundCloudM3uInfo m3uInfo = new SoundCloudM3uInfo(
        "https://api-v2.soundcloud.com/media/fixture", null);
    return new SoundCloudM3uAudioTrack(info, http, m3uInfo);
  }

  private static final class RecordingHttpInterface extends HttpInterface {
    private HttpUriRequest request;
    private int executes;

    RecordingHttpInterface() {
      super(null, HttpClientContext.create(), false, proxy(HttpContextFilter.class));
    }

    @Override
    public org.apache.http.client.methods.CloseableHttpResponse execute(HttpUriRequest request)
        throws IOException {
      this.request = request;
      executes++;
      throw new IOException("network-sentinel");
    }

    void checkRequest(String method, String uri) {
      check(executes == 1 && request != null && method.equals(request.getMethod())
          && uri.equals(request.getURI().toString()), method + " request");
    }
  }

  private static Field field(String name) throws Exception {
    Field field = SoundCloudM3uAudioTrack.class.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }

  private static void checkField(Class<?> owner, String name, Class<?> type, int modifiers)
      throws Exception {
    Field field = owner.getDeclaredField(name);
    check(field.getType() == type && field.getGenericType() == type
        && field.getModifiers() == modifiers && !field.isSynthetic(), name + " metadata");
  }

  private static <T> T proxy(Class<T> type) {
    return type.cast(Proxy.newProxyInstance(
        GateSoundCloudM3uAudioTrack.class.getClassLoader(), new Class<?>[] {type},
        (proxy, method, args) -> {
          if (method.getReturnType() == boolean.class) return false;
          if (method.getReturnType() == int.class) return 0;
          return null;
        }));
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const SOUND_CLOUD_M3U_INFO_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudM3uInfo;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudSegmentDecoder;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;

public final class GateSoundCloudM3uInfo {
  public static void main(String[] args) throws Exception {
    reflectionContract();
    behaviorContract();
    System.out.println(
        "public-concrete,2-fields,1-constructor,0-methods;identity,nulls,reflection");
  }

  private static void reflectionContract() throws Exception {
    Class<SoundCloudM3uInfo> type = SoundCloudM3uInfo.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && type.getInterfaces().length == 0 && type.getAnnotations().length == 0,
        "class metadata");
    check(type.getDeclaredFields().length == 2 && type.getDeclaredMethods().length == 0,
        "member counts");
    checkField(type, "lookupUrl", String.class);
    checkField(type, "decoderFactory", SoundCloudSegmentDecoder.Factory.class);

    Constructor<?> constructor = type.getDeclaredConstructor(
        String.class, SoundCloudSegmentDecoder.Factory.class);
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC
        && constructor.getExceptionTypes().length == 0 && !constructor.isSynthetic()
        && !constructor.isVarArgs() && constructor.getAnnotations().length == 0,
        "constructor metadata");
  }

  private static void behaviorContract() {
    String lookupUrl = new String("https://media/fixture");
    SoundCloudSegmentDecoder.Factory decoderFactory = supplier -> null;
    SoundCloudM3uInfo info = new SoundCloudM3uInfo(lookupUrl, decoderFactory);
    check(info.lookupUrl == lookupUrl && info.decoderFactory == decoderFactory,
        "argument identity");

    SoundCloudM3uInfo nulls = new SoundCloudM3uInfo(null, null);
    check(nulls.lookupUrl == null && nulls.decoderFactory == null, "null capture");
  }

  private static void checkField(Class<?> owner, String name, Class<?> type) throws Exception {
    Field field = owner.getDeclaredField(name);
    check(field.getType() == type && field.getGenericType() == type
        && field.getModifiers() == (Modifier.PUBLIC | Modifier.FINAL)
        && !field.isSynthetic() && field.getAnnotations().length == 0,
        name + " metadata");
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const SOUND_CLOUD_MP3_SEGMENT_DECODER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudMp3SegmentDecoder;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudSegmentDecoder;
import com.sedmelluq.discord.lavaplayer.tools.io.SeekableInputStream;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioProcessingContext;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;
import java.util.function.Supplier;

public final class GateSoundCloudMp3SegmentDecoder {
  private static final String HLS_DISABLED =
      "Legacy SoundCloud MP3 HLS segment playback is unsupported; "
      + "use Mantle's bounded progressive native source.";

  public static void main(String[] args) throws Exception {
    check(args.length == 1 && (args[0].equals("reference") || args[0].equals("candidate")),
        "expected disposition");
    reflectionContract();
    commonContract();
    if (args[0].equals("reference")) {
      referenceServiceContract();
      System.out.println("common=public-concrete,1-field,1-constructor,4-methods,capture,"
          + "no-op-lifecycle,generic-supplier,checked-signatures,reflection;"
          + "service=legacy-mp3-segment-supplier");
    } else {
      candidateServiceContract();
      System.out.println("common=public-concrete,1-field,1-constructor,4-methods,capture,"
          + "no-op-lifecycle,generic-supplier,checked-signatures,reflection;"
          + "service=bounded-progressive-only,no-supplier,hls-explicitly-unsupported");
    }
  }

  private static void reflectionContract() throws Exception {
    Class<SoundCloudMp3SegmentDecoder> type = SoundCloudMp3SegmentDecoder.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {SoundCloudSegmentDecoder.class})
        && type.getAnnotations().length == 0, "class metadata");
    check(type.getDeclaredFields().length == 1 && type.getDeclaredMethods().length == 4,
        "member counts");
    Field supplier = type.getDeclaredField("nextStreamProvider");
    check(supplier.getType() == Supplier.class
        && supplier.getGenericType().getTypeName().equals(
            "java.util.function.Supplier<com.sedmelluq.discord.lavaplayer.tools.io."
                + "SeekableInputStream>")
        && supplier.getModifiers() == (Modifier.PRIVATE | Modifier.FINAL)
        && !supplier.isSynthetic(), "supplier metadata");

    Constructor<?> constructor = type.getDeclaredConstructor(Supplier.class);
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC
        && constructor.getGenericParameterTypes()[0].getTypeName().equals(
            "java.util.function.Supplier<com.sedmelluq.discord.lavaplayer.tools.io."
                + "SeekableInputStream>")
        && constructor.getExceptionTypes().length == 0 && !constructor.isSynthetic()
        && !constructor.isVarArgs(), "constructor metadata");

    checkMethod(type, "prepareStream", new Class<?>[] {boolean.class}, new Class<?>[0]);
    checkMethod(type, "resetStream", new Class<?>[0], new Class<?>[0]);
    checkMethod(type, "playStream",
        new Class<?>[] {AudioProcessingContext.class, long.class, long.class},
        new Class<?>[] {InterruptedException.class, IOException.class});
    checkMethod(type, "close", new Class<?>[0], new Class<?>[0]);
  }

  private static void commonContract() throws Exception {
    CountingSupplier supplier = new CountingSupplier();
    SoundCloudMp3SegmentDecoder decoder = new SoundCloudMp3SegmentDecoder(supplier);
    Field field = SoundCloudMp3SegmentDecoder.class.getDeclaredField("nextStreamProvider");
    field.setAccessible(true);
    check(field.get(decoder) == supplier, "constructor capture");
    check(field.get(new SoundCloudMp3SegmentDecoder(null)) == null, "null capture");

    decoder.prepareStream(true);
    decoder.prepareStream(false);
    decoder.resetStream();
    decoder.close();
    check(supplier.calls == 0, "no-op lifecycle");
  }

  private static void referenceServiceContract() throws Exception {
    CountingSupplier supplier = new CountingSupplier();
    SoundCloudMp3SegmentDecoder decoder = new SoundCloudMp3SegmentDecoder(supplier);
    Throwable error = capture(() -> decoder.playStream(null, -1L, Long.MAX_VALUE));
    check(error == supplier.sentinel && supplier.calls == 1, "legacy supplier dispatch");
  }

  private static void candidateServiceContract() throws Exception {
    CountingSupplier supplier = new CountingSupplier();
    SoundCloudMp3SegmentDecoder decoder = new SoundCloudMp3SegmentDecoder(supplier);
    Throwable error = capture(() -> decoder.playStream(null, -1L, Long.MAX_VALUE));
    check(error instanceof UnsupportedOperationException
        && HLS_DISABLED.equals(error.getMessage()) && supplier.calls == 0,
        "bounded progressive-only policy");
  }

  private static Method checkMethod(Class<?> owner, String name, Class<?>[] parameters,
                                    Class<?>[] exceptions) throws Exception {
    Method method = owner.getDeclaredMethod(name, parameters);
    check(method.getReturnType() == void.class && method.getModifiers() == Modifier.PUBLIC
        && Arrays.equals(method.getExceptionTypes(), exceptions)
        && !method.isBridge() && !method.isSynthetic() && !method.isVarArgs(),
        name + " metadata");
    return method;
  }

  private static Throwable capture(Operation operation) throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected failure");
    } catch (Throwable error) {
      return error;
    }
  }

  private static final class CountingSupplier implements Supplier<SeekableInputStream> {
    private final RuntimeException sentinel = new RuntimeException("supplier-sentinel");
    private int calls;

    @Override
    public SeekableInputStream get() {
      calls++;
      throw sentinel;
    }
  }

  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const SOUND_CLOUD_OPUS_SEGMENT_DECODER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.container.ogg.OggPacketInputStream;
import com.sedmelluq.discord.lavaplayer.container.ogg.OggTrackBlueprint;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudOpusSegmentDecoder;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudSegmentDecoder;
import com.sedmelluq.discord.lavaplayer.tools.io.SeekableInputStream;
import com.sedmelluq.discord.lavaplayer.track.info.AudioTrackInfoProvider;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioProcessingContext;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.function.Supplier;

public final class GateSoundCloudOpusSegmentDecoder {
  private static final String HLS_DISABLED =
      "Legacy SoundCloud Opus HLS segment playback is unsupported; "
      + "use Mantle's bounded progressive native source.";

  public static void main(String[] args) throws Exception {
    check(args.length == 1 && (args[0].equals("reference") || args[0].equals("candidate")),
        "expected disposition");
    reflectionContract();
    commonContract();
    if (args[0].equals("reference")) {
      referenceServiceContract();
      System.out.println("common=public-concrete,3-fields,1-constructor,4-exported-methods,"
          + "capture,stateful-reset-close,generic-supplier,checked-signatures,reflection;"
          + "service=legacy-opus-segment-supplier");
    } else {
      candidateServiceContract();
      System.out.println("common=public-concrete,3-fields,1-constructor,4-exported-methods,"
          + "capture,stateful-reset-close,generic-supplier,checked-signatures,reflection;"
          + "service=bounded-progressive-only,no-supplier,hls-explicitly-unsupported");
    }
  }

  private static void reflectionContract() throws Exception {
    Class<SoundCloudOpusSegmentDecoder> type = SoundCloudOpusSegmentDecoder.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {SoundCloudSegmentDecoder.class})
        && type.getAnnotations().length == 0, "class metadata");
    check(type.getDeclaredFields().length == 3 && type.getDeclaredMethods().length == 5,
        "member counts");
    Field supplier = checkField(type, "nextStreamProvider", Supplier.class,
        Modifier.PRIVATE | Modifier.FINAL);
    check(supplier.getGenericType().getTypeName().equals(
        "java.util.function.Supplier<com.sedmelluq.discord.lavaplayer.tools.io."
            + "SeekableInputStream>"), "supplier generic metadata");
    checkField(type, "lastJoinedStream", OggPacketInputStream.class, Modifier.PRIVATE);
    checkField(type, "blueprint", OggTrackBlueprint.class, Modifier.PRIVATE);

    Constructor<?> constructor = type.getDeclaredConstructor(Supplier.class);
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC
        && constructor.getGenericParameterTypes()[0].getTypeName().equals(
            "java.util.function.Supplier<com.sedmelluq.discord.lavaplayer.tools.io."
                + "SeekableInputStream>")
        && constructor.getExceptionTypes().length == 0 && !constructor.isSynthetic()
        && !constructor.isVarArgs(), "constructor metadata");

    checkMethod(type, "prepareStream", Modifier.PUBLIC,
        new Class<?>[] {boolean.class}, new Class<?>[] {IOException.class});
    checkMethod(type, "resetStream", Modifier.PUBLIC,
        new Class<?>[0], new Class<?>[] {IOException.class});
    checkMethod(type, "playStream", Modifier.PUBLIC,
        new Class<?>[] {AudioProcessingContext.class, long.class, long.class},
        new Class<?>[] {InterruptedException.class, IOException.class});
    checkMethod(type, "close", Modifier.PUBLIC,
        new Class<?>[0], new Class<?>[] {Exception.class});
    checkMethod(type, "obtainStream", Modifier.PRIVATE, new Class<?>[0], new Class<?>[0]);
  }

  private static void commonContract() throws Exception {
    CountingSupplier supplier = new CountingSupplier();
    SoundCloudOpusSegmentDecoder decoder = new SoundCloudOpusSegmentDecoder(supplier);
    check(field("nextStreamProvider").get(decoder) == supplier, "constructor capture");
    check(field("nextStreamProvider").get(new SoundCloudOpusSegmentDecoder(null)) == null,
        "null capture");
    check(field("lastJoinedStream").get(decoder) == null
        && field("blueprint").get(decoder) == null, "initial state");
    decoder.resetStream();
    decoder.close();
    check(supplier.calls == 0, "clean lifecycle");

    OggTrackBlueprint blueprint = (OggTrackBlueprint) Proxy.newProxyInstance(
        GateSoundCloudOpusSegmentDecoder.class.getClassLoader(),
        new Class<?>[] {OggTrackBlueprint.class}, (proxy, method, values) -> null);
    field("blueprint").set(decoder, blueprint);
    ClosingStream resetInput = new ClosingStream();
    field("lastJoinedStream").set(decoder, new OggPacketInputStream(resetInput, true));
    decoder.resetStream();
    check(resetInput.closed && field("lastJoinedStream").get(decoder) == null
        && field("blueprint").get(decoder) == blueprint, "stateful reset");

    ClosingStream closeInput = new ClosingStream();
    field("lastJoinedStream").set(decoder, new OggPacketInputStream(closeInput, true));
    decoder.close();
    check(closeInput.closed && field("lastJoinedStream").get(decoder) == null
        && field("blueprint").get(decoder) == blueprint, "stateful close");
  }

  private static void referenceServiceContract() throws Exception {
    assertSupplierFailure(decoder -> decoder.prepareStream(true));
    assertSupplierFailure(decoder -> decoder.prepareStream(false));
    assertSupplierFailure(decoder -> decoder.playStream(null, -1L, Long.MAX_VALUE));
  }

  private static void candidateServiceContract() throws Exception {
    assertUnsupported(decoder -> decoder.prepareStream(true));
    assertUnsupported(decoder -> decoder.prepareStream(false));
    assertUnsupported(decoder -> decoder.playStream(null, -1L, Long.MAX_VALUE));
  }

  private static void assertSupplierFailure(DecoderOperation operation) throws Exception {
    CountingSupplier supplier = new CountingSupplier();
    Throwable error = capture(() -> operation.run(new SoundCloudOpusSegmentDecoder(supplier)));
    check(error == supplier.sentinel && supplier.calls == 1, "legacy supplier dispatch");
  }

  private static void assertUnsupported(DecoderOperation operation) throws Exception {
    CountingSupplier supplier = new CountingSupplier();
    Throwable error = capture(() -> operation.run(new SoundCloudOpusSegmentDecoder(supplier)));
    check(error instanceof UnsupportedOperationException
        && HLS_DISABLED.equals(error.getMessage()) && supplier.calls == 0,
        "bounded progressive-only policy");
  }

  private static Field checkField(Class<?> owner, String name, Class<?> type, int modifiers)
      throws Exception {
    Field field = owner.getDeclaredField(name);
    check(field.getType() == type && field.getModifiers() == modifiers && !field.isSynthetic(),
        name + " metadata");
    return field;
  }

  private static Method checkMethod(Class<?> owner, String name, int modifiers,
                                    Class<?>[] parameters, Class<?>[] exceptions) throws Exception {
    Method method = owner.getDeclaredMethod(name, parameters);
    check(method.getModifiers() == modifiers && Arrays.equals(method.getExceptionTypes(), exceptions)
        && !method.isBridge() && !method.isSynthetic() && !method.isVarArgs(),
        name + " metadata");
    return method;
  }

  private static Field field(String name) throws Exception {
    Field field = SoundCloudOpusSegmentDecoder.class.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }

  private static Throwable capture(Operation operation) throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected failure");
    } catch (Throwable error) {
      return error;
    }
  }

  private static final class CountingSupplier implements Supplier<SeekableInputStream> {
    private final RuntimeException sentinel = new RuntimeException("supplier-sentinel");
    private int calls;

    @Override
    public SeekableInputStream get() {
      calls++;
      throw sentinel;
    }
  }

  private static final class ClosingStream extends SeekableInputStream {
    private boolean closed;

    ClosingStream() { super(0L, 0L); }
    @Override public int read() { return -1; }
    @Override public long getPosition() { return 0L; }
    @Override protected void seekHard(long position) { }
    @Override public boolean canSeekHard() { return true; }
    @Override public List<AudioTrackInfoProvider> getTrackInfoProviders() {
      return Collections.emptyList();
    }
    @Override public void close() { closed = true; }
  }

  private interface DecoderOperation {
    void run(SoundCloudOpusSegmentDecoder decoder) throws Exception;
  }
  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const DEFAULT_SOUND_CLOUD_DATA_LOADER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudDataLoader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudDataLoader;
import com.sedmelluq.discord.lavaplayer.tools.JsonBrowser;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;
import org.apache.http.HttpEntity;
import org.apache.http.ProtocolVersion;
import org.apache.http.client.methods.CloseableHttpResponse;
import org.apache.http.client.methods.HttpUriRequest;
import org.apache.http.entity.ContentType;
import org.apache.http.entity.StringEntity;
import org.apache.http.message.BasicStatusLine;

public final class GateDefaultSoundCloudDataLoader {
  public static void main(String[] args) throws Exception {
    check(args.length == 1 && (args[0].equals("reference") || args[0].equals("candidate")),
        "expected disposition");
    reflectionContract();
    behaviorContract();
    System.out.println("public-concrete,0-fields,1-constructor,1-exported-method;"
        + "resolve-v2,get,encoded-url,404-null-browser,json,close,status-error,suppressed-close");
  }

  private static void reflectionContract() throws Exception {
    Class<DefaultSoundCloudDataLoader> type = DefaultSoundCloudDataLoader.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {SoundCloudDataLoader.class}),
        "class metadata");
    check(type.getDeclaredFields().length == 0, "field count");
    Constructor<?> constructor = type.getDeclaredConstructor();
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC, "constructor metadata");
    check(type.getDeclaredMethods().length == 2, "method count");
    checkMethod(type.getDeclaredMethod("load", HttpInterface.class, String.class),
        JsonBrowser.class, Modifier.PUBLIC, IOException.class);
    checkMethod(type.getDeclaredMethod("buildUri", String.class),
        java.net.URI.class, Modifier.PRIVATE);
  }

  private static void behaviorContract() throws Exception {
    DefaultSoundCloudDataLoader loader = new DefaultSoundCloudDataLoader();
    String sourceUrl = "https://soundcloud.com/a b/tr?x=1&emoji=é";
    String expectedUri = "https://api-v2.soundcloud.com/resolve?url="
        + "https%3A%2F%2Fsoundcloud.com%2Fa+b%2Ftr%3Fx%3D1%26emoji%3D%C3%A9";

    RecordingHttpInterface success = new RecordingHttpInterface(
        200, "{\"kind\":\"track\",\"id\":42}", null, null);
    JsonBrowser result = loader.load(success, sourceUrl);
    check("track".equals(result.get("kind").text()) && result.get("id").asLong(-1) == 42L,
        "parsed JSON");
    success.checkRequest(expectedUri);
    check(success.closeCount == 1, "success response close");

    RecordingHttpInterface missing = new RecordingHttpInterface(404, null, null, null);
    check(loader.load(missing, sourceUrl) == JsonBrowser.NULL_BROWSER, "404 null browser identity");
    missing.checkRequest(expectedUri);
    check(missing.closeCount == 1, "404 response close");

    RecordingHttpInterface failed = new RecordingHttpInterface(500, null, null, null);
    IOException status = expect(IOException.class, () -> loader.load(failed, sourceUrl));
    check("Invalid status code for video page response: 500".equals(status.getMessage()),
        "status diagnostic");
    check(failed.closeCount == 1, "failed response close");

    IOException closeFailure = new IOException("close-failure");
    RecordingHttpInterface malformed = new RecordingHttpInterface(
        200, "{", null, closeFailure);
    IOException parse = expect(IOException.class, () -> loader.load(malformed, sourceUrl));
    check(parse.getSuppressed().length == 1 && parse.getSuppressed()[0] == closeFailure,
        "suppressed close failure");
    check(malformed.closeCount == 1, "malformed response close");

    IOException executeFailure = new IOException("execute-failure");
    RecordingHttpInterface unavailable = new RecordingHttpInterface(
        200, "{}", executeFailure, null);
    check(expect(IOException.class, () -> loader.load(unavailable, sourceUrl)) == executeFailure,
        "execute failure identity");
    check(unavailable.closeCount == 0, "no response to close");
  }

  private static void checkMethod(Method method, Class<?> returnType, int modifiers,
                                  Class<?>... exceptions) {
    check(method.getReturnType() == returnType && method.getModifiers() == modifiers
        && !method.isBridge() && !method.isSynthetic() && !method.isVarArgs()
        && Arrays.equals(method.getExceptionTypes(), exceptions), method + " metadata");
  }

  private static final class RecordingHttpInterface extends HttpInterface {
    private final int status;
    private final String body;
    private final IOException executeFailure;
    private final IOException closeFailure;
    private HttpUriRequest request;
    private int closeCount;

    RecordingHttpInterface(int status, String body, IOException executeFailure,
                           IOException closeFailure) {
      super(null, null, false, null);
      this.status = status;
      this.body = body;
      this.executeFailure = executeFailure;
      this.closeFailure = closeFailure;
    }

    @Override
    public CloseableHttpResponse execute(HttpUriRequest request) throws IOException {
      this.request = request;
      if (executeFailure != null) throw executeFailure;
      HttpEntity entity = body == null ? null : new StringEntity(body, ContentType.APPLICATION_JSON);
      InvocationHandler handler = (proxy, method, args) -> {
        switch (method.getName()) {
          case "getStatusLine":
            return new BasicStatusLine(new ProtocolVersion("HTTP", 1, 1), status, "");
          case "getEntity":
            return entity;
          case "close":
            closeCount++;
            if (closeFailure != null) throw closeFailure;
            return null;
          case "toString":
            return "RecordingCloseableHttpResponse(" + status + ")";
          default:
            throw new AssertionError("unexpected response method: " + method);
        }
      };
      return (CloseableHttpResponse) Proxy.newProxyInstance(
          CloseableHttpResponse.class.getClassLoader(),
          new Class<?>[] {CloseableHttpResponse.class}, handler);
    }

    void checkRequest(String expectedUri) {
      check(request != null && "GET".equals(request.getMethod())
          && expectedUri.equals(request.getURI().toASCIIString()), "resolve request");
    }
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const DEFAULT_SOUND_CLOUD_DATA_READER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudDataReader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudDataReader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudTrackFormat;
import com.sedmelluq.discord.lavaplayer.tools.JsonBrowser;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;
import java.util.List;

public final class GateDefaultSoundCloudDataReader {
  public static void main(String[] args) throws Exception {
    check(args.length == 1 && (args[0].equals("reference") || args[0].equals("candidate")),
        "expected disposition");
    reflectionContract();
    behaviorContract();
    System.out.println("public-concrete,1-field,1-constructor,10-exported-methods;"
        + "kind-identity,ids,policy,track-info,thumbnail,formats,format-filter-order,"
        + "playlist-values,missing-quirks,generic-signatures");
  }

  private static void reflectionContract() throws Exception {
    Class<DefaultSoundCloudDataReader> type = DefaultSoundCloudDataReader.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {SoundCloudDataReader.class}),
        "class metadata");
    check(type.getDeclaredFields().length == 1, "field count");
    Field log = type.getDeclaredField("log");
    check(log.getType().getName().equals("org.slf4j.Logger")
        && log.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL),
        "logger metadata");
    log.setAccessible(true);
    check(log.get(null) != null, "logger initialization");
    Constructor<?> constructor = type.getDeclaredConstructor();
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC, "constructor metadata");
    check(type.getDeclaredMethods().length == 10, "method count");
    checkMethod(type, "findTrackData", JsonBrowser.class, Modifier.PUBLIC,
        new Class<?>[] {JsonBrowser.class});
    checkMethod(type, "readTrackId", String.class, Modifier.PUBLIC,
        new Class<?>[] {JsonBrowser.class});
    checkMethod(type, "isTrackBlocked", boolean.class, Modifier.PUBLIC,
        new Class<?>[] {JsonBrowser.class});
    checkMethod(type, "readTrackInfo", AudioTrackInfo.class, Modifier.PUBLIC,
        new Class<?>[] {JsonBrowser.class, String.class});
    Method formats = checkMethod(type, "readTrackFormats", List.class, Modifier.PUBLIC,
        new Class<?>[] {JsonBrowser.class});
    check(formats.getGenericReturnType().getTypeName().equals(
        "java.util.List<com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudTrackFormat>"),
        "format generic return");
    checkMethod(type, "findPlaylistData", JsonBrowser.class, Modifier.PUBLIC,
        new Class<?>[] {JsonBrowser.class, String.class});
    checkMethod(type, "readPlaylistName", String.class, Modifier.PUBLIC,
        new Class<?>[] {JsonBrowser.class});
    checkMethod(type, "readPlaylistIdentifier", String.class, Modifier.PUBLIC,
        new Class<?>[] {JsonBrowser.class});
    Method tracks = checkMethod(type, "readPlaylistTracks", List.class, Modifier.PUBLIC,
        new Class<?>[] {JsonBrowser.class});
    check(tracks.getGenericReturnType().getTypeName().equals(
        "java.util.List<com.sedmelluq.discord.lavaplayer.tools.JsonBrowser>"),
        "playlist generic return");
    checkMethod(type, "findEntryOfKind", JsonBrowser.class, Modifier.PROTECTED,
        new Class<?>[] {JsonBrowser.class, String.class});
  }

  private static void behaviorContract() throws Exception {
    ExposedReader reader = new ExposedReader();
    JsonBrowser track = JsonBrowser.parse("{"
        + "\"kind\":\"track\",\"id\":123,\"policy\":\"BLOCK\","
        + "\"title\":\"Fixture Song\",\"full_duration\":9876,"
        + "\"permalink_url\":\"https://soundcloud.com/fixture/song\","
        + "\"artwork_url\":\"https://img.example/art-large.jpg\","
        + "\"user\":{\"username\":\"Fixture Artist\","
        + "\"avatar_url\":\"https://img.example/avatar-large.jpg\"},"
        + "\"publisher_metadata\":{\"isrc\":\"US-ABC-12-34567\"},"
        + "\"media\":{\"transcodings\":["
        + "{\"format\":{\"protocol\":\"progressive\",\"mime_type\":\"audio/mpeg\"},"
        + "\"url\":\"https://api-v2.soundcloud.com/media/one\"},"
        + "{\"format\":{\"protocol\":\"hls\",\"mime_type\":\"audio/ogg; codecs=opus\"},"
        + "\"url\":\"https://api-v2.soundcloud.com/media/two\"},"
        + "{\"format\":{\"protocol\":\"hls\",\"mime_type\":\"audio/mpeg\"}},"
        + "{\"format\":{\"mime_type\":\"audio/mpeg\"},\"url\":\"ignored\"}"
        + "]}}"
    );

    check(reader.findTrackData(track) == track && reader.find(track, "track") == track,
        "track kind identity");
    check(reader.findTrackData(JsonBrowser.parse("{\"kind\":\"playlist\"}")) == null
        && reader.findTrackData(JsonBrowser.NULL_BROWSER) == null, "track kind rejection");
    check("123".equals(reader.readTrackId(track)), "track ID");
    check(reader.isTrackBlocked(track)
        && !reader.isTrackBlocked(JsonBrowser.parse("{\"policy\":\"block\"}"))
        && !reader.isTrackBlocked(JsonBrowser.parse("{}")), "policy");

    AudioTrackInfo info = reader.readTrackInfo(track, "chosen-id");
    check("Fixture Song".equals(info.title) && "Fixture Artist".equals(info.author)
        && info.length == 9876L && "chosen-id".equals(info.identifier) && !info.isStream
        && "https://soundcloud.com/fixture/song".equals(info.uri)
        && "https://img.example/art-original.jpg".equals(info.artworkUrl)
        && "US-ABC-12-34567".equals(info.isrc), "track info");
    JsonBrowser avatarTrack = JsonBrowser.parse("{"
        + "\"title\":\"T\",\"full_duration\":1,\"user\":{\"username\":\"A\","
        + "\"avatar_url\":\"https://img.example/avatar-large.jpg\"}}"
    );
    check("https://img.example/avatar-original.jpg".equals(
        reader.readTrackInfo(avatarTrack, "id").artworkUrl), "avatar fallback");
    expect(NullPointerException.class,
        () -> reader.readTrackInfo(JsonBrowser.parse("{}"), "id"));

    List<SoundCloudTrackFormat> formats = reader.readTrackFormats(track);
    check(formats.size() == 2, "format filtering");
    checkFormat(formats.get(0), "123", "progressive", "audio/mpeg",
        "https://api-v2.soundcloud.com/media/one");
    checkFormat(formats.get(1), "123", "hls", "audio/ogg; codecs=opus",
        "https://api-v2.soundcloud.com/media/two");
    List<SoundCloudTrackFormat> emptyFormats = reader.readTrackFormats(JsonBrowser.parse("{}"));
    check(emptyFormats.isEmpty(), "missing formats");
    emptyFormats.add(null);
    check(emptyFormats.size() == 1, "mutable format list");

    JsonBrowser playlist = JsonBrowser.parse("{\"kind\":\"playlist\","
        + "\"title\":\"Fixture Playlist\",\"permalink\":\"fixture-list\","
        + "\"tracks\":[{\"id\":1},{\"id\":2}]}"
    );
    check(reader.findPlaylistData(playlist, "playlist") == playlist
        && reader.findPlaylistData(playlist, "system-playlist") == null,
        "playlist kind identity");
    check("Fixture Playlist".equals(reader.readPlaylistName(playlist))
        && "fixture-list".equals(reader.readPlaylistIdentifier(playlist)), "playlist values");
    List<JsonBrowser> tracks = reader.readPlaylistTracks(playlist);
    check(tracks.size() == 2 && tracks.get(0).get("id").asLong(-1) == 1L
        && tracks.get(1).get("id").asLong(-1) == 2L, "playlist tracks");
    check(reader.readPlaylistTracks(JsonBrowser.parse("{}" )).isEmpty()
        && "".equals(reader.readPlaylistName(JsonBrowser.parse("{}")))
        && "".equals(reader.readPlaylistIdentifier(JsonBrowser.parse("{}"))),
        "missing playlist values");
  }

  private static void checkFormat(SoundCloudTrackFormat format, String id, String protocol,
                                  String mimeType, String url) {
    check(id.equals(format.getTrackId()) && protocol.equals(format.getProtocol())
        && mimeType.equals(format.getMimeType()) && url.equals(format.getLookupUrl()),
        "format values");
  }

  private static Method checkMethod(Class<?> type, String name, Class<?> returnType,
                                    int modifiers, Class<?>[] parameters) throws Exception {
    Method method = type.getDeclaredMethod(name, parameters);
    check(method.getReturnType() == returnType && method.getModifiers() == modifiers
        && method.getExceptionTypes().length == 0 && !method.isBridge()
        && !method.isSynthetic() && !method.isVarArgs(), method + " metadata");
    return method;
  }

  private static final class ExposedReader extends DefaultSoundCloudDataReader {
    JsonBrowser find(JsonBrowser data, String kind) { return super.findEntryOfKind(data, kind); }
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const DEFAULT_SOUND_CLOUD_FORMAT_HANDLER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudFormatHandler;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudTrackFormat;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudFormatHandler;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudM3uInfo;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudMp3SegmentDecoder;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudOpusSegmentDecoder;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudSegmentDecoder;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudTrackFormat;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;

public final class GateDefaultSoundCloudFormatHandler {
  public static void main(String[] args) throws Exception {
    check(args.length == 1 && (args[0].equals("reference") || args[0].equals("candidate")),
        "expected disposition");
    reflectionContract();
    behaviorContract();
    System.out.println("public-concrete,1-field,1-constructor,4-exported-methods;"
        + "opus-hls-priority,mp3-hls,progressive-mp3,exact-mime,stable-order,"
        + "identifier-prefixes,unknown-fallback,m3u-factories,mp3-lookup,error-quirks");
  }

  private static void reflectionContract() throws Exception {
    Class<DefaultSoundCloudFormatHandler> type = DefaultSoundCloudFormatHandler.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {SoundCloudFormatHandler.class}),
        "class metadata");
    check(type.getDeclaredFields().length == 1, "field count");
    Field types = type.getDeclaredField("TYPES");
    check(types.getType().isArray()
        && types.getType().getComponentType().getName().endsWith("$FormatType")
        && types.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL),
        "format type table metadata");
    types.setAccessible(true);
    check(java.lang.reflect.Array.getLength(types.get(null)) == 3, "format type table values");
    Constructor<?> constructor = type.getDeclaredConstructor();
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC, "constructor metadata");
    check(type.getDeclaredMethods().length == 5, "method count");
    Method choose = checkMethod(type, "chooseBestFormat", SoundCloudTrackFormat.class,
        Modifier.PUBLIC, new Class<?>[] {List.class});
    check(choose.getGenericParameterTypes()[0].getTypeName().equals(
        "java.util.List<com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudTrackFormat>"),
        "choose generic parameter");
    checkMethod(type, "buildFormatIdentifier", String.class, Modifier.PUBLIC,
        new Class<?>[] {SoundCloudTrackFormat.class});
    checkMethod(type, "getM3uInfo", SoundCloudM3uInfo.class, Modifier.PUBLIC,
        new Class<?>[] {String.class});
    checkMethod(type, "getMp3LookupUrl", String.class, Modifier.PUBLIC,
        new Class<?>[] {String.class});
    checkMethod(type, "findFormat", SoundCloudTrackFormat.class,
        Modifier.PRIVATE | Modifier.STATIC,
        new Class<?>[] {List.class, types.getType().getComponentType()});
  }

  private static void behaviorContract() throws Exception {
    DefaultSoundCloudFormatHandler handler = new DefaultSoundCloudFormatHandler();
    SoundCloudTrackFormat progressive = format("progressive", "audio/mpeg", "progressive");
    SoundCloudTrackFormat mp3Hls = format("hls", "audio/mpeg", "mp3-hls");
    SoundCloudTrackFormat opusHls = format("hls", "audio/ogg", "opus-hls");
    SoundCloudTrackFormat secondOpus = format("hls", "audio/ogg", "second-opus");
    SoundCloudTrackFormat codecMime = format("hls", "audio/ogg; codecs=opus", "codec-mime");
    SoundCloudTrackFormat unknown = format("dash", "audio/aac", "unknown");

    check(handler.chooseBestFormat(Arrays.asList(progressive, mp3Hls, opusHls)) == opusHls,
        "type priority over input order");
    check(handler.chooseBestFormat(Arrays.asList(secondOpus, opusHls)) == secondOpus,
        "stable order within type");
    check(handler.chooseBestFormat(Arrays.asList(progressive, mp3Hls)) == mp3Hls,
        "MP3 HLS priority");
    check(handler.chooseBestFormat(Collections.singletonList(progressive)) == progressive,
        "progressive fallback");
    RuntimeException unsupported = expect(RuntimeException.class,
        () -> handler.chooseBestFormat(Arrays.asList(codecMime, unknown)));
    check("Did not detect any supported formats".equals(unsupported.getMessage()),
        "unsupported diagnostic");
    RuntimeException empty = expect(RuntimeException.class,
        () -> handler.chooseBestFormat(Collections.emptyList()));
    check("Did not detect any supported formats".equals(empty.getMessage()), "empty diagnostic");

    check("O:https://media/opus-hls".equals(handler.buildFormatIdentifier(opusHls)),
        "Opus identifier");
    check("U:https://media/mp3-hls".equals(handler.buildFormatIdentifier(mp3Hls)),
        "MP3 HLS identifier");
    check("M:https://media/progressive".equals(handler.buildFormatIdentifier(progressive)),
        "progressive identifier");
    check("X:https://media/codec-mime".equals(handler.buildFormatIdentifier(codecMime))
        && "X:https://media/unknown".equals(handler.buildFormatIdentifier(unknown)),
        "unknown identifier fallback");
    check("M:null".equals(handler.buildFormatIdentifier(
        new DefaultSoundCloudTrackFormat("track", "progressive", "audio/mpeg", null))),
        "null lookup concatenation");

    checkM3u(handler.getM3uInfo("O:https://media/opus"), "https://media/opus",
        SoundCloudOpusSegmentDecoder.class);
    checkM3u(handler.getM3uInfo("U:https://media/mp3"), "https://media/mp3",
        SoundCloudMp3SegmentDecoder.class);
    checkM3u(handler.getM3uInfo("O:"), "", SoundCloudOpusSegmentDecoder.class);
    check(handler.getM3uInfo("o:https://media/opus") == null
        && handler.getM3uInfo("M:https://media/direct") == null
        && handler.getM3uInfo("X:https://media/unknown") == null, "M3U rejection");

    check("https://media/direct".equals(handler.getMp3LookupUrl("M:https://media/direct"))
        && "".equals(handler.getMp3LookupUrl("M:"))
        && handler.getMp3LookupUrl("m:https://media/direct") == null
        && handler.getMp3LookupUrl("U:https://media/mp3") == null, "MP3 lookup");

    expect(NullPointerException.class, () -> handler.chooseBestFormat(null));
    expect(NullPointerException.class, () -> handler.buildFormatIdentifier(null));
    expect(NullPointerException.class, () -> handler.getM3uInfo(null));
    expect(NullPointerException.class, () -> handler.getMp3LookupUrl(null));
  }

  private static SoundCloudTrackFormat format(String protocol, String mimeType, String suffix) {
    return new DefaultSoundCloudTrackFormat(
        "track", protocol, mimeType, "https://media/" + suffix);
  }

  private static void checkM3u(SoundCloudM3uInfo info, String lookupUrl,
                               Class<?> decoderType) {
    check(info != null && lookupUrl.equals(info.lookupUrl), "M3U info");
    SoundCloudSegmentDecoder decoder = info.decoderFactory.create(() -> null);
    check(decoderType.isInstance(decoder), "M3U decoder factory");
  }

  private static Method checkMethod(Class<?> type, String name, Class<?> returnType,
                                    int modifiers, Class<?>[] parameters) throws Exception {
    Method method = type.getDeclaredMethod(name, parameters);
    check(method.getReturnType() == returnType && method.getModifiers() == modifiers
        && method.getExceptionTypes().length == 0 && !method.isBridge()
        && !method.isSynthetic() && !method.isVarArgs(), method + " metadata");
    return method;
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const DEFAULT_SOUND_CLOUD_PLAYLIST_LOADER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudDataReader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudFormatHandler;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudPlaylistLoader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudDataLoader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudDataReader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudFormatHandler;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudPlaylistLoader;
import com.sedmelluq.discord.lavaplayer.tools.FriendlyException;
import com.sedmelluq.discord.lavaplayer.tools.JsonBrowser;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterfaceManager;
import com.sedmelluq.discord.lavaplayer.track.AudioPlaylist;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import com.sedmelluq.discord.lavaplayer.track.BasicAudioPlaylist;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.net.URI;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.function.Function;
import java.util.regex.Pattern;
import org.apache.http.HttpEntity;
import org.apache.http.ProtocolVersion;
import org.apache.http.client.methods.CloseableHttpResponse;
import org.apache.http.client.methods.HttpUriRequest;
import org.apache.http.entity.ContentType;
import org.apache.http.entity.StringEntity;
import org.apache.http.message.BasicStatusLine;

public final class GateDefaultSoundCloudPlaylistLoader {
  private static final String REGEX = "^(?:http://|https://|)(?:www\\.|)(?:m\\.|)"
      + "soundcloud\\.com/([a-zA-Z0-9-_:]+)/sets/([a-zA-Z0-9-_:]+)/?"
      + "([a-zA-Z0-9-_:]+)?(?:\\?.*|)$";

  public static void main(String[] args) throws Exception {
    check(args.length == 1 && (args[0].equals("reference") || args[0].equals("candidate")),
        "expected disposition");
    reflectionContract();
    routingAndHelpersContract();
    playlistLoadingContract();
    failureContract();
    System.out.println("public-concrete,5-exported-fields,1-constructor,5-exported-methods;"
        + "url-regex,mobile-normalization,dependency-capture,track-url-encoding,stable-sort,"
        + "v2-batches-of-50,response-close,playlist-order,blocked-omit,bad-track-omit,"
        + "factory-metadata,http-interface-close,friendly-io-wrap,suppressed-close,generics");
  }

  private static void reflectionContract() throws Exception {
    Class<DefaultSoundCloudPlaylistLoader> type = DefaultSoundCloudPlaylistLoader.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {SoundCloudPlaylistLoader.class}),
        "class metadata");
    check(type.getDeclaredFields().length == 6, "field count");
    checkField(type, "log", "org.slf4j.Logger",
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    Field regex = checkField(type, "PLAYLIST_URL_REGEX", String.class.getName(),
        Modifier.PROTECTED | Modifier.STATIC | Modifier.FINAL);
    regex.setAccessible(true);
    check(REGEX.equals(regex.get(null)), "regex constant");
    Field pattern = checkField(type, "playlistUrlPattern", Pattern.class.getName(),
        Modifier.PROTECTED | Modifier.STATIC | Modifier.FINAL);
    pattern.setAccessible(true);
    check(REGEX.equals(((Pattern) pattern.get(null)).pattern()), "compiled pattern");
    checkField(type, "dataLoader", SoundCloudDataLoader.class.getName(),
        Modifier.PROTECTED | Modifier.FINAL);
    checkField(type, "dataReader", SoundCloudDataReader.class.getName(),
        Modifier.PROTECTED | Modifier.FINAL);
    checkField(type, "formatHandler", SoundCloudFormatHandler.class.getName(),
        Modifier.PROTECTED | Modifier.FINAL);

    Constructor<?> constructor = type.getDeclaredConstructor(
        SoundCloudDataLoader.class, SoundCloudDataReader.class, SoundCloudFormatHandler.class);
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC, "constructor metadata");
    check(type.getDeclaredMethods().length == 6, "method count");
    Method load = checkMethod(type, "load", AudioPlaylist.class, Modifier.PUBLIC,
        new Class<?>[] {String.class, HttpInterfaceManager.class, Function.class});
    check(load.getGenericParameterTypes()[2].getTypeName().equals(
        "java.util.function.Function<com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo, "
            + "com.sedmelluq.discord.lavaplayer.track.AudioTrack>"), "load generic factory");
    checkMethod(type, "loadFromSet", AudioPlaylist.class, Modifier.PROTECTED,
        new Class<?>[] {HttpInterfaceManager.class, String.class, Function.class});
    Method tracks = checkMethod(type, "loadPlaylistTracks", List.class, Modifier.PROTECTED,
        new Class<?>[] {HttpInterface.class, JsonBrowser.class, Function.class}, IOException.class);
    check(tracks.getGenericReturnType().getTypeName().equals(
        "java.util.List<com.sedmelluq.discord.lavaplayer.track.AudioTrack>"),
        "track generic return");
    checkMethod(type, "buildTrackListUrl", URI.class, Modifier.PROTECTED,
        new Class<?>[] {List.class});
    checkMethod(type, "sortPlaylistTracks", void.class, Modifier.PROTECTED,
        new Class<?>[] {List.class, List.class});
    Method lambda = Arrays.stream(type.getDeclaredMethods())
        .filter(method -> method.getName().startsWith("lambda$sortPlaylistTracks$"))
        .findFirst().orElseThrow(AssertionError::new);
    check(lambda.getReturnType() == int.class && lambda.getModifiers()
        == (Modifier.PRIVATE | 0x1000) && lambda.isSynthetic(), "sort lambda metadata");
  }

  private static void routingAndHelpersContract() throws Exception {
    SoundCloudDataLoader dataLoader = (http, url) -> JsonBrowser.NULL_BROWSER;
    SoundCloudDataReader dataReader = new DefaultSoundCloudDataReader();
    SoundCloudFormatHandler formatHandler = new DefaultSoundCloudFormatHandler();
    RoutingLoader loader = new RoutingLoader(dataLoader, dataReader, formatHandler);
    check(loader.exposedDataLoader() == dataLoader && loader.exposedDataReader() == dataReader
        && loader.exposedFormatHandler() == formatHandler, "dependency capture");

    AudioPlaylist marker = loader.marker;
    check(loader.load("https://m.soundcloud.com/user/sets/list?x=1", null, null) == marker
        && "https://soundcloud.com/user/sets/list?x=1".equals(loader.url),
        "mobile normalization");
    check(loader.load("soundcloud.com/a-b_c:1/sets/list-2", null, null) == marker
        && loader.load("http://www.soundcloud.com/u/sets/s/secret?si=x", null, null) == marker
        && loader.load("https://soundcloud.com/u/sets/s/", null, null) == marker,
        "accepted set URLs");
    check(loader.load("https://SOUNDCLOUD.com/u/sets/s", null, null) == null
        && loader.load("https://soundcloud.com/u.name/sets/s", null, null) == null
        && loader.load("https://soundcloud.com/u/tracks/s", null, null) == null
        && loader.load("https://soundcloud.com/u/sets", null, null) == null,
        "rejected set URLs");

    ExposedLoader helpers = new ExposedLoader(dataLoader, dataReader, formatHandler);
    check("https://api-v2.soundcloud.com/tracks?ids=1%2Ca+b%2C%C3%A9".equals(
        helpers.build(Arrays.asList("1", "a b", "é")).toASCIIString()),
        "track URL encoding");
    check("https://api-v2.soundcloud.com/tracks?ids=".equals(
        helpers.build(Collections.emptyList()).toASCIIString()), "empty track URL");

    List<JsonBrowser> values = new ArrayList<>(Arrays.asList(
        jsonTrack("b", false, true), jsonTrack("z", false, true),
        jsonTrack("a", false, true), jsonTrack("y", false, true)));
    helpers.sort(values, Arrays.asList("a", "b"));
    check(ids(values).equals(Arrays.asList("a", "b", "z", "y")), "stable playlist sort");
  }

  private static void playlistLoadingContract() throws Exception {
    StringBuilder references = new StringBuilder(
        "{\"kind\":\"playlist\",\"title\":\"Fixture Playlist\","
            + "\"permalink\":\"fixture-list\",\"tracks\":[");
    for (int id = 1; id <= 52; id++) {
      if (id > 1) references.append(',');
      references.append("{\"id\":").append(id).append('}');
    }
    references.append("]}");
    JsonBrowser playlistData = JsonBrowser.parse(references.toString());
    SoundCloudDataLoader dataLoader = (http, url) -> {
      check("https://soundcloud.com/user/sets/list".equals(url), "resolved set URL");
      return playlistData;
    };
    RecordingHttpInterface http = new RecordingHttpInterface(false);
    ExposedLoader loader = new ExposedLoader(
        dataLoader, new DefaultSoundCloudDataReader(), new DefaultSoundCloudFormatHandler());
    List<AudioTrackInfo> captured = new ArrayList<>();
    AudioPlaylist playlist = loader.load(
        "https://soundcloud.com/user/sets/list", manager(http), info -> {
          captured.add(info);
          return null;
        });

    check(playlist instanceof BasicAudioPlaylist
        && "Fixture Playlist".equals(playlist.getName()) && !playlist.isSearchResult()
        && playlist.getSelectedTrack() == null && playlist.getTracks().size() == 50,
        "playlist values: " + playlist.getClass() + "," + playlist.getName() + ","
            + playlist.isSearchResult() + "," + playlist.getSelectedTrack() + ","
            + playlist.getTracks().size());
    check(http.requests.size() == 2 && queryIds(http.requests.get(0)).size() == 50
        && queryIds(http.requests.get(1)).equals(Arrays.asList("51", "52")),
        "batches of 50");
    check(http.responseCloseCount == 2 && http.interfaceCloseCount == 1,
        "HTTP cleanup");
    check(captured.size() == 50 && "M:https://media/1".equals(captured.get(0).identifier)
        && "M:https://media/4".equals(captured.get(1).identifier)
        && "M:https://media/52".equals(captured.get(49).identifier),
        "blocked and bad track omission with playlist order");
  }

  private static void failureContract() throws Exception {
    IOException loadFailure = new IOException("load-failure");
    RecordingHttpInterface http = new RecordingHttpInterface(true);
    SoundCloudDataLoader failing = (ignored, url) -> { throw loadFailure; };
    ExposedLoader loader = new ExposedLoader(
        failing, new DefaultSoundCloudDataReader(), new DefaultSoundCloudFormatHandler());
    FriendlyException error = expect(FriendlyException.class,
        () -> loader.fromSet(manager(http), "https://soundcloud.com/u/sets/s", info -> null));
    check("Loading playlist from SoundCloud failed.".equals(error.getMessage())
        && error.severity == FriendlyException.Severity.SUSPICIOUS
        && error.getCause() == loadFailure && loadFailure.getSuppressed().length == 1
        && "close-failure".equals(loadFailure.getSuppressed()[0].getMessage())
        && http.interfaceCloseCount == 1,
        "friendly IO failure and suppressed close");
  }

  private static JsonBrowser jsonTrack(String id, boolean blocked, boolean validFormat)
      throws IOException {
    return JsonBrowser.parse(trackJson(id, blocked, validFormat));
  }

  private static String trackJson(String id, boolean blocked, boolean validFormat) {
    return "{\"id\":\"" + id + "\",\"policy\":\""
        + (blocked ? "BLOCK" : "ALLOW") + "\",\"title\":\"Track " + id
        + "\",\"full_duration\":1000,\"permalink_url\":\"https://soundcloud.com/t/" + id
        + "\",\"user\":{\"username\":\"Artist\",\"avatar_url\":"
        + "\"https://img/avatar-large.jpg\"},\"media\":{\"transcodings\":"
        + (validFormat ? "[{\"format\":{\"protocol\":\"progressive\","
            + "\"mime_type\":\"audio/mpeg\"},\"url\":\"https://media/" + id + "\"}]"
            : "[]") + "}}";
  }

  private static List<String> ids(List<JsonBrowser> values) {
    List<String> ids = new ArrayList<>();
    for (JsonBrowser value : values) ids.add(value.get("id").text());
    return ids;
  }

  private static List<String> queryIds(HttpUriRequest request) {
    String query = request.getURI().getQuery();
    check(query != null && query.startsWith("ids="), "track request query");
    return Arrays.asList(query.substring(4).split(","));
  }

  private static HttpInterfaceManager manager(HttpInterface http) {
    InvocationHandler handler = (proxy, method, args) -> {
      if (method.getName().equals("getInterface")) return http;
      if (method.getName().equals("close")) return null;
      if (method.getName().equals("toString")) return "FixtureHttpInterfaceManager";
      throw new AssertionError("unexpected manager method: " + method);
    };
    return (HttpInterfaceManager) Proxy.newProxyInstance(
        HttpInterfaceManager.class.getClassLoader(),
        new Class<?>[] {HttpInterfaceManager.class}, handler);
  }

  private static Field checkField(Class<?> type, String name, String fieldType, int modifiers)
      throws Exception {
    Field field = type.getDeclaredField(name);
    check(field.getType().getName().equals(fieldType) && field.getModifiers() == modifiers,
        field + " metadata");
    return field;
  }

  private static Method checkMethod(Class<?> type, String name, Class<?> returnType,
                                    int modifiers, Class<?>[] parameters,
                                    Class<?>... exceptions) throws Exception {
    Method method = type.getDeclaredMethod(name, parameters);
    check(method.getReturnType() == returnType && method.getModifiers() == modifiers
        && Arrays.equals(method.getExceptionTypes(), exceptions) && !method.isBridge()
        && !method.isSynthetic() && !method.isVarArgs(), method + " metadata");
    return method;
  }

  private static final class RoutingLoader extends DefaultSoundCloudPlaylistLoader {
    final AudioPlaylist marker = new BasicAudioPlaylist("marker", Collections.emptyList(), null, false);
    String url;

    RoutingLoader(SoundCloudDataLoader loader, SoundCloudDataReader reader,
                  SoundCloudFormatHandler handler) {
      super(loader, reader, handler);
    }

    @Override
    protected AudioPlaylist loadFromSet(HttpInterfaceManager manager, String url,
                                        Function<AudioTrackInfo, AudioTrack> factory) {
      this.url = url;
      return marker;
    }

    SoundCloudDataLoader exposedDataLoader() { return dataLoader; }
    SoundCloudDataReader exposedDataReader() { return dataReader; }
    SoundCloudFormatHandler exposedFormatHandler() { return formatHandler; }
  }

  private static final class ExposedLoader extends DefaultSoundCloudPlaylistLoader {
    ExposedLoader(SoundCloudDataLoader loader, SoundCloudDataReader reader,
                  SoundCloudFormatHandler handler) {
      super(loader, reader, handler);
    }

    URI build(List<String> ids) { return buildTrackListUrl(ids); }
    void sort(List<JsonBrowser> values, List<String> ids) { sortPlaylistTracks(values, ids); }
    AudioPlaylist fromSet(HttpInterfaceManager manager, String url,
                          Function<AudioTrackInfo, AudioTrack> factory) {
      return loadFromSet(manager, url, factory);
    }
  }

  private static final class RecordingHttpInterface extends HttpInterface {
    final List<HttpUriRequest> requests = new ArrayList<>();
    final boolean failClose;
    int responseCloseCount;
    int interfaceCloseCount;

    RecordingHttpInterface(boolean failClose) {
      super(null, null, false, null);
      this.failClose = failClose;
    }

    @Override
    public CloseableHttpResponse execute(HttpUriRequest request) throws IOException {
      requests.add(request);
      List<String> ids = queryIds(request);
      StringBuilder body = new StringBuilder("[");
      for (int index = ids.size() - 1; index >= 0; index--) {
        if (index < ids.size() - 1) body.append(',');
        String id = ids.get(index);
        body.append(trackJson(id, id.equals("2"), !id.equals("3")));
      }
      body.append(']');
      HttpEntity entity = new StringEntity(body.toString(), ContentType.APPLICATION_JSON);
      InvocationHandler handler = (proxy, method, args) -> {
        switch (method.getName()) {
          case "getStatusLine":
            return new BasicStatusLine(new ProtocolVersion("HTTP", 1, 1), 200, "");
          case "getEntity": return entity;
          case "close": responseCloseCount++; return null;
          case "toString": return "FixtureTrackListResponse";
          default: throw new AssertionError("unexpected response method: " + method);
        }
      };
      return (CloseableHttpResponse) Proxy.newProxyInstance(
          CloseableHttpResponse.class.getClassLoader(),
          new Class<?>[] {CloseableHttpResponse.class}, handler);
    }

    @Override
    public void close() throws IOException {
      interfaceCloseCount++;
      if (failClose) throw new IOException("close-failure");
    }
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const DEFAULT_SOUND_CLOUD_TRACK_FORMAT_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudTrackFormat;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudTrackFormat;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;

public final class GateDefaultSoundCloudTrackFormat {
  public static void main(String[] args) throws Exception {
    check(args.length == 1 && (args[0].equals("reference") || args[0].equals("candidate")),
        "expected disposition");
    reflectionContract();
    valueContract();
    System.out.println("public-concrete,4-private-final-fields,1-constructor,4-methods;"
        + "reference-preserving,null-preserving,no-value-overrides");
  }

  private static void reflectionContract() throws Exception {
    Class<DefaultSoundCloudTrackFormat> type = DefaultSoundCloudTrackFormat.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {SoundCloudTrackFormat.class}),
        "class metadata");
    check(type.getDeclaredFields().length == 4, "field count");
    checkField(type, "trackId");
    checkField(type, "protocol");
    checkField(type, "mimeType");
    checkField(type, "lookupUrl");

    Constructor<?> constructor = type.getDeclaredConstructor(
        String.class, String.class, String.class, String.class);
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC, "constructor metadata");
    check(type.getDeclaredMethods().length == 4, "method count");
    checkMethod(type, "getTrackId");
    checkMethod(type, "getProtocol");
    checkMethod(type, "getMimeType");
    checkMethod(type, "getLookupUrl");
  }

  private static void valueContract() throws Exception {
    String trackId = new String("track-id");
    String protocol = new String("progressive");
    String mimeType = new String("audio/mpeg");
    String lookupUrl = new String("https://example.invalid/lookup");
    DefaultSoundCloudTrackFormat value = new DefaultSoundCloudTrackFormat(
        trackId, protocol, mimeType, lookupUrl);
    check(value.getTrackId() == trackId && value.getProtocol() == protocol
        && value.getMimeType() == mimeType && value.getLookupUrl() == lookupUrl,
        "constructor references");
    check(value instanceof SoundCloudTrackFormat, "interface implementation");
    check(!value.equals(new DefaultSoundCloudTrackFormat(
        trackId, protocol, mimeType, lookupUrl)), "identity equality");
    check(value.toString().startsWith(value.getClass().getName() + "@"), "object string");

    DefaultSoundCloudTrackFormat nulls = new DefaultSoundCloudTrackFormat(null, null, null, null);
    check(nulls.getTrackId() == null && nulls.getProtocol() == null
        && nulls.getMimeType() == null && nulls.getLookupUrl() == null, "null preservation");
    for (String fieldName : new String[] {"trackId", "protocol", "mimeType", "lookupUrl"}) {
      Field field = DefaultSoundCloudTrackFormat.class.getDeclaredField(fieldName);
      field.setAccessible(true);
      check(field.get(value) == getterValue(value, fieldName), "field value " + fieldName);
    }
  }

  private static Object getterValue(DefaultSoundCloudTrackFormat value, String fieldName) {
    switch (fieldName) {
      case "trackId": return value.getTrackId();
      case "protocol": return value.getProtocol();
      case "mimeType": return value.getMimeType();
      case "lookupUrl": return value.getLookupUrl();
      default: throw new AssertionError(fieldName);
    }
  }

  private static void checkField(Class<?> type, String name) throws Exception {
    Field field = type.getDeclaredField(name);
    check(field.getType() == String.class
        && field.getModifiers() == (Modifier.PRIVATE | Modifier.FINAL)
        && !field.isSynthetic(), "field metadata " + name);
  }

  private static void checkMethod(Class<?> type, String name) throws Exception {
    Method method = type.getDeclaredMethod(name);
    check(method.getReturnType() == String.class && method.getParameterCount() == 0
        && method.getModifiers() == Modifier.PUBLIC && !method.isBridge() && !method.isSynthetic(),
        "method metadata " + name);
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const SOUND_CLOUD_AUDIO_SOURCE_MANAGER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudDataLoader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudDataReader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudFormatHandler;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudPlaylistLoader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudAudioTrack;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudClientIdTracker;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudDataLoader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudDataReader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudFormatHandler;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudPlaylistLoader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudTrackFormat;
import com.sedmelluq.discord.lavaplayer.tools.FriendlyException;
import com.sedmelluq.discord.lavaplayer.tools.JsonBrowser;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpConfigurable;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterfaceManager;
import com.sedmelluq.discord.lavaplayer.track.AudioItem;
import com.sedmelluq.discord.lavaplayer.track.AudioPlaylist;
import com.sedmelluq.discord.lavaplayer.track.AudioReference;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.net.URI;
import java.util.ArrayDeque;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Queue;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;
import java.util.function.Consumer;
import java.util.function.Function;
import java.util.regex.Pattern;
import org.apache.http.ProtocolVersion;
import org.apache.http.client.config.RequestConfig;
import org.apache.http.client.methods.CloseableHttpResponse;
import org.apache.http.client.methods.HttpUriRequest;
import org.apache.http.client.protocol.HttpClientContext;
import org.apache.http.entity.ContentType;
import org.apache.http.entity.StringEntity;
import org.apache.http.impl.client.HttpClientBuilder;
import org.apache.http.message.BasicStatusLine;

public final class GateSoundCloudAudioSourceManager {
  private static final String MOBILE_REGEX =
      "^(?:http://|https://|)soundcloud\\.app\\.goo\\.gl/([a-zA-Z0-9-_]+)/?(?:\\?.*|)$";
  private static final String TRACK_REGEX = "^(?:http://|https://|)(?:www\\.|)(?:m\\.|)"
      + "soundcloud\\.com/([a-zA-Z0-9-_]+)/([a-zA-Z0-9-_]+)/?(?:\\?.*|)$";
  private static final String SHORT_REGEX =
      "^https://on.soundcloud\\.com/[a-zA-Z0-9-_]+/?(?:\\?.*|)$";
  private static final String UNLISTED_REGEX = "^(?:http://|https://|)(?:www\\.|)(?:m\\.|)"
      + "soundcloud\\.com/([a-zA-Z0-9-_]+)/([a-zA-Z0-9-_]+)/s-"
      + "([a-zA-Z0-9-_]+)(?:\\?.*|)$";
  private static final String LIKED_REGEX = "^(?:http://|https://|)(?:www\\.|)(?:m\\.|)"
      + "soundcloud\\.com/([a-zA-Z0-9-_]+)/likes/?(?:\\?.*|)$";
  private static final String SEARCH_REGEX =
      "scsearch\\[([0-9]{1,9}),([0-9]{1,9})\\]:\\s*(.*)\\s*";
  private static final AudioTrackInfo INFO =
      new AudioTrackInfo("title", "author", 123L, "format-id", false, "https://track");
  private static final SoundCloudTrackFormat FORMAT = proxy(SoundCloudTrackFormat.class);

  public static void main(String[] args) throws Exception {
    check(args.length == 1 && (args[0].equals("reference") || args[0].equals("candidate")),
        "expected disposition");
    reflectionContract();
    constructionAndHttpContract();
    serializationContract();
    routingContract();
    trackLoadingContract();
    searchAndLikesContract();
    System.out.println("public-concrete,27-fields,2-constructors,16-exported-methods;"
        + "defaults,builder,dependency-capture,http-config,source-name,always-encodable,"
        + "empty-encode,decode-owner,track-routing,playlist-fallback,load-pipeline,preview-filter,"
        + "search-range-cap,liked-tracks,blocked-omit,resource-close,friendly-failures,generics");
  }

  private static void reflectionContract() throws Exception {
    Class<SoundCloudAudioSourceManager> type = SoundCloudAudioSourceManager.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(),
            new Class<?>[] {AudioSourceManager.class, HttpConfigurable.class}), "class metadata");
    check(type.getDeclaredFields().length == 27 && type.getDeclaredMethods().length == 29,
        "declared shape");
    check(type.getDeclaredConstructors().length == 2, "constructor count");
    for (Field field : type.getDeclaredFields()) {
      check(field.getModifiers() == (field.getName().endsWith("Pattern")
          || field.getName().equals("DEFAULT_SEARCH_RESULTS")
          || field.getName().equals("MAXIMUM_SEARCH_RESULTS")
          || field.getName().endsWith("REGEX") || field.getName().startsWith("SEARCH_")
          || field.getName().equals("FULL_TRACK_UNAVAILABLE_MARKER")
          ? Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL
          : Modifier.PRIVATE | Modifier.FINAL), "field modifiers " + field.getName());
    }
    check(intField("DEFAULT_SEARCH_RESULTS") == 10 && intField("MAXIMUM_SEARCH_RESULTS") == 200,
        "search constants");
    check(stringField("MOBILE_URL_REGEX").equals(MOBILE_REGEX)
        && stringField("TRACK_URL_REGEX").equals(TRACK_REGEX)
        && stringField("SHORT_TRACK_URL_REGEX").equals(SHORT_REGEX)
        && stringField("UNLISTED_URL_REGEX").equals(UNLISTED_REGEX)
        && stringField("LIKED_URL_REGEX").equals(LIKED_REGEX)
        && stringField("SEARCH_REGEX").equals(SEARCH_REGEX)
        && stringField("SEARCH_PREFIX").equals("scsearch")
        && stringField("SEARCH_PREFIX_DEFAULT").equals("scsearch:")
        && stringField("FULL_TRACK_UNAVAILABLE_MARKER").equals("SUB_HIGH_TIER"),
        "string constants");
    check(pattern("mobileUrlPattern").pattern().equals(MOBILE_REGEX)
        && pattern("trackUrlPattern").pattern().equals(TRACK_REGEX)
        && pattern("shortTrackUrlPattern").pattern().equals(SHORT_REGEX)
        && pattern("unlistedUrlPattern").pattern().equals(UNLISTED_REGEX)
        && pattern("likedUrlPattern").pattern().equals(LIKED_REGEX)
        && pattern("searchPattern").pattern().equals(SEARCH_REGEX), "compiled patterns");
    Method configureRequests = type.getDeclaredMethod("configureRequests", Function.class);
    Method configureBuilder = type.getDeclaredMethod("configureBuilder", Consumer.class);
    check(configureRequests.getGenericParameterTypes()[0].getTypeName().equals(
        "java.util.function.Function<org.apache.http.client.config.RequestConfig, "
            + "org.apache.http.client.config.RequestConfig>"), "request generic");
    check(configureBuilder.getGenericParameterTypes()[0].getTypeName().equals(
        "java.util.function.Consumer<org.apache.http.impl.client.HttpClientBuilder>"),
        "builder generic");
  }

  private static void constructionAndHttpContract() throws Exception {
    ReaderHandler readerState = new ReaderHandler();
    SoundCloudDataReader reader = readerState.proxy();
    SoundCloudDataLoader loader = (http, url) -> JsonBrowser.NULL_BROWSER;
    SoundCloudFormatHandler handler = formatHandler();
    SoundCloudPlaylistLoader playlists = (url, manager, factory) -> null;
    SoundCloudAudioSourceManager manager =
        new SoundCloudAudioSourceManager(true, reader, loader, handler, playlists, true);
    check(field("dataReader").get(manager) == reader && field("dataLoader").get(manager) == loader
        && field("formatHandler").get(manager) == handler
        && field("playlistLoader").get(manager) == playlists
        && field("allowSearch").getBoolean(manager)
        && field("filterOutPreviewTracks").getBoolean(manager), "dependency capture");
    check(manager.getFormatHandler() == handler && manager.getSourceName().equals("soundcloud"),
        "basic getters");
    check(field("httpInterfaceManager").get(manager) != null
        && field("clientIdTracker").get(manager) != null, "http collaborators");
    SoundCloudAudioSourceManager legacy =
        new SoundCloudAudioSourceManager(false, reader, loader, handler, playlists);
    check(!field("allowSearch").getBoolean(legacy)
        && !field("filterOutPreviewTracks").getBoolean(legacy), "legacy constructor defaults");

    SoundCloudAudioSourceManager defaults = SoundCloudAudioSourceManager.createDefault();
    check(field("dataReader").get(defaults) instanceof DefaultSoundCloudDataReader
        && field("dataLoader").get(defaults) instanceof DefaultSoundCloudDataLoader
        && field("formatHandler").get(defaults) instanceof DefaultSoundCloudFormatHandler
        && field("playlistLoader").get(defaults) instanceof DefaultSoundCloudPlaylistLoader
        && field("allowSearch").getBoolean(defaults)
        && !field("filterOutPreviewTracks").getBoolean(defaults), "default factory");
    check(SoundCloudAudioSourceManager.builder() != SoundCloudAudioSourceManager.builder(),
        "fresh builders");

    RecordingHttpInterface http = new RecordingHttpInterface();
    ManagerHandler managerState = injectManager(manager, http);
    Function<RequestConfig, RequestConfig> requestConfig = value -> value;
    Consumer<HttpClientBuilder> builderConfig = value -> { };
    manager.configureRequests(requestConfig);
    manager.configureBuilder(builderConfig);
    check(manager.getHttpInterface() == http && managerState.requestConfig == requestConfig
        && managerState.builderConfig == builderConfig, "http delegation");
    manager.shutdown();
    check(managerState.closes == 0, "shutdown no-op");
    SoundCloudClientIdTracker tracker =
        (SoundCloudClientIdTracker) field("clientIdTracker").get(manager);
    Field clientId = SoundCloudClientIdTracker.class.getDeclaredField("clientId");
    clientId.setAccessible(true);
    clientId.set(tracker, "fixture-client-id");
    check(manager.getClientId().equals("fixture-client-id"), "client ID delegation");
  }

  private static void serializationContract() throws Exception {
    SoundCloudAudioSourceManager manager = manager(false, new ReaderHandler(), false, null);
    AudioTrack arbitrary = proxy(AudioTrack.class);
    check(manager.isTrackEncodable(null) && manager.isTrackEncodable(arbitrary),
        "always encodable");
    ByteArrayOutputStream bytes = new ByteArrayOutputStream();
    manager.encodeTrack(arbitrary, new DataOutputStream(bytes));
    check(bytes.size() == 0, "empty encoding");
    AudioTrack decoded = manager.decodeTrack(INFO,
        new DataInputStream(new ByteArrayInputStream(new byte[] {1, 2, 3})));
    check(decoded instanceof SoundCloudAudioTrack && decoded.getInfo() == INFO
        && decoded.getSourceManager() == manager, "decode owner");
    ExposedManager exposed = new ExposedManager(false, new ReaderHandler(), false, null);
    AudioTrack built = exposed.from(JsonBrowser.parse("{\"id\":\"44\"}"));
    check(built instanceof SoundCloudAudioTrack && built.getSourceManager() == exposed
        && built.getInfo() == INFO, "track data build");
  }

  private static void routingContract() throws Exception {
    AtomicInteger playlistCalls = new AtomicInteger();
    AtomicReference<String> playlistUrl = new AtomicReference<>();
    AudioPlaylist playlist = proxy(AudioPlaylist.class);
    SoundCloudPlaylistLoader playlists = (url, manager, factory) -> {
      playlistCalls.incrementAndGet();
      playlistUrl.set(url);
      check(factory.apply(INFO).getSourceManager() instanceof SoundCloudAudioSourceManager,
          "playlist factory");
      return playlist;
    };
    RoutingManager manager = new RoutingManager(false, playlists);
    AudioTrack direct = manager.track;
    check(manager.loadItem(null, new AudioReference(
        "https://m.soundcloud.com/user/song?x=1", null)) == direct
        && manager.loadedUrl.equals("https://soundcloud.com/user/song?x=1")
        && manager.loadFlags, "direct mobile track");
    check(manager.loadItem(null, new AudioReference(
        "https://soundcloud.com/user/song/s-secret", null)) == direct
        && manager.loadedUrl.equals("https://soundcloud.com/user/song/s-secret"),
        "unlisted track");
    check(playlistCalls.get() == 0, "single-track precedence");
    check(manager.loadItem(null, new AudioReference("not-a-track", null)) == playlist
        && playlistCalls.get() == 1 && playlistUrl.get().equals("not-a-track"),
        "playlist fallback");
    RoutingManager disabled = new RoutingManager(false, (url, http, factory) -> null);
    check(disabled.loadItem(null, new AudioReference("scsearch: query", null)) == null,
        "disabled search");
  }

  private static void trackLoadingContract() throws Exception {
    ReaderHandler state = new ReaderHandler();
    state.trackData = JsonBrowser.parse("{\"id\":\"11\"}");
    AtomicReference<String> loadedUrl = new AtomicReference<>();
    SoundCloudDataLoader loader = (http, url) -> {
      loadedUrl.set(url);
      return JsonBrowser.parse("{\"kind\":\"track\"}");
    };
    SoundCloudAudioSourceManager manager = manager(false, state, false, loader);
    RecordingHttpInterface http = new RecordingHttpInterface();
    injectManager(manager, http);
    AudioTrack loaded = manager.loadTrack("https://soundcloud.com/u/t");
    check(loaded instanceof SoundCloudAudioTrack && loaded.getInfo() == INFO
        && loadedUrl.get().equals("https://soundcloud.com/u/t") && http.closes == 1,
        "track pipeline and close");

    state.trackData = JsonBrowser.parse("{\"monetization_model\":\"SUB_HIGH_TIER\"}");
    SoundCloudAudioSourceManager filtered = manager(true, state, true, loader);
    RecordingHttpInterface filteredHttp = new RecordingHttpInterface();
    injectManager(filtered, filteredHttp);
    check(filtered.loadTrack("preview", true) == null && filteredHttp.closes == 1,
        "preview filter");
    RecordingHttpInterface allowedHttp = new RecordingHttpInterface();
    injectManager(filtered, allowedHttp);
    check(filtered.loadTrack("preview", false) instanceof SoundCloudAudioTrack
        && allowedHttp.closes == 1, "preview bypass");

    state.trackData = null;
    RecordingHttpInterface missingHttp = new RecordingHttpInterface();
    injectManager(manager, missingHttp);
    FriendlyException common = expect(FriendlyException.class,
        () -> manager.loadTrack("missing"));
    check(common.severity == FriendlyException.Severity.COMMON
        && common.getMessage().equals("This track is not available")
        && missingHttp.closes == 1, "missing track failure");

    SoundCloudDataLoader failingLoader = (value, url) -> { throw new IOException("load-sentinel"); };
    SoundCloudAudioSourceManager failing = manager(false, state, false, failingLoader);
    RecordingHttpInterface failingHttp = new RecordingHttpInterface();
    failingHttp.closeFailure = new IOException("close-sentinel");
    injectManager(failing, failingHttp);
    FriendlyException suspicious = expect(FriendlyException.class,
        () -> failing.loadTrack("failure"));
    check(suspicious.severity == FriendlyException.Severity.SUSPICIOUS
        && suspicious.getCause() instanceof IOException
        && suspicious.getCause().getSuppressed().length == 1, "IO wrapping and suppression");
  }

  private static void searchAndLikesContract() throws Exception {
    ReaderHandler state = new ReaderHandler();
    SoundCloudAudioSourceManager search = manager(true, state, true, null);
    RecordingHttpInterface searchHttp = new RecordingHttpInterface();
    searchHttp.responses.add(response(200, "{\"collection\":["
        + "{\"id\":\"1\",\"monetization_model\":\"SUB_HIGH_TIER\"},"
        + "{\"id\":\"2\"}]}", searchHttp.responseCloses));
    injectManager(search, searchHttp);
    AudioItem searchItem = search.loadItem(null, new AudioReference("scsearch[3,999]:hello", null));
    check(searchItem instanceof AudioPlaylist, "search playlist type");
    AudioPlaylist searchPlaylist = (AudioPlaylist) searchItem;
    check(searchPlaylist.isSearchResult() && searchPlaylist.getTracks().size() == 1,
        "search filtering");
    check(searchPlaylist.getName().equals("Search results for: hello"), "search name");
    check(searchHttp.uris.get(0).toString().equals(
        "https://api-v2.soundcloud.com/search/tracks?q=hello&offset=3&limit=200"), "search URI");
    check(searchHttp.closes == 1 && searchHttp.responseCloses.get() == 1,
        "search resource close");

    SoundCloudAudioSourceManager likes = manager(false, state, true, null);
    RecordingHttpInterface likedHttp = new RecordingHttpInterface();
    likedHttp.responses.add(response(200,
        "prefix {\"urn\":\"soundcloud:users:42\",\"username\":\"Alice\"} suffix",
        likedHttp.responseCloses));
    likedHttp.responses.add(response(200, "{\"collection\":["
        + "{\"track\":{\"id\":\"3\"}},"
        + "{\"track\":{\"id\":\"4\",\"blocked\":\"yes\"}},"
        + "{\"track\":{\"id\":\"5\",\"monetization_model\":\"SUB_HIGH_TIER\"}}]}",
        likedHttp.responseCloses));
    injectManager(likes, likedHttp);
    AudioItem likedItem = likes.loadItem(null,
        new AudioReference("https://soundcloud.com/alice/likes", null));
    check(likedItem instanceof AudioPlaylist, "liked playlist type");
    AudioPlaylist likedPlaylist = (AudioPlaylist) likedItem;
    check(likedPlaylist.getName().equals("Liked by Alice"), "liked playlist name");
    check(likedPlaylist.getTracks().size() == 1, "liked omissions");
    check(likedHttp.uris.get(0).toString().equals("https://soundcloud.com/alice/likes"),
        "liked page URI");
    check(likedHttp.uris.get(1).toString().equals(
        "https://api-v2.soundcloud.com/users/42/likes?limit=200&offset=0"),
        "liked API URI");
    check(likedHttp.closes == 1 && likedHttp.responseCloses.get() == 2,
        "liked resource close");
  }

  private static SoundCloudAudioSourceManager manager(
      boolean allowSearch, ReaderHandler state, boolean filter, SoundCloudDataLoader loader) {
    SoundCloudDataLoader actualLoader = loader == null
        ? (http, url) -> JsonBrowser.NULL_BROWSER : loader;
    return new SoundCloudAudioSourceManager(allowSearch, state.proxy(), actualLoader,
        formatHandler(), (url, http, factory) -> null, filter);
  }

  private static SoundCloudFormatHandler formatHandler() {
    return (SoundCloudFormatHandler) Proxy.newProxyInstance(
        SoundCloudFormatHandler.class.getClassLoader(),
        new Class<?>[] {SoundCloudFormatHandler.class}, (instance, method, arguments) -> {
          if (method.getName().equals("chooseBestFormat")) return FORMAT;
          if (method.getName().equals("buildFormatIdentifier")) return "format-id";
          return defaultValue(method.getReturnType());
        });
  }

  private static ManagerHandler injectManager(
      SoundCloudAudioSourceManager manager, RecordingHttpInterface http) throws Exception {
    ManagerHandler state = new ManagerHandler(http);
    field("httpInterfaceManager").set(manager, state.proxy());
    return state;
  }

  private static CloseableHttpResponse response(
      int status, String body, AtomicInteger closes) {
    return (CloseableHttpResponse) Proxy.newProxyInstance(
        CloseableHttpResponse.class.getClassLoader(),
        new Class<?>[] {CloseableHttpResponse.class}, (instance, method, arguments) -> {
          if (method.getName().equals("getStatusLine")) {
            return new BasicStatusLine(new ProtocolVersion("HTTP", 1, 1), status, "fixture");
          }
          if (method.getName().equals("getEntity")) {
            return new StringEntity(body, ContentType.APPLICATION_JSON);
          }
          if (method.getName().equals("close")) { closes.incrementAndGet(); return null; }
          return defaultValue(method.getReturnType());
        });
  }

  private static Field field(String name) throws Exception {
    Field field = SoundCloudAudioSourceManager.class.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }
  private static int intField(String name) throws Exception { return field(name).getInt(null); }
  private static String stringField(String name) throws Exception {
    return (String) field(name).get(null);
  }
  private static Pattern pattern(String name) throws Exception {
    return (Pattern) field(name).get(null);
  }

  private static final class ReaderHandler implements InvocationHandler {
    private JsonBrowser trackData = JsonBrowser.NULL_BROWSER;
    private SoundCloudDataReader proxy() {
      return (SoundCloudDataReader) Proxy.newProxyInstance(
          SoundCloudDataReader.class.getClassLoader(), new Class<?>[] {SoundCloudDataReader.class},
          this);
    }
    public Object invoke(Object instance, Method method, Object[] arguments) {
      if (method.getName().equals("findTrackData")) return trackData;
      if (method.getName().equals("readTrackFormats")) return Collections.singletonList(FORMAT);
      if (method.getName().equals("readTrackInfo")) return INFO;
      if (method.getName().equals("isTrackBlocked")) {
        return "yes".equals(((JsonBrowser) arguments[0]).get("blocked").text());
      }
      return defaultValue(method.getReturnType());
    }
  }

  private static final class ManagerHandler implements InvocationHandler {
    private final HttpInterface http;
    private Object requestConfig;
    private Object builderConfig;
    private int closes;
    private ManagerHandler(HttpInterface http) { this.http = http; }
    private HttpInterfaceManager proxy() {
      return (HttpInterfaceManager) Proxy.newProxyInstance(HttpInterfaceManager.class.getClassLoader(),
          new Class<?>[] {HttpInterfaceManager.class}, this);
    }
    public Object invoke(Object instance, Method method, Object[] arguments) {
      if (method.getName().equals("getInterface")) return http;
      if (method.getName().equals("configureRequests")) requestConfig = arguments[0];
      if (method.getName().equals("configureBuilder")) builderConfig = arguments[0];
      if (method.getName().equals("close")) closes++;
      return defaultValue(method.getReturnType());
    }
  }

  private static final class RecordingHttpInterface extends HttpInterface {
    private final Queue<CloseableHttpResponse> responses = new ArrayDeque<>();
    private final List<URI> uris = new java.util.ArrayList<>();
    private final AtomicInteger responseCloses = new AtomicInteger();
    private int closes;
    private IOException closeFailure;
    private RecordingHttpInterface() { super(null, HttpClientContext.create(), false, null); }
    public CloseableHttpResponse execute(HttpUriRequest request) throws IOException {
      uris.add(request.getURI());
      if (responses.isEmpty()) throw new IOException("no fixture response");
      return responses.remove();
    }
    public void close() throws IOException {
      closes++;
      if (closeFailure != null) throw closeFailure;
    }
  }

  private static class ExposedManager extends SoundCloudAudioSourceManager {
    private ExposedManager(
        boolean allowSearch, ReaderHandler state, boolean filter, SoundCloudDataLoader loader) {
      super(allowSearch, state.proxy(), loader == null
          ? (http, url) -> JsonBrowser.NULL_BROWSER : loader,
          formatHandler(), (url, http, factory) -> null, filter);
    }
    private AudioTrack from(JsonBrowser data) { return loadFromTrackData(data); }
  }

  private static final class RoutingManager extends SoundCloudAudioSourceManager {
    private final AudioTrack track = proxy(AudioTrack.class);
    private String loadedUrl;
    private boolean loadFlags;
    private RoutingManager(boolean search, SoundCloudPlaylistLoader playlists) {
      super(search, new ReaderHandler().proxy(), (http, url) -> JsonBrowser.NULL_BROWSER,
          formatHandler(), playlists);
    }
    public AudioTrack loadTrack(String url, boolean checkPreview) {
      loadedUrl = url;
      loadFlags = checkPreview;
      return track;
    }
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type) {
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] {type},
        (instance, method, arguments) -> defaultValue(method.getReturnType()));
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == byte.class) return (byte) 0;
    if (type == short.class) return (short) 0;
    if (type == int.class) return 0;
    if (type == long.class) return 0L;
    if (type == float.class) return 0F;
    if (type == double.class) return 0D;
    if (type == char.class) return (char) 0;
    return null;
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw error;
      return type.cast(error);
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
  private interface Operation { void run() throws Exception; }
}
"#;

const SOUND_CLOUD_AUDIO_SOURCE_MANAGER_BUILDER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudDataLoader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudDataReader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudFormatHandler;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudPlaylistLoader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudDataLoader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudDataReader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudFormatHandler;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudPlaylistLoader;
import com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudTrackFormat;
import com.sedmelluq.discord.lavaplayer.tools.JsonBrowser;
import com.sedmelluq.discord.lavaplayer.track.AudioPlaylist;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;

public final class GateSoundCloudAudioSourceManagerBuilder {
  private static final String OWNER =
      "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudAudioSourceManager";

  public static void main(String[] args) throws Exception {
    reflectionContract();
    defaultsAndFluentSetters();
    defaultBuildContract();
    explicitAndFactoryBuildContract();
    System.out.println("public-static,7-fields,1-constructor,8-methods;"
        + "defaults,self-return,null-reset,fresh-defaults,explicit-capture,playlist-precedence,"
        + "factory-order,factory-null-fallback,policy-forwarding");
  }

  private static void reflectionContract() throws Exception {
    Class<?> type = SoundCloudAudioSourceManager.Builder.class;
    check(type.getModifiers() == (Modifier.PUBLIC | Modifier.STATIC)
        && type.getSuperclass() == Object.class && type.getInterfaces().length == 0,
        "class metadata");
    check(type.getEnclosingClass() == SoundCloudAudioSourceManager.class
        && type.getDeclaredFields().length == 7 && type.getDeclaredMethods().length == 8
        && type.getDeclaredConstructors().length == 1, "declared shape");
    checkField(type, "allowSearch", boolean.class);
    checkField(type, "dataReader", SoundCloudDataReader.class);
    checkField(type, "dataLoader", SoundCloudDataLoader.class);
    checkField(type, "formatHandler", SoundCloudFormatHandler.class);
    checkField(type, "playlistLoader", SoundCloudPlaylistLoader.class);
    checkField(type, "playlistLoaderFactory", Class.forName(OWNER + "$Builder$PlaylistLoaderFactory"));
    checkField(type, "filterOutPreviewTracks", boolean.class);
    check(type.getDeclaredConstructor().getModifiers() == Modifier.PUBLIC, "constructor metadata");
    for (Method method : type.getDeclaredMethods()) {
      check(method.getModifiers() == Modifier.PUBLIC && !method.isBridge()
          && !method.isSynthetic() && !method.isVarArgs()
          && method.getExceptionTypes().length == 0, "method metadata " + method.getName());
    }
  }

  private static void defaultsAndFluentSetters() throws Exception {
    SoundCloudAudioSourceManager.Builder builder = new SoundCloudAudioSourceManager.Builder();
    check(field("allowSearch").getBoolean(builder)
        && !field("filterOutPreviewTracks").getBoolean(builder)
        && field("dataReader").get(builder) == null && field("dataLoader").get(builder) == null
        && field("formatHandler").get(builder) == null
        && field("playlistLoader").get(builder) == null
        && field("playlistLoaderFactory").get(builder) == null, "builder defaults");

    SoundCloudDataReader reader = proxy(SoundCloudDataReader.class);
    SoundCloudDataLoader loader = (http, url) -> JsonBrowser.NULL_BROWSER;
    SoundCloudFormatHandler format = proxy(SoundCloudFormatHandler.class);
    SoundCloudPlaylistLoader playlist = (url, manager, factory) -> null;
    check(builder.withAllowSearch(false) == builder && builder.withDataReader(reader) == builder
        && builder.withDataLoader(loader) == builder && builder.withFormatHandler(format) == builder
        && builder.withPlaylistLoader(playlist) == builder
        && builder.withFilterOutPreviewTracks(true) == builder, "fluent identity");
    check(!field("allowSearch").getBoolean(builder)
        && field("filterOutPreviewTracks").getBoolean(builder)
        && field("dataReader").get(builder) == reader && field("dataLoader").get(builder) == loader
        && field("formatHandler").get(builder) == format
        && field("playlistLoader").get(builder) == playlist, "setter capture");
    check(builder.withDataReader(null) == builder && builder.withDataLoader(null) == builder
        && builder.withFormatHandler(null) == builder && builder.withPlaylistLoader(null) == builder,
        "null reset identity");
  }

  private static void defaultBuildContract() throws Exception {
    SoundCloudAudioSourceManager.Builder builder = SoundCloudAudioSourceManager.builder();
    SoundCloudAudioSourceManager first = builder.build();
    SoundCloudAudioSourceManager second = builder.build();
    check(managerField("allowSearch").getBoolean(first)
        && !managerField("filterOutPreviewTracks").getBoolean(first), "default policies");
    check(managerField("dataReader").get(first) instanceof DefaultSoundCloudDataReader
        && managerField("dataLoader").get(first) instanceof DefaultSoundCloudDataLoader
        && managerField("formatHandler").get(first) instanceof DefaultSoundCloudFormatHandler
        && managerField("playlistLoader").get(first) instanceof DefaultSoundCloudPlaylistLoader,
        "default collaborators");
    for (String name : Arrays.asList("dataReader", "dataLoader", "formatHandler", "playlistLoader")) {
      check(managerField(name).get(first) != managerField(name).get(second), "fresh " + name);
    }
  }

  private static void explicitAndFactoryBuildContract() throws Exception {
    SoundCloudDataReader reader = proxy(SoundCloudDataReader.class);
    SoundCloudDataLoader loader = (http, url) -> JsonBrowser.NULL_BROWSER;
    SoundCloudFormatHandler format = proxy(SoundCloudFormatHandler.class);
    SoundCloudPlaylistLoader playlist = (url, manager, factory) -> proxy(AudioPlaylist.class);
    AtomicInteger factoryCalls = new AtomicInteger();
    AtomicReference<Object[]> factoryArguments = new AtomicReference<>();
    Object factory = factory((instance, method, arguments) -> {
      factoryCalls.incrementAndGet();
      factoryArguments.set(arguments);
      return playlist;
    });

    SoundCloudAudioSourceManager.Builder explicit = new SoundCloudAudioSourceManager.Builder()
        .withAllowSearch(false).withDataReader(reader).withDataLoader(loader)
        .withFormatHandler(format).withPlaylistLoader(playlist)
        .withFilterOutPreviewTracks(true);
    setFactory(explicit, factory);
    SoundCloudAudioSourceManager direct = explicit.build();
    check(factoryCalls.get() == 0 && managerField("dataReader").get(direct) == reader
        && managerField("dataLoader").get(direct) == loader
        && managerField("formatHandler").get(direct) == format
        && managerField("playlistLoader").get(direct) == playlist
        && !managerField("allowSearch").getBoolean(direct)
        && managerField("filterOutPreviewTracks").getBoolean(direct), "explicit precedence");

    SoundCloudAudioSourceManager.Builder factoryBuilder = new SoundCloudAudioSourceManager.Builder()
        .withDataReader(reader).withDataLoader(loader).withFormatHandler(format);
    setFactory(factoryBuilder, factory);
    SoundCloudAudioSourceManager made = factoryBuilder.build();
    Object[] arguments = factoryArguments.get();
    check(factoryCalls.get() == 1 && arguments.length == 3 && arguments[0] == reader
        && arguments[1] == loader && arguments[2] == format
        && managerField("playlistLoader").get(made) == playlist, "factory order and result");

    Object nullFactory = factory((instance, method, argumentsValue) -> null);
    SoundCloudAudioSourceManager.Builder fallbackBuilder = new SoundCloudAudioSourceManager.Builder()
        .withDataReader(reader).withDataLoader(loader).withFormatHandler(format);
    setFactory(fallbackBuilder, nullFactory);
    SoundCloudAudioSourceManager fallback = fallbackBuilder.build();
    Object fallbackPlaylist = managerField("playlistLoader").get(fallback);
    check(fallbackPlaylist instanceof DefaultSoundCloudPlaylistLoader
        && objectField(fallbackPlaylist, "dataReader") == reader
        && objectField(fallbackPlaylist, "dataLoader") == loader
        && objectField(fallbackPlaylist, "formatHandler") == format, "factory null fallback");
  }

  private static Object factory(java.lang.reflect.InvocationHandler handler) throws Exception {
    Class<?> type = Class.forName(OWNER + "$Builder$PlaylistLoaderFactory");
    return Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] {type}, handler);
  }

  private static void setFactory(SoundCloudAudioSourceManager.Builder builder, Object factory)
      throws Exception {
    Class<?> type = Class.forName(OWNER + "$Builder$PlaylistLoaderFactory");
    Method method = SoundCloudAudioSourceManager.Builder.class
        .getDeclaredMethod("withPlaylistLoaderFactory", type);
    check(method.invoke(builder, factory) == builder, "factory fluent identity");
  }

  private static Field field(String name) throws Exception {
    Field field = SoundCloudAudioSourceManager.Builder.class.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }

  private static Field managerField(String name) throws Exception {
    Field field = SoundCloudAudioSourceManager.class.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }

  private static Object objectField(Object owner, String name) throws Exception {
    Field field = owner.getClass().getDeclaredField(name);
    field.setAccessible(true);
    return field.get(owner);
  }

  private static void checkField(Class<?> owner, String name, Class<?> type) throws Exception {
    Field field = owner.getDeclaredField(name);
    check(field.getType() == type && field.getModifiers() == Modifier.PRIVATE,
        "field metadata " + name);
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type) {
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] {type},
        (instance, method, arguments) -> defaultValue(method.getReturnType()));
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == byte.class) return (byte) 0;
    if (type == short.class) return (short) 0;
    if (type == int.class) return 0;
    if (type == long.class) return 0L;
    if (type == float.class) return 0F;
    if (type == double.class) return 0D;
    if (type == char.class) return (char) 0;
    return null;
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const LOCAL_SEEKABLE_INPUT_STREAM_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.local.LocalSeekableInputStream;
import com.sedmelluq.discord.lavaplayer.tools.io.ExtendedBufferedInputStream;
import com.sedmelluq.discord.lavaplayer.tools.io.SeekableInputStream;
import com.sedmelluq.discord.lavaplayer.track.info.AudioTrackInfoProvider;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.nio.channels.FileChannel;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;

public final class GateLocalSeekableInputStream {
  public static void main(String[] args) throws Exception {
    reflectionContract();
    constructionAndReading();
    seekingAndReset();
    failuresAndClose();
    System.out.println(
        "construction=length,zero-position,5-fields;"
        + "reads=single,bulk,skip,available,eof-quirk;"
        + "seek=hard,buffer-discard,beyond-eof,negative;"
        + "lifecycle=reset,close,missing,null;"
        + "reflection=public-concrete,1-constructor,11-methods");
  }

  private static void reflectionContract() throws Exception {
    Class<LocalSeekableInputStream> type = LocalSeekableInputStream.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == SeekableInputStream.class
        && type.getInterfaces().length == 0, "class metadata");

    Field[] fields = type.getDeclaredFields();
    check(fields.length == 5, "field count");
    checkFieldName(type.getDeclaredField("log"), "org.slf4j.Logger",
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type.getDeclaredField("inputStream"), FileInputStream.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField(type.getDeclaredField("channel"), FileChannel.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField(type.getDeclaredField("bufferedStream"), ExtendedBufferedInputStream.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField(type.getDeclaredField("position"), long.class, Modifier.PRIVATE);

    Constructor<?> constructor = type.getDeclaredConstructor(File.class);
    check(constructor.getModifiers() == Modifier.PUBLIC && !constructor.isVarArgs()
        && constructor.getExceptionTypes().length == 0
        && type.getDeclaredConstructors().length == 1, "constructor metadata");
    check(type.getDeclaredMethods().length == 11, "method count");
    checkMethod(type.getDeclaredMethod("read"), int.class, Modifier.PUBLIC, true);
    checkMethod(type.getDeclaredMethod("read", byte[].class, int.class, int.class),
        int.class, Modifier.PUBLIC, true);
    checkMethod(type.getDeclaredMethod("skip", long.class),
        long.class, Modifier.PUBLIC, true);
    checkMethod(type.getDeclaredMethod("available"), int.class, Modifier.PUBLIC, true);
    checkMethod(type.getDeclaredMethod("reset"), void.class,
        Modifier.PUBLIC | Modifier.SYNCHRONIZED, true);
    checkMethod(type.getDeclaredMethod("markSupported"),
        boolean.class, Modifier.PUBLIC, false);
    checkMethod(type.getDeclaredMethod("close"), void.class, Modifier.PUBLIC, true);
    checkMethod(type.getDeclaredMethod("getPosition"), long.class, Modifier.PUBLIC, false);
    checkMethod(type.getDeclaredMethod("canSeekHard"), boolean.class, Modifier.PUBLIC, false);
    Method providers = type.getDeclaredMethod("getTrackInfoProviders");
    checkMethod(providers, List.class, Modifier.PUBLIC, false);
    check(providers.getGenericReturnType().getTypeName().equals(
        "java.util.List<com.sedmelluq.discord.lavaplayer.track.info.AudioTrackInfoProvider>"),
        "provider signature");
    checkMethod(type.getDeclaredMethod("seekHard", long.class),
        void.class, Modifier.PROTECTED, true);

    Field log = type.getDeclaredField("log");
    log.setAccessible(true);
    check(log.get(null) != null, "logger initialized");
  }

  private static void constructionAndReading() throws Exception {
    Path path = Files.createTempFile("mantle-local-stream-read-", ".bin");
    Files.write(path, new byte[] { 10, 20, 30, 40, 50 });
    LocalSeekableInputStream stream = new LocalSeekableInputStream(path.toFile());
    check(stream.getContentLength() == 5L && stream.getMaxSkipDistance() == 0L
        && stream.getPosition() == 0L && stream.canSeekHard() && !stream.markSupported(),
        "initial stream state");
    check(stream.available() == 5, "initial available");

    Field inputField = LocalSeekableInputStream.class.getDeclaredField("inputStream");
    Field channelField = LocalSeekableInputStream.class.getDeclaredField("channel");
    Field bufferedField = LocalSeekableInputStream.class.getDeclaredField("bufferedStream");
    inputField.setAccessible(true);
    channelField.setAccessible(true);
    bufferedField.setAccessible(true);
    FileInputStream input = (FileInputStream) inputField.get(stream);
    check(channelField.get(stream) == input.getChannel()
        && bufferedField.get(stream) instanceof ExtendedBufferedInputStream,
        "captured stream identities");

    check(stream.read() == 10 && stream.getPosition() == 1L && stream.available() == 4,
        "single read");
    byte[] bytes = new byte[] { -1, -1, -1, -1 };
    check(stream.read(bytes, 1, 2) == 2
        && Arrays.equals(bytes, new byte[] { -1, 20, 30, -1 })
        && stream.getPosition() == 3L && stream.available() == 2, "bulk read");
    check(stream.skip(1L) == 1L && stream.getPosition() == 4L
        && stream.read() == 50 && stream.getPosition() == 5L, "skip and final byte");
    check(stream.read() == -1 && stream.getPosition() == 5L, "single eof position");
    check(stream.read(new byte[2], 0, 2) == -1 && stream.getPosition() == 4L,
        "bulk eof position quirk");
    check(stream.read(new byte[0], 0, 0) == 0 && stream.getPosition() == 4L,
        "zero-length read");
    check(stream.getTrackInfoProviders().isEmpty()
        && stream.getTrackInfoProviders()
            == Collections.<AudioTrackInfoProvider>emptyList(), "empty providers");
    stream.close();
    Files.delete(path);
  }

  private static void seekingAndReset() throws Exception {
    Path path = Files.createTempFile("mantle-local-stream-seek-", ".bin");
    Files.write(path, new byte[] { 1, 2, 3, 4, 5, 6 });
    ExposedStream stream = new ExposedStream(path.toFile());
    check(stream.read() == 1 && stream.read() == 2, "prime buffer");
    stream.hardSeek(4L);
    check(stream.getPosition() == 4L && stream.read() == 5,
        "hard seek discards buffered bytes");
    stream.seek(1L);
    check(stream.getPosition() == 1L && stream.read() == 2, "public hard seek");
    stream.seek(9L);
    check(stream.getPosition() == 9L && stream.read() == -1
        && stream.getPosition() == 9L, "seek beyond eof");
    expect(IllegalArgumentException.class, () -> stream.hardSeek(-1L));
    check(stream.getPosition() == 9L, "failed seek preserves logical position");
    IOException reset = expect(IOException.class, stream::reset);
    check(reset.getMessage().equals("mark/reset not supported"), "reset message");
    stream.close();
    Files.delete(path);
  }

  private static void failuresAndClose() throws Exception {
    Path path = Files.createTempFile("mantle-local-stream-close-", ".bin");
    Files.write(path, new byte[] { 7, 8 });
    LocalSeekableInputStream stream = new LocalSeekableInputStream(path.toFile());
    stream.close();
    stream.close();
    expect(IOException.class, stream::read);
    expect(IOException.class, () -> stream.seek(1L));
    Files.delete(path);

    Path missing = path.resolveSibling("missing-local-stream-" + System.nanoTime());
    RuntimeException missingError = expect(
        RuntimeException.class, () -> new LocalSeekableInputStream(missing.toFile()));
    check(missingError.getCause() instanceof FileNotFoundException, "missing cause");
    expect(NullPointerException.class, () -> new LocalSeekableInputStream(null));
  }

  private static void checkField(Field field, Class<?> type, int modifiers) {
    check(field.getModifiers() == modifiers && field.getType() == type && !field.isSynthetic(),
        field + " metadata");
  }

  private static void checkFieldName(Field field, String type, int modifiers) {
    check(field.getModifiers() == modifiers && field.getType().getName().equals(type)
        && !field.isSynthetic(), field + " metadata");
  }

  private static void checkMethod(
      Method method, Class<?> returnType, int modifiers, boolean throwsIo) {
    check(method.getModifiers() == modifiers && method.getReturnType() == returnType
        && !method.isBridge() && !method.isSynthetic() && !method.isVarArgs()
        && Arrays.equals(method.getExceptionTypes(), throwsIo
            ? new Class<?>[] { IOException.class } : new Class<?>[0]), method + " metadata");
  }

  private static <T extends Throwable> T expect(
      Class<T> type, Operation operation) throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private static final class ExposedStream extends LocalSeekableInputStream {
    ExposedStream(File file) { super(file); }
    void hardSeek(long position) throws IOException { super.seekHard(position); }
  }

  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const LOCAL_AUDIO_TRACK_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.container.MediaContainerDescriptor;
import com.sedmelluq.discord.lavaplayer.container.MediaContainerHints;
import com.sedmelluq.discord.lavaplayer.container.MediaContainerProbe;
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.local.LocalAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.local.LocalAudioTrack;
import com.sedmelluq.discord.lavaplayer.tools.io.SeekableInputStream;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import com.sedmelluq.discord.lavaplayer.track.DelegatedAudioTrack;
import com.sedmelluq.discord.lavaplayer.track.InternalAudioTrack;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioTrackExecutor;
import com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor;
import java.io.File;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;

public final class GateLocalAudioTrack {
  public static void main(String[] args) throws Exception {
    structureAndConstruction();
    cloningAndIdentity();
    processing();
    processingFailures();
    System.out.println(
        "construction=info,file,descriptor,source,nulls;"
        + "clone=fresh,info,descriptor,source;"
        + "process=factory,stream,assign,delegate,close;"
        + "failures=factory,cast,delegate,close;"
        + "reflection=public-concrete,3-private-final-fields,1-constructor,4-methods");
  }

  private static void structureAndConstruction() throws Exception {
    Path path = Files.createTempFile("mantle-local-track-", ".bin");
    AudioTrackInfo info = info(path);
    MediaContainerProbe probe = new ProbeState(null).proxy();
    MediaContainerDescriptor descriptor = new MediaContainerDescriptor(probe, "settings");
    LocalAudioSourceManager source = new LocalAudioSourceManager();
    LocalAudioTrack track = new LocalAudioTrack(info, descriptor, source);
    check(track.getInfo() == info && track.getContainerTrackFactory() == descriptor
        && track.getSourceManager() == source, "constructor identities");

    Class<LocalAudioTrack> type = LocalAudioTrack.class;
    check(Modifier.isPublic(type.getModifiers()) && !Modifier.isAbstract(type.getModifiers())
        && !Modifier.isFinal(type.getModifiers())
        && type.getSuperclass() == DelegatedAudioTrack.class
        && type.getInterfaces().length == 0, "class metadata");
    Field[] fields = type.getDeclaredFields();
    check(fields.length == 3, "field count");
    checkField(type.getDeclaredField("file"), File.class);
    checkField(type.getDeclaredField("containerTrackFactory"), MediaContainerDescriptor.class);
    checkField(type.getDeclaredField("sourceManager"), LocalAudioSourceManager.class);
    Field file = type.getDeclaredField("file");
    file.setAccessible(true);
    check(file.get(track).equals(path.toFile()), "captured file");

    Constructor<?> constructor = type.getDeclaredConstructor(
        AudioTrackInfo.class, MediaContainerDescriptor.class, LocalAudioSourceManager.class);
    check(constructor.getModifiers() == Modifier.PUBLIC && !constructor.isVarArgs()
        && constructor.getExceptionTypes().length == 0, "constructor metadata");
    check(type.getDeclaredConstructors().length == 1 && type.getDeclaredMethods().length == 4,
        "member counts");
    checkMethod(type.getDeclaredMethod("getContainerTrackFactory"),
        MediaContainerDescriptor.class, Modifier.PUBLIC, false);
    checkMethod(type.getDeclaredMethod("process", LocalAudioTrackExecutor.class),
        void.class, Modifier.PUBLIC, true);
    checkMethod(type.getDeclaredMethod("makeShallowClone"),
        AudioTrack.class, Modifier.PROTECTED, false);
    checkMethod(type.getDeclaredMethod("getSourceManager"),
        AudioSourceManager.class, Modifier.PUBLIC, false);

    LocalAudioTrack nullable = new LocalAudioTrack(info, null, null);
    check(nullable.getContainerTrackFactory() == null && nullable.getSourceManager() == null,
        "nullable collaborators");
    expect(NullPointerException.class, () -> new LocalAudioTrack(null, descriptor, source));
    Files.delete(path);
  }

  private static void cloningAndIdentity() throws Exception {
    Path path = Files.createTempFile("mantle-local-clone-", ".bin");
    AudioTrackInfo info = info(path);
    MediaContainerDescriptor descriptor =
        new MediaContainerDescriptor(new ProbeState(null).proxy(), "clone-settings");
    LocalAudioSourceManager source = new LocalAudioSourceManager();
    ExposedTrack original = new ExposedTrack(info, descriptor, source);
    LocalAudioTrack clone = (LocalAudioTrack) original.shallow();
    check(clone != original && clone.getClass() == LocalAudioTrack.class
        && clone.getInfo() == info && clone.getContainerTrackFactory() == descriptor
        && clone.getSourceManager() == source, "shallow clone identities");
    Field file = LocalAudioTrack.class.getDeclaredField("file");
    file.setAccessible(true);
    check(file.get(clone).equals(path.toFile()) && file.get(clone) != file.get(original),
        "clone file value and identity");
    Files.delete(path);
  }

  private static void processing() throws Exception {
    Path path = Files.createTempFile("mantle-local-process-", ".bin");
    Files.write(path, new byte[] { 42, 43, 44 });
    AudioTrackInfo info = info(path);
    InternalState internal = new InternalState(info);
    ProbeState probe = new ProbeState(internal.proxy());
    LocalAudioTrack track = new LocalAudioTrack(info,
        new MediaContainerDescriptor(probe.proxy(), "process-settings"),
        new LocalAudioSourceManager());
    track.process(null);
    check(probe.parameters.equals("process-settings") && probe.info == info
        && probe.firstByte == 42 && probe.creates == 1, "factory dispatch");
    check(internal.assigns == 1 && internal.processes == 1
        && internal.executor == null && !internal.noInterrupt, "delegate dispatch");
    expect(IOException.class, () -> probe.stream.seek(3));
    Files.delete(path);
  }

  private static void processingFailures() throws Exception {
    Path path = Files.createTempFile("mantle-local-fail-", ".bin");
    Files.write(path, new byte[] { 7, 8, 9 });
    AudioTrackInfo info = info(path);

    AssertionError factoryFailure = new AssertionError("factory-sentinel");
    ProbeState failingProbe = new ProbeState(null);
    failingProbe.failure = factoryFailure;
    LocalAudioTrack failingFactory = new LocalAudioTrack(info,
        new MediaContainerDescriptor(failingProbe.proxy(), "factory-failure"),
        new LocalAudioSourceManager());
    expectIdentity(factoryFailure, () -> failingFactory.process(null));
    expect(IOException.class, () -> failingProbe.stream.seek(3));

    ProbeState wrongType = new ProbeState(proxy(AudioTrack.class));
    LocalAudioTrack badCast = new LocalAudioTrack(info,
        new MediaContainerDescriptor(wrongType.proxy(), "bad-cast"),
        new LocalAudioSourceManager());
    expect(ClassCastException.class, () -> badCast.process(null));
    expect(IOException.class, () -> wrongType.stream.seek(3));

    Exception delegateFailure = new Exception("delegate-sentinel");
    InternalState internal = new InternalState(info);
    internal.processFailure = delegateFailure;
    ProbeState delegateProbe = new ProbeState(internal.proxy());
    LocalAudioTrack failingDelegate = new LocalAudioTrack(info,
        new MediaContainerDescriptor(delegateProbe.proxy(), "delegate-failure"),
        new LocalAudioSourceManager());
    expectIdentity(delegateFailure, () -> failingDelegate.process(null));
    check(internal.assigns == 1 && internal.processes == 1, "delegate failure dispatch");
    expect(IOException.class, () -> delegateProbe.stream.seek(3));

    AudioTrackInfo missing = new AudioTrackInfo(
        "title", "author", 3L, path.resolveSibling("missing-local-track").toString(),
        false, null);
    LocalAudioTrack missingTrack = new LocalAudioTrack(missing,
        new MediaContainerDescriptor(new ProbeState(null).proxy(), "missing"),
        new LocalAudioSourceManager());
    expect(RuntimeException.class, () -> missingTrack.process(null));
    Files.delete(path);
  }

  private static AudioTrackInfo info(Path path) {
    return new AudioTrackInfo(
        "title", "author", 3L, path.toString(), false, path.toUri().toString());
  }

  private static void checkField(Field field, Class<?> type) {
    check(field.getModifiers() == (Modifier.PRIVATE | Modifier.FINAL)
        && field.getType() == type && !field.isSynthetic(), field + " metadata");
  }

  private static void checkMethod(
      Method method, Class<?> returnType, int modifiers, boolean throwsException) {
    check(method.getModifiers() == modifiers && method.getReturnType() == returnType
        && !method.isBridge() && !method.isSynthetic() && !method.isVarArgs()
        && Arrays.equals(method.getExceptionTypes(), throwsException
            ? new Class<?>[] { Exception.class } : new Class<?>[0]), method + " metadata");
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type) {
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] { type },
        (instance, method, arguments) -> {
          if (method.getName().equals("toString")) return type.getSimpleName() + "Proxy";
          if (method.getName().equals("hashCode")) return System.identityHashCode(instance);
          if (method.getName().equals("equals")) return instance == arguments[0];
          return defaultValue(method.getReturnType());
        });
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == byte.class) return (byte) 0;
    if (type == short.class) return (short) 0;
    if (type == int.class) return 0;
    if (type == long.class) return 0L;
    if (type == float.class) return 0.0f;
    if (type == double.class) return 0.0d;
    if (type == char.class) return (char) 0;
    return null;
  }

  private static final class ProbeState {
    final AudioTrack result;
    String parameters;
    AudioTrackInfo info;
    SeekableInputStream stream;
    int firstByte;
    int creates;
    AssertionError failure;

    ProbeState(AudioTrack result) { this.result = result; }

    MediaContainerProbe proxy() {
      return (MediaContainerProbe) Proxy.newProxyInstance(
          MediaContainerProbe.class.getClassLoader(), new Class<?>[] { MediaContainerProbe.class },
          (instance, method, arguments) -> {
            switch (method.getName()) {
              case "getName": return "local-track-probe";
              case "matchesHints": return false;
              case "createTrack":
                creates++;
                parameters = (String) arguments[0];
                info = (AudioTrackInfo) arguments[1];
                stream = (SeekableInputStream) arguments[2];
                firstByte = stream.read();
                if (failure != null) throw failure;
                return result;
              case "toString": return "LocalTrackProbe";
              case "hashCode": return System.identityHashCode(instance);
              case "equals": return instance == arguments[0];
              default: return null;
            }
          });
    }
  }

  private static final class InternalState {
    final AudioTrackInfo info;
    int assigns;
    int processes;
    AudioTrackExecutor executor;
    boolean noInterrupt;
    Exception processFailure;

    InternalState(AudioTrackInfo info) { this.info = info; }

    InternalAudioTrack proxy() {
      return (InternalAudioTrack) Proxy.newProxyInstance(
          InternalAudioTrack.class.getClassLoader(), new Class<?>[] { InternalAudioTrack.class },
          (instance, method, arguments) -> {
            switch (method.getName()) {
              case "assignExecutor":
                assigns++;
                executor = (AudioTrackExecutor) arguments[0];
                noInterrupt = (Boolean) arguments[1];
                return null;
              case "process":
                processes++;
                if (processFailure != null) throw processFailure;
                return null;
              case "getInfo": return info;
              case "toString": return "InternalTrackProxy";
              case "hashCode": return System.identityHashCode(instance);
              case "equals": return instance == arguments[0];
              default: return defaultValue(method.getReturnType());
            }
          });
    }
  }

  private static final class ExposedTrack extends LocalAudioTrack {
    ExposedTrack(AudioTrackInfo info, MediaContainerDescriptor descriptor,
                 LocalAudioSourceManager source) {
      super(info, descriptor, source);
    }
    AudioTrack shallow() { return super.makeShallowClone(); }
  }

  private static void expect(Class<? extends Throwable> type, Operation operation) {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
    }
  }

  private static void expectIdentity(Throwable expected, Operation operation) {
    try {
      operation.run();
      throw new AssertionError("failure was swallowed");
    } catch (Throwable error) {
      check(error == expected, "failure identity");
    }
  }

  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const TRACK_MARKER_TRACKER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.track.TrackMarker;
import com.sedmelluq.discord.lavaplayer.track.TrackMarkerHandler.MarkerState;
import com.sedmelluq.discord.lavaplayer.track.TrackMarkerTracker;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

public final class GateTrackMarkerTracker {
  public static void main(String[] args) throws Exception {
    emptyAndViews();
    addRemoveAndClear();
    setAndTrigger();
    playbackAndSeek();
    reentrancyAndFailures();
    concurrentAdds();
    reflection();
    System.out.println(
        "empty=remove-null;views=live,distinct,unmodifiable,generic;"
        + "add=null,late-boundary,ordered,duplicates;remove=identity,deprecated-no-callback;"
        + "set=overwritten,removed,late;trigger=ordered,null,reentrant-clear;"
        + "checks=reached,bypassed,threshold,exception-after-remove;"
        + "concurrency=copy-on-write;reflection=class,1-field,1-constructor,9-public-methods");
  }

  private static void emptyAndViews() {
    TrackMarkerTracker tracker = new TrackMarkerTracker();
    check(tracker.remove() == null, "empty deprecated remove");
    List<TrackMarker> first = tracker.getMarkers();
    List<TrackMarker> second = tracker.getMarkers();
    check(first.isEmpty() && first != second, "distinct empty views");
    TrackMarker marker = marker(10, new ArrayList<>());
    tracker.add(marker, 0);
    check(first.size() == 1 && first.get(0) == marker, "live view");
    try {
      first.clear();
      throw new AssertionError("view was mutable");
    } catch (UnsupportedOperationException expected) {
      check(tracker.getMarkers().size() == 1, "unmodifiable view retained data");
    }
  }

  private static void addRemoveAndClear() {
    TrackMarkerTracker tracker = new TrackMarkerTracker();
    List<String> events = new ArrayList<>();
    tracker.add(null, 0);
    TrackMarker late = namedMarker("late", 5, events);
    tracker.add(late, 5);
    check(events.equals(Arrays.asList("late:LATE")) && tracker.getMarkers().isEmpty(),
        "boundary marker is late");

    TrackMarker first = namedMarker("first", 30, events);
    TrackMarker duplicate = namedMarker("duplicate", 40, events);
    tracker.add(first, 0);
    tracker.add(duplicate, 0);
    tracker.add(duplicate, 0);
    check(tracker.getMarkers().equals(Arrays.asList(first, duplicate, duplicate)),
        "insertion and duplicate order");
    tracker.remove(namedMarker("unknown", 40, events));
    check(events.size() == 1 && tracker.getMarkers().size() == 3, "identity removal");
    tracker.remove(duplicate);
    check(events.equals(Arrays.asList("late:LATE", "duplicate:REMOVED"))
        && tracker.getMarkers().equals(Arrays.asList(first, duplicate)),
        "single duplicate removal");
    check(tracker.remove() == first && events.size() == 2
        && tracker.getMarkers().equals(Arrays.asList(duplicate)),
        "deprecated remove has no callback");
    tracker.clear();
    check(tracker.getMarkers().isEmpty() && events.size() == 2, "clear has no callback");
  }

  private static void setAndTrigger() {
    TrackMarkerTracker tracker = new TrackMarkerTracker();
    List<String> events = new ArrayList<>();
    tracker.add(namedMarker("one", 30, events), 0);
    tracker.add(namedMarker("two", 40, events), 0);
    tracker.set(namedMarker("late", 20, events), 20);
    check(events.equals(Arrays.asList("one:OVERWRITTEN", "two:OVERWRITTEN", "late:LATE"))
        && tracker.getMarkers().isEmpty(), "set overwrite then late");
    TrackMarker future = namedMarker("future", 80, events);
    tracker.set(future, 20);
    check(tracker.getMarkers().equals(Arrays.asList(future)), "future set retained");
    tracker.set(null, 20);
    check(events.get(events.size() - 1).equals("future:REMOVED")
        && tracker.getMarkers().isEmpty(), "set null removes");
    tracker.add(namedMarker("null-state", 90, events), 0);
    tracker.trigger(null);
    check(events.get(events.size() - 1).equals("null-state:null")
        && tracker.getMarkers().isEmpty(), "null trigger state");
  }

  private static void playbackAndSeek() {
    TrackMarkerTracker tracker = new TrackMarkerTracker();
    List<String> events = new ArrayList<>();
    TrackMarker high = namedMarker("high", 100, events);
    TrackMarker low = namedMarker("low", 20, events);
    TrackMarker edge = namedMarker("edge", 50, events);
    tracker.add(high, 0);
    tracker.add(low, 0);
    tracker.add(edge, 0);
    tracker.checkPlaybackTimecode(50);
    check(events.equals(Arrays.asList("low:REACHED", "edge:REACHED"))
        && tracker.getMarkers().equals(Arrays.asList(high)), "playback threshold and order");
    tracker.checkSeekTimecode(99);
    check(events.size() == 2 && tracker.getMarkers().equals(Arrays.asList(high)),
        "seek below threshold");
    tracker.checkSeekTimecode(100);
    check(events.get(2).equals("high:BYPASSED") && tracker.getMarkers().isEmpty(),
        "seek boundary");
  }

  private static void reentrancyAndFailures() {
    TrackMarkerTracker tracker = new TrackMarkerTracker();
    List<String> events = new ArrayList<>();
    TrackMarker added = namedMarker("added", 200, events);
    tracker.add(new TrackMarker(20, state -> {
      events.add("outer:" + String.valueOf(state));
      tracker.add(added, 0);
    }), 0);
    tracker.trigger(MarkerState.ENDED);
    check(events.equals(Arrays.asList("outer:ENDED")) && tracker.getMarkers().isEmpty(),
        "trigger snapshot and final clear");

    RuntimeException triggerFailure = new RuntimeException("trigger-sentinel");
    TrackMarker failing = new TrackMarker(20, state -> { throw triggerFailure; });
    TrackMarker later = namedMarker("later", 30, events);
    tracker.add(failing, 0);
    tracker.add(later, 0);
    try {
      tracker.trigger(MarkerState.STOPPED);
      throw new AssertionError("trigger failure swallowed");
    } catch (RuntimeException error) {
      check(error == triggerFailure && tracker.getMarkers().equals(Arrays.asList(failing, later)),
          "trigger failure identity and retained list");
    }
    tracker.clear();

    RuntimeException checkFailure = new RuntimeException("check-sentinel");
    TrackMarker reached = new TrackMarker(20, state -> { throw checkFailure; });
    tracker.add(reached, 0);
    tracker.add(later, 0);
    try {
      tracker.checkPlaybackTimecode(30);
      throw new AssertionError("check failure swallowed");
    } catch (RuntimeException error) {
      check(error == checkFailure && tracker.getMarkers().equals(Arrays.asList(later)),
          "check removes before callback");
    }
  }

  private static void concurrentAdds() throws Exception {
    TrackMarkerTracker tracker = new TrackMarkerTracker();
    Thread[] threads = new Thread[4];
    for (int thread = 0; thread < threads.length; thread++) {
      final int offset = thread * 100;
      threads[thread] = new Thread(() -> {
        for (int index = 0; index < 100; index++) {
          tracker.add(new TrackMarker(1000 + offset + index, state -> { }), 0);
        }
      });
      threads[thread].start();
    }
    for (Thread thread : threads) thread.join();
    check(tracker.getMarkers().size() == 400, "concurrent add count");
    tracker.trigger(MarkerState.ENDED);
    check(tracker.getMarkers().isEmpty(), "concurrent collection clear");
  }

  private static void reflection() throws Exception {
    Class<TrackMarkerTracker> type = TrackMarkerTracker.class;
    check(Modifier.isPublic(type.getModifiers()) && !Modifier.isFinal(type.getModifiers())
        && type.getSuperclass() == Object.class && type.getInterfaces().length == 0,
        "class metadata");
    Field[] fields = type.getDeclaredFields();
    check(fields.length == 1 && fields[0].getName().equals("markerList")
        && fields[0].getType() == List.class
        && Modifier.isPrivate(fields[0].getModifiers())
        && Modifier.isFinal(fields[0].getModifiers()), "field metadata");
    ParameterizedType fieldType = (ParameterizedType) fields[0].getGenericType();
    check(fieldType.getActualTypeArguments()[0] == TrackMarker.class, "field generic type");
    Constructor<?>[] constructors = type.getDeclaredConstructors();
    check(constructors.length == 1 && Modifier.isPublic(constructors[0].getModifiers())
        && constructors[0].getParameterCount() == 0, "constructor metadata");
    int publicDeclared = 0;
    for (Method method : type.getDeclaredMethods()) {
      if (Modifier.isPublic(method.getModifiers())) publicDeclared++;
    }
    check(publicDeclared == 9, "public method count");
    check(type.getDeclaredMethod("remove").isAnnotationPresent(Deprecated.class),
        "deprecated remove annotation");
    ParameterizedType returnType = (ParameterizedType)
        type.getDeclaredMethod("getMarkers").getGenericReturnType();
    check(returnType.getActualTypeArguments()[0] == TrackMarker.class, "return generic type");
  }

  private static TrackMarker marker(long timecode, List<MarkerState> events) {
    return new TrackMarker(timecode, events::add);
  }

  private static TrackMarker namedMarker(String name, long timecode, List<String> events) {
    return new TrackMarker(timecode, state -> events.add(name + ":" + String.valueOf(state)));
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const AUDIO_TRACK_EXECUTOR_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.track.AudioTrackState;
import com.sedmelluq.discord.lavaplayer.track.TrackMarker;
import com.sedmelluq.discord.lavaplayer.track.TrackStateListener;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameBuffer;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameProvider;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioTrackExecutor;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

public final class GateAudioTrackExecutor {
  public static void main(String[] args) throws Exception {
    AudioFrameBuffer buffer = proxy(AudioFrameBuffer.class, (instance, method, arguments) ->
        defaultValue(method.getReturnType()));
    TrackStateListener listener = proxy(TrackStateListener.class, (instance, method, arguments) ->
        defaultValue(method.getReturnType()));
    TrackMarker marker = new TrackMarker(123456789L, state -> { });
    List<String> calls = new ArrayList<>();
    int[] positionReads = { 0 };
    int[] stateReads = { 0 };
    int[] failureReads = { 0 };

    AudioTrackExecutor executor = proxy(AudioTrackExecutor.class, (instance, method, arguments) -> {
      switch (method.getName()) {
        case "getAudioBuffer":
          calls.add("buffer");
          return buffer;
        case "execute":
          calls.add(arguments[0] == listener ? "execute-listener" : "execute-null");
          return null;
        case "stop":
          calls.add("stop");
          return null;
        case "getPosition":
          calls.add(positionReads[0]++ == 0 ? "position-min" : "position-max");
          return positionReads[0] == 1 ? Long.MIN_VALUE : Long.MAX_VALUE;
        case "setPosition":
          calls.add(((Long) arguments[0]) == Long.MIN_VALUE ? "set-min" : "set-max");
          return null;
        case "getState":
          calls.add(stateReads[0]++ == 0 ? "state-seeking" : "state-null");
          return stateReads[0] == 1 ? AudioTrackState.SEEKING : null;
        case "setMarker":
        case "addMarker":
        case "removeMarker":
          calls.add(method.getName() + (arguments[0] == marker ? "-marker" : "-null"));
          return null;
        case "failedBeforeLoad":
          calls.add(failureReads[0]++ == 0 ? "failed-true" : "failed-false");
          return failureReads[0] == 1;
        default:
          return defaultValue(method.getReturnType());
      }
    });

    check(executor.getAudioBuffer() == buffer, "audio buffer identity");
    executor.execute(listener);
    executor.execute(null);
    executor.stop();
    check(executor.getPosition() == Long.MIN_VALUE, "minimum position width");
    check(executor.getPosition() == Long.MAX_VALUE, "maximum position width");
    executor.setPosition(Long.MIN_VALUE);
    executor.setPosition(Long.MAX_VALUE);
    check(executor.getState() == AudioTrackState.SEEKING, "state identity");
    check(executor.getState() == null, "nullable state");
    executor.setMarker(marker);
    executor.setMarker(null);
    executor.addMarker(marker);
    executor.addMarker(null);
    executor.removeMarker(marker);
    executor.removeMarker(null);
    check(executor.failedBeforeLoad(), "failed true result");
    check(!executor.failedBeforeLoad(), "failed false result");
    check(executor instanceof AudioFrameProvider, "provider inheritance");
    check(calls.equals(Arrays.asList(
        "buffer", "execute-listener", "execute-null", "stop", "position-min", "position-max",
        "set-min", "set-max", "state-seeking", "state-null", "setMarker-marker",
        "setMarker-null", "addMarker-marker", "addMarker-null", "removeMarker-marker",
        "removeMarker-null", "failed-true", "failed-false")), "dispatch order");

    checkReflection();
    System.out.println(
        "dispatch=buffer,execute,stop,position,state,markers,failed;"
        + "edges=nulls,long-min-max,true-false,identity;"
        + "reflection=interface,AudioFrameProvider,0-fields,10-methods,0-constructors");
  }

  private static void checkReflection() throws Exception {
    Class<AudioTrackExecutor> type = AudioTrackExecutor.class;
    int modifiers = type.getModifiers();
    check(Modifier.isPublic(modifiers) && Modifier.isInterface(modifiers)
        && Modifier.isAbstract(modifiers) && !Modifier.isFinal(modifiers)
        && type.getSuperclass() == null && type.getTypeParameters().length == 0
        && type.getDeclaredAnnotations().length == 0, "interface structure");
    check(Arrays.equals(type.getInterfaces(), new Class<?>[] { AudioFrameProvider.class }),
        "direct interface");
    check(type.getDeclaredFields().length == 0 && type.getDeclaredMethods().length == 10
        && type.getDeclaredConstructors().length == 0, "member counts");

    checkAbstract(type.getDeclaredMethod("getAudioBuffer"), AudioFrameBuffer.class,
        new Class<?>[0]);
    checkAbstract(type.getDeclaredMethod("execute", TrackStateListener.class), void.class,
        new Class<?>[] { TrackStateListener.class });
    checkAbstract(type.getDeclaredMethod("stop"), void.class, new Class<?>[0]);
    checkAbstract(type.getDeclaredMethod("getPosition"), long.class, new Class<?>[0]);
    checkAbstract(type.getDeclaredMethod("setPosition", long.class), void.class,
        new Class<?>[] { long.class });
    checkAbstract(type.getDeclaredMethod("getState"), AudioTrackState.class, new Class<?>[0]);
    checkAbstract(type.getDeclaredMethod("setMarker", TrackMarker.class), void.class,
        new Class<?>[] { TrackMarker.class });
    checkAbstract(type.getDeclaredMethod("addMarker", TrackMarker.class), void.class,
        new Class<?>[] { TrackMarker.class });
    checkAbstract(type.getDeclaredMethod("removeMarker", TrackMarker.class), void.class,
        new Class<?>[] { TrackMarker.class });
    checkAbstract(type.getDeclaredMethod("failedBeforeLoad"), boolean.class, new Class<?>[0]);
  }

  private static void checkAbstract(Method method, Class<?> returnType,
      Class<?>[] parameterTypes) {
    int modifiers = method.getModifiers();
    check(Modifier.isPublic(modifiers) && Modifier.isAbstract(modifiers)
        && !Modifier.isStatic(modifiers) && !Modifier.isFinal(modifiers)
        && !method.isDefault() && !method.isBridge() && !method.isSynthetic(),
        method.getName() + " modifiers");
    check(method.getReturnType() == returnType
        && Arrays.equals(method.getParameterTypes(), parameterTypes)
        && method.getExceptionTypes().length == 0
        && method.getTypeParameters().length == 0
        && method.getDeclaredAnnotations().length == 0,
        method.getName() + " metadata");
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type, InvocationHandler handler) {
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] { type }, handler);
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == byte.class) return (byte) 0;
    if (type == short.class) return (short) 0;
    if (type == int.class) return 0;
    if (type == long.class) return 0L;
    if (type == float.class) return 0.0f;
    if (type == double.class) return 0.0d;
    if (type == char.class) return (char) 0;
    return null;
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const INTERNAL_AUDIO_TRACK_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.player.AudioPlayerManager;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.InternalAudioTrack;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameProvider;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioTrackExecutor;
import com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

public final class GateInternalAudioTrack {
  public static void main(String[] args) throws Exception {
    AudioTrackExecutor active = proxy(AudioTrackExecutor.class, (instance, method, arguments) ->
        defaultValue(method.getReturnType()));
    AudioTrackExecutor custom = proxy(AudioTrackExecutor.class, (instance, method, arguments) ->
        defaultValue(method.getReturnType()));
    AudioPlayerManager manager = proxy(AudioPlayerManager.class, (instance, method, arguments) ->
        defaultValue(method.getReturnType()));
    LocalAudioTrackExecutor local = allocate(LocalAudioTrackExecutor.class);
    Exception processFailure = new Exception("process-sentinel");
    List<String> calls = new ArrayList<>();

    InternalAudioTrack track = proxy(InternalAudioTrack.class, (instance, method, arguments) -> {
      switch (method.getName()) {
        case "assignExecutor":
          check(arguments[0] == active, "assigned executor identity");
          calls.add("assign:" + arguments[1]);
          return null;
        case "getActiveExecutor":
          calls.add("active");
          return active;
        case "process":
          check(arguments[0] == local, "process executor identity");
          calls.add("process");
          throw processFailure;
        case "createLocalExecutor":
          check(arguments[0] == manager, "manager identity");
          calls.add("create");
          return custom;
        default:
          return defaultValue(method.getReturnType());
      }
    });

    track.assignExecutor(active, true);
    track.assignExecutor(active, false);
    check(track.getActiveExecutor() == active, "active executor return identity");
    try {
      track.process(local);
      throw new AssertionError("process exception was swallowed");
    } catch (Exception error) {
      check(error == processFailure, "process checked exception identity");
    }
    check(track.createLocalExecutor(manager) == custom, "custom executor return identity");
    check(track instanceof AudioTrack && track instanceof AudioFrameProvider,
        "inherited interfaces");
    check(calls.equals(Arrays.asList(
        "assign:true", "assign:false", "active", "process", "create")),
        "dispatch order");

    checkReflection();
    System.out.println(
        "dispatch=assign-true,assign-false,active,process-exception,custom;"
        + "inheritance=AudioTrack,AudioFrameProvider;"
        + "reflection=interface,0-fields,4-methods,0-constructors,process-throws-Exception");
  }

  private static void checkReflection() throws Exception {
    Class<InternalAudioTrack> type = InternalAudioTrack.class;
    int modifiers = type.getModifiers();
    check(Modifier.isPublic(modifiers) && Modifier.isInterface(modifiers)
        && Modifier.isAbstract(modifiers) && !Modifier.isFinal(modifiers)
        && type.getSuperclass() == null && type.getTypeParameters().length == 0
        && type.getDeclaredAnnotations().length == 0, "interface structure");
    check(Arrays.equals(type.getInterfaces(), new Class<?>[] {
        AudioTrack.class, AudioFrameProvider.class }), "direct interface order");
    check(type.getDeclaredFields().length == 0 && type.getDeclaredMethods().length == 4
        && type.getDeclaredConstructors().length == 0, "member counts");

    Method assign = type.getDeclaredMethod(
        "assignExecutor", AudioTrackExecutor.class, boolean.class);
    checkAbstract(assign, void.class,
        new Class<?>[] { AudioTrackExecutor.class, boolean.class }, new Class<?>[0]);
    Method active = type.getDeclaredMethod("getActiveExecutor");
    checkAbstract(active, AudioTrackExecutor.class, new Class<?>[0], new Class<?>[0]);
    Method process = type.getDeclaredMethod("process", LocalAudioTrackExecutor.class);
    checkAbstract(process, void.class, new Class<?>[] { LocalAudioTrackExecutor.class },
        new Class<?>[] { Exception.class });
    Method create = type.getDeclaredMethod("createLocalExecutor", AudioPlayerManager.class);
    checkAbstract(create, AudioTrackExecutor.class,
        new Class<?>[] { AudioPlayerManager.class }, new Class<?>[0]);
  }

  private static void checkAbstract(Method method, Class<?> returnType,
      Class<?>[] parameterTypes, Class<?>[] exceptionTypes) {
    int modifiers = method.getModifiers();
    check(Modifier.isPublic(modifiers) && Modifier.isAbstract(modifiers)
        && !Modifier.isStatic(modifiers) && !Modifier.isFinal(modifiers)
        && !method.isDefault() && !method.isBridge() && !method.isSynthetic(),
        method.getName() + " modifiers");
    check(method.getReturnType() == returnType
        && Arrays.equals(method.getParameterTypes(), parameterTypes)
        && Arrays.equals(method.getExceptionTypes(), exceptionTypes)
        && method.getTypeParameters().length == 0
        && method.getDeclaredAnnotations().length == 0,
        method.getName() + " metadata");
  }

  @SuppressWarnings("unchecked")
  private static <T> T proxy(Class<T> type, InvocationHandler handler) {
    return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] { type }, handler);
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == byte.class) return (byte) 0;
    if (type == short.class) return (short) 0;
    if (type == int.class) return 0;
    if (type == long.class) return 0L;
    if (type == float.class) return 0.0f;
    if (type == double.class) return 0.0d;
    if (type == char.class) return (char) 0;
    return null;
  }

  private static <T> T allocate(Class<T> type) throws Exception {
    Class<?> unsafeType = Class.forName("sun.misc.Unsafe");
    Field singleton = unsafeType.getDeclaredField("theUnsafe");
    singleton.setAccessible(true);
    Object unsafe = singleton.get(null);
    return type.cast(unsafeType.getMethod("allocateInstance", Class.class).invoke(unsafe, type));
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const CLASSLOADER_CONSUMER: &str = r#"
import java.lang.ref.WeakReference;
import java.net.URL;
import java.net.URLClassLoader;
import java.nio.file.Path;
import java.util.concurrent.TimeUnit;

public final class GateClassloader {
  public static void main(String[] args) throws Exception {
    WeakReference<ClassLoader> reference = runOnce(args[0], args[1]);
    long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(20);
    while (reference.get() != null && System.nanoTime() < deadline) {
      System.gc();
      Thread.sleep(10);
    }
    if (reference.get() != null) throw new AssertionError("compatibility classloader remained pinned");
    System.out.println("{\"probe\":\"classloader\",\"collected\":true}");
  }

  private static WeakReference<ClassLoader> runOnce(String jar, String nativeLibrary) throws Exception {
    URL url = Path.of(jar).toUri().toURL();
    URLClassLoader loader = new URLClassLoader(new URL[] { url }, ClassLoader.getPlatformClassLoader());
    Class<?> nativeLoader = Class.forName("dev.mantle.internal.NativeLoader", true, loader);
    nativeLoader.getMethod("load", String.class).invoke(null, nativeLibrary);
    Class<?> managerClass = Class.forName(
        "com.sedmelluq.discord.lavaplayer.player.DefaultAudioPlayerManager", true, loader);
    Object manager = managerClass.getConstructor().newInstance();
    nativeLoader.getMethod("shutdown", Object.class).invoke(null, manager);
    WeakReference<ClassLoader> reference = new WeakReference<>(loader);
    manager = null;
    managerClass = null;
    nativeLoader = null;
    loader.close();
    loader = null;
    return reference;
  }
}
"#;

const M3U_STREAM_SEGMENT_URL_PROVIDER_CONSUMER: &str = r##"
import com.sedmelluq.discord.lavaplayer.container.playlists.ExtendedM3uParser;
import com.sedmelluq.discord.lavaplayer.source.stream.M3uStreamSegmentUrlProvider;
import com.sedmelluq.discord.lavaplayer.tools.FriendlyException;
import com.sedmelluq.discord.lavaplayer.tools.FriendlyException.Severity;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Proxy;
import java.nio.charset.StandardCharsets;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import org.apache.http.HttpEntity;
import org.apache.http.StatusLine;
import org.apache.http.client.config.RequestConfig;
import org.apache.http.client.methods.CloseableHttpResponse;
import org.apache.http.client.methods.HttpGet;
import org.apache.http.client.methods.HttpUriRequest;
import org.apache.http.client.protocol.HttpClientContext;

public final class GateM3uStreamSegmentUrlProvider {
  private static final String PROVIDER_NAME =
      "com.sedmelluq.discord.lavaplayer.source.stream.M3uStreamSegmentUrlProvider";

  public static void main(String[] args) throws Exception {
    reflectionContract();
    constructorsAndUrls();
    channelPlaylistParsing();
    segmentPlaylistParsing();
    selectionAndNextUrl();
    segmentStreams();
    System.out.println("provider=public-abstract,4-fields,2-constructors,13-methods;"
        + "nested=protected-static-values,5-fields,1-constructor;"
        + "behavior=base-url,uri-resolution,channels,segments,generics,selection,lazy-wait,"
        + "timeouts,response-ownership,identity,failures,reflection");
  }

  private static void reflectionContract() throws Exception {
    Class<M3uStreamSegmentUrlProvider> type = M3uStreamSegmentUrlProvider.class;
    check(type.getName().equals(PROVIDER_NAME)
        && type.getModifiers() == (Modifier.PUBLIC | Modifier.ABSTRACT)
        && type.getSuperclass() == Object.class && type.getInterfaces().length == 0,
        "provider class metadata");
    check(type.getDeclaredFields().length == 4 && type.getDeclaredConstructors().length == 2
        && type.getDeclaredMethods().length == 13, "provider shell counts");
    checkField(type, "SEGMENT_WAIT_STEP_MS", long.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "streamingRequestConfig", RequestConfig.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "baseUrl", String.class, Modifier.PROTECTED);
    Class<?> segmentType = Class.forName(PROVIDER_NAME + "$SegmentInfo");
    Class<?> channelType = Class.forName(PROVIDER_NAME + "$ChannelStreamInfo");
    checkField(type, "lastSegment", segmentType, Modifier.PROTECTED);

    checkConstructor(type, new Class<?>[0], Modifier.PROTECTED);
    checkConstructor(type, new Class<?>[] {String.class}, Modifier.PROTECTED);
    checkMethod(type, "createSegmentUrl", String.class,
        Modifier.PROTECTED | Modifier.STATIC, new Class<?>[] {String.class, String.class}, none());
    checkMethod(type, "getQualityFromM3uDirective", String.class,
        Modifier.PROTECTED | Modifier.ABSTRACT,
        new Class<?>[] {ExtendedM3uParser.Line.class}, none());
    checkMethod(type, "fetchSegmentPlaylistUrl", String.class,
        Modifier.PROTECTED | Modifier.ABSTRACT, new Class<?>[] {HttpInterface.class},
        new Class<?>[] {IOException.class});
    checkMethod(type, "getNextSegmentUrl", String.class, Modifier.PROTECTED,
        new Class<?>[] {HttpInterface.class}, none());
    checkMethod(type, "getNextSegmentStream", InputStream.class, Modifier.PUBLIC,
        new Class<?>[] {HttpInterface.class}, none());
    checkMethod(type, "createSegmentGetRequest", HttpUriRequest.class,
        Modifier.PROTECTED | Modifier.ABSTRACT, new Class<?>[] {String.class}, none());
    checkMethod(type, "isAbsoluteUrl", boolean.class, Modifier.PROTECTED,
        new Class<?>[] {String.class}, none());
    checkMethod(type, "getAbsoluteUrl", String.class, Modifier.PROTECTED,
        new Class<?>[] {String.class}, none());
    Method channels = checkMethod(type, "loadChannelStreamsList", List.class,
        Modifier.PROTECTED, new Class<?>[] {String[].class}, none());
    Method segments = checkMethod(type, "loadStreamSegmentsList", List.class,
        Modifier.PROTECTED, new Class<?>[] {HttpInterface.class, String.class},
        new Class<?>[] {IOException.class});
    Method choose = checkMethod(type, "chooseNextSegment", segmentType, Modifier.PROTECTED,
        new Class<?>[] {List.class, segmentType}, none());
    check(genericName(channels.getGenericReturnType()).equals("java.util.List<" +
        PROVIDER_NAME + "$ChannelStreamInfo>"), "channel generic return");
    check(genericName(segments.getGenericReturnType()).equals("java.util.List<" +
        PROVIDER_NAME + "$SegmentInfo>"), "segment generic return");
    check(genericName(choose.getGenericParameterTypes()[0]).equals("java.util.List<" +
        PROVIDER_NAME + "$SegmentInfo>"), "choose generic parameter");
    check(Arrays.stream(type.getDeclaredMethods()).filter(method ->
        Modifier.isPublic(method.getModifiers()) || Modifier.isProtected(method.getModifiers()))
        .count() == 11L, "exported method count");

    check(channelType.getModifiers() == (Modifier.PROTECTED | Modifier.STATIC)
        && channelType.getDeclaringClass() == type && channelType.getDeclaredFields().length == 2,
        "channel class metadata");
    checkField(channelType, "quality", String.class, Modifier.PUBLIC | Modifier.FINAL);
    checkField(channelType, "url", String.class, Modifier.PUBLIC | Modifier.FINAL);
    check(channelType.getDeclaredConstructors().length == 1
        && channelType.getDeclaredConstructors()[0].getModifiers() == Modifier.PRIVATE,
        "channel constructor metadata");
    check(Arrays.stream(channelType.getDeclaredMethods()).noneMatch(method ->
        Modifier.isPublic(method.getModifiers()) || Modifier.isProtected(method.getModifiers())),
        "channel exported methods");

    check(segmentType.getModifiers() == (Modifier.PROTECTED | Modifier.STATIC)
        && segmentType.getDeclaringClass() == type && segmentType.getDeclaredFields().length == 3,
        "segment class metadata");
    checkField(segmentType, "url", String.class, Modifier.PUBLIC | Modifier.FINAL);
    checkField(segmentType, "duration", Long.class, Modifier.PUBLIC | Modifier.FINAL);
    checkField(segmentType, "name", String.class, Modifier.PUBLIC | Modifier.FINAL);
    check(segmentType.getDeclaredConstructors().length == 1
        && segmentType.getDeclaredConstructors()[0].getModifiers() == Modifier.PUBLIC,
        "segment constructor metadata");

    Field wait = type.getDeclaredField("SEGMENT_WAIT_STEP_MS");
    wait.setAccessible(true);
    Field config = type.getDeclaredField("streamingRequestConfig");
    config.setAccessible(true);
    RequestConfig value = (RequestConfig) config.get(null);
    check(wait.getLong(null) == 200L && value.getSocketTimeout() == 5000
        && value.getConnectionRequestTimeout() == 5000 && value.getConnectTimeout() == 5000,
        "static configuration");
  }

  private static void constructorsAndUrls() throws Exception {
    ExposedProvider empty = new ExposedProvider();
    check(empty.base() == null && empty.last() == null && empty.absolute("relative")
        && empty.absolute("http://[") && empty.absolute(null), "null-base behavior");
    ExposedProvider provider = new ExposedProvider("https://origin.example/root/master.m3u");
    check("https://origin.example/root".equals(provider.base()) && provider.last() == null,
        "origin base");
    check("https://origin.example".equals(
        new ExposedProvider("https://origin.example/root/").base()), "trailing slash quirk");
    expect(StringIndexOutOfBoundsException.class, () -> new ExposedProvider("master.m3u"));
    check("https://cdn.example/live/a.ts".equals(ExposedProvider.resolve(
        "https://cdn.example/live/list.m3u", "a.ts")), "relative segment resolution");
    check("https://other.example/a.ts".equals(ExposedProvider.resolve(
        "https://cdn.example/live/list.m3u", "https://other.example/a.ts")),
        "absolute segment resolution");
    check(provider.absolute("https://cdn.example/x") && !provider.absolute("relative/x")
        && !provider.absolute("http://[") && "https://origin.example/root/x".equals(
            provider.makeAbsolute("x")) && "https://origin.example/root/x".equals(
            provider.makeAbsolute("/x")), "URL helpers");
    expect(NullPointerException.class, () -> provider.absolute(null));
  }

  private static void channelPlaylistParsing() throws Exception {
    ExposedProvider provider = new ExposedProvider("https://origin.example/root/master.m3u");
    provider.qualities.add("low");
    provider.qualities.add("absolute");
    provider.qualities.add(null);
    List<?> streams = provider.channels(new String[] {
        "#EXTM3U", "#EXT-X-STREAM-INF:BANDWIDTH=64000", "low/index.m3u",
        "#EXTINF:ignored", "https://cdn.example/high.m3u",
        "#EXT-X-STREAM-INF:SKIP", "skip.m3u", "orphan.m3u"
    });
    check(streams.size() == 2 && provider.qualityCalls == 3, "channel filtering");
    check("low".equals(value(streams.get(0), "quality"))
        && "https://origin.example/root/low/index.m3u".equals(value(streams.get(0), "url")),
        "relative channel");
    check("absolute".equals(value(streams.get(1), "quality"))
        && "https://cdn.example/high.m3u".equals(value(streams.get(1), "url")),
        "absolute channel");
  }

  private static void segmentPlaylistParsing() throws Exception {
    String payload = "#EXTM3U\n#EXTINF:1.5,First\nseg1.ts\nbare.ts\n"
        + "#EXTINF:bad,\nseg3.ts\n#EXTINF:2.25,Fourth,Comma\nseg4.ts\n";
    RecordingResponse response = new RecordingResponse(200, payload);
    RecordingHttp http = new RecordingHttp(response);
    ExposedProvider provider = new ExposedProvider();
    List<?> segments = provider.segments(http, "https://cdn.example/live/list.m3u");
    check(http.executes == 1 && http.request instanceof HttpGet
        && http.request.getURI().toString().equals("https://cdn.example/live/list.m3u")
        && response.closes == 1, "playlist request and cleanup");
    check(segments.size() == 4, "segment count");
    checkSegment(segments.get(0), "seg1.ts", 1500L, "First");
    checkSegment(segments.get(1), "bare.ts", 1500L, "First");
    checkSegment(segments.get(2), "seg3.ts", null, "");
    checkSegment(segments.get(3), "seg4.ts", 2250L, "Fourth,Comma");
  }

  private static void selectionAndNextUrl() throws Exception {
    ExposedProvider provider = new ExposedProvider();
    Object a = provider.segment("a.ts", 1000L, "a");
    Object b = provider.segment("b.ts", 1000L, "b");
    Object c = provider.segment("c.ts", 1000L, "c");
    List<Object> values = Arrays.asList(a, b, c);
    check(provider.choose(values, null) == a && provider.choose(values, a) == b
        && provider.choose(values, b) == c
        && provider.choose(values, provider.segment("missing", null, null)) == a
        && provider.choose(new ArrayList<>(), null) == null, "segment selection");

    provider.playlistUrl = "https://cdn.example/live/list.m3u";
    provider.useRounds = true;
    provider.round(a, b);
    provider.round(a, b, c);
    check("https://cdn.example/live/a.ts".equals(provider.baseNext(null))
        && provider.last() == a, "first next URL");
    check("https://cdn.example/live/b.ts".equals(provider.baseNext(null))
        && provider.last() == b && provider.loadCalls == 2, "second next URL");

    ExposedProvider none = new ExposedProvider();
    none.playlistUrl = null;
    check(none.baseNext(null) == null && none.loadCalls == 0, "null playlist URL");
    IOException loadFailure = new IOException("playlist-lines");
    none.playlistUrl = "https://cdn.example/list.m3u";
    none.loadFailure = loadFailure;
    FriendlyException friendly = expect(FriendlyException.class, () -> none.baseNext(null));
    check(friendly.getCause() == loadFailure && friendly.severity == Severity.SUSPICIOUS
        && friendly.getMessage().equals("Failed to get next part of the stream."),
        "IO wrapping");

    ExposedProvider interrupted = new ExposedProvider();
    interrupted.playlistUrl = "https://cdn.example/live/list.m3u";
    interrupted.useRounds = true;
    Object current = interrupted.segment("same.ts", 10_000L, "same");
    interrupted.setLast(current);
    interrupted.round(interrupted.segment("same.ts", 10_000L, "same"));
    Thread.currentThread().interrupt();
    RuntimeException runtime = expect(RuntimeException.class, () -> interrupted.baseNext(null));
    check(runtime.getCause() instanceof InterruptedException
        && !Thread.currentThread().isInterrupted(), "interruption wrapping");
  }

  private static void segmentStreams() throws Exception {
    ExposedProvider provider = new ExposedProvider();
    provider.overrideNext = true;
    RecordingHttp nullHttp = new RecordingHttp(null);
    check(provider.nextStream(nullHttp) == null && nullHttp.executes == 0,
        "null segment stream");
    checkTimeouts(nullHttp.getContext().getRequestConfig());

    provider.nextUrl = "https://cdn.example/segment.ts";
    byte[] bytes = new byte[] {1, 2, 3};
    RecordingResponse success = new RecordingResponse(200, bytes);
    RecordingHttp successHttp = new RecordingHttp(success);
    InputStream stream = provider.nextStream(successHttp);
    check(stream == success.content && success.closes == 0 && successHttp.executes == 1
        && provider.createdUrl.equals(provider.nextUrl)
        && successHttp.request == provider.createdRequest, "successful stream ownership");
    checkTimeouts(successHttp.getContext().getRequestConfig());

    RecordingResponse bad = new RecordingResponse(404, bytes);
    RecordingHttp badHttp = new RecordingHttp(bad);
    RuntimeException badError = expect(RuntimeException.class, () -> provider.nextStream(badHttp));
    check(badError.getCause() instanceof IOException
        && badError.getCause().getMessage().equals(
            "Invalid status code from segment data URL: 404") && bad.closes == 1,
        "bad status cleanup");

    IOException executeFailure = new IOException("execute");
    RecordingHttp failingHttp = new RecordingHttp(null);
    failingHttp.failure = executeFailure;
    RuntimeException executeError = expect(
        RuntimeException.class, () -> provider.nextStream(failingHttp));
    check(executeError.getCause() == executeFailure, "execute failure identity");

    IOException contentFailure = new IOException("content");
    RecordingResponse content = new RecordingResponse(200, bytes);
    content.contentFailure = contentFailure;
    RuntimeException contentError = expect(RuntimeException.class,
        () -> provider.nextStream(new RecordingHttp(content)));
    check(contentError.getCause() == contentFailure && content.closes == 0,
        "post-success content failure ownership");
  }

  private static final class ExposedProvider extends M3uStreamSegmentUrlProvider {
    private final List<String> qualities = new ArrayList<>();
    private final ArrayDeque<List<SegmentInfo>> rounds = new ArrayDeque<>();
    private String playlistUrl;
    private IOException fetchFailure;
    private IOException loadFailure;
    private boolean useRounds;
    private boolean overrideNext;
    private String nextUrl;
    private String createdUrl;
    private HttpUriRequest createdRequest;
    private int qualityCalls;
    private int loadCalls;

    ExposedProvider() { super(); }
    ExposedProvider(String origin) { super(origin); }
    String base() { return baseUrl; }
    Object last() { return lastSegment; }
    void setLast(Object value) { lastSegment = (SegmentInfo) value; }
    boolean absolute(String value) { return isAbsoluteUrl(value); }
    String makeAbsolute(String value) { return getAbsoluteUrl(value); }
    static String resolve(String playlist, String segment) {
      return createSegmentUrl(playlist, segment);
    }
    List<?> channels(String[] lines) { return loadChannelStreamsList(lines); }
    List<?> segments(HttpInterface http, String url) throws IOException {
      return super.loadStreamSegmentsList(http, url);
    }
    Object segment(String url, Long duration, String name) {
      return new SegmentInfo(url, duration, name);
    }
    @SuppressWarnings({"rawtypes", "unchecked"})
    Object choose(List<?> values, Object last) {
      return chooseNextSegment((List) values, (SegmentInfo) last);
    }
    void round(Object... values) {
      List<SegmentInfo> round = new ArrayList<>();
      for (Object value : values) round.add((SegmentInfo) value);
      rounds.add(round);
    }
    String baseNext(HttpInterface http) { return super.getNextSegmentUrl(http); }
    InputStream nextStream(HttpInterface http) { return getNextSegmentStream(http); }

    protected String getQualityFromM3uDirective(ExtendedM3uParser.Line line) {
      qualityCalls++;
      return qualities.remove(0);
    }
    protected String fetchSegmentPlaylistUrl(HttpInterface http) throws IOException {
      if (fetchFailure != null) throw fetchFailure;
      return playlistUrl;
    }
    protected List<SegmentInfo> loadStreamSegmentsList(HttpInterface http, String url)
        throws IOException {
      if (!useRounds && loadFailure == null) return super.loadStreamSegmentsList(http, url);
      loadCalls++;
      if (loadFailure != null) throw loadFailure;
      return rounds.remove();
    }
    protected String getNextSegmentUrl(HttpInterface http) {
      return overrideNext ? nextUrl : super.getNextSegmentUrl(http);
    }
    protected HttpUriRequest createSegmentGetRequest(String url) {
      createdUrl = url;
      createdRequest = new HttpGet(url);
      return createdRequest;
    }
  }

  private static final class RecordingHttp extends HttpInterface {
    private final RecordingResponse response;
    private HttpUriRequest request;
    private IOException failure;
    private int executes;
    RecordingHttp(RecordingResponse response) {
      super(null, HttpClientContext.create(), false, null);
      this.response = response;
    }
    public CloseableHttpResponse execute(HttpUriRequest request) throws IOException {
      this.request = request;
      executes++;
      if (failure != null) throw failure;
      return response.proxy;
    }
  }

  private static final class RecordingResponse {
    private final CloseableHttpResponse proxy;
    private final InputStream content;
    private IOException contentFailure;
    private int closes;
    RecordingResponse(int status, String content) {
      this(status, content.getBytes(StandardCharsets.UTF_8));
    }
    RecordingResponse(int status, byte[] content) {
      this.content = new ByteArrayInputStream(content);
      StatusLine statusLine = proxy(StatusLine.class, (method, arguments) ->
          method.getName().equals("getStatusCode") ? status : defaultValue(method.getReturnType()));
      HttpEntity entity = proxy(HttpEntity.class, (method, arguments) -> {
        if (method.getName().equals("getContent")) {
          if (contentFailure != null) throw contentFailure;
          return this.content;
        }
        return defaultValue(method.getReturnType());
      });
      this.proxy = proxy(CloseableHttpResponse.class, (method, arguments) -> {
        if (method.getName().equals("getStatusLine")) return statusLine;
        if (method.getName().equals("getEntity")) return entity;
        if (method.getName().equals("close")) { closes++; return null; }
        return defaultValue(method.getReturnType());
      });
    }
  }

  private static void checkTimeouts(RequestConfig config) {
    check(config != null && config.getSocketTimeout() == 5000
        && config.getConnectionRequestTimeout() == 5000 && config.getConnectTimeout() == 5000,
        "stream request timeouts");
  }

  private static void checkSegment(Object value, String url, Long duration, String name)
      throws Exception {
    check(equals(url, value(value, "url")) && equals(duration, value(value, "duration"))
        && equals(name, value(value, "name")), "segment values");
  }

  private static Object value(Object instance, String name) throws Exception {
    return instance.getClass().getField(name).get(instance);
  }

  private static boolean equals(Object left, Object right) {
    return left == null ? right == null : left.equals(right);
  }

  private static String genericName(java.lang.reflect.Type type) {
    return type.getTypeName();
  }

  private static void checkField(Class<?> owner, String name, Class<?> type, int modifiers)
      throws Exception {
    Field field = owner.getDeclaredField(name);
    check(field.getType() == type && field.getGenericType() == type
        && field.getModifiers() == modifiers && !field.isSynthetic(), name + " metadata");
  }

  private static void checkConstructor(Class<?> owner, Class<?>[] parameters, int modifiers)
      throws Exception {
    Constructor<?> constructor = owner.getDeclaredConstructor(parameters);
    check(constructor.getModifiers() == modifiers && constructor.getExceptionTypes().length == 0
        && !constructor.isSynthetic() && !constructor.isVarArgs(), "constructor metadata");
  }

  private static Method checkMethod(Class<?> owner, String name, Class<?> returnType,
      int modifiers, Class<?>[] parameters, Class<?>[] exceptions) throws Exception {
    Method method = owner.getDeclaredMethod(name, parameters);
    check(method.getReturnType() == returnType && method.getModifiers() == modifiers
        && Arrays.equals(method.getExceptionTypes(), exceptions)
        && !method.isSynthetic() && !method.isBridge() && !method.isVarArgs(), name + " metadata");
    return method;
  }

  private static Class<?>[] none() { return new Class<?>[0]; }

  private interface ProxyCall { Object invoke(Method method, Object[] arguments) throws Throwable; }
  private static <T> T proxy(Class<T> type, ProxyCall call) {
    return type.cast(Proxy.newProxyInstance(
        GateM3uStreamSegmentUrlProvider.class.getClassLoader(), new Class<?>[] {type},
        (instance, method, arguments) -> call.invoke(method, arguments)));
  }
  private static Object defaultValue(Class<?> type) {
    if (type == boolean.class) return false;
    if (type == int.class) return 0;
    if (type == long.class) return 0L;
    return null;
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }
  private interface Operation { void run() throws Exception; }
  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"##;

const M3U_STREAM_AUDIO_TRACK_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.container.playlists.ExtendedM3uParser;
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.stream.M3uStreamAudioTrack;
import com.sedmelluq.discord.lavaplayer.source.stream.M3uStreamSegmentUrlProvider;
import com.sedmelluq.discord.lavaplayer.tools.http.HttpContextFilter;
import com.sedmelluq.discord.lavaplayer.tools.io.ChainedInputStream;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import com.sedmelluq.discord.lavaplayer.track.DelegatedAudioTrack;
import com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.lang.reflect.Constructor;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import org.apache.http.client.methods.HttpUriRequest;
import org.apache.http.client.protocol.HttpClientContext;

public final class GateM3uStreamAudioTrack {
  public static void main(String[] args) throws Exception {
    reflectionContract();
    constructionAndHooks();
    lazyJoinedStream();
    cleanupAndSuppression();
    acquisitionFailure();
    System.out.println("public-abstract,delegated,0-fields,1-constructor,4-exported-methods;"
        + "construction,hooks,lazy-chain,segment-order,identity,cleanup,suppression,failures,reflection");
  }

  private static void reflectionContract() throws Exception {
    Class<M3uStreamAudioTrack> type = M3uStreamAudioTrack.class;
    check(type.getModifiers() == (Modifier.PUBLIC | Modifier.ABSTRACT)
        && type.getSuperclass() == DelegatedAudioTrack.class
        && type.getInterfaces().length == 0 && type.getDeclaredFields().length == 0,
        "class metadata");

    Constructor<?> constructor = type.getDeclaredConstructor(AudioTrackInfo.class);
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC
        && constructor.getExceptionTypes().length == 0 && !constructor.isSynthetic(),
        "constructor metadata");
    checkMethod(type, "getSegmentUrlProvider", M3uStreamSegmentUrlProvider.class,
        Modifier.PROTECTED | Modifier.ABSTRACT, new Class<?>[0], new Class<?>[0], false);
    checkMethod(type, "getHttpInterface", HttpInterface.class,
        Modifier.PROTECTED | Modifier.ABSTRACT, new Class<?>[0], new Class<?>[0], false);
    checkMethod(type, "processJoinedStream", void.class,
        Modifier.PROTECTED | Modifier.ABSTRACT,
        new Class<?>[] {LocalAudioTrackExecutor.class, InputStream.class},
        new Class<?>[] {Exception.class}, false);
    checkMethod(type, "process", void.class, Modifier.PUBLIC,
        new Class<?>[] {LocalAudioTrackExecutor.class}, new Class<?>[] {Exception.class}, false);
    checkMethod(type, "lambda$process$0", InputStream.class,
        Modifier.PRIVATE | 0x1000, new Class<?>[] {HttpInterface.class},
        new Class<?>[] {IOException.class}, true);
    check(type.getDeclaredMethods().length == 5, "declared method count");
    long exported = Arrays.stream(type.getDeclaredMethods())
        .filter(method -> Modifier.isPublic(method.getModifiers())
            || Modifier.isProtected(method.getModifiers()))
        .count();
    check(exported == 4L, "exported method count");
  }

  private static void constructionAndHooks() {
    StringBuilder events = new StringBuilder();
    QueueProvider provider = new QueueProvider(events);
    RecordingHttpInterface http = new RecordingHttpInterface(events);
    AudioTrackInfo info = info();
    TestTrack track = new TestTrack(info, provider, http, events);
    check(track.getInfo() == info && track.exposedProvider() == provider
        && track.exposedHttp() == http, "construction and hook identity");
  }

  private static void lazyJoinedStream() throws Exception {
    StringBuilder events = new StringBuilder();
    QueueProvider provider = new QueueProvider(events,
        new TrackingInputStream("a", "ab", events, null),
        new TrackingInputStream("b", "cd", events, null));
    RecordingHttpInterface http = new RecordingHttpInterface(events);
    TestTrack track = new TestTrack(info(), provider, http, events);
    track.process(null);

    check(track.observedExecutor == null && track.observedStream instanceof ChainedInputStream,
        "joined arguments");
    check("abcd".equals(track.joinedData), "joined data");
    check(provider.calls == 3 && provider.observedHttp == http,
        "lazy provider order and HTTP identity");
    check(provider.streams[0].closes == 1 && provider.streams[1].closes == 1
        && http.closes == 1, "ordinary cleanup");
    String order = events.toString();
    check(order.startsWith("http;joined;")
        && order.indexOf("close:a;") < order.indexOf("close:b;")
        && order.indexOf("close:b;") < order.indexOf("http-close;"),
        "resource order: " + order);
  }

  private static void cleanupAndSuppression() throws Exception {
    StringBuilder events = new StringBuilder();
    IOException chainClose = new IOException("chain-close");
    IOException httpClose = new IOException("http-close");
    QueueProvider provider = new QueueProvider(events,
        new TrackingInputStream("failing", "x", events, chainClose));
    RecordingHttpInterface http = new RecordingHttpInterface(events);
    http.closeFailure = httpClose;
    TestTrack track = new TestTrack(info(), provider, http, events);
    Exception primary = new Exception("joined-primary");
    track.joinedFailure = primary;
    track.primeStreamBeforeFailure = true;

    Exception thrown = expect(Exception.class, () -> track.process(null));
    check(thrown == primary && Arrays.equals(thrown.getSuppressed(),
        new Throwable[] {chainClose, httpClose}), "suppressed failure order");
    check(provider.calls == 1 && provider.streams[0].closes == 1 && http.closes == 1,
        "failure cleanup");
  }

  private static void acquisitionFailure() throws Exception {
    StringBuilder events = new StringBuilder();
    QueueProvider provider = new QueueProvider(events);
    RecordingHttpInterface http = new RecordingHttpInterface(events);
    TestTrack track = new TestTrack(info(), provider, http, events);
    RuntimeException failure = new RuntimeException("http-hook");
    track.httpFailure = failure;
    RuntimeException thrown = expect(RuntimeException.class, () -> track.process(null));
    check(thrown == failure && provider.calls == 0 && http.closes == 0
        && track.joinedCalls == 0, "acquisition failure identity");
  }

  private static AudioTrackInfo info() {
    return new AudioTrackInfo("title", "author", 123L, "fixture", false,
        "https://example.invalid/fixture", null, null);
  }

  private static void checkMethod(Class<?> owner, String name, Class<?> returnType,
      int modifiers, Class<?>[] parameters, Class<?>[] exceptions, boolean synthetic)
      throws Exception {
    Method method = owner.getDeclaredMethod(name, parameters);
    check(method.getReturnType() == returnType && method.getModifiers() == modifiers
        && Arrays.equals(method.getExceptionTypes(), exceptions)
        && method.isSynthetic() == synthetic && !method.isBridge() && !method.isVarArgs(),
        name + " metadata");
  }

  private static final class TestTrack extends M3uStreamAudioTrack {
    private final QueueProvider provider;
    private final RecordingHttpInterface http;
    private final StringBuilder events;
    private LocalAudioTrackExecutor observedExecutor;
    private InputStream observedStream;
    private String joinedData;
    private Exception joinedFailure;
    private RuntimeException httpFailure;
    private boolean primeStreamBeforeFailure;
    private int joinedCalls;

    TestTrack(AudioTrackInfo info, QueueProvider provider, RecordingHttpInterface http,
        StringBuilder events) {
      super(info);
      this.provider = provider;
      this.http = http;
      this.events = events;
    }

    protected M3uStreamSegmentUrlProvider getSegmentUrlProvider() { return provider; }
    protected HttpInterface getHttpInterface() {
      events.append("http;");
      if (httpFailure != null) throw httpFailure;
      return http;
    }
    protected void processJoinedStream(LocalAudioTrackExecutor executor, InputStream stream)
        throws Exception {
      joinedCalls++;
      observedExecutor = executor;
      observedStream = stream;
      events.append("joined;");
      if (joinedFailure != null) {
        if (primeStreamBeforeFailure) stream.read();
        throw joinedFailure;
      }
      ByteArrayOutputStream output = new ByteArrayOutputStream();
      byte[] buffer = new byte[8];
      int count;
      while ((count = stream.read(buffer)) != -1) output.write(buffer, 0, count);
      joinedData = new String(output.toByteArray(), StandardCharsets.UTF_8);
    }
    M3uStreamSegmentUrlProvider exposedProvider() { return getSegmentUrlProvider(); }
    HttpInterface exposedHttp() { return getHttpInterface(); }
    protected AudioTrack makeShallowClone() {
      return new TestTrack(trackInfo, provider, http, events);
    }
    public AudioSourceManager getSourceManager() { return null; }
  }

  private static final class QueueProvider extends M3uStreamSegmentUrlProvider {
    private final StringBuilder events;
    private final TrackingInputStream[] streams;
    private HttpInterface observedHttp;
    private int calls;

    QueueProvider(StringBuilder events, TrackingInputStream... streams) {
      this.events = events;
      this.streams = streams;
    }
    public InputStream getNextSegmentStream(HttpInterface http) {
      calls++;
      if (observedHttp == null) observedHttp = http;
      check(observedHttp == http, "HTTP identity changed");
      events.append("next;");
      return calls <= streams.length ? streams[calls - 1] : null;
    }
    protected String getQualityFromM3uDirective(ExtendedM3uParser.Line line) { return null; }
    protected String fetchSegmentPlaylistUrl(HttpInterface http) { return null; }
    protected HttpUriRequest createSegmentGetRequest(String url) { return null; }
  }

  private static final class TrackingInputStream extends ByteArrayInputStream {
    private final String name;
    private final StringBuilder events;
    private final IOException closeFailure;
    private int closes;
    TrackingInputStream(String name, String data, StringBuilder events, IOException closeFailure) {
      super(data.getBytes(StandardCharsets.UTF_8));
      this.name = name;
      this.events = events;
      this.closeFailure = closeFailure;
    }
    public void close() throws IOException {
      closes++;
      events.append("close:").append(name).append(';');
      if (closeFailure != null) throw closeFailure;
      super.close();
    }
  }

  private static final class RecordingHttpInterface extends HttpInterface {
    private final StringBuilder events;
    private IOException closeFailure;
    private int closes;
    RecordingHttpInterface(StringBuilder events) {
      super(null, HttpClientContext.create(), false, proxy(HttpContextFilter.class));
      this.events = events;
    }
    public void close() throws IOException {
      closes++;
      events.append("http-close;");
      if (closeFailure != null) throw closeFailure;
    }
  }

  private static <T> T proxy(Class<T> type) {
    return type.cast(Proxy.newProxyInstance(
        GateM3uStreamAudioTrack.class.getClassLoader(), new Class<?>[] {type},
        (proxy, method, args) -> {
          if (method.getReturnType() == boolean.class) return false;
          if (method.getReturnType() == int.class) return 0;
          return null;
        }));
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private interface Operation { void run() throws Exception; }
  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const MPEG_TS_M3U_STREAM_AUDIO_TRACK_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.container.adts.AdtsAudioTrack;
import com.sedmelluq.discord.lavaplayer.container.mpegts.MpegTsElementaryInputStream;
import com.sedmelluq.discord.lavaplayer.container.mpegts.PesPacketInputStream;
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.stream.M3uStreamAudioTrack;
import com.sedmelluq.discord.lavaplayer.source.stream.M3uStreamSegmentUrlProvider;
import com.sedmelluq.discord.lavaplayer.source.stream.MpegTsM3uStreamAudioTrack;
import com.sedmelluq.discord.lavaplayer.tools.io.GreedyInputStream;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import com.sedmelluq.discord.lavaplayer.track.InternalAudioTrack;
import com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor;
import java.io.FilterInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;

public final class GateMpegTsM3uStreamAudioTrack {
  private static final Object UNSAFE = loadUnsafe();

  public static void main(String[] args) throws Exception {
    reflectionContract();
    constructionContract();
    wrapperAndDelegateContract();
    nullAndFailureContract();
    System.out.println("public-abstract,m3u-super,0-fields,1-constructor,1-method;"
        + "construction,track-info,executor,ts-adts,pes,elementary-type,raw-identity,"
        + "no-eager-read,nulls,failure-identity,reflection");
  }

  private static void reflectionContract() throws Exception {
    Class<MpegTsM3uStreamAudioTrack> type = MpegTsM3uStreamAudioTrack.class;
    check(type.getModifiers() == (Modifier.PUBLIC | Modifier.ABSTRACT)
        && type.getSuperclass() == M3uStreamAudioTrack.class
        && type.getInterfaces().length == 0 && type.getDeclaredFields().length == 0,
        "class metadata");
    Constructor<?> constructor = type.getDeclaredConstructor(AudioTrackInfo.class);
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC
        && constructor.getExceptionTypes().length == 0 && !constructor.isSynthetic(),
        "constructor metadata");
    Method process = type.getDeclaredMethod("processJoinedStream",
        LocalAudioTrackExecutor.class, InputStream.class);
    check(type.getDeclaredMethods().length == 1
        && process.getReturnType() == void.class
        && process.getModifiers() == Modifier.PROTECTED
        && Arrays.equals(process.getExceptionTypes(), new Class<?>[] {Exception.class})
        && !process.isSynthetic() && !process.isBridge() && !process.isVarArgs(),
        "process metadata");
  }

  private static void constructionContract() {
    AudioTrackInfo info = info();
    TestTrack track = new TestTrack(info);
    check(track.getInfo() == info, "constructor track info identity");
    check(new TestTrack(null).getInfo() == null, "null track info");
  }

  private static void wrapperAndDelegateContract() throws Exception {
    AudioTrackInfo info = info();
    TestTrack track = new TestTrack(info);
    CountingInputStream raw = new CountingInputStream();
    LocalAudioTrackExecutor executor = allocate(LocalAudioTrackExecutor.class);
    track.exposedProcess(executor, raw);

    check(track.calls == 1 && track.executor == executor, "delegate executor identity");
    check(track.delegate != null && track.delegate.getClass() == AdtsAudioTrack.class
        && track.delegate.getInfo() == info, "ADTS delegate and track info");

    Object pes = field(track.delegate, AdtsAudioTrack.class, "inputStream");
    check(pes != null && pes.getClass() == PesPacketInputStream.class, "PES wrapper");
    Object outerGreedy = field(pes, PesPacketInputStream.class, "inputStream");
    check(outerGreedy != null && outerGreedy.getClass() == GreedyInputStream.class,
        "outer greedy wrapper");
    Object elementary = field(outerGreedy, FilterInputStream.class, "in");
    check(elementary != null && elementary.getClass() == MpegTsElementaryInputStream.class,
        "MPEG-TS elementary wrapper");
    check(intField(elementary, MpegTsElementaryInputStream.class, "elementaryDataType") == 15,
        "ADTS elementary type");
    Object innerGreedy = field(elementary, MpegTsElementaryInputStream.class, "inputStream");
    check(innerGreedy != null && innerGreedy.getClass() == GreedyInputStream.class,
        "inner greedy wrapper");
    check(field(innerGreedy, FilterInputStream.class, "in") == raw,
        "raw stream identity");
    check(raw.reads == 0, "construction eagerly read input");
  }

  private static void nullAndFailureContract() throws Exception {
    TestTrack nullTrack = new TestTrack(info());
    nullTrack.exposedProcess(null, null);
    Object pes = field(nullTrack.delegate, AdtsAudioTrack.class, "inputStream");
    Object outerGreedy = field(pes, PesPacketInputStream.class, "inputStream");
    Object elementary = field(outerGreedy, FilterInputStream.class, "in");
    Object innerGreedy = field(elementary, MpegTsElementaryInputStream.class, "inputStream");
    check(nullTrack.executor == null && field(innerGreedy, FilterInputStream.class, "in") == null,
        "null identities");

    TestTrack failing = new TestTrack(info());
    Exception failure = new Exception("delegate-failure");
    failing.failure = failure;
    Exception thrown = expect(Exception.class,
        () -> failing.exposedProcess(null, new CountingInputStream()));
    check(thrown == failure && failing.calls == 1 && failing.delegate != null,
        "delegate failure identity");
  }

  private static AudioTrackInfo info() {
    return new AudioTrackInfo("title", "author", 123L, "fixture", false,
        "https://example.invalid/fixture", null, null);
  }

  private static final class TestTrack extends MpegTsM3uStreamAudioTrack {
    private InternalAudioTrack delegate;
    private LocalAudioTrackExecutor executor;
    private Exception failure;
    private int calls;

    TestTrack(AudioTrackInfo info) { super(info); }
    void exposedProcess(LocalAudioTrackExecutor executor, InputStream stream) throws Exception {
      processJoinedStream(executor, stream);
    }
    protected synchronized void processDelegate(InternalAudioTrack delegate,
        LocalAudioTrackExecutor executor) throws Exception {
      calls++;
      this.delegate = delegate;
      this.executor = executor;
      if (failure != null) throw failure;
    }
    protected M3uStreamSegmentUrlProvider getSegmentUrlProvider() { return null; }
    protected HttpInterface getHttpInterface() { return null; }
    protected AudioTrack makeShallowClone() { return new TestTrack(trackInfo); }
    public AudioSourceManager getSourceManager() { return null; }
  }

  private static final class CountingInputStream extends InputStream {
    private int reads;
    public int read() { reads++; return -1; }
    public int read(byte[] buffer, int offset, int length) {
      reads++;
      return -1;
    }
  }

  private static Object field(Object target, Class<?> owner, String name) throws Exception {
    Field field = owner.getDeclaredField(name);
    long offset = (Long) UNSAFE.getClass().getMethod("objectFieldOffset", Field.class)
        .invoke(UNSAFE, field);
    return UNSAFE.getClass().getMethod("getObject", Object.class, long.class)
        .invoke(UNSAFE, target, offset);
  }

  private static int intField(Object target, Class<?> owner, String name) throws Exception {
    Field field = owner.getDeclaredField(name);
    long offset = (Long) UNSAFE.getClass().getMethod("objectFieldOffset", Field.class)
        .invoke(UNSAFE, field);
    return (Integer) UNSAFE.getClass().getMethod("getInt", Object.class, long.class)
        .invoke(UNSAFE, target, offset);
  }

  private static <T> T allocate(Class<T> type) throws Exception {
    return type.cast(UNSAFE.getClass().getMethod("allocateInstance", Class.class)
        .invoke(UNSAFE, type));
  }

  private static Object loadUnsafe() {
    try {
      Class<?> unsafeType = Class.forName("sun.misc.Unsafe");
      Field singleton = unsafeType.getDeclaredField("theUnsafe");
      singleton.setAccessible(true);
      return singleton.get(null);
    } catch (Exception error) {
      throw new AssertionError(error);
    }
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private interface Operation { void run() throws Exception; }
  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const TWITCH_CONSTANTS_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.twitch.TwitchConstants;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.util.LinkedHashMap;
import java.util.Map;

public final class GateTwitchConstants {
  public static void main(String[] args) throws Exception {
    Class<TwitchConstants> type = TwitchConstants.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && type.getInterfaces().length == 0 && type.getDeclaredMethods().length == 0,
        "class metadata");

    Constructor<?> constructor = type.getDeclaredConstructor();
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC
        && constructor.getExceptionTypes().length == 0 && !constructor.isSynthetic(),
        "constructor metadata");
    Object first = constructor.newInstance();
    Object second = constructor.newInstance();
    check(first.getClass() == type && second.getClass() == type && first != second,
        "construction");

    Map<String, String> expected = new LinkedHashMap<>();
    expected.put("TWITCH_GRAPHQL_BASE_URL", "https://gql.twitch.tv/gql");
    expected.put("TWITCH_URL", "https://www.twitch.tv");
    expected.put("TWITCH_IMAGE_PREVIEW_URL",
        "https://static-cdn.jtvnw.net/previews-ttv/live_user_%s-440x248.jpg");
    expected.put("METADATA_PAYLOAD",
        "{\"operationName\":\"StreamMetadata\",\"variables\":{\"channelLogin\":\"%s\"},\"extensions\":{\"persistedQuery\":{\"version\":1,\"sha256Hash\":\"1c719a40e481453e5c48d9bb585d971b8b372f8ebb105b17076722264dfa5b3e\"}}}");
    expected.put("ACCESS_TOKEN_PAYLOAD",
        "{\"operationName\":\"PlaybackAccessToken_Template\",\"query\":\"query PlaybackAccessToken_Template($login: String!,$isLive:Boolean!,$vodID:ID!,$isVod:Boolean!,$playerType:String!){streamPlaybackAccessToken(channelName:$login,params:{platform:\\\"web\\\",playerBackend:\\\"mediaplayer\\\",playerType:$playerType})@include(if:$isLive){value signature __typename}videoPlaybackAccessToken(id:$vodID,params:{platform:\\\"web\\\",playerBackend:\\\"mediaplayer\\\",playerType:$playerType})@include(if:$isVod){value signature __typename}}\",\"variables\":{\"isLive\":true,\"login\":\"%s\",\"isVod\":false,\"vodID\":\"\",\"playerType\":\"site\"}}");

    check(type.getDeclaredFields().length == expected.size(), "field count");
    for (Map.Entry<String, String> entry : expected.entrySet()) {
      Field field = type.getDeclaredField(entry.getKey());
      check(field.getType() == String.class
          && field.getModifiers() == (Modifier.STATIC | Modifier.FINAL)
          && !field.isSynthetic(), entry.getKey() + " metadata");
      field.setAccessible(true);
      String actual = (String) field.get(null);
      check(actual.equals(entry.getValue()) && actual == entry.getValue(),
          entry.getKey() + " value and constant identity");
    }

    check(String.format(expected.get("TWITCH_IMAGE_PREVIEW_URL"), "mixed")
        .endsWith("live_user_mixed-440x248.jpg"), "preview substitution");
    check(String.format(expected.get("METADATA_PAYLOAD"), "mixed")
        .contains("\"channelLogin\":\"mixed\""), "metadata substitution");
    check(String.format(expected.get("ACCESS_TOKEN_PAYLOAD"), "mixed")
        .contains("\"login\":\"mixed\""), "access-token substitution");

    System.out.println("public-object-shell,5-package-constants,1-constructor,0-methods;"
        + "construction,urls,image-template,metadata-payload,access-token-payload,"
        + "format-substitution,constant-identity,reflection");
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const TWITCH_STREAM_AUDIO_SOURCE_MANAGER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.twitch.TwitchStreamAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.twitch.TwitchStreamAudioTrack;
import com.sedmelluq.discord.lavaplayer.tools.JsonBrowser;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpConfigurable;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterfaceManager;
import com.sedmelluq.discord.lavaplayer.track.AudioReference;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.io.DataInput;
import java.io.DataOutput;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.net.URI;
import java.util.Arrays;
import java.util.Locale;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.regex.Pattern;
import org.apache.http.client.methods.HttpUriRequest;
import org.apache.http.client.protocol.HttpClientContext;

public final class GateTwitchStreamAudioSourceManager {
  private static final Object UNSAFE = loadUnsafe();

  public static void main(String[] args) throws Exception {
    check(args.length >= 1 && args.length <= 2, "expected disposition and optional native path");
    boolean reference = args[0].equals("reference");
    check(reference || args[0].equals("candidate"), "unknown disposition");
    check(reference == (args.length == 1), "candidate requires native path");
    reflectionContract();
    commonBehavior();
    if (!reference) currentDisposition(args[1]);
    System.out.println(
        "common=public-concrete,6-fields,1-constructor,15-exported-methods,"
        + "source-name,legacy-route,empty-details,decode,requests,headers,http-config,shutdown;service="
        + (reference ? "legacy-homepage-graphql" :
            "current-helix,explicit-credentials,no-homepage-scrape,bounded-native"));
  }

  private static void reflectionContract() throws Exception {
    Class<TwitchStreamAudioSourceManager> type = TwitchStreamAudioSourceManager.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(),
            new Class<?>[] {AudioSourceManager.class, HttpConfigurable.class}), "class metadata");
    check(type.getDeclaredFields().length == 6, "field count");
    checkField(type, "log", Class.forName("org.slf4j.Logger"),
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "STREAM_NAME_REGEX", String.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "streamNameRegex", Pattern.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "httpInterfaceManager", HttpInterfaceManager.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "twitchClientId", String.class, Modifier.PRIVATE);
    checkField(type, "twitchDeviceId", String.class, Modifier.PRIVATE);

    Constructor<?> constructor = type.getDeclaredConstructor();
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC
        && constructor.getExceptionTypes().length == 0 && !constructor.isSynthetic(),
        "constructor metadata");
    check(type.getDeclaredMethods().length == 18, "method count");
    long exported = Arrays.stream(type.getDeclaredMethods())
        .filter(method -> Modifier.isPublic(method.getModifiers())
            || Modifier.isProtected(method.getModifiers()))
        .count();
    check(exported == 15L, "exported method count");
    checkMethod(type, "getClientId", String.class, Modifier.PUBLIC, new Class<?>[0]);
    checkMethod(type, "getDeviceId", String.class, Modifier.PUBLIC, new Class<?>[0]);
    checkMethod(type, "getSourceName", String.class, Modifier.PUBLIC, new Class<?>[0]);
    checkMethod(type, "loadItem",
        com.sedmelluq.discord.lavaplayer.track.AudioItem.class, Modifier.PUBLIC,
        new Class<?>[] {com.sedmelluq.discord.lavaplayer.player.AudioPlayerManager.class,
            AudioReference.class});
    checkMethod(type, "isTrackEncodable", boolean.class, Modifier.PUBLIC,
        new Class<?>[] {AudioTrack.class});
    checkMethod(type, "encodeTrack", void.class, Modifier.PUBLIC,
        new Class<?>[] {AudioTrack.class, DataOutput.class}, java.io.IOException.class);
    checkMethod(type, "decodeTrack", AudioTrack.class, Modifier.PUBLIC,
        new Class<?>[] {AudioTrackInfo.class, DataInput.class}, java.io.IOException.class);
    checkMethod(type, "getChannelIdentifierFromUrl", String.class,
        Modifier.PUBLIC | Modifier.STATIC, new Class<?>[] {String.class});
    checkMethod(type, "createGetRequest", HttpUriRequest.class, Modifier.PUBLIC,
        new Class<?>[] {String.class});
    checkMethod(type, "createGetRequest", HttpUriRequest.class, Modifier.PUBLIC,
        new Class<?>[] {URI.class});
    checkMethod(type, "getHttpInterface", HttpInterface.class, Modifier.PUBLIC,
        new Class<?>[0]);
    checkMethod(type, "configureRequests", void.class, Modifier.PUBLIC,
        new Class<?>[] {java.util.function.Function.class});
    checkMethod(type, "configureBuilder", void.class, Modifier.PUBLIC,
        new Class<?>[] {java.util.function.Consumer.class});
    checkMethod(type, "fetchAccessToken", JsonBrowser.class, Modifier.PROTECTED,
        new Class<?>[] {String.class});
    checkMethod(type, "shutdown", void.class, Modifier.PUBLIC, new Class<?>[0]);
  }

  private static void commonBehavior() throws Exception {
    RecordingHttpInterface http = new RecordingHttpInterface();
    ManagerHandler handler = new ManagerHandler(http);
    TwitchStreamAudioSourceManager manager = fabricated(handler.proxy(), "client-fixture",
        "device-fixture");
    check(manager.getSourceName().equals("twitch")
        && manager.getClientId().equals("client-fixture")
        && manager.getDeviceId().equals("device-fixture"), "source and credential getters");

    Locale previous = Locale.getDefault();
    try {
      Locale.setDefault(Locale.forLanguageTag("tr-TR"));
      check("mixed_i".equals(TwitchStreamAudioSourceManager.getChannelIdentifierFromUrl(
          "https://www.twitch.tv/MIXED_I")), "locale-root route normalization");
    } finally {
      Locale.setDefault(previous);
    }
    check("channel".equals(TwitchStreamAudioSourceManager.getChannelIdentifierFromUrl(
        "https://twitch.tv/Channel")), "bare host route");
    check("channel".equals(TwitchStreamAudioSourceManager.getChannelIdentifierFromUrl(
        "https://go.twitch.tv/Channel")), "go host route");
    check("channel".equals(TwitchStreamAudioSourceManager.getChannelIdentifierFromUrl(
        "https://m.twitch.tv/Channel")), "mobile host route");
    for (String rejected : new String[] {"http://twitch.tv/channel",
        "https://example.invalid/channel", "https://twitch.tv/channel/extra",
        "twitch.tv/channel"}) {
      check(TwitchStreamAudioSourceManager.getChannelIdentifierFromUrl(rejected) == null,
          "route rejection: " + rejected);
    }
    expect(NullPointerException.class,
        () -> TwitchStreamAudioSourceManager.getChannelIdentifierFromUrl(null));

    check(manager.isTrackEncodable(null), "encodability");
    manager.encodeTrack(null, proxy(DataOutput.class));
    AudioTrackInfo info = new AudioTrackInfo("title", "author", 1234L, "channel", true,
        "https://www.twitch.tv/channel", "art", null);
    AudioTrack decoded = manager.decodeTrack(info, proxy(DataInput.class));
    check(decoded instanceof TwitchStreamAudioTrack && decoded.getInfo() == info
        && decoded.getSourceManager() == manager, "empty-detail decode");

    HttpUriRequest fromText = manager.createGetRequest("https://example.invalid/text");
    HttpUriRequest fromUri = manager.createGetRequest(URI.create("https://example.invalid/uri"));
    check(fromText.getURI().equals(URI.create("https://example.invalid/text"))
        && fromUri.getURI().equals(URI.create("https://example.invalid/uri")), "request URIs");
    for (HttpUriRequest request : new HttpUriRequest[] {fromText, fromUri}) {
      check("client-fixture".equals(request.getFirstHeader("Client-ID").getValue())
          && "device-fixture".equals(request.getFirstHeader("X-Device-ID").getValue()),
          "request headers");
    }

    java.util.function.Function<org.apache.http.client.config.RequestConfig,
        org.apache.http.client.config.RequestConfig> requestConfig = value -> value;
    java.util.function.Consumer<org.apache.http.impl.client.HttpClientBuilder> builderConfig =
        value -> {};
    check(manager.getHttpInterface() == http, "HTTP interface identity");
    manager.configureRequests(requestConfig);
    manager.configureBuilder(builderConfig);
    check(handler.requestConfig == requestConfig && handler.builderConfig == builderConfig,
        "HTTP configuration identity");
    manager.shutdown();
    manager.shutdown();
    check(handler.closes.get() == 2, "shutdown delegation");
  }

  private static void currentDisposition(String nativeLibrary) throws Exception {
    Class.forName("dev.mantle.internal.NativeLoader")
        .getMethod("load", String.class).invoke(null, nativeLibrary);
    clearProperties();
    TwitchStreamAudioSourceManager empty = new TwitchStreamAudioSourceManager();
    check(empty.getClientId() == null && empty.getDeviceId() == null,
        "constructor does not scrape credentials");
    empty.shutdown();

    System.setProperty("dev.mantle.twitch.clientId", "client-current");
    System.setProperty("dev.mantle.twitch.accessToken", "oauth-current");
    System.setProperty("dev.mantle.twitch.deviceId", "device-current");
    TwitchStreamAudioSourceManager configured = new TwitchStreamAudioSourceManager();
    check("client-current".equals(configured.getClientId())
        && "device-current".equals(configured.getDeviceId()), "explicit constructor credentials");
    Class<?> nativeType = Class.forName("dev.mantle.internal.MantleNative");
    Method load = nativeType.getDeclaredMethod("loadTwitchItem",
        TwitchStreamAudioSourceManager.class, AudioReference.class);
    check(Modifier.isPublic(load.getModifiers()) && Modifier.isStatic(load.getModifiers())
        && Modifier.isNative(load.getModifiers()), "current native route");
    check(configured.loadItem(null,
        new AudioReference("https://example.invalid/channel", null)) == null,
        "foreign route rejected without service traffic");

    System.clearProperty("dev.mantle.twitch.clientId");
    System.clearProperty("dev.mantle.twitch.accessToken");
    RuntimeException missing = expect(RuntimeException.class, () -> configured.loadItem(null,
        new AudioReference("https://www.twitch.tv/fixture_channel", null)));
    check(missing.getMessage().contains("dev.mantle.twitch.clientId"),
        "missing credentials fail before network");
    Method token = TwitchStreamAudioSourceManager.class.getDeclaredMethod(
        "fetchAccessToken", String.class);
    token.setAccessible(true);
    UnsupportedOperationException unsupported = expectInvocation(
        UnsupportedOperationException.class, () -> token.invoke(configured, "fixture_channel"));
    check(unsupported.getMessage().contains("bounded credential-separated bridge"),
        "legacy token hook disabled");
    configured.shutdown();
    clearProperties();
  }

  private static TwitchStreamAudioSourceManager fabricated(
      HttpInterfaceManager http, String clientId, String deviceId) throws Exception {
    TwitchStreamAudioSourceManager manager = allocate(TwitchStreamAudioSourceManager.class);
    set(manager, "httpInterfaceManager", http);
    set(manager, "twitchClientId", clientId);
    set(manager, "twitchDeviceId", deviceId);
    return manager;
  }

  private static void set(Object owner, String name, Object value) throws Exception {
    Field field = TwitchStreamAudioSourceManager.class.getDeclaredField(name);
    field.setAccessible(true);
    field.set(owner, value);
  }

  private static void clearProperties() {
    System.clearProperty("dev.mantle.twitch.clientId");
    System.clearProperty("dev.mantle.twitch.accessToken");
    System.clearProperty("dev.mantle.twitch.deviceId");
  }

  private static void checkField(Class<?> owner, String name, Class<?> type, int modifiers)
      throws Exception {
    Field field = owner.getDeclaredField(name);
    check(field.getType() == type && field.getGenericType() == type
        && field.getModifiers() == modifiers && !field.isSynthetic(), name + " metadata");
  }

  private static Method checkMethod(Class<?> owner, String name, Class<?> returnType,
                                    int modifiers, Class<?>[] parameters,
                                    Class<?>... exceptions) throws Exception {
    Method method = owner.getDeclaredMethod(name, parameters);
    check(method.getReturnType() == returnType && method.getModifiers() == modifiers
        && Arrays.equals(method.getParameterTypes(), parameters)
        && Arrays.equals(method.getExceptionTypes(), exceptions)
        && method.getTypeParameters().length == 0 && !method.isBridge()
        && !method.isSynthetic() && !method.isVarArgs(), method + " metadata");
    return method;
  }

  private static final class RecordingHttpInterface extends HttpInterface {
    RecordingHttpInterface() { super(null, HttpClientContext.create(), false, null); }
  }

  private static final class ManagerHandler implements InvocationHandler {
    private final HttpInterface http;
    private Object requestConfig;
    private Object builderConfig;
    private final AtomicInteger closes = new AtomicInteger();
    ManagerHandler(HttpInterface http) { this.http = http; }
    HttpInterfaceManager proxy() {
      return (HttpInterfaceManager) Proxy.newProxyInstance(
          HttpInterfaceManager.class.getClassLoader(),
          new Class<?>[] {HttpInterfaceManager.class}, this);
    }
    public Object invoke(Object instance, Method method, Object[] arguments) {
      if (method.getName().equals("getInterface")) return http;
      if (method.getName().equals("configureRequests")) requestConfig = arguments[0];
      if (method.getName().equals("configureBuilder")) builderConfig = arguments[0];
      if (method.getName().equals("close")) closes.incrementAndGet();
      if (method.getName().equals("toString")) return "TwitchManagerFixture";
      return defaultValue(method.getReturnType());
    }
  }

  private static <T> T proxy(Class<T> type) {
    return type.cast(Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] {type},
        (instance, method, arguments) -> {
          throw new AssertionError("unexpected detail access: " + method);
        }));
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == byte.class) return (byte) 0;
    if (type == short.class) return (short) 0;
    if (type == int.class) return 0;
    if (type == long.class) return 0L;
    if (type == float.class) return 0.0f;
    if (type == double.class) return 0.0d;
    if (type == char.class) return (char) 0;
    return null;
  }

  private static <T> T allocate(Class<T> type) throws Exception {
    return type.cast(UNSAFE.getClass().getMethod("allocateInstance", Class.class)
        .invoke(UNSAFE, type));
  }

  private static Object loadUnsafe() {
    try {
      Class<?> unsafeType = Class.forName("sun.misc.Unsafe");
      Field singleton = unsafeType.getDeclaredField("theUnsafe");
      singleton.setAccessible(true);
      return singleton.get(null);
    } catch (Exception error) {
      throw new AssertionError(error);
    }
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private static <T extends Throwable> T expectInvocation(
      Class<T> type, Operation operation) throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (java.lang.reflect.InvocationTargetException error) {
      Throwable cause = error.getCause();
      if (!type.isInstance(cause)) throw new AssertionError("wrong exception", cause);
      return type.cast(cause);
    }
  }

  private interface Operation { void run() throws Exception; }
  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const TWITCH_STREAM_AUDIO_TRACK_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.stream.M3uStreamSegmentUrlProvider;
import com.sedmelluq.discord.lavaplayer.source.stream.MpegTsM3uStreamAudioTrack;
import com.sedmelluq.discord.lavaplayer.source.twitch.TwitchStreamAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.twitch.TwitchStreamAudioTrack;
import com.sedmelluq.discord.lavaplayer.source.twitch.TwitchStreamSegmentUrlProvider;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterfaceManager;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;
import java.util.concurrent.atomic.AtomicInteger;
import org.apache.http.client.protocol.HttpClientContext;

public final class GateTwitchStreamAudioTrack {
  private static final Object UNSAFE = loadUnsafe();

  public static void main(String[] args) throws Exception {
    check(args.length >= 1 && args.length <= 2, "expected disposition and optional native path");
    boolean reference = args[0].equals("reference");
    check(reference || args[0].equals("candidate"), "unknown disposition");
    check(reference == (args.length == 1), "candidate requires native path");
    reflectionContract();
    commonBehavior();
    if (!reference) currentDisposition(args[1]);
    System.out.println(
        "common=public-concrete,mpeg-super,3-fields,1-constructor,6-exported-methods;"
        + "construction,channel,provider,http,source-identity,shallow-clone,reflection;service="
        + (reference ? "legacy-provider-mpeg" :
            "current-native-bounded-hls,no-legacy-provider-playback"));
  }

  private static void reflectionContract() throws Exception {
    Class<TwitchStreamAudioTrack> type = TwitchStreamAudioTrack.class;
    check(type.getModifiers() == Modifier.PUBLIC
        && type.getSuperclass() == MpegTsM3uStreamAudioTrack.class
        && type.getInterfaces().length == 0, "class metadata");
    check(type.getDeclaredFields().length == 3, "field count");
    checkField(type, "log", Class.forName("org.slf4j.Logger"),
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "sourceManager", TwitchStreamAudioSourceManager.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "segmentUrlProvider", M3uStreamSegmentUrlProvider.class,
        Modifier.PRIVATE | Modifier.FINAL);
    Constructor<?> constructor = type.getDeclaredConstructor(
        AudioTrackInfo.class, TwitchStreamAudioSourceManager.class);
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC
        && constructor.getExceptionTypes().length == 0 && !constructor.isSynthetic(),
        "constructor metadata");
    check(type.getDeclaredMethods().length == 6, "method count");
    long exported = Arrays.stream(type.getDeclaredMethods())
        .filter(method -> Modifier.isPublic(method.getModifiers())
            || Modifier.isProtected(method.getModifiers()))
        .count();
    check(exported == 6L, "exported method count");
    checkMethod(type, "getChannelName", String.class, Modifier.PUBLIC, new Class<?>[0]);
    checkMethod(type, "getSegmentUrlProvider", M3uStreamSegmentUrlProvider.class,
        Modifier.PROTECTED, new Class<?>[0]);
    checkMethod(type, "getHttpInterface", HttpInterface.class,
        Modifier.PROTECTED, new Class<?>[0]);
    checkMethod(type, "process", void.class, Modifier.PUBLIC,
        new Class<?>[] {LocalAudioTrackExecutor.class}, Exception.class);
    checkMethod(type, "makeShallowClone", AudioTrack.class,
        Modifier.PROTECTED, new Class<?>[0]);
    checkMethod(type, "getSourceManager", AudioSourceManager.class,
        Modifier.PUBLIC, new Class<?>[0]);
  }

  private static void commonBehavior() throws Exception {
    RecordingHttpInterface http = new RecordingHttpInterface();
    ManagerHandler handler = new ManagerHandler(http);
    TwitchStreamAudioSourceManager source = fabricatedManager(handler.proxy());
    AudioTrackInfo info = new AudioTrackInfo("title", "author", Long.MAX_VALUE,
        "https://www.twitch.tv/MIXED_Channel", true,
        "https://www.twitch.tv/MIXED_Channel", "art", null);
    ExposedTrack track = new ExposedTrack(info, source);
    check(track.getInfo() == info && track.getSourceManager() == source
        && field("sourceManager").get(track) == source, "captured identity");
    check(track.getChannelName().equals("mixed_channel"), "channel normalization");
    M3uStreamSegmentUrlProvider provider = track.provider();
    check(provider instanceof TwitchStreamSegmentUrlProvider
        && field("segmentUrlProvider").get(track) == provider, "provider construction");
    check(track.http() == http && handler.interfaces.get() == 1, "HTTP delegation");
    AudioTrack clone = track.shallowClone();
    check(clone instanceof TwitchStreamAudioTrack && clone != track && clone.getInfo() == info
        && clone.getSourceManager() == source, "shallow clone identity");
    Object cloneProvider = field("segmentUrlProvider").get(clone);
    check(cloneProvider != provider && cloneProvider instanceof TwitchStreamSegmentUrlProvider,
        "fresh shallow-clone provider");
    check(field("log").get(null) != null, "static logger");
    source.shutdown();
    check(handler.closes.get() == 1, "source cleanup");
  }

  private static void currentDisposition(String nativeLibrary) throws Exception {
    Class.forName("dev.mantle.internal.NativeLoader")
        .getMethod("load", String.class).invoke(null, nativeLibrary);
    Class<?> nativeType = Class.forName("dev.mantle.internal.MantleNative");
    Method process = nativeType.getDeclaredMethod(
        "processTwitchTrack", TwitchStreamAudioTrack.class, LocalAudioTrackExecutor.class);
    check(Modifier.isPublic(process.getModifiers()) && Modifier.isStatic(process.getModifiers())
        && Modifier.isNative(process.getModifiers()), "current native route");
    clearProperties();
    TwitchStreamAudioSourceManager source = new TwitchStreamAudioSourceManager();
    TwitchStreamAudioTrack track = new TwitchStreamAudioTrack(new AudioTrackInfo(
        "title", "author", Long.MAX_VALUE, "https://www.twitch.tv/fixture_channel", true,
        "https://www.twitch.tv/fixture_channel", null, null), source);
    RuntimeException missing = expect(RuntimeException.class, () -> track.process(null));
    check(missing.getMessage().contains("dev.mantle.twitch.clientId"),
        "explicit playback credentials fail before service traffic");
    source.shutdown();
    clearProperties();
  }

  private static TwitchStreamAudioSourceManager fabricatedManager(HttpInterfaceManager http)
      throws Exception {
    TwitchStreamAudioSourceManager manager = allocate(TwitchStreamAudioSourceManager.class);
    Field field = TwitchStreamAudioSourceManager.class.getDeclaredField("httpInterfaceManager");
    field.setAccessible(true);
    field.set(manager, http);
    return manager;
  }

  private static Field field(String name) throws Exception {
    Field field = TwitchStreamAudioTrack.class.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }

  private static void clearProperties() {
    System.clearProperty("dev.mantle.twitch.clientId");
    System.clearProperty("dev.mantle.twitch.accessToken");
    System.clearProperty("dev.mantle.twitch.deviceId");
  }

  private static void checkField(Class<?> owner, String name, Class<?> type, int modifiers)
      throws Exception {
    Field field = owner.getDeclaredField(name);
    check(field.getType() == type && field.getGenericType() == type
        && field.getModifiers() == modifiers && !field.isSynthetic(), name + " metadata");
  }

  private static void checkMethod(Class<?> owner, String name, Class<?> returnType,
                                  int modifiers, Class<?>[] parameters,
                                  Class<?>... exceptions) throws Exception {
    Method method = owner.getDeclaredMethod(name, parameters);
    check(method.getReturnType() == returnType && method.getModifiers() == modifiers
        && Arrays.equals(method.getParameterTypes(), parameters)
        && Arrays.equals(method.getExceptionTypes(), exceptions)
        && method.getTypeParameters().length == 0 && !method.isBridge()
        && !method.isSynthetic() && !method.isVarArgs(), method + " metadata");
  }

  private static final class ExposedTrack extends TwitchStreamAudioTrack {
    ExposedTrack(AudioTrackInfo info, TwitchStreamAudioSourceManager source) {
      super(info, source);
    }
    M3uStreamSegmentUrlProvider provider() { return super.getSegmentUrlProvider(); }
    HttpInterface http() { return super.getHttpInterface(); }
    AudioTrack shallowClone() { return super.makeShallowClone(); }
  }

  private static final class RecordingHttpInterface extends HttpInterface {
    RecordingHttpInterface() { super(null, HttpClientContext.create(), false, null); }
  }

  private static final class ManagerHandler implements java.lang.reflect.InvocationHandler {
    private final HttpInterface http;
    private final AtomicInteger interfaces = new AtomicInteger();
    private final AtomicInteger closes = new AtomicInteger();
    ManagerHandler(HttpInterface http) { this.http = http; }
    HttpInterfaceManager proxy() {
      return (HttpInterfaceManager) Proxy.newProxyInstance(
          HttpInterfaceManager.class.getClassLoader(),
          new Class<?>[] {HttpInterfaceManager.class}, this);
    }
    public Object invoke(Object instance, Method method, Object[] arguments) {
      if (method.getName().equals("getInterface")) {
        interfaces.incrementAndGet();
        return http;
      }
      if (method.getName().equals("close")) closes.incrementAndGet();
      if (method.getName().equals("toString")) return "TwitchTrackManagerFixture";
      return defaultValue(method.getReturnType());
    }
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == byte.class) return (byte) 0;
    if (type == short.class) return (short) 0;
    if (type == int.class) return 0;
    if (type == long.class) return 0L;
    if (type == float.class) return 0.0f;
    if (type == double.class) return 0.0d;
    if (type == char.class) return (char) 0;
    return null;
  }

  private static <T> T allocate(Class<T> type) throws Exception {
    return type.cast(UNSAFE.getClass().getMethod("allocateInstance", Class.class)
        .invoke(UNSAFE, type));
  }

  private static Object loadUnsafe() {
    try {
      Class<?> unsafeType = Class.forName("sun.misc.Unsafe");
      Field singleton = unsafeType.getDeclaredField("theUnsafe");
      singleton.setAccessible(true);
      return singleton.get(null);
    } catch (Exception error) {
      throw new AssertionError(error);
    }
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private interface Operation { void run() throws Exception; }
  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const TWITCH_STREAM_SEGMENT_URL_PROVIDER_CONSUMER: &str = r##"
import com.sedmelluq.discord.lavaplayer.container.playlists.ExtendedM3uParser;
import com.sedmelluq.discord.lavaplayer.source.stream.M3uStreamSegmentUrlProvider;
import com.sedmelluq.discord.lavaplayer.source.twitch.TwitchStreamAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.twitch.TwitchStreamSegmentUrlProvider;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;
import org.apache.http.Header;
import org.apache.http.client.methods.HttpGet;
import org.apache.http.client.methods.HttpUriRequest;

public final class GateTwitchStreamSegmentUrlProvider {
  private static final Object UNSAFE = loadUnsafe();

  public static void main(String[] args) throws Exception {
    check(args.length == 1, "expected disposition");
    boolean reference = args[0].equals("reference");
    check(reference || args[0].equals("candidate"), "unknown disposition");
    reflectionContract();
    commonBehavior();
    if (!reference) currentDisposition();
    System.out.println(
        "common=public-concrete,m3u-provider-super,6-fields,1-constructor,3-exported-methods;"
        + "construction,video-quality,manager-get-request,reflection;service="
        + (reference ? "legacy-graphql-token-and-usher-playlist" :
            "deterministic-retired-provider,bounded-native-playback"));
  }

  private static void reflectionContract() throws Exception {
    Class<TwitchStreamSegmentUrlProvider> type = TwitchStreamSegmentUrlProvider.class;
    check(type.getModifiers() == Modifier.PUBLIC
        && type.getSuperclass() == M3uStreamSegmentUrlProvider.class
        && type.getInterfaces().length == 0, "class metadata");
    check(type.getDeclaredFields().length == 6, "field count");
    checkField(type, "TOKEN_PARAMETER", String.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "log", Class.forName("org.slf4j.Logger"),
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "channelName", String.class, Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "manager", TwitchStreamAudioSourceManager.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "streamSegmentPlaylistUrl", String.class, Modifier.PRIVATE);
    checkField(type, "tokenExpirationTime", long.class, Modifier.PRIVATE);
    Constructor<?> constructor = type.getDeclaredConstructor(
        String.class, TwitchStreamAudioSourceManager.class);
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC
        && constructor.getExceptionTypes().length == 0 && !constructor.isSynthetic(),
        "constructor metadata");
    check(type.getDeclaredMethods().length == 6, "declared method count");
    long exported = Arrays.stream(type.getDeclaredMethods())
        .filter(method -> Modifier.isPublic(method.getModifiers())
            || Modifier.isProtected(method.getModifiers()))
        .count();
    check(exported == 3L, "exported method count");
    checkMethod(type, "getQualityFromM3uDirective", String.class, Modifier.PROTECTED,
        new Class<?>[] {ExtendedM3uParser.Line.class});
    checkMethod(type, "fetchSegmentPlaylistUrl", String.class, Modifier.PROTECTED,
        new Class<?>[] {HttpInterface.class}, java.io.IOException.class);
    checkMethod(type, "createSegmentGetRequest", HttpUriRequest.class, Modifier.PROTECTED,
        new Class<?>[] {String.class});
  }

  private static void commonBehavior() throws Exception {
    TwitchStreamAudioSourceManager manager = fabricatedManager();
    ExposedProvider provider = new ExposedProvider("fixture_channel", manager);
    check(field("TOKEN_PARAMETER").get(null).equals("token")
        && field("log").get(null) != null, "static fields");
    check(field("channelName").get(provider).equals("fixture_channel")
        && field("manager").get(provider) == manager
        && field("streamSegmentPlaylistUrl").get(provider) == null
        && field("tokenExpirationTime").getLong(provider) == -1L
        && provider.base() == null, "constructor state");
    ExtendedM3uParser.Line quality = ExtendedM3uParser.parseLine(
        "#EXT-X-STREAM-INF:BANDWIDTH=128000,VIDEO=720p60");
    ExtendedM3uParser.Line missing = ExtendedM3uParser.parseLine(
        "#EXT-X-STREAM-INF:BANDWIDTH=64000");
    check(provider.quality(quality).equals("720p60") && provider.quality(missing) == null,
        "VIDEO quality selection");
    HttpUriRequest request = provider.request("https://example.invalid/live/segment.ts");
    check(request instanceof HttpGet
        && request.getURI().toString().equals("https://example.invalid/live/segment.ts")
        && header(request, "Client-ID").equals("fixture-client")
        && header(request, "X-Device-ID").equals("fixture-device"),
        "manager request delegation");
  }

  private static void currentDisposition() throws Exception {
    ExposedProvider provider = new ExposedProvider("fixture_channel", fabricatedManager());
    RuntimeException retired = expect(RuntimeException.class, () -> provider.fetch(null));
    check(retired.getMessage().contains("bounded native Twitch HLS pipeline"),
        "legacy provider fails before network access");
  }

  private static TwitchStreamAudioSourceManager fabricatedManager() throws Exception {
    TwitchStreamAudioSourceManager manager = allocate(TwitchStreamAudioSourceManager.class);
    set(manager, "twitchClientId", "fixture-client");
    set(manager, "twitchDeviceId", "fixture-device");
    return manager;
  }

  private static String header(HttpUriRequest request, String name) {
    Header header = request.getFirstHeader(name);
    return header == null ? null : header.getValue();
  }

  private static Field field(String name) throws Exception {
    Field field = TwitchStreamSegmentUrlProvider.class.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }

  private static void set(Object target, String name, Object value) throws Exception {
    Field field = target.getClass().getDeclaredField(name);
    field.setAccessible(true);
    field.set(target, value);
  }

  private static void checkField(Class<?> owner, String name, Class<?> type, int modifiers)
      throws Exception {
    Field field = owner.getDeclaredField(name);
    check(field.getType() == type && field.getGenericType() == type
        && field.getModifiers() == modifiers && !field.isSynthetic(), name + " metadata");
  }

  private static void checkMethod(Class<?> owner, String name, Class<?> returnType,
                                  int modifiers, Class<?>[] parameters,
                                  Class<?>... exceptions) throws Exception {
    Method method = owner.getDeclaredMethod(name, parameters);
    check(method.getReturnType() == returnType && method.getModifiers() == modifiers
        && Arrays.equals(method.getParameterTypes(), parameters)
        && Arrays.equals(method.getExceptionTypes(), exceptions)
        && method.getTypeParameters().length == 0 && !method.isBridge()
        && !method.isSynthetic() && !method.isVarArgs(), method + " metadata");
  }

  private static final class ExposedProvider extends TwitchStreamSegmentUrlProvider {
    ExposedProvider(String channel, TwitchStreamAudioSourceManager manager) {
      super(channel, manager);
    }
    String quality(ExtendedM3uParser.Line line) { return super.getQualityFromM3uDirective(line); }
    String fetch(HttpInterface http) throws Exception { return super.fetchSegmentPlaylistUrl(http); }
    HttpUriRequest request(String url) { return super.createSegmentGetRequest(url); }
    String base() { return super.baseUrl; }
  }

  private static <T> T allocate(Class<T> type) throws Exception {
    return type.cast(UNSAFE.getClass().getMethod("allocateInstance", Class.class)
        .invoke(UNSAFE, type));
  }

  private static Object loadUnsafe() {
    try {
      Class<?> unsafeType = Class.forName("sun.misc.Unsafe");
      Field singleton = unsafeType.getDeclaredField("theUnsafe");
      singleton.setAccessible(true);
      return singleton.get(null);
    } catch (Exception error) {
      throw new AssertionError(error);
    }
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private interface Operation { void run() throws Exception; }
  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"##;

const VIMEO_AUDIO_SOURCE_MANAGER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.player.AudioPlayerManager;
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.vimeo.VimeoAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.vimeo.VimeoAudioTrack;
import com.sedmelluq.discord.lavaplayer.tools.JsonBrowser;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpConfigurable;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterfaceManager;
import com.sedmelluq.discord.lavaplayer.track.AudioItem;
import com.sedmelluq.discord.lavaplayer.track.AudioReference;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.io.DataInput;
import java.io.DataOutput;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.regex.Pattern;
import org.apache.http.client.protocol.HttpClientContext;

public final class GateVimeoAudioSourceManager {
  private static final Object UNSAFE = loadUnsafe();

  public static void main(String[] args) throws Exception {
    check(args.length >= 1 && args.length <= 2, "expected disposition and optional native path");
    boolean reference = args[0].equals("reference");
    check(reference || args[0].equals("candidate"), "unknown disposition");
    check(reference == (args.length == 1), "candidate requires native path");
    reflectionContract();
    commonBehavior();
    if (!reference) currentDisposition(args[1]);
    System.out.println(
        "common=public-concrete,audio-source-http-configurable,3-fields,1-constructor,"
        + "11-exported-methods;construction,source-name,empty-details,decode,http-config,shutdown,"
        + "reflection;service="
        + (reference ? "legacy-viewer-jwt-api-and-config" :
            "current-public-config-or-caller-token,bounded-native,no-viewer-jwt"));
  }

  private static void reflectionContract() throws Exception {
    Class<VimeoAudioSourceManager> type = VimeoAudioSourceManager.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(),
            new Class<?>[] {AudioSourceManager.class, HttpConfigurable.class}), "class metadata");
    check(type.getDeclaredFields().length == 3, "field count");
    checkField(type, "TRACK_URL_REGEX", String.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "trackUrlPattern", Pattern.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "httpInterfaceManager", HttpInterfaceManager.class,
        Modifier.PRIVATE | Modifier.FINAL);
    Constructor<?> constructor = type.getDeclaredConstructor();
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC
        && constructor.getExceptionTypes().length == 0 && !constructor.isSynthetic(),
        "constructor metadata");
    check(type.getDeclaredMethods().length == 17, "declared method count");
    long exported = Arrays.stream(type.getDeclaredMethods())
        .filter(method -> Modifier.isPublic(method.getModifiers())
            || Modifier.isProtected(method.getModifiers()))
        .count();
    check(exported == 11L, "exported method count");
    checkMethod(type, "getSourceName", String.class, Modifier.PUBLIC, new Class<?>[0]);
    checkMethod(type, "loadItem", AudioItem.class, Modifier.PUBLIC,
        new Class<?>[] {AudioPlayerManager.class, AudioReference.class});
    checkMethod(type, "isTrackEncodable", boolean.class, Modifier.PUBLIC,
        new Class<?>[] {AudioTrack.class});
    checkMethod(type, "encodeTrack", void.class, Modifier.PUBLIC,
        new Class<?>[] {AudioTrack.class, DataOutput.class}, java.io.IOException.class);
    checkMethod(type, "decodeTrack", AudioTrack.class, Modifier.PUBLIC,
        new Class<?>[] {AudioTrackInfo.class, DataInput.class}, java.io.IOException.class);
    checkMethod(type, "shutdown", void.class, Modifier.PUBLIC, new Class<?>[0]);
    checkMethod(type, "getHttpInterface", HttpInterface.class, Modifier.PUBLIC, new Class<?>[0]);
    checkMethod(type, "configureRequests", void.class, Modifier.PUBLIC,
        new Class<?>[] {java.util.function.Function.class});
    checkMethod(type, "configureBuilder", void.class, Modifier.PUBLIC,
        new Class<?>[] {java.util.function.Consumer.class});
    checkMethod(type, "getVideoFromApi", JsonBrowser.class, Modifier.PUBLIC,
        new Class<?>[] {HttpInterface.class, String.class}, java.io.IOException.class,
        java.net.URISyntaxException.class);
    checkMethod(type, "getPlaybackFormat", VimeoAudioSourceManager.PlaybackFormat.class,
        Modifier.PUBLIC, new Class<?>[] {HttpInterface.class, String.class},
        java.io.IOException.class);
  }

  private static void commonBehavior() throws Exception {
    VimeoAudioSourceManager constructed = new VimeoAudioSourceManager();
    Object constructedHttp = field("httpInterfaceManager").get(constructed);
    check(constructedHttp instanceof HttpInterfaceManager
        && field("TRACK_URL_REGEX").get(null).equals("^https?://vimeo.com/([0-9]+)(?:\\?.*|)$")
        && field("trackUrlPattern").get(null) instanceof Pattern, "construction and statics");
    constructed.shutdown();

    RecordingHttpInterface http = new RecordingHttpInterface();
    ManagerHandler handler = new ManagerHandler(http);
    VimeoAudioSourceManager manager = fabricated(handler.proxy());
    check(manager.getSourceName().equals("vimeo") && manager.isTrackEncodable(null),
        "source identity and encodability");
    manager.encodeTrack(null, proxy(DataOutput.class));
    AudioTrackInfo info = new AudioTrackInfo("title", "author", 1234L, "76979871", false,
        "https://vimeo.com/76979871", "art", null);
    AudioTrack decoded = manager.decodeTrack(info, proxy(DataInput.class));
    check(decoded instanceof VimeoAudioTrack && decoded.getInfo() == info
        && decoded.getSourceManager() == manager, "empty-detail decode");
    Method getManager = VimeoAudioSourceManager.class.getDeclaredMethod("getHttpInterfaceManager");
    getManager.setAccessible(true);
    check(manager.getHttpInterface() == http && getManager.invoke(manager) == handler.proxyValue,
        "HTTP identity");
    java.util.function.Function<org.apache.http.client.config.RequestConfig,
        org.apache.http.client.config.RequestConfig> requests = value -> value;
    java.util.function.Consumer<org.apache.http.impl.client.HttpClientBuilder> builder = value -> {};
    manager.configureRequests(requests);
    manager.configureBuilder(builder);
    check(handler.requestConfig == requests && handler.builderConfig == builder,
        "HTTP configuration identity");
    manager.shutdown();
    manager.shutdown();
    check(handler.closes.get() == 2, "shutdown delegation");
  }

  private static void currentDisposition(String nativeLibrary) throws Exception {
    Class.forName("dev.mantle.internal.NativeLoader")
        .getMethod("load", String.class).invoke(null, nativeLibrary);
    Class<?> nativeType = Class.forName("dev.mantle.internal.MantleNative");
    Method load = nativeType.getDeclaredMethod(
        "loadVimeoItem", VimeoAudioSourceManager.class, AudioReference.class);
    check(Modifier.isPublic(load.getModifiers()) && Modifier.isStatic(load.getModifiers())
        && Modifier.isNative(load.getModifiers()), "current native route");
    clearProperties();
    VimeoAudioSourceManager manager = new VimeoAudioSourceManager();
    check(manager.loadItem(null,
        new AudioReference("https://example.invalid/76979871", null)) == null,
        "foreign route rejected without service traffic");
    System.setProperty("dev.mantle.vimeo.accessToken", "");
    RuntimeException invalid = expect(RuntimeException.class, () -> manager.loadItem(null,
        new AudioReference("https://vimeo.com/76979871", null)));
    check(invalid.getMessage().contains("invalid Vimeo JVM access token"),
        "invalid caller token fails before service traffic");
    UnsupportedOperationException api = expect(UnsupportedOperationException.class,
        () -> manager.getVideoFromApi(null, "76979871"));
    check(api.getMessage().contains("explicit caller token"), "legacy viewer JWT disabled");
    UnsupportedOperationException format = expect(UnsupportedOperationException.class,
        () -> manager.getPlaybackFormat(null, "https://example.invalid/config"));
    check(format.getMessage().contains("bounded progressive MP4 discovery"),
        "legacy format hook disabled");
    manager.shutdown();
    clearProperties();
  }

  private static VimeoAudioSourceManager fabricated(HttpInterfaceManager http) throws Exception {
    VimeoAudioSourceManager manager = allocate(VimeoAudioSourceManager.class);
    Field field = VimeoAudioSourceManager.class.getDeclaredField("httpInterfaceManager");
    field.setAccessible(true);
    field.set(manager, http);
    return manager;
  }

  private static Field field(String name) throws Exception {
    Field field = VimeoAudioSourceManager.class.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }

  private static void clearProperties() {
    System.clearProperty("dev.mantle.vimeo.accessToken");
  }

  private static void checkField(Class<?> owner, String name, Class<?> type, int modifiers)
      throws Exception {
    Field field = owner.getDeclaredField(name);
    check(field.getType() == type && field.getGenericType() == type
        && field.getModifiers() == modifiers && !field.isSynthetic(), name + " metadata");
  }

  private static void checkMethod(Class<?> owner, String name, Class<?> returnType,
                                  int modifiers, Class<?>[] parameters,
                                  Class<?>... exceptions) throws Exception {
    Method method = owner.getDeclaredMethod(name, parameters);
    check(method.getReturnType() == returnType && method.getModifiers() == modifiers
        && Arrays.equals(method.getParameterTypes(), parameters)
        && Arrays.equals(method.getExceptionTypes(), exceptions)
        && method.getTypeParameters().length == 0 && !method.isBridge()
        && !method.isSynthetic() && !method.isVarArgs(), method + " metadata");
  }

  private static final class RecordingHttpInterface extends HttpInterface {
    RecordingHttpInterface() { super(null, HttpClientContext.create(), false, null); }
  }

  private static final class ManagerHandler implements InvocationHandler {
    private final HttpInterface http;
    private final AtomicInteger closes = new AtomicInteger();
    private HttpInterfaceManager proxyValue;
    private Object requestConfig;
    private Object builderConfig;
    ManagerHandler(HttpInterface http) { this.http = http; }
    HttpInterfaceManager proxy() {
      if (proxyValue == null) {
        proxyValue = (HttpInterfaceManager) Proxy.newProxyInstance(
            HttpInterfaceManager.class.getClassLoader(),
            new Class<?>[] {HttpInterfaceManager.class}, this);
      }
      return proxyValue;
    }
    public Object invoke(Object instance, Method method, Object[] arguments) {
      if (method.getName().equals("getInterface")) return http;
      if (method.getName().equals("configureRequests")) requestConfig = arguments[0];
      if (method.getName().equals("configureBuilder")) builderConfig = arguments[0];
      if (method.getName().equals("close")) closes.incrementAndGet();
      if (method.getName().equals("toString")) return "VimeoManagerFixture";
      return defaultValue(method.getReturnType());
    }
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == byte.class) return (byte) 0;
    if (type == short.class) return (short) 0;
    if (type == int.class) return 0;
    if (type == long.class) return 0L;
    if (type == float.class) return 0.0f;
    if (type == double.class) return 0.0d;
    if (type == char.class) return (char) 0;
    return null;
  }

  private static <T> T allocate(Class<T> type) throws Exception {
    return type.cast(UNSAFE.getClass().getMethod("allocateInstance", Class.class)
        .invoke(UNSAFE, type));
  }

  private static <T> T proxy(Class<T> type) {
    return type.cast(Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] {type},
        (instance, method, arguments) -> defaultValue(method.getReturnType())));
  }

  private static Object loadUnsafe() {
    try {
      Class<?> unsafeType = Class.forName("sun.misc.Unsafe");
      Field singleton = unsafeType.getDeclaredField("theUnsafe");
      singleton.setAccessible(true);
      return singleton.get(null);
    } catch (Exception error) {
      throw new AssertionError(error);
    }
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (error instanceof InvocationTargetException) error = error.getCause();
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private interface Operation { void run() throws Exception; }
  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const VIMEO_PLAYBACK_FORMAT_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.vimeo.VimeoAudioSourceManager;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;

public final class GateVimeoPlaybackFormat {
  public static void main(String[] args) throws Exception {
    check(args.length == 1
        && (args[0].equals("reference") || args[0].equals("candidate")),
        "expected disposition");
    Class<VimeoAudioSourceManager.PlaybackFormat> type =
        VimeoAudioSourceManager.PlaybackFormat.class;
    check(type.getModifiers() == (Modifier.PUBLIC | Modifier.STATIC)
        && type.getSuperclass() == Object.class && type.getInterfaces().length == 0
        && type.getDeclaringClass() == VimeoAudioSourceManager.class
        && type.getEnclosingClass() == VimeoAudioSourceManager.class
        && type.getSimpleName().equals("PlaybackFormat"), "class metadata");
    check(type.getDeclaredFields().length == 2 && type.getDeclaredMethods().length == 0,
        "member counts");
    checkField(type, "url", String.class, Modifier.PUBLIC | Modifier.FINAL);
    checkField(type, "isHls", boolean.class, Modifier.PUBLIC | Modifier.FINAL);
    Constructor<?> constructor = type.getDeclaredConstructor(String.class, boolean.class);
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC
        && constructor.getExceptionTypes().length == 0 && !constructor.isSynthetic(),
        "constructor metadata");

    String progressiveUrl = new String("https://player.vimeo.com/audio.mp4");
    VimeoAudioSourceManager.PlaybackFormat progressive =
        new VimeoAudioSourceManager.PlaybackFormat(progressiveUrl, false);
    VimeoAudioSourceManager.PlaybackFormat hls =
        new VimeoAudioSourceManager.PlaybackFormat(null, true);
    check(progressive.url == progressiveUrl && !progressive.isHls
        && hls.url == null && hls.isHls && progressive != hls && !progressive.equals(hls),
        "constructor value identity");
    System.out.println(
        "public-static,object-super,2-public-final-fields,1-public-constructor;"
        + "url-and-hls-value-identity,null-preserved,reflection");
  }

  private static void checkField(Class<?> owner, String name, Class<?> type, int modifiers)
      throws Exception {
    Field field = owner.getDeclaredField(name);
    check(field.getType() == type && field.getGenericType() == type
        && field.getModifiers() == modifiers && !field.isSynthetic(), name + " metadata");
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const VIMEO_AUDIO_TRACK_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.vimeo.VimeoAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.vimeo.VimeoAudioTrack;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import com.sedmelluq.discord.lavaplayer.track.DelegatedAudioTrack;
import com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;

public final class GateVimeoAudioTrack {
  public static void main(String[] args) throws Exception {
    check(args.length >= 1 && args.length <= 2, "expected disposition and optional native path");
    boolean reference = args[0].equals("reference");
    check(reference || args[0].equals("candidate"), "unknown disposition");
    check(reference == (args.length == 1), "candidate requires native path");
    reflectionContract();
    commonBehavior();
    if (!reference) currentDisposition(args[1]);
    System.out.println(
        "common=public-concrete,delegated-super,2-fields,1-constructor,4-exported-methods;"
        + "capture,source-identity,relative-url,shallow-clone,reflection;service="
        + (reference ? "legacy-viewer-jwt-config,hls-or-mpeg" :
            "current-native-bounded-progressive-mp4,no-viewer-jwt-or-legacy-hls"));
  }

  private static void reflectionContract() throws Exception {
    Class<VimeoAudioTrack> type = VimeoAudioTrack.class;
    check(type.getModifiers() == Modifier.PUBLIC
        && type.getSuperclass() == DelegatedAudioTrack.class
        && type.getInterfaces().length == 0, "class metadata");
    check(type.getDeclaredFields().length == 2, "field count");
    checkField(type, "log", Class.forName("org.slf4j.Logger"),
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "sourceManager", VimeoAudioSourceManager.class,
        Modifier.PRIVATE | Modifier.FINAL);
    Constructor<?> constructor = type.getDeclaredConstructor(
        AudioTrackInfo.class, VimeoAudioSourceManager.class);
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC
        && constructor.getExceptionTypes().length == 0 && !constructor.isSynthetic(),
        "constructor metadata");
    check(type.getDeclaredMethods().length == 5, "declared method count");
    long exported = Arrays.stream(type.getDeclaredMethods())
        .filter(method -> Modifier.isPublic(method.getModifiers())
            || Modifier.isProtected(method.getModifiers()))
        .count();
    check(exported == 4L, "exported method count");
    checkMethod(type, "process", void.class, Modifier.PUBLIC,
        new Class<?>[] {LocalAudioTrackExecutor.class}, Exception.class);
    checkMethod(type, "resolveRelativeUrl", String.class, Modifier.PROTECTED,
        new Class<?>[] {String.class, String.class});
    checkMethod(type, "extractHlsAudioPlaylistUrl", String.class, Modifier.PRIVATE,
        new Class<?>[] {HttpInterface.class, String.class}, IOException.class);
    checkMethod(type, "makeShallowClone", AudioTrack.class, Modifier.PROTECTED,
        new Class<?>[0]);
    checkMethod(type, "getSourceManager", AudioSourceManager.class, Modifier.PUBLIC,
        new Class<?>[0]);
  }

  private static void commonBehavior() throws Exception {
    VimeoAudioSourceManager source = new VimeoAudioSourceManager();
    AudioTrackInfo info = new AudioTrackInfo("title", "author", 1234L, "76979871", false,
        "https://vimeo.com/76979871", "art", null);
    ExposedTrack track = new ExposedTrack(info, source);
    check(track.getInfo() == info && track.getSourceManager() == source
        && field("sourceManager").get(track) == source, "captured identity");
    check(field("log").get(null) != null, "static logger");
    String base = "https://player.vimeo.com/video/76979871/master";
    check(track.resolve(base, "audio.m3u8").equals(base + "/audio.m3u8")
        && track.resolve(base, "/audio.m3u8").equals(base + "/audio.m3u8")
        && track.resolve(base, "../audio.m3u8")
            .equals("https://player.vimeo.com/video/76979871/audio.m3u8")
        && track.resolve(base, "../../audio.m3u8")
            .equals("https://player.vimeo.com/video/audio.m3u8")
        && track.resolve(null, "audio.m3u8").equals("null/audio.m3u8"),
        "relative URL behavior");
    expect(NullPointerException.class, () -> track.resolve(base, null));
    expect(StringIndexOutOfBoundsException.class, () -> track.resolve("root", "../audio"));
    AudioTrack clone = track.shallowClone();
    check(clone instanceof VimeoAudioTrack && clone != track && clone.getInfo() == info
        && clone.getSourceManager() == source, "shallow clone identity");
    source.shutdown();
  }

  private static void currentDisposition(String nativeLibrary) throws Exception {
    Class.forName("dev.mantle.internal.NativeLoader")
        .getMethod("load", String.class).invoke(null, nativeLibrary);
    Class<?> nativeType = Class.forName("dev.mantle.internal.MantleNative");
    Method process = nativeType.getDeclaredMethod(
        "processVimeoTrack", VimeoAudioTrack.class, LocalAudioTrackExecutor.class);
    check(Modifier.isPublic(process.getModifiers()) && Modifier.isStatic(process.getModifiers())
        && Modifier.isNative(process.getModifiers()), "current native route");
    System.setProperty("dev.mantle.vimeo.accessToken", "");
    VimeoAudioSourceManager source = new VimeoAudioSourceManager();
    VimeoAudioTrack track = new VimeoAudioTrack(new AudioTrackInfo(
        "title", "author", 1234L, "76979871", false,
        "https://vimeo.com/76979871", null, null), source);
    RuntimeException invalid = expect(RuntimeException.class, () -> track.process(null));
    check(invalid.getMessage().contains("invalid Vimeo JVM access token"),
        "invalid caller token fails before service traffic");
    source.shutdown();
    System.clearProperty("dev.mantle.vimeo.accessToken");
  }

  private static Field field(String name) throws Exception {
    Field field = VimeoAudioTrack.class.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }

  private static void checkField(Class<?> owner, String name, Class<?> type, int modifiers)
      throws Exception {
    Field field = owner.getDeclaredField(name);
    check(field.getType() == type && field.getGenericType() == type
        && field.getModifiers() == modifiers && !field.isSynthetic(), name + " metadata");
  }

  private static void checkMethod(Class<?> owner, String name, Class<?> returnType,
                                  int modifiers, Class<?>[] parameters,
                                  Class<?>... exceptions) throws Exception {
    Method method = owner.getDeclaredMethod(name, parameters);
    check(method.getReturnType() == returnType && method.getModifiers() == modifiers
        && Arrays.equals(method.getParameterTypes(), parameters)
        && Arrays.equals(method.getExceptionTypes(), exceptions)
        && method.getTypeParameters().length == 0 && !method.isBridge()
        && !method.isSynthetic() && !method.isVarArgs(), method + " metadata");
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private static final class ExposedTrack extends VimeoAudioTrack {
    ExposedTrack(AudioTrackInfo info, VimeoAudioSourceManager source) { super(info, source); }
    String resolve(String base, String url) { return super.resolveRelativeUrl(base, url); }
    AudioTrack shallowClone() { return super.makeShallowClone(); }
  }

  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const ABSTRACT_YANDEX_MUSIC_API_LOADER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.yamusic.AbstractYandexMusicApiLoader;
import com.sedmelluq.discord.lavaplayer.source.yamusic.DefaultYandexMusicTrackLoader;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicApiLoader;
import com.sedmelluq.discord.lavaplayer.tools.http.ExtendedHttpConfigurable;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterfaceManager;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import java.lang.reflect.TypeVariable;
import java.lang.reflect.Proxy;
import java.util.concurrent.atomic.AtomicInteger;

public final class GateAbstractYandexMusicApiLoader {
  public static void main(String[] args) throws Exception {
    check(args.length == 1, "expected service disposition");
    boolean reference = args[0].equals("reference");
    check(reference || args[0].equals("candidate"), "unknown disposition");
    reflectionContract();
    lifecycleContract();
    if (!reference) currentDisposition();
    System.out.println(
        "common=public-abstract,object-super,yandex-api-loader,1-protected-field,"
        + "1-package-constructor,3-exported-methods;generic-extractor,http-config-identity,"
        + "mutable-manager,repeatable-warning-close,reflection;service="
        + (reference ? "legacy-arbitrary-url-get-unbounded-json" :
            "deterministic-no-network,current-bounded-native-source"));
  }

  private static void reflectionContract() throws Exception {
    Class<AbstractYandexMusicApiLoader> type = AbstractYandexMusicApiLoader.class;
    check(type.getModifiers() == (Modifier.PUBLIC | Modifier.ABSTRACT)
        && type.getSuperclass() == Object.class
        && type.getInterfaces().length == 1
        && type.getInterfaces()[0] == YandexMusicApiLoader.class, "class metadata");
    check(type.getDeclaredFields().length == 1, "field count");
    Field manager = type.getDeclaredField("httpInterfaceManager");
    check(manager.getType() == HttpInterfaceManager.class
        && manager.getGenericType() == HttpInterfaceManager.class
        && manager.getModifiers() == Modifier.PROTECTED && !manager.isSynthetic(),
        "manager field metadata");
    Constructor<?> constructor = type.getDeclaredConstructor();
    check(type.getDeclaredConstructors().length == 1 && constructor.getModifiers() == 0
        && constructor.getExceptionTypes().length == 0 && !constructor.isSynthetic(),
        "constructor metadata");
    check(type.getDeclaredMethods().length == 3, "declared method count");
    Method extract = type.getDeclaredMethod("extractFromApi", String.class,
        Class.forName(type.getName() + "$ApiExtractor"));
    check(extract.getModifiers() == Modifier.PROTECTED && extract.getReturnType() == Object.class
        && extract.getExceptionTypes().length == 0 && !extract.isBridge()
        && !extract.isSynthetic() && !extract.isVarArgs(), "extract metadata");
    TypeVariable<Method>[] parameters = extract.getTypeParameters();
    Type genericExtractor = extract.getGenericParameterTypes()[1];
    check(parameters.length == 1 && extract.getGenericReturnType().equals(parameters[0])
        && genericExtractor instanceof ParameterizedType
        && ((ParameterizedType) genericExtractor).getActualTypeArguments().length == 1
        && ((ParameterizedType) genericExtractor).getActualTypeArguments()[0].equals(parameters[0]),
        "generic extractor signature");
    checkMethod(type, "getHttpConfiguration", ExtendedHttpConfigurable.class, Modifier.PUBLIC);
    checkMethod(type, "shutdown", void.class, Modifier.PUBLIC);
  }

  private static void lifecycleContract() throws Exception {
    AbstractYandexMusicApiLoader loader = new DefaultYandexMusicTrackLoader();
    Field managerField = AbstractYandexMusicApiLoader.class
        .getDeclaredField("httpInterfaceManager");
    managerField.setAccessible(true);
    Object initial = managerField.get(loader);
    check(initial instanceof HttpInterfaceManager && loader.getHttpConfiguration() == initial,
        "initial manager identity");
    loader.shutdown();

    AtomicInteger closes = new AtomicInteger();
    HttpInterfaceManager replacement = (HttpInterfaceManager) Proxy.newProxyInstance(
        GateAbstractYandexMusicApiLoader.class.getClassLoader(),
        new Class<?>[] {HttpInterfaceManager.class},
        (proxy, method, arguments) -> {
          if (method.getName().equals("close")) {
            if (closes.incrementAndGet() == 3) throw new IOException("expected close failure");
            return null;
          }
          if (method.getName().equals("toString")) return "gate-yandex-http-manager";
          if (method.getName().equals("hashCode")) return System.identityHashCode(proxy);
          if (method.getName().equals("equals")) return proxy == arguments[0];
          Class<?> returnType = method.getReturnType();
          if (returnType == boolean.class) return false;
          if (returnType == int.class) return 0;
          if (returnType == long.class) return 0L;
          return null;
        });
    managerField.set(loader, replacement);
    check(loader.getHttpConfiguration() == replacement, "replacement manager identity");
    loader.shutdown();
    loader.shutdown();
    loader.shutdown();
    check(closes.get() == 3, "repeatable warning close");
  }

  private static void currentDisposition() throws Exception {
    AbstractYandexMusicApiLoader loader = new DefaultYandexMusicTrackLoader();
    try {
      Method extract = AbstractYandexMusicApiLoader.class.getDeclaredMethod("extractFromApi",
          String.class, Class.forName(AbstractYandexMusicApiLoader.class.getName() + "$ApiExtractor"));
      extract.setAccessible(true);
      extract.invoke(loader, "http://127.0.0.1:1/legacy-yandex-api", null);
      throw new AssertionError("legacy API extraction unexpectedly succeeded");
    } catch (InvocationTargetException error) {
      Throwable cause = error.getCause();
      check(cause instanceof UnsupportedOperationException
          && cause.getMessage().contains("Legacy Yandex API extraction is unsupported"),
          "legacy extraction fails before network access");
    } finally {
      loader.shutdown();
    }
  }

  private static void checkMethod(Class<?> owner, String name, Class<?> returnType, int modifiers)
      throws Exception {
    Method method = owner.getDeclaredMethod(name);
    check(method.getReturnType() == returnType && method.getModifiers() == modifiers
        && method.getParameterCount() == 0 && method.getExceptionTypes().length == 0
        && method.getTypeParameters().length == 0 && !method.isBridge()
        && !method.isSynthetic() && !method.isVarArgs(), name + " metadata");
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const YANDEX_MUSIC_API_EXTRACTOR_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.yamusic.AbstractYandexMusicApiLoader;
import com.sedmelluq.discord.lavaplayer.tools.JsonBrowser;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.lang.reflect.TypeVariable;
import java.util.Arrays;
import java.util.concurrent.atomic.AtomicInteger;

public final class GateYandexMusicApiExtractor {
  public static void main(String[] args) throws Exception {
    check(args.length == 0, "unexpected arguments");
    Class<?> owner = AbstractYandexMusicApiLoader.class;
    Class<?> type = Class.forName(owner.getName() + "$ApiExtractor");
    int modifiers = Modifier.PROTECTED | Modifier.STATIC | Modifier.INTERFACE | Modifier.ABSTRACT;
    check(type.getModifiers() == modifiers && type.isInterface() && !type.isAnnotation()
        && !type.isEnum() && !type.isSynthetic(), "interface metadata");
    check(type.getSuperclass() == null && type.getInterfaces().length == 0
        && type.getGenericInterfaces().length == 0 && type.getGenericSuperclass() == null,
        "interface hierarchy");
    check(type.getDeclaringClass() == owner && type.getNestHost() == owner
        && type.getSimpleName().equals("ApiExtractor")
        && Arrays.asList(owner.getDeclaredClasses()).contains(type), "nested metadata");
    TypeVariable<?>[] variables = type.getTypeParameters();
    check(variables.length == 1 && variables[0].getName().equals("T")
        && variables[0].getBounds().length == 1
        && variables[0].getBounds()[0] == Object.class, "class generic signature");
    check(type.getDeclaredFields().length == 0 && type.getDeclaredConstructors().length == 0
        && type.getDeclaredMethods().length == 1, "member counts");
    Method extract = type.getDeclaredMethod("extract", HttpInterface.class, JsonBrowser.class);
    check(extract.getModifiers() == (Modifier.PUBLIC | Modifier.ABSTRACT)
        && extract.getReturnType() == Object.class
        && Arrays.equals(extract.getParameterTypes(),
            new Class<?>[] {HttpInterface.class, JsonBrowser.class})
        && Arrays.equals(extract.getGenericParameterTypes(),
            new Class<?>[] {HttpInterface.class, JsonBrowser.class})
        && extract.getGenericReturnType().equals(variables[0])
        && Arrays.equals(extract.getExceptionTypes(), new Class<?>[] {Exception.class})
        && extract.getTypeParameters().length == 0 && !extract.isDefault()
        && !extract.isBridge() && !extract.isSynthetic() && !extract.isVarArgs(),
        "extract metadata");
    extract.setAccessible(true);
    AtomicInteger calls = new AtomicInteger();
    Object sentinel = new Object();
    Object success = Proxy.newProxyInstance(GateYandexMusicApiExtractor.class.getClassLoader(),
        new Class<?>[] {type}, (proxy, method, arguments) -> {
          check(method.getName().equals("extract") && arguments.length == 2,
              "proxy invocation shape");
          calls.incrementAndGet();
          return sentinel;
        });
    check(extract.invoke(success, null, null) == sentinel && calls.get() == 1,
        "generic return invocation");
    Object failure = Proxy.newProxyInstance(GateYandexMusicApiExtractor.class.getClassLoader(),
        new Class<?>[] {type}, (proxy, method, arguments) -> {
          throw new Exception("expected checked failure");
        });
    try {
      extract.invoke(failure, null, null);
      throw new AssertionError("checked exception unexpectedly absent");
    } catch (InvocationTargetException error) {
      check(error.getCause().getClass() == Exception.class
          && error.getCause().getMessage().equals("expected checked failure"),
          "checked exception propagation");
    }
    System.out.println(
        "protected-static-generic-interface,no-reflection-super,1-type-variable,"
        + "1-public-abstract-method;erased-object-return,generic-T-return,"
        + "http-json-parameters,checked-exception,proxy-invocation,reflection");
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const DEFAULT_YANDEX_MUSIC_DIRECT_URL_LOADER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.yamusic.AbstractYandexMusicApiLoader;
import com.sedmelluq.discord.lavaplayer.source.yamusic.DefaultYandexMusicDirectUrlLoader;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicDirectUrlLoader;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;

public final class GateDefaultYandexMusicDirectUrlLoader {
  public static void main(String[] args) throws Exception {
    check(args.length == 1, "mode required");
    Class<?> type = DefaultYandexMusicDirectUrlLoader.class;
    check(type.getModifiers() == Modifier.PUBLIC && !type.isInterface() && !type.isEnum()
        && !type.isAnnotation() && !type.isSynthetic(), "class metadata");
    check(type.getSuperclass() == AbstractYandexMusicApiLoader.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {YandexMusicDirectUrlLoader.class}),
        "class hierarchy");
    check(type.getDeclaredFields().length == 3 && type.getDeclaredConstructors().length == 1
        && type.getDeclaredMethods().length == 4, "member counts");
    checkConstant(type, "TRACK_DOWNLOAD_INFO",
        "https://api.music.yandex.net/tracks/%s/download-info");
    checkConstant(type, "DIRECT_URL_FORMAT", "https://%s/get-%s/%s/%s%s");
    checkConstant(type, "MP3_SALT", "XGRlBW9FXlekgbPrRHuSiA");
    Constructor<?> constructor = type.getDeclaredConstructor();
    check(constructor.getModifiers() == Modifier.PUBLIC && !constructor.isSynthetic()
        && constructor.getExceptionTypes().length == 0, "constructor metadata");
    Method directUrl = type.getDeclaredMethod("getDirectUrl", String.class, String.class);
    check(directUrl.getModifiers() == Modifier.PUBLIC && directUrl.getReturnType() == String.class
        && directUrl.getExceptionTypes().length == 0 && !directUrl.isBridge()
        && !directUrl.isSynthetic() && !directUrl.isVarArgs(), "direct URL metadata");
    Method downloadInfo = type.getDeclaredMethod("extractDownloadInfo", String.class);
    check(downloadInfo.getModifiers() == Modifier.PRIVATE
        && downloadInfo.getReturnType().getName().equals(type.getName() + "$DownloadInfo")
        && Arrays.equals(downloadInfo.getExceptionTypes(), new Class<?>[] {IOException.class})
        && !downloadInfo.isSynthetic(), "download-info helper metadata");
    Method lambdaOne = type.getDeclaredMethod("lambda$getDirectUrl$1", String.class,
        Class.forName("com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface"),
        Class.forName("com.sedmelluq.discord.lavaplayer.tools.JsonBrowser"));
    check(lambdaOne.getModifiers() == (Modifier.PRIVATE | 0x1000)
        && lambdaOne.getReturnType() == String.class
        && Arrays.equals(lambdaOne.getExceptionTypes(), new Class<?>[] {Exception.class}),
        "capturing lambda metadata");
    Method lambdaZero = type.getDeclaredMethod("lambda$getDirectUrl$0", String.class,
        Class.forName("com.sedmelluq.discord.lavaplayer.tools.JsonBrowser"));
    check(lambdaZero.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC | 0x1000)
        && lambdaZero.getReturnType() == boolean.class
        && lambdaZero.getExceptionTypes().length == 0, "predicate lambda metadata");

    DefaultYandexMusicDirectUrlLoader loader =
        (DefaultYandexMusicDirectUrlLoader) constructor.newInstance();
    check(loader != null && loader instanceof YandexMusicDirectUrlLoader
        && loader.getHttpConfiguration() != null, "construction and inherited configuration");
    try {
      if (args[0].equals("candidate")) {
        assertUnsupported(loader, "http://127.0.0.1:1/legacy-track", "mp3");
        assertUnsupported(loader, null, null);
        System.out.println("common=public-concrete,abstract-api-super,direct-url-interface,"
            + "3-private-constants,1-constructor,1-exported-method;construction,http-config,"
            + "private-helper-signatures,reflection;service=deterministic-no-network,"
            + "current-bounded-native-source");
      } else {
        check(args[0].equals("reference"), "unknown mode");
        System.out.println("common=public-concrete,abstract-api-super,direct-url-interface,"
            + "3-private-constants,1-constructor,1-exported-method;construction,http-config,"
            + "private-helper-signatures,reflection;service=legacy-api-json-and-storage-xml,"
            + "md5-signed-direct-url");
      }
    } finally {
      loader.shutdown();
      loader.shutdown();
    }
  }

  private static void assertUnsupported(DefaultYandexMusicDirectUrlLoader loader,
      String trackId, String codec) {
    try {
      loader.getDirectUrl(trackId, codec);
      throw new AssertionError("legacy direct URL discovery unexpectedly succeeded");
    } catch (UnsupportedOperationException error) {
      check(error.getMessage().contains("Legacy Yandex direct-URL discovery is unsupported"),
          "stable unsupported disposition");
    }
  }

  private static void checkConstant(Class<?> type, String name, String value) throws Exception {
    Field field = type.getDeclaredField(name);
    field.setAccessible(true);
    check(field.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL)
        && field.getType() == String.class && field.get(null).equals(value), name + " metadata");
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const DEFAULT_YANDEX_MUSIC_PLAYLIST_LOADER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.yamusic.DefaultYandexMusicPlaylistLoader;
import com.sedmelluq.discord.lavaplayer.source.yamusic.DefaultYandexMusicTrackLoader;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicPlaylistLoader;
import com.sedmelluq.discord.lavaplayer.tools.FriendlyException;
import com.sedmelluq.discord.lavaplayer.tools.JsonBrowser;
import com.sedmelluq.discord.lavaplayer.track.AudioItem;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import java.util.Arrays;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.Function;

public final class GateDefaultYandexMusicPlaylistLoader {
  public static void main(String[] args) throws Exception {
    check(args.length == 1, "mode required");
    Class<?> type = DefaultYandexMusicPlaylistLoader.class;
    check(type.getModifiers() == Modifier.PUBLIC && !type.isInterface() && !type.isEnum()
        && !type.isAnnotation() && !type.isSynthetic(), "class metadata");
    check(type.getSuperclass() == DefaultYandexMusicTrackLoader.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {YandexMusicPlaylistLoader.class}),
        "class hierarchy");
    check(type.getDeclaredFields().length == 4 && type.getDeclaredConstructors().length == 1
        && type.getDeclaredMethods().length == 9, "member counts");
    checkConstant(type, "PLAYLIST_INFO_FORMAT",
        "https://api.music.yandex.net/users/%s/playlists/%s");
    checkConstant(type, "ALBUM_INFO_FORMAT",
        "https://api.music.yandex.net/albums/%s/with-tracks");
    checkConstant(type, "ARTIST_INFO_FORMAT",
        "https://api.music.yandex.net/artists/%s/brief-info");
    Field executorField = type.getDeclaredField("tracksLoader");
    check(executorField.getModifiers() == (Modifier.PRIVATE | Modifier.FINAL)
        && executorField.getType() == ExecutorService.class, "executor field metadata");
    executorField.setAccessible(true);

    Constructor<?> constructor = type.getDeclaredConstructor();
    check(constructor.getModifiers() == Modifier.PUBLIC && !constructor.isSynthetic()
        && constructor.getExceptionTypes().length == 0, "constructor metadata");
    Method userPlaylist = type.getDeclaredMethod("loadPlaylist", String.class, String.class,
        String.class, Function.class);
    checkLoadMethod(userPlaylist, 4);
    Method collectionPlaylist = type.getDeclaredMethod("loadPlaylist", String.class,
        String.class, Function.class);
    checkLoadMethod(collectionPlaylist, 3);
    Method shutdown = type.getDeclaredMethod("shutdown");
    check(shutdown.getModifiers() == Modifier.PUBLIC && shutdown.getReturnType() == void.class
        && shutdown.getExceptionTypes().length == 0 && !shutdown.isSynthetic(),
        "shutdown metadata");
    Method hasError = type.getDeclaredMethod("hasError", JsonBrowser.class);
    check(hasError.getModifiers() == Modifier.STATIC && hasError.getReturnType() == boolean.class
        && hasError.getExceptionTypes().length == 0 && !hasError.isSynthetic(),
        "error parser metadata");
    hasError.setAccessible(true);
    check(!(Boolean) hasError.invoke(null, JsonBrowser.parse("{}"))
        && (Boolean) hasError.invoke(null, JsonBrowser.parse("{\"error\":\"not-found\"}")),
        "error parser values");
    try {
      hasError.invoke(null, JsonBrowser.parse("{\"error\":\"denied\"}"));
      throw new AssertionError("unexpected Yandex error code was accepted");
    } catch (InvocationTargetException error) {
      check(error.getCause() instanceof FriendlyException
          && error.getCause().getMessage().equals("Yandex Music returned an error code: denied")
          && ((FriendlyException) error.getCause()).severity
              == FriendlyException.Severity.SUSPICIOUS,
          "error parser failure");
    }
    checkPrivateMethods(type);

    DefaultYandexMusicPlaylistLoader loader =
        (DefaultYandexMusicPlaylistLoader) constructor.newInstance();
    ExecutorService executor = (ExecutorService) executorField.get(loader);
    check(loader instanceof YandexMusicPlaylistLoader && loader.getHttpConfiguration() != null
        && executor != null && !executor.isShutdown(), "construction and executor lifecycle");
    AtomicInteger factoryCalls = new AtomicInteger();
    Function<AudioTrackInfo, AudioTrack> factory = info -> {
      factoryCalls.incrementAndGet();
      return null;
    };
    try {
      if (args[0].equals("candidate")) {
        assertUnsupported(() -> loader.loadPlaylist("http://127.0.0.1:1", "id", "tracks",
            factory));
        assertUnsupported(() -> loader.loadPlaylist(null, null, factory));
        check(factoryCalls.get() == 0, "factory was not invoked");
        System.out.println("common=public-concrete,track-loader-super,playlist-interface,"
            + "4-fields,1-constructor,4-exported-methods;constants,construction,http-config,"
            + "pure-error-parser,executor-shutdown,private-signatures,reflection;"
            + "service=deterministic-no-network,current-bounded-native-source");
      } else {
        check(args[0].equals("reference"), "unknown mode");
        System.out.println("common=public-concrete,track-loader-super,playlist-interface,"
            + "4-fields,1-constructor,4-exported-methods;constants,construction,http-config,"
            + "pure-error-parser,executor-shutdown,private-signatures,reflection;"
            + "service=legacy-api-json,unbounded-cached-track-fanout");
      }
    } finally {
      loader.shutdown();
      loader.shutdown();
      check(executor.isShutdown(), "executor shutdown");
    }
  }

  private static void checkLoadMethod(Method method, int parameterCount) {
    check(method.getModifiers() == Modifier.PUBLIC && method.getReturnType() == AudioItem.class
        && method.getParameterCount() == parameterCount && method.getExceptionTypes().length == 0
        && !method.isBridge() && !method.isSynthetic() && !method.isVarArgs(),
        "load method metadata");
    Type genericFactory = method.getGenericParameterTypes()[parameterCount - 1];
    check(genericFactory instanceof ParameterizedType
        && ((ParameterizedType) genericFactory).getRawType() == Function.class
        && Arrays.equals(((ParameterizedType) genericFactory).getActualTypeArguments(),
            new Type[] {AudioTrackInfo.class, AudioTrack.class}), "factory generic metadata");
  }

  private static void checkPrivateMethods(Class<?> type) throws Exception {
    Method url = type.getDeclaredMethod("loadPlaylistUrl", String.class, String.class,
        Function.class);
    Method track = type.getDeclaredMethod("loadTrack", JsonBrowser.class, Function.class);
    check(url.getModifiers() == Modifier.PRIVATE && url.getReturnType() == AudioItem.class
        && track.getModifiers() == Modifier.PRIVATE && track.getReturnType() == AudioTrack.class,
        "private helper metadata");
    Class<?> http = Class.forName("com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface");
    Method lambdaTwo = type.getDeclaredMethod("lambda$loadPlaylistUrl$2", String.class,
        Function.class, http, JsonBrowser.class);
    Method lambdaOne = type.getDeclaredMethod("lambda$loadPlaylistUrl$1", JsonBrowser.class,
        Function.class);
    Method lambdaZero = type.getDeclaredMethod("lambda$loadPlaylistUrl$0", JsonBrowser.class,
        Function.class);
    check(lambdaTwo.getModifiers() == (Modifier.PRIVATE | 0x1000)
        && lambdaTwo.getReturnType() == AudioItem.class
        && Arrays.equals(lambdaTwo.getExceptionTypes(), new Class<?>[] {Exception.class})
        && lambdaOne.getModifiers() == (Modifier.PRIVATE | 0x1000)
        && lambdaOne.getReturnType() == AudioTrack.class
        && Arrays.equals(lambdaOne.getExceptionTypes(), new Class<?>[] {Exception.class})
        && lambdaZero.getModifiers() == (Modifier.PRIVATE | 0x1000)
        && lambdaZero.getReturnType() == AudioTrack.class
        && Arrays.equals(lambdaZero.getExceptionTypes(), new Class<?>[] {Exception.class}),
        "synthetic lambda metadata");
  }

  private static void assertUnsupported(Invocation invocation) throws Exception {
    try {
      invocation.run();
      throw new AssertionError("legacy playlist discovery unexpectedly succeeded");
    } catch (UnsupportedOperationException error) {
      check(error.getMessage().contains("Legacy Yandex playlist discovery is unsupported"),
          "stable unsupported disposition");
    }
  }

  private static void checkConstant(Class<?> type, String name, String value) throws Exception {
    Field field = type.getDeclaredField(name);
    field.setAccessible(true);
    check(field.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL)
        && field.getType() == String.class && field.get(null).equals(value), name + " metadata");
  }

  private interface Invocation {
    void run() throws Exception;
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const DEFAULT_YANDEX_MUSIC_TRACK_LOADER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.yamusic.AbstractYandexMusicApiLoader;
import com.sedmelluq.discord.lavaplayer.source.yamusic.DefaultYandexMusicTrackLoader;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicTrackLoader;
import com.sedmelluq.discord.lavaplayer.track.AudioItem;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import java.util.Arrays;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.Function;

public final class GateDefaultYandexMusicTrackLoader {
  public static void main(String[] args) throws Exception {
    check(args.length == 1, "mode required");
    Class<?> type = DefaultYandexMusicTrackLoader.class;
    check(type.getModifiers() == Modifier.PUBLIC && !type.isInterface() && !type.isEnum()
        && !type.isAnnotation() && !type.isSynthetic(), "class metadata");
    check(type.getSuperclass() == AbstractYandexMusicApiLoader.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {YandexMusicTrackLoader.class}),
        "class hierarchy");
    check(type.getDeclaredFields().length == 1 && type.getDeclaredConstructors().length == 1
        && type.getDeclaredMethods().length == 2, "member counts");
    Field format = type.getDeclaredField("TRACKS_INFO_FORMAT");
    format.setAccessible(true);
    check(format.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL)
        && format.getType() == String.class
        && format.get(null).equals("https://api.music.yandex.net/tracks?trackIds="),
        "constant metadata");
    Constructor<?> constructor = type.getDeclaredConstructor();
    check(constructor.getModifiers() == Modifier.PUBLIC && !constructor.isSynthetic()
        && constructor.getExceptionTypes().length == 0, "constructor metadata");
    Method loadTrack = type.getDeclaredMethod("loadTrack", String.class, String.class,
        Function.class);
    check(loadTrack.getModifiers() == Modifier.PUBLIC && loadTrack.getReturnType() == AudioItem.class
        && loadTrack.getExceptionTypes().length == 0 && !loadTrack.isBridge()
        && !loadTrack.isSynthetic() && !loadTrack.isVarArgs(), "load metadata");
    Type genericFactory = loadTrack.getGenericParameterTypes()[2];
    check(genericFactory instanceof ParameterizedType
        && ((ParameterizedType) genericFactory).getRawType() == Function.class
        && Arrays.equals(((ParameterizedType) genericFactory).getActualTypeArguments(),
            new Type[] {AudioTrackInfo.class, AudioTrack.class}), "factory generic metadata");
    Class<?> http = Class.forName("com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface");
    Class<?> json = Class.forName("com.sedmelluq.discord.lavaplayer.tools.JsonBrowser");
    Method lambda = type.getDeclaredMethod("lambda$loadTrack$0", Function.class, http, json);
    check(lambda.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC | 0x1000)
        && lambda.getReturnType() == AudioItem.class
        && Arrays.equals(lambda.getExceptionTypes(), new Class<?>[] {Exception.class}),
        "synthetic lambda metadata");

    DefaultYandexMusicTrackLoader loader =
        (DefaultYandexMusicTrackLoader) constructor.newInstance();
    check(loader instanceof YandexMusicTrackLoader && loader.getHttpConfiguration() != null,
        "construction and inherited configuration");
    AtomicInteger factoryCalls = new AtomicInteger();
    Function<AudioTrackInfo, AudioTrack> factory = info -> {
      factoryCalls.incrementAndGet();
      return null;
    };
    try {
      if (args[0].equals("candidate")) {
        assertUnsupported(loader, "http://127.0.0.1:1", "track", factory);
        assertUnsupported(loader, null, null, factory);
        check(factoryCalls.get() == 0, "factory was not invoked");
        System.out.println("common=public-concrete,abstract-api-super,track-loader-interface,"
            + "1-private-constant,1-constructor,1-exported-method;construction,http-config,"
            + "generic-factory,synthetic-lambda,reflection;service=deterministic-no-network,"
            + "current-bounded-native-source");
      } else {
        check(args[0].equals("reference"), "unknown mode");
        System.out.println("common=public-concrete,abstract-api-super,track-loader-interface,"
            + "1-private-constant,1-constructor,1-exported-method;construction,http-config,"
            + "generic-factory,synthetic-lambda,reflection;service=legacy-query-api-json");
      }
    } finally {
      loader.shutdown();
      loader.shutdown();
    }
  }

  private static void assertUnsupported(DefaultYandexMusicTrackLoader loader, String album,
      String track, Function<AudioTrackInfo, AudioTrack> factory) {
    try {
      loader.loadTrack(album, track, factory);
      throw new AssertionError("legacy track discovery unexpectedly succeeded");
    } catch (UnsupportedOperationException error) {
      check(error.getMessage().contains("Legacy Yandex track discovery is unsupported"),
          "stable unsupported disposition");
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const DEFAULT_YANDEX_SEARCH_PROVIDER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.yamusic.AbstractYandexMusicApiLoader;
import com.sedmelluq.discord.lavaplayer.source.yamusic.DefaultYandexSearchProvider;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicPlaylistLoader;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicSearchResultLoader;
import com.sedmelluq.discord.lavaplayer.tools.JsonBrowser;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.track.AudioItem;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Proxy;
import java.lang.reflect.Type;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.Function;
import java.util.regex.Pattern;

public final class GateDefaultYandexSearchProvider {
  public static void main(String[] args) throws Exception {
    check(args.length == 1, "mode required");
    Class<?> type = DefaultYandexSearchProvider.class;
    check(type.getModifiers() == Modifier.PUBLIC && !type.isInterface() && !type.isEnum()
        && !type.isAnnotation() && !type.isSynthetic(), "class metadata");
    check(type.getSuperclass() == AbstractYandexMusicApiLoader.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {YandexMusicSearchResultLoader.class}),
        "class hierarchy");
    check(type.getDeclaredFields().length == 4 && type.getDeclaredConstructors().length == 1
        && type.getDeclaredMethods().length == 8, "member counts");

    checkConstant(type, "DEFAULT_LIMIT", int.class, Integer.valueOf(10));
    checkConstant(type, "TRACKS_INFO_FORMAT", String.class,
        "https://api.music.yandex.net/search?type=%s&page=0&text=%s");
    checkConstant(type, "SEARCH_PREFIX", String.class, "ymsearch");
    Field patternField = type.getDeclaredField("SEARCH_PATTERN");
    patternField.setAccessible(true);
    check(patternField.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL)
        && patternField.getType() == Pattern.class, "pattern field metadata");
    Pattern pattern = (Pattern) patternField.get(null);
    check(pattern.pattern().equals("ymsearch(:([a-zA-Z]+))?(:([0-9]+))?:([^:]+)")
        && pattern.flags() == 0, "compiled pattern");

    Constructor<?> constructor = type.getDeclaredConstructor();
    check(constructor.getModifiers() == Modifier.PUBLIC && !constructor.isSynthetic()
        && constructor.getExceptionTypes().length == 0, "constructor metadata");
    Method load = type.getDeclaredMethod("loadSearchResult", String.class,
        YandexMusicPlaylistLoader.class, Function.class);
    check(load.getModifiers() == Modifier.PUBLIC && load.getReturnType() == AudioItem.class
        && load.getExceptionTypes().length == 0 && !load.isBridge() && !load.isSynthetic()
        && !load.isVarArgs(), "load metadata");
    Type genericFactory = load.getGenericParameterTypes()[2];
    check(genericFactory instanceof ParameterizedType
        && ((ParameterizedType) genericFactory).getRawType() == Function.class
        && Arrays.equals(((ParameterizedType) genericFactory).getActualTypeArguments(),
            new Type[] {AudioTrackInfo.class, AudioTrack.class}), "factory generic metadata");

    checkPrivate(type.getDeclaredMethod("loadTracks", List.class, int.class, Function.class),
        AudioItem.class);
    checkPrivate(type.getDeclaredMethod("loadPlaylist", List.class,
        YandexMusicPlaylistLoader.class, Function.class), AudioItem.class);
    checkPrivate(type.getDeclaredMethod("loadAlbum", List.class,
        YandexMusicPlaylistLoader.class, Function.class), AudioItem.class);
    checkPrivate(type.getDeclaredMethod("getResults", JsonBrowser.class, String.class), List.class);
    checkPrivate(type.getDeclaredMethod("getValidType", String.class), String.class);
    checkPrivate(type.getDeclaredMethod("getValidLimit", String.class), Integer.class);
    Method lambda = type.getDeclaredMethod("lambda$loadSearchResult$0", String.class, int.class,
        Function.class, YandexMusicPlaylistLoader.class, HttpInterface.class, JsonBrowser.class);
    check(lambda.getModifiers() == (Modifier.PRIVATE | 0x1000)
        && lambda.getReturnType() == AudioItem.class
        && Arrays.equals(lambda.getExceptionTypes(), new Class<?>[] {Exception.class}),
        "synthetic lambda metadata");

    DefaultYandexSearchProvider provider =
        (DefaultYandexSearchProvider) constructor.newInstance();
    check(provider instanceof YandexMusicSearchResultLoader
        && provider.getHttpConfiguration() != null, "construction and inherited configuration");
    AtomicInteger collaboratorCalls = new AtomicInteger();
    YandexMusicPlaylistLoader playlistLoader = (YandexMusicPlaylistLoader) Proxy.newProxyInstance(
        GateDefaultYandexSearchProvider.class.getClassLoader(),
        new Class<?>[] {YandexMusicPlaylistLoader.class}, (proxy, method, arguments) -> {
          collaboratorCalls.incrementAndGet();
          throw new AssertionError("playlist loader was invoked");
        });
    Function<AudioTrackInfo, AudioTrack> trackFactory = info -> {
      collaboratorCalls.incrementAndGet();
      throw new AssertionError("track factory was invoked");
    };
    try {
      for (String query : new String[] {null, "", "search:test", "ymsearch", "ymsearch:",
          "ymsearch::test", "prefix-ymsearch:test"}) {
        check(provider.loadSearchResult(query, playlistLoader, trackFactory) == null,
            "unrecognized query must fall through: " + query);
      }
      check(collaboratorCalls.get() == 0, "invalid queries touched collaborators");
      if (args[0].equals("candidate")) {
        assertUnsupported(provider, "ymsearch:animals & architects", playlistLoader, trackFactory);
        assertUnsupported(provider, "ymsearch:album:1:animals", playlistLoader, trackFactory);
        assertUnsupported(provider, "ymsearch:playlist:99:animals", playlistLoader, trackFactory);
        check(collaboratorCalls.get() == 0, "recognized queries touched collaborators");
        System.out.println("common=public-concrete,abstract-api-super,search-loader-interface,"
            + "4-private-constants,1-constructor,1-exported-method;construction,http-config,"
            + "compiled-pattern,generic-factory,private-helper-and-lambda-signatures,"
            + "invalid-query-fallthrough,reflection;service=deterministic-no-network,"
            + "current-bounded-native-search");
      } else {
        check(args[0].equals("reference"), "unknown mode");
        System.out.println("common=public-concrete,abstract-api-super,search-loader-interface,"
            + "4-private-constants,1-constructor,1-exported-method;construction,http-config,"
            + "compiled-pattern,generic-factory,private-helper-and-lambda-signatures,"
            + "invalid-query-fallthrough,reflection;service=legacy-query-api-json");
      }
    } finally {
      provider.shutdown();
      provider.shutdown();
    }
  }

  private static void assertUnsupported(DefaultYandexSearchProvider provider, String query,
      YandexMusicPlaylistLoader playlistLoader,
      Function<AudioTrackInfo, AudioTrack> trackFactory) {
    try {
      provider.loadSearchResult(query, playlistLoader, trackFactory);
      throw new AssertionError("legacy search discovery unexpectedly succeeded");
    } catch (UnsupportedOperationException error) {
      check(error.getMessage().contains("Legacy Yandex search discovery is unsupported"),
          "stable unsupported disposition");
    }
  }

  private static void checkConstant(Class<?> type, String name, Class<?> fieldType, Object value)
      throws Exception {
    Field field = type.getDeclaredField(name);
    field.setAccessible(true);
    check(field.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL)
        && field.getType() == fieldType && field.get(null).equals(value), name + " metadata");
  }

  private static void checkPrivate(Method method, Class<?> returnType) {
    check(method.getModifiers() == Modifier.PRIVATE && method.getReturnType() == returnType
        && method.getExceptionTypes().length == 0 && !method.isSynthetic(),
        method.getName() + " metadata");
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const YANDEX_HTTP_CONTEXT_FILTER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexHttpContextFilter;
import com.sedmelluq.discord.lavaplayer.tools.http.HttpContextFilter;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;
import org.apache.http.HttpResponse;
import org.apache.http.client.CookieStore;
import org.apache.http.client.methods.HttpGet;
import org.apache.http.client.methods.HttpUriRequest;
import org.apache.http.client.protocol.HttpClientContext;
import org.apache.http.impl.client.BasicCookieStore;
import org.apache.http.impl.cookie.BasicClientCookie;

public final class GateYandexHttpContextFilter {
  public static void main(String[] args) throws Exception {
    check(args.length == 1, "mode required");
    Class<?> type = YandexHttpContextFilter.class;
    check(type.getModifiers() == Modifier.PUBLIC && !type.isInterface() && !type.isEnum()
        && !type.isAnnotation() && !type.isSynthetic(), "class metadata");
    check(type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {HttpContextFilter.class}),
        "class hierarchy");
    check(type.getDeclaredFields().length == 1 && type.getDeclaredConstructors().length == 1
        && type.getDeclaredMethods().length == 6, "member counts");
    Field token = type.getDeclaredField("oAuthToken");
    token.setAccessible(true);
    check(token.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC)
        && token.getType() == String.class && token.get(null) == null, "token field metadata");

    Constructor<?> constructor = type.getDeclaredConstructor();
    check(constructor.getModifiers() == Modifier.PUBLIC && !constructor.isSynthetic()
        && constructor.getExceptionTypes().length == 0, "constructor metadata");
    Method setter = checkMethod(type, "setOAuthToken", void.class,
        Modifier.PUBLIC | Modifier.STATIC, String.class);
    Method open = checkMethod(type, "onContextOpen", void.class, Modifier.PUBLIC,
        HttpClientContext.class);
    Method close = checkMethod(type, "onContextClose", void.class, Modifier.PUBLIC,
        HttpClientContext.class);
    Method requestMethod = checkMethod(type, "onRequest", void.class, Modifier.PUBLIC,
        HttpClientContext.class, HttpUriRequest.class, boolean.class);
    Method responseMethod = checkMethod(type, "onRequestResponse", boolean.class, Modifier.PUBLIC,
        HttpClientContext.class, HttpUriRequest.class, HttpResponse.class);
    Method exceptionMethod = checkMethod(type, "onRequestException", boolean.class, Modifier.PUBLIC,
        HttpClientContext.class, HttpUriRequest.class, Throwable.class);
    check(setter != null && open != null && close != null && requestMethod != null
        && responseMethod != null && exceptionMethod != null, "method lookup");

    YandexHttpContextFilter filter = (YandexHttpContextFilter) constructor.newInstance();
    check(filter instanceof HttpContextFilter, "construction");
    HttpClientContext context = HttpClientContext.create();
    check(context.getCookieStore() == null, "fresh context cookie state");
    filter.onContextOpen(context);
    check(context.getCookieStore() != null && context.getCookieStore().getCookies().isEmpty(),
        "cookie store creation");
    CookieStore existing = new BasicCookieStore();
    BasicClientCookie cookie = new BasicClientCookie("session", "secret-cookie");
    cookie.setDomain("music.yandex.net");
    cookie.setPath("/");
    existing.addCookie(cookie);
    context.setCookieStore(existing);
    filter.onContextOpen(context);
    check(context.getCookieStore() == existing && existing.getCookies().isEmpty(),
        "existing cookie store reset");
    filter.onContextClose(null);
    check(!filter.onRequestResponse(null, null, null), "response retry policy");
    check(!filter.onRequestException(null, null, new RuntimeException("fixture")),
        "exception retry policy");

    YandexHttpContextFilter.setOAuthToken(null);
    HttpGet anonymous = new HttpGet("https://music.yandex.net/album/1/track/2");
    filter.onRequest(context, anonymous, false);
    checkHeader(anonymous, "User-Agent", "Yandex-Music-API");
    checkHeader(anonymous, "X-Yandex-Music-Client", "WindowsPhone/3.20");
    check(anonymous.getFirstHeader("Authorization") == null, "anonymous authorization");
    filter.onRequest(context, anonymous, true);
    check(anonymous.getHeaders("User-Agent").length == 1
        && anonymous.getHeaders("X-Yandex-Music-Client").length == 1,
        "repetition replaces headers");

    YandexHttpContextFilter.setOAuthToken("secret-marker");
    check(token.get(null).equals("secret-marker"), "setter stores token exactly");
    HttpGet authenticated = new HttpGet("https://music.yandex.net/album/1/track/2");
    if (args[0].equals("candidate")) {
      try {
        filter.onRequest(context, authenticated, false);
        throw new AssertionError("legacy global OAuth forwarding unexpectedly succeeded");
      } catch (UnsupportedOperationException error) {
        check(error.getMessage().contains("Legacy global Yandex OAuth forwarding is unsupported"),
            "stable unsupported disposition");
      }
      checkHeader(authenticated, "User-Agent", "Yandex-Music-API");
      checkHeader(authenticated, "X-Yandex-Music-Client", "WindowsPhone/3.20");
      check(authenticated.getFirstHeader("Authorization") == null,
          "candidate leaked global authorization");
      System.out.println("common=public-concrete,object-super,http-context-filter-interface,"
          + "1-private-static-field,1-constructor,6-exported-methods;construction,static-setter,"
          + "cookie-reset,context-close,non-secret-request-headers,repetition,false-retry-policy,"
          + "reflection;service=global-oauth-rejected,current-origin-bounded-manager-auth");
    } else {
      check(args[0].equals("reference"), "unknown mode");
      filter.onRequest(context, authenticated, false);
      checkHeader(authenticated, "Authorization", "OAuth secret-marker");
      System.out.println("common=public-concrete,object-super,http-context-filter-interface,"
          + "1-private-static-field,1-constructor,6-exported-methods;construction,static-setter,"
          + "cookie-reset,context-close,non-secret-request-headers,repetition,false-retry-policy,"
          + "reflection;service=legacy-global-oauth-forwarding");
    }
    YandexHttpContextFilter.setOAuthToken(null);
    check(token.get(null) == null, "token reset");
  }

  private static Method checkMethod(Class<?> type, String name, Class<?> returnType,
      int modifiers, Class<?>... parameters) throws Exception {
    Method method = type.getDeclaredMethod(name, parameters);
    check(method.getModifiers() == modifiers && method.getReturnType() == returnType
        && method.getExceptionTypes().length == 0 && !method.isBridge() && !method.isSynthetic()
        && !method.isVarArgs(), name + " metadata");
    return method;
  }

  private static void checkHeader(HttpUriRequest request, String name, String value) {
    check(request.getFirstHeader(name) != null
        && request.getFirstHeader(name).getValue().equals(value), name + " header");
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const YANDEX_MUSIC_API_LOADER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicApiLoader;
import com.sedmelluq.discord.lavaplayer.tools.http.ExtendedHttpConfigurable;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;

public final class GateYandexMusicApiLoader {
  public static void main(String[] args) throws Exception {
    Class<YandexMusicApiLoader> type = YandexMusicApiLoader.class;
    check(type.getModifiers() == (Modifier.PUBLIC | Modifier.INTERFACE | Modifier.ABSTRACT)
        && type.isInterface() && !type.isAnnotation() && !type.isEnum() && !type.isSynthetic(),
        "interface metadata");
    check(type.getSuperclass() == null && type.getInterfaces().length == 0
        && type.getDeclaredFields().length == 0 && type.getDeclaredConstructors().length == 0
        && type.getDeclaredMethods().length == 2 && type.getTypeParameters().length == 0,
        "interface shape");
    Method configuration = type.getDeclaredMethod("getHttpConfiguration");
    checkMethod(configuration, ExtendedHttpConfigurable.class);
    Method shutdown = type.getDeclaredMethod("shutdown");
    checkMethod(shutdown, void.class);

    ExtendedHttpConfigurable first = proxy(ExtendedHttpConfigurable.class);
    RecordingLoader loader = new RecordingLoader(first);
    check(loader.getHttpConfiguration() == first && loader.configurationCalls == 1,
        "configuration identity");
    loader.configuration = null;
    check(loader.getHttpConfiguration() == null && loader.configurationCalls == 2,
        "null configuration identity");
    loader.shutdown();
    loader.shutdown();
    check(loader.shutdowns == 2, "repeatable shutdown dispatch");
    check(type.isAssignableFrom(RecordingLoader.class)
        && Arrays.equals(RecordingLoader.class.getInterfaces(),
            new Class<?>[] {YandexMusicApiLoader.class}), "caller implementation hierarchy");
    System.out.println("interface=public-abstract,object-root,0-superinterfaces,0-fields,"
        + "0-constructors,2-abstract-methods;implementation=configuration-identity,null-identity,"
        + "repeatable-shutdown;reflection=exact");
  }

  private static void checkMethod(Method method, Class<?> returnType) {
    check(method.getModifiers() == (Modifier.PUBLIC | Modifier.ABSTRACT)
        && method.getReturnType() == returnType && method.getParameterCount() == 0
        && method.getExceptionTypes().length == 0 && !method.isDefault() && !method.isBridge()
        && !method.isSynthetic() && !method.isVarArgs(), method.getName() + " metadata");
  }

  private static <T> T proxy(Class<T> type) {
    Object value = Proxy.newProxyInstance(GateYandexMusicApiLoader.class.getClassLoader(),
        new Class<?>[] {type}, (proxy, method, arguments) -> {
          throw new AssertionError("configuration proxy invoked: " + method.getName());
        });
    return type.cast(value);
  }

  private static final class RecordingLoader implements YandexMusicApiLoader {
    private ExtendedHttpConfigurable configuration;
    private int configurationCalls;
    private int shutdowns;

    private RecordingLoader(ExtendedHttpConfigurable configuration) {
      this.configuration = configuration;
    }

    @Override
    public ExtendedHttpConfigurable getHttpConfiguration() {
      configurationCalls++;
      return configuration;
    }

    @Override
    public void shutdown() {
      shutdowns++;
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const YANDEX_MUSIC_AUDIO_SOURCE_MANAGER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.player.AudioPlayerManager;
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicAudioTrack;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicDirectUrlLoader;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicPlaylistLoader;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicSearchResultLoader;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicTrackLoader;
import com.sedmelluq.discord.lavaplayer.tools.http.ExtendedHttpConfigurable;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpConfigurable;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterfaceManager;
import com.sedmelluq.discord.lavaplayer.track.AudioItem;
import com.sedmelluq.discord.lavaplayer.track.AudioReference;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.io.DataInput;
import java.io.DataOutput;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.function.Consumer;
import java.util.function.Function;
import java.util.regex.Pattern;
import org.apache.http.client.config.RequestConfig;
import org.apache.http.impl.client.HttpClientBuilder;

public final class GateYandexMusicAudioSourceManager {
  public static void main(String[] args) throws Exception {
    check(args.length >= 1 && args.length <= 2, "expected disposition and optional native path");
    boolean reference = args[0].equals("reference");
    check(reference || args[0].equals("candidate"), "unknown disposition");
    check(reference == (args.length == 1), "candidate requires native path");
    reflectionContract();
    commonBehavior();
    if (reference) legacyDisposition();
    else currentDisposition(args[1]);
    System.out.println(
        "common=public-concrete,audio-source-http-configurable,19-fields,3-constructors,"
        + "17-methods;construction,source-name,empty-details,track-factory,loader-identity,"
        + "http-config,shutdown,reflection;service="
        + (reference ? "legacy-loader-routing" :
            "current-native-bounded-authenticated-routing,no-global-oauth"));
  }

  private static void reflectionContract() throws Exception {
    Class<YandexMusicAudioSourceManager> type = YandexMusicAudioSourceManager.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(),
            new Class<?>[] {AudioSourceManager.class, HttpConfigurable.class}), "class metadata");
    check(type.getDeclaredFields().length == 19 && type.getDeclaredConstructors().length == 3
        && type.getDeclaredMethods().length == 17, "member counts");
    for (String name : new String[] {"PROTOCOL_REGEX", "DOMAIN_REGEX", "TRACK_ID_REGEX",
        "ALBUM_ID_REGEX", "ARTIST_ID_REGEX", "PLAYLIST_ID_REGEX", "USER_REGEX"}) {
      checkField(type, name, String.class, Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    }
    for (String name : new String[] {"trackUrlPattern", "shortTrackUrlPattern",
        "albumUrlPattern", "artistUrlPattern", "playlistUrlPattern"}) {
      checkField(type, name, Pattern.class, Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    }
    checkField(type, "allowSearch", boolean.class, Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "httpInterfaceManager", HttpInterfaceManager.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "combinedHttpConfiguration", ExtendedHttpConfigurable.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "trackLoader", YandexMusicTrackLoader.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "playlistLoader", YandexMusicPlaylistLoader.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "directUrlLoader", YandexMusicDirectUrlLoader.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField(type, "searchResultLoader", YandexMusicSearchResultLoader.class,
        Modifier.PRIVATE | Modifier.FINAL);

    checkConstructor(type.getDeclaredConstructor(), new Class<?>[0]);
    checkConstructor(type.getDeclaredConstructor(boolean.class), new Class<?>[] {boolean.class});
    checkConstructor(type.getDeclaredConstructor(boolean.class, YandexMusicTrackLoader.class,
        YandexMusicPlaylistLoader.class, YandexMusicDirectUrlLoader.class,
        YandexMusicSearchResultLoader.class), new Class<?>[] {boolean.class,
            YandexMusicTrackLoader.class, YandexMusicPlaylistLoader.class,
            YandexMusicDirectUrlLoader.class, YandexMusicSearchResultLoader.class});
    checkMethod(type, "loadItem", AudioItem.class,
        new Class<?>[] {AudioPlayerManager.class, AudioReference.class});
    checkMethod(type, "isTrackEncodable", boolean.class, new Class<?>[] {AudioTrack.class});
    checkMethod(type, "encodeTrack", void.class,
        new Class<?>[] {AudioTrack.class, DataOutput.class}, java.io.IOException.class);
    checkMethod(type, "decodeTrack", AudioTrack.class,
        new Class<?>[] {AudioTrackInfo.class, DataInput.class}, java.io.IOException.class);
    checkMethod(type, "getTrack", AudioTrack.class, new Class<?>[] {AudioTrackInfo.class});
    checkMethod(type, "shutdown", void.class, new Class<?>[0]);
    checkMethod(type, "getDirectUrlLoader", YandexMusicDirectUrlLoader.class, new Class<?>[0]);
    checkMethod(type, "getHttpInterface", HttpInterface.class, new Class<?>[0]);
    checkMethod(type, "configureRequests", void.class, new Class<?>[] {Function.class});
    checkMethod(type, "configureBuilder", void.class, new Class<?>[] {Consumer.class});
    checkMethod(type, "getHttpConfiguration", ExtendedHttpConfigurable.class, new Class<?>[0]);
    checkMethod(type, "getMainHttpConfiguration", ExtendedHttpConfigurable.class,
        new Class<?>[0]);
    checkMethod(type, "getTrackLHttpConfiguration", ExtendedHttpConfigurable.class,
        new Class<?>[0]);
    checkMethod(type, "getPlaylistLHttpConfiguration", ExtendedHttpConfigurable.class,
        new Class<?>[0]);
    checkMethod(type, "getDirectUrlLHttpConfiguration", ExtendedHttpConfigurable.class,
        new Class<?>[0]);
    checkMethod(type, "getSearchHttpConfiguration", ExtendedHttpConfigurable.class,
        new Class<?>[0]);
    checkMethod(type, "getSourceName", String.class, new Class<?>[0]);
    check(type.getDeclaredMethod("configureRequests", Function.class)
            .toGenericString().contains("Function<org.apache.http.client.config.RequestConfig,"
                + " org.apache.http.client.config.RequestConfig>")
        && type.getDeclaredMethod("configureBuilder", Consumer.class)
            .toGenericString().contains("Consumer<org.apache.http.impl.client.HttpClientBuilder>"),
        "generic configuration signatures");
  }

  private static void commonBehavior() throws Exception {
    check(field("PROTOCOL_REGEX").get(null).equals("https?://")
        && field("DOMAIN_REGEX").get(null).equals("music\\.yandex\\.[a-zA-Z]+")
        && pattern("trackUrlPattern").equals(
            "^https?://music\\.yandex\\.[a-zA-Z]+/album/([0-9]+)(?:\\?.*|)/track/"
                + "([0-9]+)(?:\\?.*|)$")
        && pattern("shortTrackUrlPattern").contains("/track/([0-9]+)")
        && pattern("albumUrlPattern").contains("/album/([0-9]+)")
        && pattern("artistUrlPattern").contains("/artist/([0-9]+)")
        && pattern("playlistUrlPattern").contains("/users/(.+)/playlists/([0-9]+)"),
        "static routing patterns");

    YandexMusicAudioSourceManager defaults = new YandexMusicAudioSourceManager();
    check((boolean) field("allowSearch").get(defaults), "default search flag");
    defaults.shutdown();
    YandexMusicAudioSourceManager disabled = new YandexMusicAudioSourceManager(false);
    check(!(boolean) field("allowSearch").get(disabled), "explicit search flag");
    disabled.shutdown();

    Fixture fixture = new Fixture(true);
    YandexMusicAudioSourceManager manager = fixture.manager;
    check(manager.getSourceName().equals("yandex-music") && manager.isTrackEncodable(null),
        "source identity and encodability");
    manager.encodeTrack(null, rejectingProxy(DataOutput.class));
    AudioTrackInfo info = new AudioTrackInfo("title", "author", 1234L, "71663565", false,
        "https://music.yandex.ru/track/71663565", "art", null);
    AudioTrack decoded = manager.decodeTrack(info, rejectingProxy(DataInput.class));
    AudioTrack created = manager.getTrack(info);
    check(decoded instanceof YandexMusicAudioTrack && created instanceof YandexMusicAudioTrack
        && decoded != created && decoded.getInfo() == info && created.getInfo() == info
        && decoded.getSourceManager() == manager && created.getSourceManager() == manager,
        "empty-detail track reconstruction");
    check(manager.getDirectUrlLoader() == fixture.direct.proxy
        && manager.getTrackLHttpConfiguration() == fixture.track.configuration
        && manager.getPlaylistLHttpConfiguration() == fixture.playlist.configuration
        && manager.getDirectUrlLHttpConfiguration() == fixture.direct.configuration
        && manager.getSearchHttpConfiguration() == fixture.search.configuration,
        "loader and configuration identity");
    check(fixture.track.configurationCalls == 2 && fixture.playlist.configurationCalls == 2
        && fixture.direct.configurationCalls == 2 && fixture.search.configurationCalls == 2,
        "configuration getter dispatch");
    ExtendedHttpConfigurable combined = manager.getHttpConfiguration();
    check(combined != null && combined == manager.getHttpConfiguration()
        && manager.getMainHttpConfiguration() instanceof HttpInterfaceManager
        && manager.getMainHttpConfiguration() != combined, "combined configuration identity");
    Function<RequestConfig, RequestConfig> requests = value -> value;
    Consumer<HttpClientBuilder> builder = value -> {};
    manager.configureRequests(requests);
    manager.configureBuilder(builder);
    check(fixture.configurationOrder.equals(Arrays.asList(
            "track:requests", "playlist:requests", "direct:requests", "search:requests",
            "track:builder", "playlist:builder", "direct:builder", "search:builder"))
        && fixture.track.config.requests == requests && fixture.track.config.builder == builder
        && fixture.playlist.config.requests == requests
        && fixture.playlist.config.builder == builder
        && fixture.direct.config.requests == requests && fixture.direct.config.builder == builder
        && fixture.search.config.requests == requests && fixture.search.config.builder == builder,
        "configuration delegation identity and order");
    try (HttpInterface http = manager.getHttpInterface()) {
      check(http != null, "HTTP interface creation");
    }
    manager.shutdown();
    manager.shutdown();
    check(fixture.shutdownOrder.equals(Arrays.asList(
        "track", "playlist", "search", "direct", "track", "playlist", "search", "direct")),
        "repeatable shutdown order");
  }

  private static void legacyDisposition() throws Exception {
    Fixture fixture = new Fixture(true);
    YandexMusicAudioSourceManager manager = fixture.manager;
    check(manager.loadItem(null, new AudioReference(
        "https://music.yandex.ru/album/11/track/22?from=gate", null)) == fixture.sentinel,
        "album track result");
    fixture.track.checkCall("11", "22");
    check(manager.loadItem(null,
        new AudioReference("http://music.yandex.com/track/33", null)) == fixture.sentinel,
        "short track result");
    fixture.track.checkCall("", "33");
    check(manager.loadItem(null, new AudioReference(
        "https://music.yandex.kz/users/alice/playlists/44", null)) == fixture.sentinel,
        "user playlist result");
    fixture.playlist.checkCall("alice", "44", "tracks");
    manager.loadItem(null, new AudioReference("https://music.yandex.by/album/55", null));
    fixture.playlist.checkCall("55", "volumes");
    manager.loadItem(null,
        new AudioReference("https://music.yandex.ru/artist/66/tracks", null));
    fixture.playlist.checkCall("66", "popularTracks");
    check(manager.loadItem(null, new AudioReference("legacy search", null)) == fixture.sentinel,
        "search result");
    fixture.search.checkCall("legacy search", fixture.playlist.proxy);
    manager.shutdown();

    Fixture disabled = new Fixture(false);
    check(disabled.manager.loadItem(null, new AudioReference("legacy search", null)) == null
        && disabled.search.loadCalls == 0, "disabled search fallthrough");
    disabled.manager.shutdown();
  }

  private static void currentDisposition(String nativeLibrary) throws Exception {
    Class.forName("dev.mantle.internal.NativeLoader")
        .getMethod("load", String.class).invoke(null, nativeLibrary);
    Class<?> nativeType = Class.forName("dev.mantle.internal.MantleNative");
    Method load = nativeType.getDeclaredMethod("loadYandexMusicItem",
        YandexMusicAudioSourceManager.class, AudioReference.class);
    check(Modifier.isPublic(load.getModifiers()) && Modifier.isStatic(load.getModifiers())
        && Modifier.isNative(load.getModifiers()), "current native route");
    System.clearProperty("dev.mantle.yandex.accessToken");
    YandexMusicAudioSourceManager manager = new YandexMusicAudioSourceManager();
    check(manager.loadItem(null,
        new AudioReference("https://example.invalid/track/71663565", null)) == null,
        "foreign route rejected without credentials");
    System.setProperty("dev.mantle.yandex.accessToken", "");
    RuntimeException invalid = expect(RuntimeException.class, () -> manager.loadItem(null,
        new AudioReference("https://music.yandex.ru/track/71663565", null)));
    check(invalid.getMessage().contains("invalid Yandex Music JVM access token"),
        "invalid caller token fails before service traffic");
    manager.shutdown();
    YandexMusicAudioSourceManager disabled = new YandexMusicAudioSourceManager(false);
    check(disabled.loadItem(null, new AudioReference("ymsearch:architects", null)) == null,
        "search flag enforced before credentials");
    disabled.shutdown();
    System.clearProperty("dev.mantle.yandex.accessToken");
  }

  private static String pattern(String name) throws Exception {
    return ((Pattern) field(name).get(null)).pattern();
  }

  private static Field field(String name) throws Exception {
    Field field = YandexMusicAudioSourceManager.class.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }

  private static void checkField(Class<?> owner, String name, Class<?> type, int modifiers)
      throws Exception {
    Field field = owner.getDeclaredField(name);
    check(field.getType() == type && field.getGenericType() == type
        && field.getModifiers() == modifiers && !field.isSynthetic(), name + " metadata");
  }

  private static void checkConstructor(Constructor<?> constructor, Class<?>[] parameters) {
    check(constructor.getModifiers() == Modifier.PUBLIC
        && Arrays.equals(constructor.getParameterTypes(), parameters)
        && constructor.getExceptionTypes().length == 0 && !constructor.isSynthetic()
        && !constructor.isVarArgs(), constructor + " metadata");
  }

  private static void checkMethod(Class<?> owner, String name, Class<?> returnType,
                                  Class<?>[] parameters, Class<?>... exceptions)
      throws Exception {
    Method method = owner.getDeclaredMethod(name, parameters);
    check(method.getReturnType() == returnType && method.getModifiers() == Modifier.PUBLIC
        && Arrays.equals(method.getParameterTypes(), parameters)
        && Arrays.equals(method.getExceptionTypes(), exceptions)
        && method.getTypeParameters().length == 0 && !method.isBridge()
        && !method.isSynthetic() && !method.isVarArgs(), method + " metadata");
  }

  private static final class Fixture {
    final List<String> shutdownOrder = new ArrayList<>();
    final List<String> configurationOrder = new ArrayList<>();
    final AudioTrack sentinel = proxy(AudioTrack.class, (instance, method, arguments) ->
        defaultValue(method.getReturnType()));
    final LoaderHandler track = new LoaderHandler("track", YandexMusicTrackLoader.class, this);
    final LoaderHandler playlist =
        new LoaderHandler("playlist", YandexMusicPlaylistLoader.class, this);
    final LoaderHandler direct =
        new LoaderHandler("direct", YandexMusicDirectUrlLoader.class, this);
    final LoaderHandler search =
        new LoaderHandler("search", YandexMusicSearchResultLoader.class, this);
    final YandexMusicAudioSourceManager manager;

    Fixture(boolean allowSearch) {
      manager = new YandexMusicAudioSourceManager(allowSearch,
          (YandexMusicTrackLoader) track.proxy, (YandexMusicPlaylistLoader) playlist.proxy,
          (YandexMusicDirectUrlLoader) direct.proxy,
          (YandexMusicSearchResultLoader) search.proxy);
    }
  }

  private static final class LoaderHandler implements InvocationHandler {
    final String name;
    final Fixture fixture;
    final Object proxy;
    final ConfigHandler config;
    final ExtendedHttpConfigurable configuration;
    int configurationCalls;
    int loadCalls;
    Object[] lastArguments;

    LoaderHandler(String name, Class<?> type, Fixture fixture) {
      this.name = name;
      this.fixture = fixture;
      config = new ConfigHandler(name, fixture.configurationOrder);
      configuration = proxy(ExtendedHttpConfigurable.class, config);
      proxy = proxy(type, this);
    }

    public Object invoke(Object instance, Method method, Object[] arguments) {
      if (method.getName().equals("getHttpConfiguration")) {
        configurationCalls++;
        return configuration;
      }
      if (method.getName().equals("shutdown")) {
        fixture.shutdownOrder.add(name);
        return null;
      }
      if (method.getName().startsWith("load") || method.getName().equals("getDirectUrl")) {
        loadCalls++;
        lastArguments = arguments == null ? new Object[0] : arguments.clone();
        return method.getReturnType() == String.class ? "legacy-direct-url" : fixture.sentinel;
      }
      if (method.getName().equals("toString")) return "YandexLoaderFixture:" + name;
      return defaultValue(method.getReturnType());
    }

    void checkCall(Object... prefix) {
      check(loadCalls > 0 && lastArguments.length >= prefix.length, name + " call missing");
      for (int index = 0; index < prefix.length; index++) {
        check(lastArguments[index] == prefix[index] || lastArguments[index].equals(prefix[index]),
            name + " argument " + index);
      }
    }
  }

  private static final class ConfigHandler implements InvocationHandler {
    final String name;
    final List<String> order;
    Object requests;
    Object builder;
    ConfigHandler(String name, List<String> order) { this.name = name; this.order = order; }
    public Object invoke(Object instance, Method method, Object[] arguments) {
      if (method.getName().equals("configureRequests")) {
        requests = arguments[0];
        order.add(name + ":requests");
      }
      if (method.getName().equals("configureBuilder")) {
        builder = arguments[0];
        order.add(name + ":builder");
      }
      if (method.getName().equals("toString")) return "YandexConfigFixture:" + name;
      return defaultValue(method.getReturnType());
    }
  }

  private static <T> T rejectingProxy(Class<T> type) {
    return proxy(type, (instance, method, arguments) -> {
      throw new AssertionError(type.getSimpleName() + " unexpectedly invoked: " + method);
    });
  }

  private static <T> T proxy(Class<T> type, InvocationHandler handler) {
    return type.cast(Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] {type}, handler));
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == byte.class) return (byte) 0;
    if (type == short.class) return (short) 0;
    if (type == int.class) return 0;
    if (type == long.class) return 0L;
    if (type == float.class) return 0.0f;
    if (type == double.class) return 0.0d;
    if (type == char.class) return (char) 0;
    return null;
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (error instanceof InvocationTargetException) error = error.getCause();
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private interface Operation { void run() throws Exception; }
  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const YANDEX_MUSIC_AUDIO_TRACK_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicAudioTrack;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicDirectUrlLoader;
import com.sedmelluq.discord.lavaplayer.tools.http.ExtendedHttpConfigurable;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import com.sedmelluq.discord.lavaplayer.track.DelegatedAudioTrack;
import com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;
import org.apache.http.client.protocol.HttpClientContext;

public final class GateYandexMusicAudioTrack {
  public static void main(String[] args) throws Exception {
    check(args.length >= 1 && args.length <= 2, "expected disposition and optional native path");
    boolean reference = args[0].equals("reference");
    check(reference || args[0].equals("candidate"), "unknown disposition");
    check(reference == (args.length == 1), "candidate requires native path");
    reflectionContract();
    commonBehavior();
    if (reference) legacyDisposition();
    else currentDisposition(args[1]);
    System.out.println(
        "common=public-concrete,delegated-super,2-fields,1-constructor,3-methods;"
        + "construction,track-info,source-identity,fresh-clone,reflection;service="
        + (reference ? "legacy-direct-url-http-mp3" :
            "current-native-bounded-mp3,explicit-token,no-legacy-direct-loader"));
  }

  private static void reflectionContract() throws Exception {
    Class<YandexMusicAudioTrack> type = YandexMusicAudioTrack.class;
    check(type.getModifiers() == Modifier.PUBLIC
        && type.getSuperclass() == DelegatedAudioTrack.class
        && type.getInterfaces().length == 0, "class metadata");
    check(type.getDeclaredFields().length == 2, "field count");
    checkField(type, "log", Class.forName("org.slf4j.Logger"),
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField(type, "sourceManager", YandexMusicAudioSourceManager.class,
        Modifier.PRIVATE | Modifier.FINAL);
    Constructor<?> constructor = type.getDeclaredConstructor(
        AudioTrackInfo.class, YandexMusicAudioSourceManager.class);
    check(type.getDeclaredConstructors().length == 1
        && constructor.getModifiers() == Modifier.PUBLIC
        && constructor.getExceptionTypes().length == 0 && !constructor.isSynthetic(),
        "constructor metadata");
    check(type.getDeclaredMethods().length == 3, "declared method count");
    checkMethod(type, "process", void.class, Modifier.PUBLIC,
        new Class<?>[] {LocalAudioTrackExecutor.class}, Exception.class);
    checkMethod(type, "makeClone", AudioTrack.class, Modifier.PUBLIC, new Class<?>[0]);
    checkMethod(type, "getSourceManager", AudioSourceManager.class, Modifier.PUBLIC,
        new Class<?>[0]);
  }

  private static void commonBehavior() throws Exception {
    YandexMusicAudioSourceManager source = new YandexMusicAudioSourceManager(false);
    AudioTrackInfo info = new AudioTrackInfo("title", "author", 1234L, "71663565", false,
        "https://music.yandex.ru/track/71663565", "art", null);
    YandexMusicAudioTrack track = new YandexMusicAudioTrack(info, source);
    check(track.getInfo() == info && track.getSourceManager() == source
        && field("sourceManager").get(track) == source, "captured identity");
    check(field("log").get(null) != null, "static logger");
    AudioTrack clone = track.makeClone();
    check(clone instanceof YandexMusicAudioTrack && clone != track && clone.getInfo() == info
        && clone.getSourceManager() == source && clone.getPosition() == 0L,
        "fresh clone identity");
    source.shutdown();
  }

  private static void legacyDisposition() throws Exception {
    RecordingManager source = new RecordingManager();
    YandexMusicAudioTrack track = new YandexMusicAudioTrack(new AudioTrackInfo(
        "title", "author", 1234L, "71663565", false,
        "https://music.yandex.ru/track/71663565", null, null), source);
    RuntimeException failure = expect(RuntimeException.class, () -> track.process(null));
    check(failure == source.sentinel && source.interfaceCalls == 1
        && source.directCalls == 1 && source.closeCalls == 1,
        "legacy direct URL and HTTP lifetime");
    source.shutdown();
  }

  private static void currentDisposition(String nativeLibrary) throws Exception {
    Class.forName("dev.mantle.internal.NativeLoader")
        .getMethod("load", String.class).invoke(null, nativeLibrary);
    Class<?> nativeType = Class.forName("dev.mantle.internal.MantleNative");
    Method process = nativeType.getDeclaredMethod("processYandexMusicTrack",
        YandexMusicAudioTrack.class, LocalAudioTrackExecutor.class);
    check(Modifier.isPublic(process.getModifiers()) && Modifier.isStatic(process.getModifiers())
        && Modifier.isNative(process.getModifiers()), "current native route");
    YandexMusicAudioSourceManager source = new YandexMusicAudioSourceManager(false);
    YandexMusicAudioTrack track = new YandexMusicAudioTrack(new AudioTrackInfo(
        "title", "author", 1234L, "71663565", false,
        "https://music.yandex.ru/track/71663565", null, null), source);
    System.clearProperty("dev.mantle.yandex.accessToken");
    RuntimeException missing = expect(RuntimeException.class, () -> track.process(null));
    check(missing.getMessage().contains("requires dev.mantle.yandex.accessToken"),
        "missing caller token fails before service traffic");
    System.setProperty("dev.mantle.yandex.accessToken", "");
    RuntimeException invalid = expect(RuntimeException.class, () -> track.process(null));
    check(invalid.getMessage().contains("invalid Yandex Music JVM access token"),
        "invalid caller token fails before service traffic");
    System.clearProperty("dev.mantle.yandex.accessToken");
    source.shutdown();
  }

  private static Field field(String name) throws Exception {
    Field field = YandexMusicAudioTrack.class.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }

  private static void checkField(Class<?> owner, String name, Class<?> type, int modifiers)
      throws Exception {
    Field field = owner.getDeclaredField(name);
    check(field.getType() == type && field.getGenericType() == type
        && field.getModifiers() == modifiers && !field.isSynthetic(), name + " metadata");
  }

  private static void checkMethod(Class<?> owner, String name, Class<?> returnType,
                                  int modifiers, Class<?>[] parameters,
                                  Class<?>... exceptions) throws Exception {
    Method method = owner.getDeclaredMethod(name, parameters);
    check(method.getReturnType() == returnType && method.getModifiers() == modifiers
        && Arrays.equals(method.getParameterTypes(), parameters)
        && Arrays.equals(method.getExceptionTypes(), exceptions)
        && method.getTypeParameters().length == 0 && !method.isBridge()
        && !method.isSynthetic() && !method.isVarArgs(), method + " metadata");
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private static final class RecordingManager extends YandexMusicAudioSourceManager {
    final RuntimeException sentinel = new RuntimeException("legacy-direct-url-sentinel");
    final HttpInterface http;
    int interfaceCalls;
    int directCalls;
    int closeCalls;

    RecordingManager() {
      super(false);
      http = new HttpInterface(null, HttpClientContext.create(), false, null) {
        @Override public void close() throws IOException { closeCalls++; }
      };
    }

    @Override public HttpInterface getHttpInterface() {
      interfaceCalls++;
      return http;
    }

    @Override public YandexMusicDirectUrlLoader getDirectUrlLoader() {
      return new YandexMusicDirectUrlLoader() {
        @Override public String getDirectUrl(String identifier, String format) {
          directCalls++;
          check(identifier.equals("71663565") && format.equals("mp3"),
              "legacy direct URL arguments");
          throw sentinel;
        }
        @Override public ExtendedHttpConfigurable getHttpConfiguration() { return null; }
        @Override public void shutdown() {}
      };
    }
  }

  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const YANDEX_MUSIC_DIRECT_URL_LOADER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicApiLoader;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicDirectUrlLoader;
import com.sedmelluq.discord.lavaplayer.tools.http.ExtendedHttpConfigurable;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;

public final class GateYandexMusicDirectUrlLoader {
  public static void main(String[] args) throws Exception {
    Class<YandexMusicDirectUrlLoader> type = YandexMusicDirectUrlLoader.class;
    check(type.getModifiers() == (Modifier.PUBLIC | Modifier.INTERFACE | Modifier.ABSTRACT)
        && type.isInterface() && !type.isAnnotation() && !type.isEnum() && !type.isSynthetic(),
        "interface metadata");
    check(type.getSuperclass() == null
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {YandexMusicApiLoader.class})
        && type.getDeclaredFields().length == 0 && type.getDeclaredConstructors().length == 0
        && type.getDeclaredMethods().length == 1 && type.getMethods().length == 3
        && type.getTypeParameters().length == 0, "interface shape");
    Method direct = type.getDeclaredMethod("getDirectUrl", String.class, String.class);
    check(direct.getModifiers() == (Modifier.PUBLIC | Modifier.ABSTRACT)
        && direct.getReturnType() == String.class
        && Arrays.equals(direct.getParameterTypes(), new Class<?>[] {String.class, String.class})
        && direct.getExceptionTypes().length == 0 && direct.getTypeParameters().length == 0
        && !direct.isDefault() && !direct.isBridge() && !direct.isSynthetic()
        && !direct.isVarArgs(), "method metadata");

    ExtendedHttpConfigurable configuration = proxy(ExtendedHttpConfigurable.class);
    RecordingLoader loader = new RecordingLoader(configuration);
    String firstTrack = new String("71663565");
    String firstCodec = new String("mp3");
    check(loader.getDirectUrl(firstTrack, firstCodec) == loader.result
        && loader.trackId == firstTrack && loader.codec == firstCodec && loader.calls == 1,
        "argument and result identity");
    loader.result = null;
    check(loader.getDirectUrl(null, null) == null && loader.trackId == null
        && loader.codec == null && loader.calls == 2, "null identity");
    check(loader.getHttpConfiguration() == configuration, "inherited configuration dispatch");
    loader.shutdown();
    loader.shutdown();
    check(loader.shutdowns == 2 && type.isAssignableFrom(RecordingLoader.class)
        && Arrays.equals(RecordingLoader.class.getInterfaces(),
            new Class<?>[] {YandexMusicDirectUrlLoader.class}), "implementation hierarchy");
    System.out.println("interface=public-abstract,object-root,yandex-api-loader-superinterface,"
        + "0-fields,0-constructors,1-declared-method;implementation=argument-result-identity,"
        + "null-identity,inherited-configuration-shutdown;reflection=exact");
  }

  private static <T> T proxy(Class<T> type) {
    Object value = Proxy.newProxyInstance(GateYandexMusicDirectUrlLoader.class.getClassLoader(),
        new Class<?>[] {type}, (proxy, method, arguments) -> {
          throw new AssertionError("configuration proxy invoked: " + method.getName());
        });
    return type.cast(value);
  }

  private static final class RecordingLoader implements YandexMusicDirectUrlLoader {
    private final ExtendedHttpConfigurable configuration;
    private String result = new String("https://example.invalid/audio.mp3");
    private String trackId;
    private String codec;
    private int calls;
    private int shutdowns;

    private RecordingLoader(ExtendedHttpConfigurable configuration) {
      this.configuration = configuration;
    }

    @Override
    public String getDirectUrl(String trackId, String codec) {
      this.trackId = trackId;
      this.codec = codec;
      calls++;
      return result;
    }

    @Override
    public ExtendedHttpConfigurable getHttpConfiguration() {
      return configuration;
    }

    @Override
    public void shutdown() {
      shutdowns++;
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const YANDEX_MUSIC_PLAYLIST_LOADER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicApiLoader;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicPlaylistLoader;
import com.sedmelluq.discord.lavaplayer.tools.http.ExtendedHttpConfigurable;
import com.sedmelluq.discord.lavaplayer.track.AudioItem;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Proxy;
import java.lang.reflect.Type;
import java.util.Arrays;
import java.util.function.Function;

public final class GateYandexMusicPlaylistLoader {
  public static void main(String[] args) throws Exception {
    Class<YandexMusicPlaylistLoader> type = YandexMusicPlaylistLoader.class;
    check(type.getModifiers() == (Modifier.PUBLIC | Modifier.INTERFACE | Modifier.ABSTRACT)
        && type.isInterface() && !type.isAnnotation() && !type.isEnum() && !type.isSynthetic(),
        "interface metadata");
    check(type.getSuperclass() == null
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {YandexMusicApiLoader.class})
        && type.getDeclaredFields().length == 0 && type.getDeclaredConstructors().length == 0
        && type.getDeclaredMethods().length == 2 && type.getMethods().length == 4
        && type.getTypeParameters().length == 0, "interface shape");
    Method user = type.getDeclaredMethod("loadPlaylist", String.class, String.class,
        String.class, Function.class);
    Method album = type.getDeclaredMethod("loadPlaylist", String.class, String.class,
        Function.class);
    checkMethod(user, new Class<?>[] {String.class, String.class, String.class, Function.class});
    checkMethod(album, new Class<?>[] {String.class, String.class, Function.class});
    checkFactorySignature(user, 3);
    checkFactorySignature(album, 2);

    ExtendedHttpConfigurable configuration = proxy(ExtendedHttpConfigurable.class);
    AudioItem userResult = proxy(AudioItem.class);
    AudioItem albumResult = proxy(AudioItem.class);
    RecordingLoader loader = new RecordingLoader(configuration, userResult, albumResult);
    String login = new String("alice");
    String id = new String("44");
    String property = new String("tracks");
    Function<AudioTrackInfo, AudioTrack> factory = info -> null;
    check(loader.loadPlaylist(login, id, property, factory) == userResult
        && loader.login == login && loader.id == id && loader.property == property
        && loader.factory == factory && loader.userCalls == 1, "user overload identity");
    String albumId = new String("55");
    String volume = new String("volumes");
    check(loader.loadPlaylist(albumId, volume, factory) == albumResult
        && loader.album == albumId && loader.property == volume && loader.factory == factory
        && loader.albumCalls == 1, "album overload identity");
    loader.userResult = null;
    loader.albumResult = null;
    check(loader.loadPlaylist(null, null, null, null) == null
        && loader.loadPlaylist(null, null, null) == null
        && loader.login == null && loader.id == null && loader.album == null
        && loader.property == null && loader.factory == null,
        "null identity and overload dispatch");
    check(loader.getHttpConfiguration() == configuration, "inherited configuration dispatch");
    loader.shutdown();
    loader.shutdown();
    check(loader.shutdowns == 2 && type.isAssignableFrom(RecordingLoader.class)
        && Arrays.equals(RecordingLoader.class.getInterfaces(),
            new Class<?>[] {YandexMusicPlaylistLoader.class}), "implementation hierarchy");
    System.out.println("interface=public-abstract,object-root,yandex-api-loader-superinterface,"
        + "0-fields,0-constructors,2-overloaded-methods;generic-factory=track-info-to-track;"
        + "implementation=overload-argument-result-identity,null-identity,"
        + "inherited-configuration-shutdown;reflection=exact");
  }

  private static void checkMethod(Method method, Class<?>[] parameters) {
    check(method.getModifiers() == (Modifier.PUBLIC | Modifier.ABSTRACT)
        && method.getReturnType() == AudioItem.class
        && Arrays.equals(method.getParameterTypes(), parameters)
        && method.getExceptionTypes().length == 0 && method.getTypeParameters().length == 0
        && !method.isDefault() && !method.isBridge() && !method.isSynthetic()
        && !method.isVarArgs(), method + " metadata");
  }

  private static void checkFactorySignature(Method method, int index) {
    Type factory = method.getGenericParameterTypes()[index];
    check(factory instanceof ParameterizedType, method + " factory type");
    ParameterizedType parameterized = (ParameterizedType) factory;
    check(parameterized.getRawType() == Function.class && parameterized.getOwnerType() == null
        && Arrays.equals(parameterized.getActualTypeArguments(),
            new Type[] {AudioTrackInfo.class, AudioTrack.class}), method + " generic factory");
  }

  private static <T> T proxy(Class<T> type) {
    Object value = Proxy.newProxyInstance(GateYandexMusicPlaylistLoader.class.getClassLoader(),
        new Class<?>[] {type}, (proxy, method, arguments) -> {
          throw new AssertionError("proxy invoked: " + method.getName());
        });
    return type.cast(value);
  }

  private static final class RecordingLoader implements YandexMusicPlaylistLoader {
    private final ExtendedHttpConfigurable configuration;
    private AudioItem userResult;
    private AudioItem albumResult;
    private String login;
    private String id;
    private String album;
    private String property;
    private Function<AudioTrackInfo, AudioTrack> factory;
    private int userCalls;
    private int albumCalls;
    private int shutdowns;

    private RecordingLoader(ExtendedHttpConfigurable configuration, AudioItem userResult,
                            AudioItem albumResult) {
      this.configuration = configuration;
      this.userResult = userResult;
      this.albumResult = albumResult;
    }

    @Override
    public AudioItem loadPlaylist(String login, String id, String property,
                                  Function<AudioTrackInfo, AudioTrack> factory) {
      this.login = login;
      this.id = id;
      this.property = property;
      this.factory = factory;
      userCalls++;
      return userResult;
    }

    @Override
    public AudioItem loadPlaylist(String album, String property,
                                  Function<AudioTrackInfo, AudioTrack> factory) {
      this.album = album;
      this.property = property;
      this.factory = factory;
      albumCalls++;
      return albumResult;
    }

    @Override
    public ExtendedHttpConfigurable getHttpConfiguration() {
      return configuration;
    }

    @Override
    public void shutdown() {
      shutdowns++;
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const YANDEX_MUSIC_SEARCH_RESULT_LOADER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicApiLoader;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicPlaylistLoader;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicSearchResultLoader;
import com.sedmelluq.discord.lavaplayer.tools.http.ExtendedHttpConfigurable;
import com.sedmelluq.discord.lavaplayer.track.AudioItem;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Proxy;
import java.lang.reflect.Type;
import java.util.Arrays;
import java.util.function.Function;

public final class GateYandexMusicSearchResultLoader {
  public static void main(String[] args) throws Exception {
    Class<YandexMusicSearchResultLoader> type = YandexMusicSearchResultLoader.class;
    check(type.getModifiers() == (Modifier.PUBLIC | Modifier.INTERFACE | Modifier.ABSTRACT)
        && type.isInterface() && !type.isAnnotation() && !type.isEnum() && !type.isSynthetic(),
        "interface metadata");
    check(type.getSuperclass() == null
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {YandexMusicApiLoader.class})
        && type.getDeclaredFields().length == 0 && type.getDeclaredConstructors().length == 0
        && type.getDeclaredMethods().length == 1 && type.getMethods().length == 3
        && type.getTypeParameters().length == 0, "interface shape");
    Method load = type.getDeclaredMethod("loadSearchResult", String.class,
        YandexMusicPlaylistLoader.class, Function.class);
    check(load.getModifiers() == (Modifier.PUBLIC | Modifier.ABSTRACT)
        && load.getReturnType() == AudioItem.class
        && Arrays.equals(load.getParameterTypes(), new Class<?>[] {
            String.class, YandexMusicPlaylistLoader.class, Function.class})
        && load.getExceptionTypes().length == 0 && load.getTypeParameters().length == 0
        && !load.isDefault() && !load.isBridge() && !load.isSynthetic() && !load.isVarArgs(),
        "method metadata");
    Type factoryType = load.getGenericParameterTypes()[2];
    check(factoryType instanceof ParameterizedType, "factory generic type");
    ParameterizedType factorySignature = (ParameterizedType) factoryType;
    check(factorySignature.getRawType() == Function.class
        && factorySignature.getOwnerType() == null
        && Arrays.equals(factorySignature.getActualTypeArguments(),
            new Type[] {AudioTrackInfo.class, AudioTrack.class}), "generic factory signature");

    ExtendedHttpConfigurable configuration = proxy(ExtendedHttpConfigurable.class);
    YandexMusicPlaylistLoader playlist = proxy(YandexMusicPlaylistLoader.class);
    AudioItem result = proxy(AudioItem.class);
    RecordingLoader loader = new RecordingLoader(configuration, result);
    String query = new String("architects");
    Function<AudioTrackInfo, AudioTrack> factory = info -> null;
    check(loader.loadSearchResult(query, playlist, factory) == result
        && loader.query == query && loader.playlist == playlist && loader.factory == factory
        && loader.calls == 1, "argument and result identity");
    loader.result = null;
    check(loader.loadSearchResult(null, null, null) == null && loader.query == null
        && loader.playlist == null && loader.factory == null && loader.calls == 2,
        "null identity");
    check(loader.getHttpConfiguration() == configuration, "inherited configuration dispatch");
    loader.shutdown();
    loader.shutdown();
    check(loader.shutdowns == 2 && type.isAssignableFrom(RecordingLoader.class)
        && Arrays.equals(RecordingLoader.class.getInterfaces(),
            new Class<?>[] {YandexMusicSearchResultLoader.class}), "implementation hierarchy");
    System.out.println("interface=public-abstract,object-root,yandex-api-loader-superinterface,"
        + "0-fields,0-constructors,1-declared-method;generic-factory=track-info-to-track;"
        + "implementation=query-playlist-factory-result-identity,null-identity,"
        + "inherited-configuration-shutdown;reflection=exact");
  }

  private static <T> T proxy(Class<T> type) {
    Object value = Proxy.newProxyInstance(GateYandexMusicSearchResultLoader.class.getClassLoader(),
        new Class<?>[] {type}, (proxy, method, arguments) -> {
          throw new AssertionError("proxy invoked: " + method.getName());
        });
    return type.cast(value);
  }

  private static final class RecordingLoader implements YandexMusicSearchResultLoader {
    private final ExtendedHttpConfigurable configuration;
    private AudioItem result;
    private String query;
    private YandexMusicPlaylistLoader playlist;
    private Function<AudioTrackInfo, AudioTrack> factory;
    private int calls;
    private int shutdowns;

    private RecordingLoader(ExtendedHttpConfigurable configuration, AudioItem result) {
      this.configuration = configuration;
      this.result = result;
    }

    @Override
    public AudioItem loadSearchResult(String query, YandexMusicPlaylistLoader playlist,
                                      Function<AudioTrackInfo, AudioTrack> factory) {
      this.query = query;
      this.playlist = playlist;
      this.factory = factory;
      calls++;
      return result;
    }

    @Override
    public ExtendedHttpConfigurable getHttpConfiguration() {
      return configuration;
    }

    @Override
    public void shutdown() {
      shutdowns++;
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const YANDEX_MUSIC_TRACK_LOADER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicApiLoader;
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicTrackLoader;
import com.sedmelluq.discord.lavaplayer.tools.http.ExtendedHttpConfigurable;
import com.sedmelluq.discord.lavaplayer.track.AudioItem;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Proxy;
import java.lang.reflect.Type;
import java.util.Arrays;
import java.util.function.Function;

public final class GateYandexMusicTrackLoader {
  public static void main(String[] args) throws Exception {
    Class<YandexMusicTrackLoader> type = YandexMusicTrackLoader.class;
    check(type.getModifiers() == (Modifier.PUBLIC | Modifier.INTERFACE | Modifier.ABSTRACT)
        && type.isInterface() && !type.isAnnotation() && !type.isEnum() && !type.isSynthetic(),
        "interface metadata");
    check(type.getSuperclass() == null
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {YandexMusicApiLoader.class})
        && type.getDeclaredFields().length == 0 && type.getDeclaredConstructors().length == 0
        && type.getDeclaredMethods().length == 1 && type.getMethods().length == 3
        && type.getTypeParameters().length == 0, "interface shape");
    Method load = type.getDeclaredMethod("loadTrack", String.class, String.class, Function.class);
    check(load.getModifiers() == (Modifier.PUBLIC | Modifier.ABSTRACT)
        && load.getReturnType() == AudioItem.class
        && Arrays.equals(load.getParameterTypes(),
            new Class<?>[] {String.class, String.class, Function.class})
        && load.getExceptionTypes().length == 0 && load.getTypeParameters().length == 0
        && !load.isDefault() && !load.isBridge() && !load.isSynthetic() && !load.isVarArgs(),
        "method metadata");
    Type factoryType = load.getGenericParameterTypes()[2];
    check(factoryType instanceof ParameterizedType, "factory generic type");
    ParameterizedType factorySignature = (ParameterizedType) factoryType;
    check(factorySignature.getRawType() == Function.class
        && factorySignature.getOwnerType() == null
        && Arrays.equals(factorySignature.getActualTypeArguments(),
            new Type[] {AudioTrackInfo.class, AudioTrack.class}), "generic factory signature");

    ExtendedHttpConfigurable configuration = proxy(ExtendedHttpConfigurable.class);
    AudioItem result = proxy(AudioItem.class);
    RecordingLoader loader = new RecordingLoader(configuration, result);
    String trackId = new String("track-id");
    String albumId = new String("album-id");
    Function<AudioTrackInfo, AudioTrack> factory = info -> null;
    check(loader.loadTrack(trackId, albumId, factory) == result
        && loader.trackId == trackId && loader.albumId == albumId && loader.factory == factory
        && loader.calls == 1, "argument and result identity");
    loader.result = null;
    check(loader.loadTrack(null, null, null) == null && loader.trackId == null
        && loader.albumId == null && loader.factory == null && loader.calls == 2,
        "null identity");
    check(loader.getHttpConfiguration() == configuration, "inherited configuration dispatch");
    loader.shutdown();
    loader.shutdown();
    check(loader.shutdowns == 2 && type.isAssignableFrom(RecordingLoader.class)
        && Arrays.equals(RecordingLoader.class.getInterfaces(),
            new Class<?>[] {YandexMusicTrackLoader.class}), "implementation hierarchy");
    System.out.println("interface=public-abstract,object-root,yandex-api-loader-superinterface,"
        + "0-fields,0-constructors,1-declared-method;generic-factory=track-info-to-track;"
        + "implementation=track-album-factory-result-identity,null-identity,"
        + "inherited-configuration-shutdown;reflection=exact");
  }

  private static <T> T proxy(Class<T> type) {
    Object value = Proxy.newProxyInstance(GateYandexMusicTrackLoader.class.getClassLoader(),
        new Class<?>[] {type}, (proxy, method, arguments) -> {
          throw new AssertionError("proxy invoked: " + method.getName());
        });
    return type.cast(value);
  }

  private static final class RecordingLoader implements YandexMusicTrackLoader {
    private final ExtendedHttpConfigurable configuration;
    private AudioItem result;
    private String trackId;
    private String albumId;
    private Function<AudioTrackInfo, AudioTrack> factory;
    private int calls;
    private int shutdowns;

    private RecordingLoader(ExtendedHttpConfigurable configuration, AudioItem result) {
      this.configuration = configuration;
      this.result = result;
    }

    @Override
    public AudioItem loadTrack(String trackId, String albumId,
                               Function<AudioTrackInfo, AudioTrack> factory) {
      this.trackId = trackId;
      this.albumId = albumId;
      this.factory = factory;
      calls++;
      return result;
    }

    @Override
    public ExtendedHttpConfigurable getHttpConfiguration() {
      return configuration;
    }

    @Override
    public void shutdown() {
      shutdowns++;
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const YANDEX_MUSIC_UTILS_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicUtils;
import com.sedmelluq.discord.lavaplayer.tools.JsonBrowser;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Proxy;
import java.lang.reflect.Type;
import java.util.Arrays;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;
import java.util.function.Function;

public final class GateYandexMusicUtils {
  public static void main(String[] args) throws Exception {
    Class<YandexMusicUtils> type = YandexMusicUtils.class;
    check(type.getModifiers() == Modifier.PUBLIC && !type.isInterface() && !type.isAnnotation()
        && !type.isEnum() && !type.isSynthetic() && type.getSuperclass() == Object.class
        && type.getInterfaces().length == 0 && type.getDeclaredFields().length == 1
        && type.getDeclaredConstructors().length == 1 && type.getDeclaredMethods().length == 2
        && type.getTypeParameters().length == 0, "class shape");
    Field format = type.getDeclaredField("TRACK_URL_FORMAT");
    check(format.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL)
        && format.getType() == String.class && !format.isEnumConstant() && !format.isSynthetic(),
        "constant metadata");
    format.setAccessible(true);
    check("https://music.yandex.ru/album/%s/track/%s".equals(format.get(null)),
        "constant value");
    Constructor<YandexMusicUtils> constructor = type.getDeclaredConstructor();
    check(constructor.getModifiers() == Modifier.PUBLIC && !constructor.isSynthetic()
        && constructor.getExceptionTypes().length == 0 && constructor.getTypeParameters().length == 0
        && constructor.newInstance().getClass() == type, "constructor");
    Method extract = type.getDeclaredMethod("extractTrack", JsonBrowser.class, Function.class);
    check(extract.getModifiers() == (Modifier.PUBLIC | Modifier.STATIC)
        && extract.getReturnType() == AudioTrack.class
        && Arrays.equals(extract.getParameterTypes(),
            new Class<?>[] {JsonBrowser.class, Function.class})
        && extract.getExceptionTypes().length == 0 && extract.getTypeParameters().length == 0
        && !extract.isBridge() && !extract.isSynthetic() && !extract.isVarArgs(),
        "extract metadata");
    Type factoryType = extract.getGenericParameterTypes()[1];
    check(factoryType instanceof ParameterizedType, "factory generic type");
    ParameterizedType factorySignature = (ParameterizedType) factoryType;
    check(factorySignature.getRawType() == Function.class
        && factorySignature.getOwnerType() == null
        && Arrays.equals(factorySignature.getActualTypeArguments(),
            new Type[] {AudioTrackInfo.class, AudioTrack.class}), "generic factory signature");
    Method helper = type.getDeclaredMethod("lambda$extractTrack$0", JsonBrowser.class);
    check(helper.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC | 0x1000)
        && helper.getReturnType() == String.class && helper.isSynthetic() && !helper.isBridge()
        && !helper.isVarArgs() && helper.getExceptionTypes().length == 0, "helper metadata");

    AudioTrack result = proxy(AudioTrack.class);
    AtomicReference<AudioTrackInfo> captured = new AtomicReference<>();
    AtomicInteger calls = new AtomicInteger();
    Function<AudioTrackInfo, AudioTrack> factory = info -> {
      captured.set(info);
      calls.incrementAndGet();
      return result;
    };
    AudioTrack direct = YandexMusicUtils.extractTrack(JsonBrowser.parse("{"
        + "\"id\":\"44\",\"title\":\"Fixture Song\",\"durationMs\":9876,"
        + "\"artists\":[{\"name\":\"Artist A\"},{\"name\":\"Artist B\"}],"
        + "\"albums\":[{\"id\":\"55\",\"coverUri\":\"album/%%\"}],"
        + "\"coverUri\":\"track/%%\",\"ogImage\":\"og/%%\"}"), factory);
    AudioTrackInfo info = captured.get();
    check(direct == result && calls.get() == 1 && "Fixture Song".equals(info.title)
        && "Artist A, Artist B".equals(info.author) && info.length == 9876L
        && "44".equals(info.identifier) && !info.isStream
        && "https://music.yandex.ru/album/55/track/44".equals(info.uri)
        && "https://track/1000x1000".equals(info.artworkUrl) && info.isrc == null,
        "direct extraction and cover priority");

    YandexMusicUtils.extractTrack(JsonBrowser.parse("{\"track\":{"
        + "\"id\":\"45\",\"title\":\"Wrapped\",\"durationMs\":1,"
        + "\"artists\":[],\"albums\":[{\"id\":\"56\",\"coverUri\":\"album/%%\"}],"
        + "\"ogImage\":\"og/%%\"}}"), factory);
    check(calls.get() == 2 && "45".equals(captured.get().identifier)
        && "".equals(captured.get().author)
        && "https://og/1000x1000".equals(captured.get().artworkUrl),
        "wrapper and og-image fallback");

    YandexMusicUtils.extractTrack(JsonBrowser.parse("{"
        + "\"id\":\"46\",\"title\":\"Album Art\",\"durationMs\":2,"
        + "\"artists\":[{\"name\":\"Solo\"}],"
        + "\"albums\":[{\"id\":\"57\",\"coverUri\":\"album/%%\"}]}"), factory);
    check(calls.get() == 3
        && "https://album/1000x1000".equals(captured.get().artworkUrl),
        "album cover fallback");

    AudioTrack nullResult = YandexMusicUtils.extractTrack(JsonBrowser.parse("{"
        + "\"id\":\"47\",\"title\":\"No Art\",\"durationMs\":3,"
        + "\"artists\":[{\"name\":\"Solo\"}],\"albums\":[{\"id\":\"58\"}]}"),
        ignored -> null);
    check(nullResult == null, "null factory result identity");
    System.out.println("class=public-concrete,object-root,0-interfaces,1-private-constant,"
        + "1-constructor,2-declared-methods;generic-factory=track-info-to-track;"
        + "extraction=wrapper,direct,artist-order,metadata,url,cover-priority-og-album-null,"
        + "factory-result-identity;reflection=exact");
  }

  private static <T> T proxy(Class<T> type) {
    Object value = Proxy.newProxyInstance(GateYandexMusicUtils.class.getClassLoader(),
        new Class<?>[] {type}, (proxy, method, arguments) -> {
          throw new AssertionError("proxy invoked: " + method.getName());
        });
    return type.cast(value);
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const DEFAULT_YOUTUBE_LINK_ROUTER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.youtube.DefaultYoutubeLinkRouter;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeLinkRouter;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;
import java.util.LinkedHashMap;
import java.util.Map;

public final class GateDefaultYoutubeLinkRouter {
  public static void main(String[] args) throws Exception {
    Class<DefaultYoutubeLinkRouter> type = DefaultYoutubeLinkRouter.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {YoutubeLinkRouter.class})
        && type.getDeclaredFields().length == 9 && type.getDeclaredConstructors().length == 1
        && type.getDeclaredMethods().length == 11 && type.getTypeParameters().length == 0,
        "class shape");
    for (String name : new String[] {"SEARCH_PREFIX", "SEARCH_MUSIC_PREFIX", "PROTOCOL_REGEX",
        "DOMAIN_REGEX", "SHORT_DOMAIN_REGEX", "VIDEO_ID_REGEX", "PLAYLIST_ID_REGEX"}) {
      Field field = type.getDeclaredField(name);
      check(field.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL)
          && field.getType() == String.class, "constant " + name);
    }
    Field pattern = type.getDeclaredField("directVideoIdPattern");
    Field extractors = type.getDeclaredField("extractors");
    check(pattern.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL)
        && extractors.getModifiers() == (Modifier.PRIVATE | Modifier.FINAL)
        && extractors.getType().isArray(), "routing fields");
    pattern.setAccessible(true);
    extractors.setAccessible(true);
    Constructor<DefaultYoutubeLinkRouter> constructor = type.getDeclaredConstructor();
    DefaultYoutubeLinkRouter router = constructor.newInstance();
    check(constructor.getModifiers() == Modifier.PUBLIC && pattern.get(null) != null
        && ((Object[]) extractors.get(router)).length == 7, "constructor state");
    Method route = type.getDeclaredMethod("route", String.class, YoutubeLinkRouter.Routes.class);
    check(route.getModifiers() == Modifier.PUBLIC && route.getReturnType() == Object.class
        && route.getTypeParameters().length == 1 && "T".equals(route.getTypeParameters()[0].getName())
        && !route.isBridge() && !route.isSynthetic() && route.getExceptionTypes().length == 0,
        "route metadata");
    for (String name : new String[] {"routeDirectPlaylist", "routeFromMainDomain",
        "routeFromUrlWithVideoId", "routeFromShortDomain", "routeFromEmbed", "routeFromShorts",
        "routeFromLive"}) {
      Method method = Arrays.stream(type.getDeclaredMethods())
          .filter(value -> value.getName().equals(name)).findFirst().orElseThrow();
      check(method.getModifiers() == Modifier.PROTECTED && method.getReturnType() == Object.class
          && method.getTypeParameters().length == 1 && !method.isSynthetic(),
          "protected metadata " + name);
    }

    Routes routes = new Routes();
    expect(router.route("ytsearch:   lo fi mix  ", routes), "search:lo fi mix", routes, 1);
    expect(router.route("ytmsearch: synth wave ", routes), "music:synth wave", routes, 2);
    expect(router.route("abc_DEF-123", routes), "track:abc_DEF-123", routes, 3);
    expect(router.route("PLfixture_123", routes), "playlist:PLfixture_123:null", routes, 4);
    expect(router.route("youtube.com/watch?v=abcdefghijk", routes), "track:abcdefghijk", routes, 5);
    expect(router.route("https://www.youtube.com/watch?v=abcdefghijkTRAILING", routes),
        "track:abcdefghijk", routes, 6);
    expect(router.route("https://music.youtube.com/watch?v=abcdefghijk&list=PLfixture", routes),
        "playlist:PLfixture:abcdefghijk", routes, 7);
    expect(router.route("https://m.youtube.com/watch?v=abcdefghijk&list=RDfixture", routes),
        "mix:RDfixture:abcdefghijk", routes, 8);
    expect(router.route("https://youtube.com/playlist?list=UUfixture", routes),
        "playlist:UUfixture:null", routes, 9);
    expect(router.route("https://youtube.com/watch_videos?video_ids=a,b,c", routes),
        "anonymous:a,b,c", routes, 10);
    expect(router.route("youtu.be/abcdefghijk?list=PLshort", routes),
        "playlist:PLshort:abcdefghijk", routes, 11);
    expect(router.route("youtube.com/embed/abcdefghijk", routes), "track:abcdefghijk", routes, 12);
    expect(router.route("youtube.com/shorts/abcdefghijk?list=RDshort", routes),
        "mix:RDshort:abcdefghijk", routes, 13);
    expect(router.route("youtube.com/live/abcdefghijk", routes), "track:abcdefghijk", routes, 14);
    expect(router.route("youtube.com/watch?v=bad", routes), "none", routes, 15);
    expect(router.route("youtube.com/watch?v=abcdefghijk&v=zyxwvutsrqp", routes),
        "track:abcdefghijk", routes, 16);
    check(router.route("https://example.com/watch?v=abcdefghijk", routes) == null
        && routes.calls == 16, "unsupported URL");

    ExposedRouter exposed = new ExposedRouter();
    expect(exposed.direct(routes, "LLdirect"), "playlist:LLdirect:null", routes, 17);
    expect(exposed.main(routes, "youtube.com/watch?v=abcdefghijk&list=PLmain"),
        "playlist:PLmain:abcdefghijk", routes, 18);
    expect(exposed.shortUrl(routes, "youtu.be/abcdefghijk"), "track:abcdefghijk", routes, 19);
    expect(exposed.embed(routes, "youtube.com/embed/abcdefghijk"), "track:abcdefghijk", routes, 20);
    expect(exposed.shorts(routes, "youtube.com/shorts/abcdefghijk"), "track:abcdefghijk", routes, 21);
    expect(exposed.live(routes, "youtube.com/live/abcdefghijk"), "track:abcdefghijk", routes, 22);

    Class<?> urlInfoType = Class.forName(type.getName() + "$UrlInfo");
    Constructor<?> urlInfoConstructor = urlInfoType.getDeclaredConstructor(String.class, Map.class);
    urlInfoConstructor.setAccessible(true);
    Map<String, String> parameters = new LinkedHashMap<>();
    parameters.put("list", "RDreflect");
    Object urlInfo = urlInfoConstructor.newInstance("/watch", parameters);
    Method videoRoute = type.getDeclaredMethod("routeFromUrlWithVideoId",
        YoutubeLinkRouter.Routes.class, String.class, urlInfoType);
    videoRoute.setAccessible(true);
    expect(videoRoute.invoke(router, routes, "abcdefghijkEXTRA", urlInfo),
        "mix:RDreflect:abcdefghijk", routes, 23);

    Routes nullRoutes = new Routes();
    nullRoutes.returnNull = true;
    check(router.route("abc_DEF-123", nullRoutes) == null && nullRoutes.calls == 1,
        "null callback result");
    System.out.println("class=public-concrete,object-root,youtube-router-interface,9-private-fields,"
        + "1-constructor,11-declared-methods;routes=search,music,direct-video,direct-playlist,"
        + "main-watch-playlist-mix-anonymous,short,embed,shorts,live,none,unsupported-null,"
        + "truncate,duplicate-first,null-result;protected=7,generic-T;reflection=exact");
  }

  private static final class ExposedRouter extends DefaultYoutubeLinkRouter {
    Object direct(YoutubeLinkRouter.Routes<Object> routes, String id) {
      return routeDirectPlaylist(routes, id);
    }
    Object main(YoutubeLinkRouter.Routes<Object> routes, String url) {
      return routeFromMainDomain(routes, url);
    }
    Object shortUrl(YoutubeLinkRouter.Routes<Object> routes, String url) {
      return routeFromShortDomain(routes, url);
    }
    Object embed(YoutubeLinkRouter.Routes<Object> routes, String url) {
      return routeFromEmbed(routes, url);
    }
    Object shorts(YoutubeLinkRouter.Routes<Object> routes, String url) {
      return routeFromShorts(routes, url);
    }
    Object live(YoutubeLinkRouter.Routes<Object> routes, String url) {
      return routeFromLive(routes, url);
    }
  }

  private static final class Routes implements YoutubeLinkRouter.Routes<Object> {
    int calls;
    boolean returnNull;
    private Object value(String value) { calls++; return returnNull ? null : value; }
    @Override public Object track(String id) { return value("track:" + id); }
    @Override public Object playlist(String id, String selected) {
      return value("playlist:" + id + ":" + selected);
    }
    @Override public Object mix(String id, String selected) {
      return value("mix:" + id + ":" + selected);
    }
    @Override public Object search(String query) { return value("search:" + query); }
    @Override public Object searchMusic(String query) { return value("music:" + query); }
    @Override public Object anonymous(String ids) { return value("anonymous:" + ids); }
    @Override public Object none() { return value("none"); }
  }

  private static void expect(Object actual, String expected, Routes routes, int calls) {
    check(expected.equals(actual) && routes.calls == calls, expected + " got " + actual);
  }
  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const DEFAULT_YOUTUBE_PLAYLIST_LOADER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.youtube.DefaultYoutubePlaylistLoader;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubePlaylistLoader;
import com.sedmelluq.discord.lavaplayer.tools.JsonBrowser;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.track.AudioPlaylist;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.Function;

public final class GateDefaultYoutubePlaylistLoader {
  public static void main(String[] args) throws Exception {
    check(args.length == 1, "mode required");
    Class<DefaultYoutubePlaylistLoader> type = DefaultYoutubePlaylistLoader.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {YoutubePlaylistLoader.class})
        && type.getDeclaredFields().length == 1 && type.getDeclaredConstructors().length == 1
        && type.getDeclaredMethods().length == 7 && type.getTypeParameters().length == 0,
        "class shape");
    Field pageCount = type.getDeclaredField("playlistPageCount");
    check(pageCount.getModifiers() == (Modifier.PRIVATE | Modifier.VOLATILE)
        && pageCount.getType() == int.class, "page-count field");
    pageCount.setAccessible(true);
    Constructor<DefaultYoutubePlaylistLoader> constructor = type.getDeclaredConstructor();
    check(constructor.getModifiers() == Modifier.PUBLIC && !constructor.isSynthetic()
        && constructor.getExceptionTypes().length == 0, "constructor metadata");
    DefaultYoutubePlaylistLoader loader = constructor.newInstance();
    check(pageCount.getInt(loader) == 6, "default page count");

    Method setter = type.getDeclaredMethod("setPlaylistPageCount", int.class);
    check(setter.getModifiers() == Modifier.PUBLIC && setter.getReturnType() == void.class
        && setter.getExceptionTypes().length == 0 && !setter.isBridge() && !setter.isSynthetic(),
        "setter metadata");
    for (int value : new int[] {0, -7, Integer.MAX_VALUE, 6}) {
      loader.setPlaylistPageCount(value);
      check(pageCount.getInt(loader) == value, "unrestricted page-count mutation");
    }

    Method load = type.getDeclaredMethod("load", HttpInterface.class, String.class, String.class,
        Function.class);
    check(load.getModifiers() == Modifier.PUBLIC && load.getReturnType() == AudioPlaylist.class
        && load.getExceptionTypes().length == 0 && !load.isBridge() && !load.isSynthetic()
        && !load.isVarArgs(), "load metadata");
    Type genericFactory = load.getGenericParameterTypes()[3];
    check(genericFactory instanceof ParameterizedType
        && ((ParameterizedType) genericFactory).getRawType() == Function.class
        && Arrays.equals(((ParameterizedType) genericFactory).getActualTypeArguments(),
            new Type[] {AudioTrackInfo.class, AudioTrack.class}), "factory generic metadata");

    Method build = type.getDeclaredMethod("buildPlaylist", HttpInterface.class, JsonBrowser.class,
        String.class, Function.class);
    Method alert = type.getDeclaredMethod("findErrorAlert", JsonBrowser.class);
    Method selected = type.getDeclaredMethod("findSelectedTrack", List.class, String.class);
    Method extract = type.getDeclaredMethod("extractPlaylistTracks", JsonBrowser.class, List.class,
        Function.class);
    Method lambda = type.getDeclaredMethod("lambda$findErrorAlert$0", JsonBrowser.class);
    check(build.getModifiers() == Modifier.PRIVATE && build.getReturnType() == AudioPlaylist.class
        && Arrays.equals(build.getExceptionTypes(), new Class<?>[] {IOException.class})
        && alert.getModifiers() == Modifier.PRIVATE && alert.getReturnType() == String.class
        && selected.getModifiers() == Modifier.PRIVATE && selected.getReturnType() == AudioTrack.class
        && extract.getModifiers() == Modifier.PRIVATE && extract.getReturnType() == String.class
        && lambda.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC | 0x1000)
        && lambda.getReturnType() == String.class && lambda.isSynthetic(), "private metadata");
    lambda.setAccessible(true);
    check("joined".equals(lambda.invoke(null, JsonBrowser.parse("{\"text\":\"joined\"}"))),
        "synthetic helper value");

    AtomicInteger factoryCalls = new AtomicInteger();
    Function<AudioTrackInfo, AudioTrack> factory = info -> {
      factoryCalls.incrementAndGet();
      return null;
    };
    if (args[0].equals("candidate")) {
      assertUnsupported(loader, null, null, null, factory);
      assertUnsupported(loader, null, "PLfixture", "abcdefghijk", factory);
      check(factoryCalls.get() == 0 && pageCount.getInt(loader) == 6,
          "failure precedes collaborators and preserves state");
      System.out.println("common=public-concrete,object-root,playlist-loader-interface,"
          + "1-private-volatile-field,1-constructor,2-exported-methods;default-page-count-6,"
          + "unrestricted-mutation,generic-factory,private-signatures,synthetic-helper,reflection;"
          + "service=deterministic-no-network,current-bounded-native-source");
    } else {
      check(args[0].equals("reference"), "unknown mode");
      System.out.println("common=public-concrete,object-root,playlist-loader-interface,"
          + "1-private-volatile-field,1-constructor,2-exported-methods;default-page-count-6,"
          + "unrestricted-mutation,generic-factory,private-signatures,synthetic-helper,reflection;"
          + "service=legacy-innertube-browse,mutable-page-count");
    }
  }

  private static void assertUnsupported(DefaultYoutubePlaylistLoader loader,
      HttpInterface http, String playlist, String selected,
      Function<AudioTrackInfo, AudioTrack> factory) {
    try {
      loader.load(http, playlist, selected, factory);
      throw new AssertionError("legacy playlist discovery unexpectedly succeeded");
    } catch (UnsupportedOperationException error) {
      check(error.getMessage().contains("Legacy YouTube playlist discovery is unsupported"),
          "stable unsupported disposition");
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const DEFAULT_YOUTUBE_TRACK_DETAILS_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.youtube.DefaultYoutubeTrackDetails;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeSignatureResolver;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeTrackDetails;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeTrackFormat;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeTrackJsonData;
import com.sedmelluq.discord.lavaplayer.tools.FriendlyException;
import com.sedmelluq.discord.lavaplayer.tools.JsonBrowser;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import java.util.Arrays;
import java.util.List;

public final class GateDefaultYoutubeTrackDetails {
  private static final String VIDEO_ID = "dQw4w9WgXcQ";

  public static void main(String[] args) throws Exception {
    check(args.length == 1, "mode required");
    Class<DefaultYoutubeTrackDetails> type = DefaultYoutubeTrackDetails.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {YoutubeTrackDetails.class})
        && type.getDeclaredFields().length == 4 && type.getDeclaredConstructors().length == 1
        && type.getDeclaredMethods().length == 7 && type.getDeclaredClasses().length == 1,
        "class shape");

    Field log = type.getDeclaredField("log");
    Field extractors = type.getDeclaredField("FORMAT_EXTRACTORS");
    Field videoId = type.getDeclaredField("videoId");
    Field data = type.getDeclaredField("data");
    check(log.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL)
        && extractors.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL)
        && videoId.getModifiers() == (Modifier.PRIVATE | Modifier.FINAL)
        && data.getModifiers() == (Modifier.PRIVATE | Modifier.FINAL), "field metadata");
    log.setAccessible(true);
    extractors.setAccessible(true);
    videoId.setAccessible(true);
    data.setAccessible(true);
    check(log.get(null) != null, "logger initialization");
    Object[] chain = (Object[]) extractors.get(null);
    check(chain.length == 4
        && chain[0].getClass().getSimpleName().equals("LegacyAdaptiveFormatsExtractor")
        && chain[1].getClass().getSimpleName().equals("StreamingDataFormatsExtractor")
        && chain[2].getClass().getSimpleName().equals("LegacyDashMpdFormatsExtractor")
        && chain[3].getClass().getSimpleName().equals("LegacyStreamMapFormatsExtractor"),
        "extractor chain");

    Constructor<DefaultYoutubeTrackDetails> constructor = type.getDeclaredConstructor(
        String.class, YoutubeTrackJsonData.class);
    check(constructor.getModifiers() == Modifier.PUBLIC && !constructor.isSynthetic()
        && constructor.getExceptionTypes().length == 0, "constructor metadata");
    YoutubeTrackJsonData identityData = jsonData("{}", "{}", "/player.js");
    DefaultYoutubeTrackDetails identity = constructor.newInstance(VIDEO_ID, identityData);
    check(videoId.get(identity) == VIDEO_ID && data.get(identity) == identityData
        && identity.getPlayerScript().equals("/player.js"), "constructor and script identity");

    Method getInfo = type.getDeclaredMethod("getTrackInfo");
    Method getFormats = type.getDeclaredMethod("getFormats", HttpInterface.class,
        YoutubeSignatureResolver.class);
    Method getScript = type.getDeclaredMethod("getPlayerScript");
    check(getInfo.getModifiers() == Modifier.PUBLIC && getInfo.getReturnType() == AudioTrackInfo.class
        && getFormats.getModifiers() == Modifier.PUBLIC && getFormats.getReturnType() == List.class
        && getScript.getModifiers() == Modifier.PUBLIC && getScript.getReturnType() == String.class
        && getInfo.getExceptionTypes().length == 0 && getFormats.getExceptionTypes().length == 0
        && getScript.getExceptionTypes().length == 0, "public method metadata");
    Type genericReturn = getFormats.getGenericReturnType();
    check(genericReturn instanceof ParameterizedType
        && ((ParameterizedType) genericReturn).getRawType() == List.class
        && Arrays.equals(((ParameterizedType) genericReturn).getActualTypeArguments(),
            new Type[] {YoutubeTrackFormat.class}), "format generic metadata");

    Method loadFormats = type.getDeclaredMethod("loadTrackFormats", HttpInterface.class,
        YoutubeSignatureResolver.class);
    Method loadInfo = type.getDeclaredMethod("loadTrackInfo");
    Method loadLegacy = type.getDeclaredMethod("loadLegacyTrackInfo");
    Class<?> temporal = type.getDeclaredClasses()[0];
    Method build = type.getDeclaredMethod("buildTrackInfo", String.class, String.class,
        String.class, temporal, String.class);
    check(loadFormats.getModifiers() == Modifier.PRIVATE && loadInfo.getModifiers() == Modifier.PRIVATE
        && loadLegacy.getModifiers() == Modifier.PRIVATE && build.getModifiers() == Modifier.PRIVATE
        && loadFormats.getReturnType() == List.class
        && loadInfo.getReturnType() == AudioTrackInfo.class
        && loadLegacy.getReturnType() == AudioTrackInfo.class
        && build.getReturnType() == AudioTrackInfo.class, "private method metadata");
    check(temporal.getSimpleName().equals("TemporalInfo")
        && temporal.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC)
        && temporal.getDeclaredFields().length == 2
        && temporal.getDeclaredConstructors().length == 1
        && temporal.getDeclaredMethods().length == 1, "temporal nest metadata");

    DefaultYoutubeTrackDetails modernVod = details(
        "{\"playabilityStatus\":{\"status\":\"OK\"},\"videoDetails\":{"
            + "\"title\":\"Modern\",\"author\":\"Uploader\",\"lengthSeconds\":\"123\","
            + "\"thumbnail\":{\"thumbnails\":[{\"url\":\"small\"},{\"url\":"
            + "\"https://i.ytimg.com/vi/id/maxresdefault.jpg\"}]}}}", "{}", "modern.js");
    checkInfo(modernVod.getTrackInfo(), "Modern", "Uploader", 123000L, false,
        "https://i.ytimg.com/vi/id/maxresdefault.jpg");
    check(modernVod.getPlayerScript().equals("modern.js"), "modern player script");

    DefaultYoutubeTrackDetails modernLive = details(
        "{\"playabilityStatus\":{\"status\":\"OK\",\"liveStreamability\":{}},"
            + "\"videoDetails\":{\"title\":\"Live\",\"author\":\"Channel\","
            + "\"lengthSeconds\":\"55\"}}", "{}", null);
    checkInfo(modernLive.getTrackInfo(), "Live", "Channel", Long.MAX_VALUE, true,
        "https://i.ytimg.com/vi/" + VIDEO_ID + "/mqdefault.jpg");

    DefaultYoutubeTrackDetails legacyVod = details("{}",
        "{\"status\":\"ok\",\"title\":\"Legacy\",\"author\":\"Old\","
            + "\"length_seconds\":\"7\",\"live_playback\":\"0\"}", "legacy.js");
    checkInfo(legacyVod.getTrackInfo(), "Legacy", "Old", 7000L, false,
        "https://i.ytimg.com/vi/" + VIDEO_ID + "/mqdefault.jpg");
    DefaultYoutubeTrackDetails legacyLive = details("{}",
        "{\"status\":\"ok\",\"title\":\"Legacy Live\",\"author\":\"Old\","
            + "\"length_seconds\":\"0\",\"live_playback\":\"1\"}", null);
    checkInfo(legacyLive.getTrackInfo(), "Legacy Live", "Old", Long.MAX_VALUE, true,
        "https://i.ytimg.com/vi/" + VIDEO_ID + "/mqdefault.jpg");
    assertFriendly(details(
        "{\"playabilityStatus\":{\"status\":\"ERROR\",\"reason\":\"Modern blocked\"}}",
        "{}", null), "Modern blocked");
    assertFriendly(details("{}", "{\"status\":\"fail\",\"reason\":\"Legacy blocked\"}",
        null), "Legacy blocked");

    DefaultYoutubeTrackDetails formats = details(
        "{\"playabilityStatus\":{\"status\":\"OK\"},\"videoDetails\":{\"isLive\":false},"
            + "\"streamingData\":{\"adaptiveFormats\":[{\"url\":"
            + "\"https://media.example/audio?n=token\",\"mimeType\":"
            + "\"audio/webm; codecs=\\\"opus\\\"\",\"bitrate\":128000,"
            + "\"contentLength\":\"42\",\"audioChannels\":2}]}}", "{}", "format.js");
    if (args[0].equals("candidate")) {
      try {
        formats.getFormats(null, null);
        throw new AssertionError("legacy format extraction unexpectedly succeeded");
      } catch (UnsupportedOperationException error) {
        check(error.getMessage().contains("Legacy YouTube format extraction is unsupported"),
            "stable format disposition");
      }
      check(videoId.get(formats) == VIDEO_ID && data.get(formats) != null,
          "format failure preserves state");
      System.out.println("common=public-concrete,object-root,track-details-interface,4-private-fields,"
          + "1-constructor,7-declared-methods,temporal-nest;constructor-identity,player-script,"
          + "generic-formats,modern-vod-live-error,legacy-vod-live-error,thumbnail-duration-uri,"
          + "reflection;service=deterministic-no-network,current-bounded-native-source");
    } else {
      check(args[0].equals("reference"), "unknown mode");
      List<YoutubeTrackFormat> found = formats.getFormats(null, null);
      check(found.size() == 1 && found.get(0).getBitrate() == 128000L
          && found.get(0).getContentLength() == 42L && found.get(0).getAudioChannels() == 2L,
          "reference streaming format");
      System.out.println("common=public-concrete,object-root,track-details-interface,4-private-fields,"
          + "1-constructor,7-declared-methods,temporal-nest;constructor-identity,player-script,"
          + "generic-formats,modern-vod-live-error,legacy-vod-live-error,thumbnail-duration-uri,"
          + "reflection;service=legacy-four-extractor-chain,streaming-format");
    }
  }

  private static DefaultYoutubeTrackDetails details(String player, String polymer, String script) {
    return new DefaultYoutubeTrackDetails(VIDEO_ID, jsonData(player, polymer, script));
  }

  private static YoutubeTrackJsonData jsonData(String player, String polymer, String script) {
    try {
      return new YoutubeTrackJsonData(JsonBrowser.parse(player), JsonBrowser.parse(polymer), script);
    } catch (Exception error) {
      throw new AssertionError(error);
    }
  }

  private static void checkInfo(AudioTrackInfo info, String title, String author, long length,
      boolean stream, String artwork) {
    check(eq(info.title, title) && eq(info.author, author) && info.length == length
        && eq(info.identifier, VIDEO_ID) && info.isStream == stream
        && eq(info.uri, "https://www.youtube.com/watch?v=" + VIDEO_ID)
        && eq(info.artworkUrl, artwork) && info.isrc == null, "track info");
  }

  private static void assertFriendly(DefaultYoutubeTrackDetails details, String message) {
    try {
      details.getTrackInfo();
      throw new AssertionError("track-info failure unexpectedly succeeded");
    } catch (FriendlyException error) {
      check(eq(error.getMessage(), message)
          && error.severity == FriendlyException.Severity.COMMON && error.getCause() == null,
          "friendly failure");
    }
  }

  private static boolean eq(Object left, Object right) {
    return left == null ? right == null : left.equals(right);
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const DEFAULT_YOUTUBE_TRACK_DETAILS_LOADER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.youtube.DefaultYoutubeTrackDetailsLoader;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeTrackDetailsLoader;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeTrackJsonData;
import com.sedmelluq.discord.lavaplayer.tools.FriendlyException;
import com.sedmelluq.discord.lavaplayer.tools.JsonBrowser;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import java.io.IOException;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;
import java.util.Set;
import java.util.TreeSet;

public final class GateDefaultYoutubeTrackDetailsLoader {
  private static final class Probe extends DefaultYoutubeTrackDetailsLoader {
    String status(String json, boolean second) {
      return checkPlayabilityStatus(parse(json), second).name();
    }

    String reason(String json) {
      return getUnplayableReason(parse(json));
    }

    Object base(JsonBrowser json) throws IOException {
      return loadBaseResponse(json, null, null, null);
    }

    Object innertube() throws IOException {
      return loadTrackInfoFromInnertube(null, null, null, null);
    }

    Object augment() throws IOException {
      return augmentWithPlayerScript(null, null, null, false);
    }
  }

  public static void main(String[] args) throws Exception {
    check(args.length == 1, "mode required");
    Class<DefaultYoutubeTrackDetailsLoader> type = DefaultYoutubeTrackDetailsLoader.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {YoutubeTrackDetailsLoader.class})
        && type.getDeclaredFields().length == 2 && type.getDeclaredConstructors().length == 1
        && type.getDeclaredMethods().length == 9 && type.getDeclaredClasses().length == 2,
        "class shape");
    Field log = type.getDeclaredField("log");
    Field cached = type.getDeclaredField("cachedPlayerScript");
    check(log.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL)
        && cached.getModifiers() == (Modifier.PRIVATE | Modifier.VOLATILE), "field metadata");
    log.setAccessible(true);
    cached.setAccessible(true);
    check(log.get(null) != null, "logger initialization");
    Probe probe = new Probe();
    check(cached.get(probe) == null, "empty initial cache");

    Set<String> nested = new TreeSet<>();
    for (Class<?> child : type.getDeclaredClasses()) nested.add(child.getSimpleName());
    check(nested.equals(new TreeSet<>(Arrays.asList("CachedPlayerScript", "InfoStatus"))),
        "nested declarations");
    check(type.getDeclaredConstructor().getModifiers() == Modifier.PUBLIC,
        "constructor metadata");
    Method loadDetails = type.getDeclaredMethod("loadDetails", HttpInterface.class, String.class,
        boolean.class, YoutubeAudioSourceManager.class);
    Method load = type.getDeclaredMethod("load", HttpInterface.class, String.class, boolean.class,
        YoutubeAudioSourceManager.class);
    Method base = type.getDeclaredMethod("loadBaseResponse", JsonBrowser.class, HttpInterface.class,
        String.class, YoutubeAudioSourceManager.class);
    Class<?> statusType = Arrays.stream(type.getDeclaredClasses())
        .filter(child -> child.getSimpleName().equals("InfoStatus")).findFirst().orElseThrow();
    Method status = type.getDeclaredMethod("checkPlayabilityStatus", JsonBrowser.class, boolean.class);
    Method reason = type.getDeclaredMethod("getUnplayableReason", JsonBrowser.class);
    Method innertube = type.getDeclaredMethod("loadTrackInfoFromInnertube", HttpInterface.class,
        String.class, YoutubeAudioSourceManager.class, statusType);
    Method augment = type.getDeclaredMethod("augmentWithPlayerScript", YoutubeTrackJsonData.class,
        HttpInterface.class, String.class, boolean.class);
    Method fetch = type.getDeclaredMethod("fetchScript", String.class, HttpInterface.class);
    Method lambda = type.getDeclaredMethod("lambda$getUnplayableReason$0", StringBuilder.class,
        JsonBrowser.class);
    check(loadDetails.getModifiers() == Modifier.PUBLIC
        && load.getModifiers() == Modifier.PRIVATE
        && base.getModifiers() == Modifier.PROTECTED
        && status.getModifiers() == Modifier.PROTECTED
        && reason.getModifiers() == Modifier.PROTECTED
        && innertube.getModifiers() == Modifier.PROTECTED
        && augment.getModifiers() == Modifier.PROTECTED
        && fetch.getModifiers() == Modifier.PRIVATE
        && lambda.getModifiers() == (Modifier.PRIVATE | Modifier.STATIC | 0x1000)
        && lambda.isSynthetic(), "method modifiers");
    check(Arrays.equals(load.getExceptionTypes(), new Class<?>[] {IOException.class})
        && Arrays.equals(base.getExceptionTypes(), new Class<?>[] {IOException.class})
        && Arrays.equals(innertube.getExceptionTypes(), new Class<?>[] {IOException.class})
        && Arrays.equals(augment.getExceptionTypes(), new Class<?>[] {IOException.class})
        && Arrays.equals(fetch.getExceptionTypes(), new Class<?>[] {IOException.class})
        && loadDetails.getExceptionTypes().length == 0 && status.getExceptionTypes().length == 0
        && reason.getExceptionTypes().length == 0, "exception metadata");
    lambda.setAccessible(true);
    StringBuilder joined = new StringBuilder();
    lambda.invoke(null, joined, parse("{\"text\":\"part\"}"));
    check(joined.toString().equals("part\n"), "synthetic reason helper");

    check(probe.status("{\"playabilityStatus\":{\"status\":\"OK\"}}", false)
        .equals("INFO_PRESENT"), "OK status");
    check(probe.status("{\"playabilityStatus\":{\"status\":\"ERROR\",\"reason\":"
        + "\"This video is unavailable in this region\"}}", false).equals("DOES_NOT_EXIST"),
        "missing status");
    check(probe.status("{\"playabilityStatus\":{\"status\":\"UNPLAYABLE\",\"reason\":"
        + "\"Playback on other websites has been disabled by the video owner\"}}", false)
        .equals("NON_EMBEDDABLE"), "embed status");
    check(probe.status("{\"playabilityStatus\":{\"status\":\"LOGIN_REQUIRED\",\"reason\":"
        + "\"This video may be inappropriate for some users\"}}", false)
        .equals("REQUIRES_LOGIN"), "first login status");
    check(probe.status("{\"playabilityStatus\":{\"status\":\"LIVE_STREAM_OFFLINE\","
        + "\"errorScreen\":{\"ypcTrailerRenderer\":{}}}}", false)
        .equals("PREMIERE_TRAILER"), "premiere status");
    assertRuntime(() -> probe.status("{}", false), "No playability status block.");
    assertRuntime(() -> probe.status("{\"playabilityStatus\":{}}", false),
        "No playability status field.");
    assertFriendly(() -> probe.status("{\"playabilityStatus\":{\"status\":\"ERROR\","
        + "\"reason\":\"Blocked\"}}", false), "Blocked", FriendlyException.Severity.COMMON,
        null);
    assertFriendly(() -> probe.status("{\"playabilityStatus\":{\"status\":\"UNPLAYABLE\","
        + "\"reason\":\"fallback\",\"errorScreen\":{\"playerErrorMessageRenderer\":{"
        + "\"subreason\":{\"simpleText\":\"Detailed\"}}}}}", false), "Detailed",
        FriendlyException.Severity.COMMON, null);
    assertFriendly(() -> probe.status("{\"playabilityStatus\":{\"status\":\"LOGIN_REQUIRED\","
        + "\"reason\":\"This video is private for this account\"}}", false),
        "This is a private video.", FriendlyException.Severity.COMMON, null);
    assertFriendly(() -> probe.status("{\"playabilityStatus\":{\"status\":\"LOGIN_REQUIRED\","
        + "\"reason\":\"This video may be inappropriate for some users\"}}", true),
        "This video requires age verification.", FriendlyException.Severity.SUSPICIOUS,
        IllegalStateException.class);
    assertFriendly(() -> probe.status("{\"playabilityStatus\":{\"status\":"
        + "\"CONTENT_CHECK_REQUIRED\",\"reason\":\"Check content\"}}", false),
        "Check content", FriendlyException.Severity.COMMON, null);
    assertFriendly(() -> probe.status("{\"playabilityStatus\":{\"status\":"
        + "\"LIVE_STREAM_OFFLINE\",\"reason\":\"Offline\"}}", false), "Offline",
        FriendlyException.Severity.COMMON, null);
    assertFriendly(() -> probe.status("{\"playabilityStatus\":{\"status\":\"MYSTERY\"}}",
        false), "This video cannot be viewed anonymously.", FriendlyException.Severity.COMMON, null);

    check(probe.reason("{\"reason\":\"Fallback\"}").equals("Fallback"), "fallback reason");
    check(probe.reason("{\"reason\":\"Fallback\",\"errorScreen\":{"
        + "\"playerErrorMessageRenderer\":{\"subreason\":{\"simpleText\":\"Simple\"}}}}")
        .equals("Simple"), "simple reason");
    check(probe.reason("{\"reason\":\"Fallback\",\"errorScreen\":{"
        + "\"playerErrorMessageRenderer\":{\"subreason\":{\"runs\":[{\"text\":\"One\"},"
        + "{\"text\":\"Two\"}]}}}}" ).equals("One\nTwo\n"), "runs reason");

    if (args[0].equals("candidate")) {
      assertUnsupported(() -> probe.loadDetails(null, null, false, null));
      assertUnsupported(() -> probe.base(null));
      assertUnsupported(probe::innertube);
      assertUnsupported(probe::augment);
      check(cached.get(probe) == null, "service failures preserve cache");
      System.out.println("common=public-concrete,object-root,details-loader-interface,2-private-fields,"
          + "1-constructor,9-declared-methods,2-nested-declarations;constructor-empty-cache,"
          + "playability-matrix,reason-fallback-simple-runs,synthetic-helper,exceptions,reflection;"
          + "service=deterministic-no-network,current-bounded-native-source");
    } else {
      check(args[0].equals("reference"), "unknown mode");
      System.out.println("common=public-concrete,object-root,details-loader-interface,2-private-fields,"
          + "1-constructor,9-declared-methods,2-nested-declarations;constructor-empty-cache,"
          + "playability-matrix,reason-fallback-simple-runs,synthetic-helper,exceptions,reflection;"
          + "service=legacy-innertube-embed-player-script-cache");
    }
  }

  private interface CheckedCall {
    Object run() throws Exception;
  }

  private static JsonBrowser parse(String json) {
    try {
      return JsonBrowser.parse(json);
    } catch (Exception error) {
      throw new AssertionError(error);
    }
  }

  private static void assertRuntime(CheckedCall call, String message) {
    try {
      call.run();
      throw new AssertionError("runtime failure unexpectedly succeeded");
    } catch (RuntimeException error) {
      check(eq(error.getMessage(), message), "runtime message");
    } catch (Exception error) {
      throw new AssertionError(error);
    }
  }

  private static void assertFriendly(CheckedCall call, String message,
      FriendlyException.Severity severity, Class<?> cause) {
    try {
      call.run();
      throw new AssertionError("friendly failure unexpectedly succeeded");
    } catch (FriendlyException error) {
      check(eq(error.getMessage(), message) && error.severity == severity
          && (cause == null ? error.getCause() == null : cause.isInstance(error.getCause())),
          "friendly failure");
    } catch (Exception error) {
      throw new AssertionError(error);
    }
  }

  private static void assertUnsupported(CheckedCall call) {
    try {
      call.run();
      throw new AssertionError("legacy acquisition unexpectedly succeeded");
    } catch (UnsupportedOperationException error) {
      check(error.getMessage().contains("Legacy YouTube track-detail acquisition is unsupported"),
          "stable service disposition");
    } catch (Exception error) {
      throw new AssertionError(error);
    }
  }

  private static boolean eq(Object left, Object right) {
    return left == null ? right == null : left.equals(right);
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const YOUTUBE_CACHED_PLAYER_SCRIPT_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.youtube.DefaultYoutubeTrackDetailsLoader;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;

public final class GateYoutubeCachedPlayerScript {
  private static final class Probe extends DefaultYoutubeTrackDetailsLoader {
    Object create(String url, long timestamp) {
      return new CachedPlayerScript(url, timestamp);
    }
  }

  public static void main(String[] args) throws Exception {
    Class<?> type = Class.forName("com.sedmelluq.discord.lavaplayer.source.youtube."
        + "DefaultYoutubeTrackDetailsLoader$CachedPlayerScript");
    check(type.getModifiers() == (Modifier.PROTECTED | Modifier.STATIC)
        && type.getSuperclass() == Object.class && type.getInterfaces().length == 0
        && type.getDeclaringClass() == DefaultYoutubeTrackDetailsLoader.class
        && type.getNestHost() == DefaultYoutubeTrackDetailsLoader.class
        && type.isMemberClass() && !type.isSynthetic(), "class metadata");
    check(type.getDeclaredFields().length == 2 && type.getDeclaredMethods().length == 0
        && type.getDeclaredConstructors().length == 1, "class shape");

    Field url = type.getDeclaredField("playerScriptUrl");
    Field timestamp = type.getDeclaredField("timestamp");
    check(url.getType() == String.class && timestamp.getType() == long.class
        && url.getModifiers() == (Modifier.PUBLIC | Modifier.FINAL)
        && timestamp.getModifiers() == (Modifier.PUBLIC | Modifier.FINAL), "field metadata");
    Constructor<?> constructor = type.getDeclaredConstructor(String.class, long.class);
    check(constructor.getModifiers() == Modifier.PUBLIC
        && constructor.getExceptionTypes().length == 0, "constructor metadata");

    Probe probe = new Probe();
    String identity = new String("https://player.example/base.js");
    Object positive = probe.create(identity, Long.MAX_VALUE);
    Object negative = probe.create(null, Long.MIN_VALUE);
    check(url.get(positive) == identity && timestamp.getLong(positive) == Long.MAX_VALUE,
        "positive field capture");
    check(url.get(negative) == null && timestamp.getLong(negative) == Long.MIN_VALUE,
        "null and negative field capture");
    check(!positive.equals(probe.create(identity, Long.MAX_VALUE))
        && positive.hashCode() == System.identityHashCode(positive)
        && positive.toString().startsWith(type.getName() + "@"), "object identity semantics");

    System.out.println("shape=protected-static-member,object-root,2-public-final-fields,"
        + "1-public-constructor,0-methods;capture=reference,null,long-extremes;identity=object");
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const YOUTUBE_CACHED_AUTH_SCRIPT_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeAccessTokenTracker;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;

public final class GateYoutubeCachedAuthScript {
  private static final class Probe extends YoutubeAccessTokenTracker {
    Probe() {
      super(null, null, null);
    }

    Object create(String clientId, String clientSecret) {
      return new CachedAuthScript(clientId, clientSecret);
    }
  }

  public static void main(String[] args) throws Exception {
    Class<?> type = Class.forName("com.sedmelluq.discord.lavaplayer.source.youtube."
        + "YoutubeAccessTokenTracker$CachedAuthScript");
    check(type.getModifiers() == (Modifier.PROTECTED | Modifier.STATIC)
        && type.getSuperclass() == Object.class && type.getInterfaces().length == 0
        && type.getDeclaringClass() == YoutubeAccessTokenTracker.class
        && type.getNestHost() == YoutubeAccessTokenTracker.class
        && type.isMemberClass() && !type.isSynthetic(), "class metadata");
    check(type.getDeclaredFields().length == 2 && type.getDeclaredMethods().length == 0
        && type.getDeclaredConstructors().length == 1, "class shape");

    Field clientId = type.getDeclaredField("clientId");
    Field clientSecret = type.getDeclaredField("clientSecret");
    check(clientId.getType() == String.class && clientSecret.getType() == String.class
        && clientId.getModifiers() == (Modifier.PUBLIC | Modifier.FINAL)
        && clientSecret.getModifiers() == (Modifier.PUBLIC | Modifier.FINAL), "field metadata");
    Constructor<?> constructor = type.getDeclaredConstructor(String.class, String.class);
    check(constructor.getModifiers() == Modifier.PUBLIC
        && constructor.getExceptionTypes().length == 0, "constructor metadata");

    Probe probe = new Probe();
    String id = new String("client-id");
    String secret = new String("client-secret");
    Object populated = probe.create(id, secret);
    Object empty = probe.create(null, null);
    check(clientId.get(populated) == id && clientSecret.get(populated) == secret,
        "reference field capture");
    check(clientId.get(empty) == null && clientSecret.get(empty) == null,
        "null field capture");
    check(!populated.equals(probe.create(id, secret))
        && populated.hashCode() == System.identityHashCode(populated)
        && populated.toString().startsWith(type.getName() + "@"), "object identity semantics");

    System.out.println("shape=protected-static-member,object-root,2-public-final-fields,"
        + "1-public-constructor,0-methods;capture=client-id,client-secret,null,reference;"
        + "identity=object");
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const YOUTUBE_AUDIO_SOURCE_MANAGER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.player.AudioPlayerManager;
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeAccessTokenTracker;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeAudioSourceManager;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeAudioTrack;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeLinkRouter;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeMixLoader;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubePlaylistLoader;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeSearchMusicResultLoader;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeSearchResultLoader;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeSignatureResolver;
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeTrackDetailsLoader;
import com.sedmelluq.discord.lavaplayer.tools.FriendlyException;
import com.sedmelluq.discord.lavaplayer.tools.http.ExtendedHttpConfigurable;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpConfigurable;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterfaceManager;
import com.sedmelluq.discord.lavaplayer.track.AudioItem;
import com.sedmelluq.discord.lavaplayer.track.AudioReference;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo;
import java.io.DataInput;
import java.io.DataOutput;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.function.Consumer;
import java.util.function.Function;
import org.apache.http.client.config.RequestConfig;
import org.apache.http.impl.client.HttpClientBuilder;

public final class GateYoutubeAudioSourceManager {
  public static void main(String[] args) throws Exception {
    check(args.length >= 1 && args.length <= 2, "expected disposition and optional native path");
    boolean reference = args[0].equals("reference");
    check(reference || args[0].equals("candidate"), "unknown disposition");
    reflectionContract();
    commonBehavior();
    if (reference) legacyDisposition(); else currentDisposition(args[1]);
    System.out.println(
        "common=public-deprecated,audio-source-http-configurable,13-fields,3-constructors,"
        + "20-methods,1-nested;construction,collaborator-identity,source-name,empty-details,"
        + "track-reconstruction,playlist-pages,http-config,access-tracker,shutdown,reflection;"
        + "service=" + (reference
            ? "legacy-link-router,retry,credential-bootstrap,track-details-http"
            : "current-native-bounded-routing,no-legacy-credential-bootstrap"));
  }

  private static void reflectionContract() throws Exception {
    Class<YoutubeAudioSourceManager> type = YoutubeAudioSourceManager.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && Arrays.equals(type.getInterfaces(), new Class<?>[] {
            AudioSourceManager.class, HttpConfigurable.class})
        && type.isAnnotationPresent(Deprecated.class) && !type.isSynthetic(), "class metadata");
    check(type.getDeclaredFields().length == 13 && type.getDeclaredConstructors().length == 3
        && type.getDeclaredMethods().length == 20 && type.getDeclaredClasses().length == 1,
        "private shape");
    check(type.getDeclaredClasses()[0].getSimpleName().equals("LoadingRoutes"), "nested type");

    checkField("log", Class.forName("org.slf4j.Logger"),
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField("signatureResolver", YoutubeSignatureResolver.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField("httpInterfaceManager", HttpInterfaceManager.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField("combinedHttpConfiguration", ExtendedHttpConfigurable.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField("mixLoader", YoutubeMixLoader.class, Modifier.PRIVATE | Modifier.FINAL);
    checkField("accessTokenTracker", YoutubeAccessTokenTracker.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField("allowSearch", boolean.class, Modifier.PRIVATE | Modifier.FINAL);
    checkField("trackDetailsLoader", YoutubeTrackDetailsLoader.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField("searchResultLoader", YoutubeSearchResultLoader.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField("searchMusicResultLoader", YoutubeSearchMusicResultLoader.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField("playlistLoader", YoutubePlaylistLoader.class, Modifier.PRIVATE | Modifier.FINAL);
    checkField("linkRouter", YoutubeLinkRouter.class, Modifier.PRIVATE | Modifier.FINAL);
    checkField("loadingRoutes", type.getDeclaredClasses()[0], Modifier.PRIVATE | Modifier.FINAL);

    checkConstructor(type.getDeclaredConstructor(), new Class<?>[0]);
    checkConstructor(type.getDeclaredConstructor(boolean.class, String.class, String.class),
        new Class<?>[] {boolean.class, String.class, String.class});
    Class<?>[] collaborators = new Class<?>[] {boolean.class, String.class, String.class,
        YoutubeTrackDetailsLoader.class, YoutubeSearchResultLoader.class,
        YoutubeSearchMusicResultLoader.class, YoutubeSignatureResolver.class,
        YoutubePlaylistLoader.class, YoutubeLinkRouter.class, YoutubeMixLoader.class};
    checkConstructor(type.getDeclaredConstructor(collaborators), collaborators);
    checkMethod("getTrackDetailsLoader", YoutubeTrackDetailsLoader.class, new Class<?>[0]);
    checkMethod("getSignatureResolver", YoutubeSignatureResolver.class, new Class<?>[0]);
    checkMethod("setPlaylistPageCount", void.class, new Class<?>[] {int.class});
    checkMethod("getSourceName", String.class, new Class<?>[0]);
    checkMethod("loadItem", AudioItem.class,
        new Class<?>[] {AudioPlayerManager.class, AudioReference.class});
    checkMethod("isTrackEncodable", boolean.class, new Class<?>[] {AudioTrack.class});
    checkMethod("encodeTrack", void.class, new Class<?>[] {AudioTrack.class, DataOutput.class});
    checkMethod("decodeTrack", AudioTrack.class,
        new Class<?>[] {AudioTrackInfo.class, DataInput.class});
    checkMethod("shutdown", void.class, new Class<?>[0]);
    checkMethod("getAccessTokenTracker", YoutubeAccessTokenTracker.class, new Class<?>[0]);
    checkMethod("getHttpInterface", HttpInterface.class, new Class<?>[0]);
    checkMethod("configureRequests", void.class, new Class<?>[] {Function.class});
    checkMethod("configureBuilder", void.class, new Class<?>[] {Consumer.class});
    checkMethod("getHttpConfiguration", ExtendedHttpConfigurable.class, new Class<?>[0]);
    checkMethod("getMainHttpConfiguration", ExtendedHttpConfigurable.class, new Class<?>[0]);
    checkMethod("getSearchHttpConfiguration", ExtendedHttpConfigurable.class, new Class<?>[0]);
    checkMethod("getSearchMusicHttpConfiguration", ExtendedHttpConfigurable.class,
        new Class<?>[0]);
    checkMethod("loadTrackWithVideoId", AudioItem.class,
        new Class<?>[] {String.class, boolean.class});
    check(type.getDeclaredMethod("configureRequests", Function.class).toGenericString().contains(
            "Function<org.apache.http.client.config.RequestConfig,"
                + " org.apache.http.client.config.RequestConfig>")
        && type.getDeclaredMethod("configureBuilder", Consumer.class).toGenericString().contains(
            "Consumer<org.apache.http.impl.client.HttpClientBuilder>"),
        "generic configuration signatures");
  }

  private static void commonBehavior() throws Exception {
    YoutubeAudioSourceManager defaults = new YoutubeAudioSourceManager();
    check((boolean) field("allowSearch").get(defaults)
        && field("trackDetailsLoader").get(defaults).getClass().getSimpleName()
            .equals("DefaultYoutubeTrackDetailsLoader")
        && field("searchResultLoader").get(defaults).getClass().getSimpleName()
            .equals("YoutubeSearchProvider")
        && field("searchMusicResultLoader").get(defaults).getClass().getSimpleName()
            .equals("YoutubeSearchMusicProvider")
        && field("signatureResolver").get(defaults).getClass().getSimpleName()
            .equals("YoutubeSignatureCipherManager")
        && field("playlistLoader").get(defaults).getClass().getSimpleName()
            .equals("DefaultYoutubePlaylistLoader")
        && field("linkRouter").get(defaults).getClass().getSimpleName()
            .equals("DefaultYoutubeLinkRouter")
        && field("mixLoader").get(defaults).getClass().getSimpleName()
            .equals("YoutubeMixProvider"), "default collaborators");
    defaults.shutdown();

    Fixture fixture = new Fixture(true);
    YoutubeAudioSourceManager manager = fixture.manager;
    check((boolean) field("allowSearch").get(manager)
        && manager.getTrackDetailsLoader() == fixture.details
        && manager.getSignatureResolver() == fixture.signature
        && field("searchResultLoader").get(manager) == fixture.search
        && field("searchMusicResultLoader").get(manager) == fixture.music
        && field("playlistLoader").get(manager) == fixture.playlist
        && field("linkRouter").get(manager) == fixture.router
        && field("mixLoader").get(manager) == fixture.mix, "collaborator identity");
    check(manager.getSourceName().equals("youtube") && manager.isTrackEncodable(null),
        "source and encodability");
    manager.encodeTrack(null, rejectingProxy(DataOutput.class));
    AudioTrackInfo info = new AudioTrackInfo("title", "author", 1234L, "dQw4w9WgXcQ", false,
        "https://youtu.be/dQw4w9WgXcQ", "art", null);
    AudioTrack decoded = manager.decodeTrack(info, rejectingProxy(DataInput.class));
    check(decoded instanceof YoutubeAudioTrack && decoded.getInfo() == info
        && decoded.getSourceManager() == manager, "empty-detail track reconstruction");
    manager.setPlaylistPageCount(Integer.MIN_VALUE);
    check(fixture.playlistHandler.pageCount == Integer.MIN_VALUE, "playlist page delegation");

    check(manager.getAccessTokenTracker() != null
        && field("httpInterfaceManager").get(manager)
            == trackerField("httpInterfaceManager").get(manager.getAccessTokenTracker()),
        "access tracker ownership");
    check(manager.getHttpConfiguration() != null
        && manager.getHttpConfiguration() == manager.getHttpConfiguration()
        && manager.getMainHttpConfiguration() instanceof HttpInterfaceManager
        && manager.getMainHttpConfiguration() != manager.getHttpConfiguration()
        && manager.getSearchHttpConfiguration() == fixture.searchConfiguration
        && manager.getSearchMusicHttpConfiguration() == fixture.musicConfiguration,
        "configuration identity");
    check(fixture.searchHandler.configurationCalls == 2
        && fixture.musicHandler.configurationCalls == 2, "configuration getter dispatch");
    Function<RequestConfig, RequestConfig> requests = value -> value;
    Consumer<HttpClientBuilder> builder = value -> {};
    manager.configureRequests(requests);
    manager.configureBuilder(builder);
    check(fixture.configurationOrder.equals(Arrays.asList(
            "search:requests", "music:requests", "search:builder", "music:builder"))
        && fixture.searchConfig.requests == requests && fixture.searchConfig.builder == builder
        && fixture.musicConfig.requests == requests && fixture.musicConfig.builder == builder,
        "configuration delegation identity and order");
    try (HttpInterface http = manager.getHttpInterface()) {
      check(http != null, "HTTP interface creation");
    }
    manager.shutdown();
    manager.shutdown();
  }

  private static void legacyDisposition() throws Exception {
    Fixture fixture = new Fixture(true);
    AudioTrack sentinel = proxy(AudioTrack.class,
        (instance, method, arguments) -> defaultValue(method.getReturnType()));
    fixture.routerHandler.result = sentinel;
    AudioReference reference = new AudioReference("legacy-route", null);
    check(fixture.manager.loadItem(null, reference) == sentinel
        && fixture.routerHandler.identifier.equals("legacy-route")
        && fixture.routerHandler.routes != null, "legacy router delegation");
    check(fixture.manager.loadTrackWithVideoId("missing-id", false) == AudioReference.NO_TRACK,
        "legacy missing track sentinel");
    FriendlyException unavailable = expect(FriendlyException.class,
        () -> fixture.manager.loadTrackWithVideoId("missing-id", true));
    check(unavailable.getMessage().equals("Video unavailable"), "legacy required track failure");
    fixture.manager.shutdown();
  }

  private static void currentDisposition(String nativeLibrary) throws Exception {
    Class.forName("dev.mantle.internal.NativeLoader")
        .getMethod("load", String.class).invoke(null, nativeLibrary);
    Class<?> nativeType = Class.forName("dev.mantle.internal.MantleNative");
    Method load = nativeType.getDeclaredMethod("loadYoutubeItem",
        YoutubeAudioSourceManager.class, AudioReference.class);
    check(Modifier.isPublic(load.getModifiers()) && Modifier.isStatic(load.getModifiers())
        && Modifier.isNative(load.getModifiers()), "current native route");
    Fixture fixture = new Fixture(true);
    check(fixture.manager.loadItem(null,
        new AudioReference("https://example.invalid/not-youtube", null)) == null,
        "foreign route rejected before service traffic");
    check(fixture.manager.loadTrackWithVideoId("not-a-video-id", false) == null,
        "invalid direct video rejected before service traffic");
    fixture.manager.shutdown();
    Fixture disabled = new Fixture(false);
    check(disabled.manager.loadItem(null, new AudioReference("ytsearch:fixture", null)) == null,
        "search flag enforced before service traffic");
    disabled.manager.shutdown();
    Fixture credentials = new Fixture(true, "person@example.invalid", "secret-value");
    UnsupportedOperationException fenced = expect(UnsupportedOperationException.class,
        () -> credentials.manager.getAccessTokenTracker().updateMasterToken());
    check(fenced.getMessage().contains("Legacy YouTube JVM credential"),
        "legacy credential acquisition fenced");
    credentials.manager.shutdown();
  }

  private static Field field(String name) throws Exception {
    Field field = YoutubeAudioSourceManager.class.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }

  private static Field trackerField(String name) throws Exception {
    Field field = YoutubeAccessTokenTracker.class.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }

  private static void checkField(String name, Class<?> type, int modifiers) throws Exception {
    Field field = YoutubeAudioSourceManager.class.getDeclaredField(name);
    check(field.getType() == type && field.getGenericType() == type
        && field.getModifiers() == modifiers && !field.isSynthetic(), name + " metadata");
  }

  private static void checkConstructor(Constructor<?> constructor, Class<?>[] parameters) {
    check(constructor.getModifiers() == Modifier.PUBLIC
        && Arrays.equals(constructor.getParameterTypes(), parameters)
        && constructor.getExceptionTypes().length == 0 && !constructor.isSynthetic()
        && !constructor.isVarArgs() && constructor.isAnnotationPresent(Deprecated.class),
        constructor + " metadata");
  }

  private static void checkMethod(String name, Class<?> returnType, Class<?>[] parameters)
      throws Exception {
    Method method = YoutubeAudioSourceManager.class.getDeclaredMethod(name, parameters);
    check(method.getReturnType() == returnType && method.getModifiers() == Modifier.PUBLIC
        && method.getExceptionTypes().length == 0 && method.getTypeParameters().length == 0
        && !method.isBridge() && !method.isSynthetic() && !method.isVarArgs(),
        method + " metadata");
  }

  private static final class Fixture {
    final List<String> configurationOrder = new ArrayList<>();
    final ObjectHandler detailsHandler = new ObjectHandler();
    final YoutubeTrackDetailsLoader details = proxy(YoutubeTrackDetailsLoader.class, detailsHandler);
    final ConfigHandler searchConfig = new ConfigHandler("search", configurationOrder);
    final ConfigHandler musicConfig = new ConfigHandler("music", configurationOrder);
    final ExtendedHttpConfigurable searchConfiguration =
        proxy(ExtendedHttpConfigurable.class, searchConfig);
    final ExtendedHttpConfigurable musicConfiguration =
        proxy(ExtendedHttpConfigurable.class, musicConfig);
    final LoaderHandler searchHandler = new LoaderHandler(searchConfiguration);
    final LoaderHandler musicHandler = new LoaderHandler(musicConfiguration);
    final YoutubeSearchResultLoader search = proxy(YoutubeSearchResultLoader.class, searchHandler);
    final YoutubeSearchMusicResultLoader music =
        proxy(YoutubeSearchMusicResultLoader.class, musicHandler);
    final YoutubeSignatureResolver signature = rejectingProxy(YoutubeSignatureResolver.class);
    final PlaylistHandler playlistHandler = new PlaylistHandler();
    final YoutubePlaylistLoader playlist = proxy(YoutubePlaylistLoader.class, playlistHandler);
    final RouterHandler routerHandler = new RouterHandler();
    final YoutubeLinkRouter router = proxy(YoutubeLinkRouter.class, routerHandler);
    final YoutubeMixLoader mix = rejectingProxy(YoutubeMixLoader.class);
    final YoutubeAudioSourceManager manager;

    Fixture(boolean allowSearch) { this(allowSearch, null, null); }

    Fixture(boolean allowSearch, String email, String password) {
      manager = new YoutubeAudioSourceManager(allowSearch, email, password, details, search, music,
          signature, playlist, router, mix);
    }
  }

  private static class ObjectHandler implements InvocationHandler {
    public Object invoke(Object instance, Method method, Object[] arguments) {
      if (method.getName().equals("toString")) return "YoutubeFixture:" + method.getDeclaringClass();
      return defaultValue(method.getReturnType());
    }
  }

  private static final class LoaderHandler extends ObjectHandler {
    final ExtendedHttpConfigurable configuration;
    int configurationCalls;
    LoaderHandler(ExtendedHttpConfigurable configuration) { this.configuration = configuration; }
    public Object invoke(Object instance, Method method, Object[] arguments) {
      if (method.getName().equals("getHttpConfiguration")) {
        configurationCalls++;
        return configuration;
      }
      return super.invoke(instance, method, arguments);
    }
  }

  private static final class PlaylistHandler extends ObjectHandler {
    int pageCount;
    public Object invoke(Object instance, Method method, Object[] arguments) {
      if (method.getName().equals("setPlaylistPageCount")) {
        pageCount = (Integer) arguments[0];
        return null;
      }
      return super.invoke(instance, method, arguments);
    }
  }

  private static final class RouterHandler extends ObjectHandler {
    String identifier;
    Object routes;
    Object result;
    public Object invoke(Object instance, Method method, Object[] arguments) {
      if (method.getName().equals("route")) {
        identifier = (String) arguments[0];
        routes = arguments[1];
        return result;
      }
      return super.invoke(instance, method, arguments);
    }
  }

  private static final class ConfigHandler implements InvocationHandler {
    final String name;
    final List<String> order;
    Object requests;
    Object builder;
    ConfigHandler(String name, List<String> order) { this.name = name; this.order = order; }
    public Object invoke(Object instance, Method method, Object[] arguments) {
      if (method.getName().equals("configureRequests")) {
        requests = arguments[0];
        order.add(name + ":requests");
      }
      if (method.getName().equals("configureBuilder")) {
        builder = arguments[0];
        order.add(name + ":builder");
      }
      if (method.getName().equals("toString")) return "YoutubeConfigFixture:" + name;
      return defaultValue(method.getReturnType());
    }
  }

  private static <T> T rejectingProxy(Class<T> type) {
    return proxy(type, (instance, method, arguments) -> {
      if (method.getName().equals("toString")) return "RejectingYoutubeFixture:" + type.getName();
      throw new AssertionError(type.getSimpleName() + " unexpectedly invoked: " + method);
    });
  }

  private static <T> T proxy(Class<T> type, InvocationHandler handler) {
    return type.cast(Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] {type}, handler));
  }

  private static Object defaultValue(Class<?> type) {
    if (!type.isPrimitive()) return null;
    if (type == boolean.class) return false;
    if (type == byte.class) return (byte) 0;
    if (type == short.class) return (short) 0;
    if (type == int.class) return 0;
    if (type == long.class) return 0L;
    if (type == float.class) return 0.0f;
    if (type == double.class) return 0.0d;
    if (type == char.class) return (char) 0;
    return null;
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (error instanceof java.lang.reflect.InvocationTargetException) error = error.getCause();
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private interface Operation { void run() throws Exception; }
  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const YOUTUBE_INFO_STATUS_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.youtube.DefaultYoutubeTrackDetailsLoader;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;

public final class GateYoutubeInfoStatus {
  private static final class Probe extends DefaultYoutubeTrackDetailsLoader {
    static Object[] values() {
      return InfoStatus.values();
    }

    static Object valueOf(String name) {
      return InfoStatus.valueOf(name);
    }
  }

  public static void main(String[] args) throws Exception {
    Class<?> type = Class.forName("com.sedmelluq.discord.lavaplayer.source.youtube."
        + "DefaultYoutubeTrackDetailsLoader$InfoStatus");
    check((type.getModifiers() & ~0x4000)
            == (Modifier.PROTECTED | Modifier.STATIC | Modifier.FINAL)
        && type.isEnum() && type.getSuperclass() == Enum.class && type.getInterfaces().length == 0
        && type.getDeclaringClass() == DefaultYoutubeTrackDetailsLoader.class
        && type.getNestHost() == DefaultYoutubeTrackDetailsLoader.class
        && type.isMemberClass() && !type.isSynthetic(), "class metadata");

    Field[] fields = Arrays.stream(type.getDeclaredFields())
        .filter(field -> Modifier.isPublic(field.getModifiers()))
        .toArray(Field[]::new);
    Method values = type.getDeclaredMethod("values");
    Method valueOf = type.getDeclaredMethod("valueOf", String.class);
    Constructor<?> constructor = type.getDeclaredConstructor(String.class, int.class);
    check(fields.length == 7 && type.getDeclaredMethods().length == 2
        && type.getDeclaredConstructors().length == 1, "exported shape");
    check(Arrays.stream(fields).allMatch(field -> field.getType() == type && field.isEnumConstant()
            && (field.getModifiers() & ~0x4000)
                == (Modifier.PUBLIC | Modifier.STATIC | Modifier.FINAL))
        && values.getModifiers() == (Modifier.PUBLIC | Modifier.STATIC)
        && values.getReturnType().isArray() && values.getReturnType().getComponentType() == type
        && values.getParameterCount() == 0
        && valueOf.getModifiers() == (Modifier.PUBLIC | Modifier.STATIC)
        && valueOf.getReturnType() == type
        && Arrays.equals(valueOf.getParameterTypes(), new Class<?>[] { String.class })
        && Modifier.isPrivate(constructor.getModifiers()), "member metadata");

    Object[] constants = Probe.values();
    check(Arrays.toString(constants).equals("[INFO_PRESENT, REQUIRES_LOGIN, DOES_NOT_EXIST, "
        + "CONTENT_CHECK_REQUIRED, LIVE_STREAM_OFFLINE, PREMIERE_TRAILER, NON_EMBEDDABLE]"),
        "constant order");
    for (int index = 0; index < constants.length; index++) {
      Enum<?> value = (Enum<?>) constants[index];
      check(value.ordinal() == index && value.name().equals(value.toString())
          && Probe.valueOf(value.name()) == value && fields[index].get(null) == value,
          "constant identity " + index);
    }
    constants[0] = null;
    check(Probe.values()[0] == Probe.valueOf("INFO_PRESENT"), "values copy");
    check(Arrays.equals(type.getEnumConstants(), Probe.values()), "reflection constants");
    expect(IllegalArgumentException.class, () -> Probe.valueOf("missing"));
    expect(NullPointerException.class, () -> Probe.valueOf(null));

    System.out.println("shape=protected-static-final-enum,enum-root,7-public-constants,"
        + "2-public-static-methods,1-private-constructor;order=INFO_PRESENT,REQUIRES_LOGIN,"
        + "DOES_NOT_EXIST,CONTENT_CHECK_REQUIRED,LIVE_STREAM_OFFLINE,PREMIERE_TRAILER,"
        + "NON_EMBEDDABLE;identity=name,ordinal,field,lookup;copy=true;lookup-errors=iae,npe");
  }

  private static void expect(Class<? extends Throwable> type, Runnable operation) {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
    }
  }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

const YOUTUBE_ACCESS_TOKEN_TRACKER_CONSUMER: &str = r#"
import com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeAccessTokenTracker;
import com.sedmelluq.discord.lavaplayer.tools.io.HttpInterfaceManager;
import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.regex.Pattern;
import org.apache.http.client.protocol.HttpClientContext;

public final class GateYoutubeAccessTokenTracker {
  public static void main(String[] args) throws Exception {
    check(args.length == 1 && (args[0].equals("reference") || args[0].equals("candidate")),
        "expected disposition");
    boolean reference = args[0].equals("reference");
    reflectionContract();
    commonBehavior();
    if (reference) referenceDisposition(); else currentDisposition();
    System.out.println(
        "common=public-concrete,23-fields,1-constructor,8-exported-methods,21-declared-methods,"
        + "dependency-credential-capture,defaults,context-marker,cached-identity,private-shell;service="
        + (reference ? "legacy-email-password-android-tv-visitor-http" :
            "deterministic-no-network,native-auth-owner,cached-only"));
  }

  private static void reflectionContract() throws Exception {
    Class<YoutubeAccessTokenTracker> type = YoutubeAccessTokenTracker.class;
    check(type.getModifiers() == Modifier.PUBLIC && type.getSuperclass() == Object.class
        && type.getInterfaces().length == 0 && !type.isSynthetic(), "class metadata");
    check(type.getDeclaredFields().length == 23 && type.getDeclaredConstructors().length == 1
        && type.getDeclaredMethods().length == 21, "private shape");
    long exported = Arrays.stream(type.getDeclaredMethods())
        .filter(method -> Modifier.isPublic(method.getModifiers())
            || Modifier.isProtected(method.getModifiers()))
        .count();
    check(exported == 7L, "exported method count");

    checkField("log", Class.forName("org.slf4j.Logger"),
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField("AUTH_SCRIPT_REGEX", String.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField("IDENTITY_REGEX", String.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField("authScriptPattern", Pattern.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField("identityPattern", Pattern.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    checkField("TOKEN_FETCH_CONTEXT_ATTRIBUTE", String.class,
        Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    for (String name : new String[] {"MASTER_TOKEN_REFRESH_INTERVAL",
        "DEFAULT_ACCESS_TOKEN_REFRESH_INTERVAL", "VISITOR_ID_REFRESH_INTERVAL"}) {
      checkField(name, long.class, Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
    }
    checkField("tokenLock", Object.class, Modifier.PRIVATE | Modifier.FINAL);
    checkField("httpInterfaceManager", HttpInterfaceManager.class,
        Modifier.PRIVATE | Modifier.FINAL);
    checkField("email", String.class, Modifier.PRIVATE | Modifier.FINAL);
    checkField("password", String.class, Modifier.PRIVATE | Modifier.FINAL);
    for (String name : new String[] {"masterToken", "accessToken", "visitorId"}) {
      checkField(name, String.class, Modifier.PRIVATE);
    }
    for (String name : new String[] {"lastMasterTokenUpdate", "lastAccessTokenUpdate",
        "lastVisitorIdUpdate", "accessTokenRefreshInterval"}) {
      checkField(name, long.class, Modifier.PRIVATE);
    }
    checkField("loggedAgeRestrictionsWarning", boolean.class, Modifier.PRIVATE);
    checkField("masterTokenFromTV", boolean.class, Modifier.PRIVATE);
    checkField("cachedAuthScript", Class.forName(
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeAccessTokenTracker$CachedAuthScript"),
        Modifier.PRIVATE | Modifier.VOLATILE);

    Constructor<?> constructor = type.getDeclaredConstructor(
        HttpInterfaceManager.class, String.class, String.class);
    check(constructor.getModifiers() == Modifier.PUBLIC
        && constructor.getExceptionTypes().length == 0, "constructor metadata");
    checkMethod(type.getDeclaredMethod("updateMasterToken"), void.class, Modifier.PUBLIC);
    checkMethod(type.getDeclaredMethod("updateAccessToken"), void.class, Modifier.PUBLIC);
    checkMethod(type.getDeclaredMethod("updateVisitorId"), String.class, Modifier.PUBLIC);
    checkMethod(type.getDeclaredMethod("getMasterToken"), String.class, Modifier.PUBLIC);
    checkMethod(type.getDeclaredMethod("getAccessToken"), String.class, Modifier.PUBLIC);
    checkMethod(type.getDeclaredMethod("getVisitorId"), String.class, Modifier.PUBLIC);
    checkMethod(type.getDeclaredMethod("isTokenFetchContext", HttpClientContext.class),
        boolean.class, Modifier.PUBLIC);
    checkMethod(type.getDeclaredMethod("fetchMasterToken"), String.class, Modifier.PRIVATE,
        IOException.class);
    checkMethod(type.getDeclaredMethod("waitForAuth",
        Class.forName("com.sedmelluq.discord.lavaplayer.tools.io.HttpInterface"),
        Class.forName("com.sedmelluq.discord.lavaplayer.tools.JsonBrowser"),
        Class.forName("com.sedmelluq.discord.lavaplayer.source.youtube."
            + "YoutubeAccessTokenTracker$CachedAuthScript")), String.class, Modifier.PRIVATE,
        IOException.class, InterruptedException.class);
    checkMethod(type.getDeclaredMethod("lambda$updateMasterToken$0"), void.class,
        Modifier.PRIVATE | 0x1000);

    check(stringField("AUTH_SCRIPT_REGEX").equals(
        "<script id=\"base-js\" src=\"(.*?)\" nonce=\".*?\"></script>"), "auth regex");
    check(stringField("IDENTITY_REGEX").equals("\\{clientId:\"(.+?)\",\\n?.+?:\"(.+?)\""),
        "identity regex");
    check(stringField("TOKEN_FETCH_CONTEXT_ATTRIBUTE").equals("yt-raw"), "context constant");
    check(longField("MASTER_TOKEN_REFRESH_INTERVAL") == 604_800_000L
        && longField("DEFAULT_ACCESS_TOKEN_REFRESH_INTERVAL") == 3_600_000L
        && longField("VISITOR_ID_REFRESH_INTERVAL") == 600_000L, "refresh constants");
    check(((Pattern) field("authScriptPattern").get(null)).pattern().equals(
            stringField("AUTH_SCRIPT_REGEX"))
        && ((Pattern) field("identityPattern").get(null)).pattern().equals(
            stringField("IDENTITY_REGEX")), "compiled patterns");
  }

  private static void commonBehavior() throws Exception {
    AtomicInteger acquisitions = new AtomicInteger();
    HttpInterfaceManager manager = manager(acquisitions);
    String email = new String("person@example.invalid");
    String password = new String("secret-value");
    YoutubeAccessTokenTracker tracker = new YoutubeAccessTokenTracker(manager, email, password);
    check(field("httpInterfaceManager").get(tracker) == manager
        && field("email").get(tracker) == email && field("password").get(tracker) == password
        && field("tokenLock").get(tracker) != null
        && field("masterToken").get(tracker) == null && field("accessToken").get(tracker) == null
        && field("visitorId").get(tracker) == null
        && field("lastMasterTokenUpdate").getLong(tracker) == 0L
        && field("lastAccessTokenUpdate").getLong(tracker) == 0L
        && field("lastVisitorIdUpdate").getLong(tracker) == 0L
        && field("accessTokenRefreshInterval").getLong(tracker) == 3_600_000L
        && !field("loggedAgeRestrictionsWarning").getBoolean(tracker)
        && !field("masterTokenFromTV").getBoolean(tracker)
        && field("cachedAuthScript").get(tracker) == null, "constructor state");

    HttpClientContext context = HttpClientContext.create();
    check(!tracker.isTokenFetchContext(context), "missing context marker");
    context.setAttribute("yt-raw", Boolean.FALSE);
    check(!tracker.isTokenFetchContext(context), "false context marker");
    context.setAttribute("yt-raw", Boolean.TRUE);
    check(tracker.isTokenFetchContext(context), "true identity context marker");
    expect(NullPointerException.class, () -> tracker.isTokenFetchContext(null));

    String master = new String("cached-master");
    String access = new String("cached-access");
    String visitor = new String("cached-visitor");
    field("masterToken").set(tracker, master);
    field("accessToken").set(tracker, access);
    field("visitorId").set(tracker, visitor);
    check(tracker.getMasterToken() == master && tracker.getAccessToken() == access
        && tracker.getVisitorId() == visitor && acquisitions.get() == 0, "cached identity");
  }

  private static void referenceDisposition() throws Exception {
    AtomicInteger acquisitions = new AtomicInteger();
    YoutubeAccessTokenTracker tracker =
        new YoutubeAccessTokenTracker(manager(acquisitions), null, "");
    tracker.updateMasterToken();
    tracker.updateAccessToken();
    check(tracker.getMasterToken() == null && tracker.getAccessToken() == null
        && field("loggedAgeRestrictionsWarning").getBoolean(tracker)
        && acquisitions.get() == 0, "empty credentials stay offline");
  }

  private static void currentDisposition() throws Exception {
    AtomicInteger acquisitions = new AtomicInteger();
    String email = "private@example.invalid";
    String password = "private-password";
    YoutubeAccessTokenTracker tracker =
        new YoutubeAccessTokenTracker(manager(acquisitions), email, password);
    check(tracker.getMasterToken() == null && tracker.getAccessToken() == null
        && tracker.getVisitorId() == null, "empty cache stays empty");
    for (Operation operation : new Operation[] {tracker::updateMasterToken,
        tracker::updateAccessToken, tracker::updateVisitorId}) {
      UnsupportedOperationException error = expect(UnsupportedOperationException.class, operation);
      check(error.getMessage().contains("Mantle native YouTube authentication")
          && !error.getMessage().contains(email) && !error.getMessage().contains(password),
          "bounded unsupported diagnostic");
    }
    Method legacy = YoutubeAccessTokenTracker.class.getDeclaredMethod("fetchMasterToken");
    legacy.setAccessible(true);
    UnsupportedOperationException privateError = expectInvocation(
        UnsupportedOperationException.class, () -> legacy.invoke(tracker));
    check(privateError.getMessage().contains("unsupported")
        && acquisitions.get() == 0
        && field("lastMasterTokenUpdate").getLong(tracker) == 0L
        && field("lastAccessTokenUpdate").getLong(tracker) == 0L
        && field("lastVisitorIdUpdate").getLong(tracker) == 0L,
        "legacy acquisition fenced before state and HTTP");
  }

  private static HttpInterfaceManager manager(AtomicInteger acquisitions) {
    return (HttpInterfaceManager) java.lang.reflect.Proxy.newProxyInstance(
        GateYoutubeAccessTokenTracker.class.getClassLoader(),
        new Class<?>[] {HttpInterfaceManager.class}, (proxy, method, args) -> {
          if (method.getName().equals("getInterface")) acquisitions.incrementAndGet();
          if (method.getName().equals("toString")) return "manager-proxy";
          if (method.getReturnType() == boolean.class) return false;
          if (method.getReturnType() == int.class) return 0;
          if (method.getReturnType() == long.class) return 0L;
          return null;
        });
  }

  private static Field field(String name) throws Exception {
    Field field = YoutubeAccessTokenTracker.class.getDeclaredField(name);
    field.setAccessible(true);
    return field;
  }

  private static String stringField(String name) throws Exception {
    return (String) field(name).get(null);
  }

  private static long longField(String name) throws Exception {
    return field(name).getLong(null);
  }

  private static void checkField(String name, Class<?> type, int modifiers) throws Exception {
    Field field = field(name);
    check(field.getType() == type && field.getModifiers() == modifiers && !field.isSynthetic(),
        field + " metadata");
  }

  private static void checkMethod(Method method, Class<?> returnType, int modifiers,
                                  Class<?>... exceptions) {
    check(method.getReturnType() == returnType && method.getModifiers() == modifiers
        && Arrays.equals(method.getExceptionTypes(), exceptions) && !method.isBridge(),
        method + " metadata");
  }

  private static <T extends Throwable> T expect(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error)) throw new AssertionError("wrong exception", error);
      return type.cast(error);
    }
  }

  private static <T extends Throwable> T expectInvocation(Class<T> type, Operation operation)
      throws Exception {
    try {
      operation.run();
      throw new AssertionError("expected " + type.getName());
    } catch (InvocationTargetException error) {
      if (!type.isInstance(error.getCause())) throw new AssertionError("wrong cause", error);
      return type.cast(error.getCause());
    }
  }

  private interface Operation { void run() throws Exception; }

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
"#;

fn required_path(args: &[String], name: &str) -> Result<PathBuf> {
    Ok(PathBuf::from(required_value(args, name)?))
}

fn optional_path(args: &[String], name: &str) -> Option<PathBuf> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| PathBuf::from(&pair[1]))
}

fn required_value(args: &[String], name: &str) -> Result<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
        .ok_or_else(|| format!("missing required option {name}").into())
}
