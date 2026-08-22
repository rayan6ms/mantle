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
        "write-terminator-audio-frame-consumer" => Some(TERMINATOR_AUDIO_FRAME_CONSUMER),
        "write-reference-mutable-audio-frame-consumer" => {
            Some(REFERENCE_MUTABLE_AUDIO_FRAME_CONSUMER)
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
      throw new AssertionError("track identifier");
    }
    if (starts.get() != 1 || !player.isPaused()) throw new AssertionError("reentrant start callback");
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
