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
    match args.first().map(String::as_str) {
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
        Some("write-smoke-consumer") => {
            let output = required_path(&args, "--output")?;
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(output, SMOKE_CONSUMER)?;
            Ok(())
        }
        Some("write-probe-consumer") => {
            let output = required_path(&args, "--output")?;
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(output, PROBE_CONSUMER)?;
            Ok(())
        }
        Some("write-integration-consumer") => {
            let output = required_path(&args, "--output")?;
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(output, INTEGRATION_CONSUMER)?;
            Ok(())
        }
        Some("write-classloader-consumer") => {
            let output = required_path(&args, "--output")?;
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(output, CLASSLOADER_CONSUMER)?;
            Ok(())
        }
        Some("write-event-consumer") => {
            let output = required_path(&args, "--output")?;
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(output, EVENT_CONSUMER)?;
            Ok(())
        }
        Some("write-track-value-consumer") => {
            let output = required_path(&args, "--output")?;
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(output, TRACK_VALUE_CONSUMER)?;
            Ok(())
        }
        Some("write-track-enum-consumer") => {
            let output = required_path(&args, "--output")?;
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(output, TRACK_ENUM_CONSUMER)?;
            Ok(())
        }
        Some("write-track-contract-consumer") => {
            let output = required_path(&args, "--output")?;
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(output, TRACK_CONTRACT_CONSUMER)?;
            Ok(())
        }
        _ => Err(
            "usage: mantle-jvm-gate <emit|write-smoke-consumer|write-probe-consumer> [options]"
                .into(),
        ),
    }
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

    TrackMarkerHandler handler = state -> { };
    TrackMarker marker = new TrackMarker(987654321L, handler);
    check(marker.timecode == 987654321L && marker.handler == handler, "marker fields");

    System.out.println(
        "reference=identifier,title,container-params,null-defaults;"
        + "info=123456789,true,optional-fields;"
        + "playlist=identity,mutable,true;marker=987654321,identity;"
        + "playlist-contract=AudioItem,4,List<AudioTrack>");
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
