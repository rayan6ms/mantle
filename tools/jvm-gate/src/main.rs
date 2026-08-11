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
import com.sedmelluq.discord.lavaplayer.track.AudioPlaylist;
import com.sedmelluq.discord.lavaplayer.track.AudioTrack;
import com.sedmelluq.discord.lavaplayer.track.TrackMarker;
import com.sedmelluq.discord.lavaplayer.track.TrackMarkerHandler.MarkerState;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrame;
import java.util.Arrays;
import java.util.concurrent.Future;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;

public final class GateIntegration {
  public static void main(String[] args) throws Exception {
    System.load(args[0]);
    DefaultAudioPlayerManager manager = new DefaultAudioPlayerManager();
    AudioPlayer player = manager.createPlayer();
    AtomicReference<AudioTrack> loaded = new AtomicReference<>();
    AtomicInteger starts = new AtomicInteger();
    AtomicInteger markers = new AtomicInteger();
    Object userData = new Object();

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

    Future<Void> completed = manager.loadItem("gate:track", handler);
    if (!completed.isDone() || completed.isCancelled()) throw new AssertionError("future completion");
    if (loaded.get() == null || !"gate:track".equals(loaded.get().getIdentifier())) {
      throw new AssertionError("track identifier");
    }
    if (starts.get() != 1 || !player.isPaused()) throw new AssertionError("reentrant start callback");
    if (markers.get() != 1) throw new AssertionError("marker callback");
    player.setPaused(false);

    byte[] details = manager.encodeTrackDetails(loaded.get());
    byte[] expectedDetails = new byte[] {
        0, 13, 'm', 'a', 'n', 't', 'l', 'e', '-', 'o', 'r', 'a', 'c', 'l', 'e',
        0, 9, 'o', 'r', 'a', 'c', 'l', 'e', '-', 'v', '1'
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

    AudioFrame frame = player.provide();
    if (frame == null || frame.getTimecode() != 0 || frame.getVolume() != 100
        || frame.getDataLength() != 4 || !Arrays.equals(frame.getData(), new byte[] {1, 2, 3, 4})) {
      throw new AssertionError("frame behavior");
    }
    if (player.provide() != null) throw new AssertionError("frame should be consumed");

    Future<Void> pending = manager.loadItem("gate:pending", handler);
    if (pending.isDone()) throw new AssertionError("pending future completed");
    if (!pending.cancel(true) || !pending.isCancelled()) throw new AssertionError("future cancellation");

    loaded.get().stop();
    decoded.stop();
    player.destroy();
    manager.shutdown();
    System.out.printf(
        "{\"probe\":\"integration\",\"starts\":%d,\"markers\":%d,\"serialization\":true,\"future_complete\":true,\"future_cancel\":true}%n",
        starts.get(), markers.get());
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
