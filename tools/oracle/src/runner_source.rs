pub const REFERENCE: &str = r#"
import com.sedmelluq.discord.lavaplayer.format.StandardAudioDataFormats;
import com.sedmelluq.discord.lavaplayer.player.AudioLoadResultHandler;
import com.sedmelluq.discord.lavaplayer.player.AudioPlayer;
import com.sedmelluq.discord.lavaplayer.player.DefaultAudioPlayerManager;
import com.sedmelluq.discord.lavaplayer.player.event.*;
import com.sedmelluq.discord.lavaplayer.source.AudioSourceManager;
import com.sedmelluq.discord.lavaplayer.tools.FriendlyException;
import com.sedmelluq.discord.lavaplayer.track.*;
import com.sedmelluq.discord.lavaplayer.track.playback.ImmutableAudioFrame;
import com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicReference;

public final class OracleReference {
  private static DefaultAudioPlayerManager manager;
  private static AudioPlayer player;
  private static OracleSourceManager source;
  private static final Map<String, AudioTrack> tracks = new HashMap<>();
  private static final Map<String, TrackMarker> markers = new HashMap<>();
  private static final Map<String, Encoding> encodings = new HashMap<>();
  private static final List<String> events = Collections.synchronizedList(new ArrayList<>());
  private static final List<String> markerEvents = Collections.synchronizedList(new ArrayList<>());

  public static void main(String[] args) throws Exception {
    try (BufferedReader input = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8))) {
      String line;
      while ((line = input.readLine()) != null) {
        if (line.isEmpty()) continue;
        String[] fields = line.split("\\t", -1);
        try {
          execute(fields);
        } catch (Throwable failure) {
          Throwable cause = failure instanceof java.lang.reflect.InvocationTargetException
              ? failure.getCause() : failure;
          emit(fields[0], "error", "type", cause.getClass().getSimpleName());
        }
      }
    } finally {
      if (manager != null) manager.shutdown();
    }
  }

  private static void execute(String[] fields) throws Exception {
    String id = fields[0];
    switch (fields[1]) {
      case "create_manager":
        manager = new DefaultAudioPlayerManager();
        source = new OracleSourceManager();
        manager.registerSourceManager(source);
        emit(id, "manager", "created", true);
        break;
      case "create_player":
        player = manager.createPlayer();
        player.addListener(event -> {
          if (event instanceof TrackStartEvent) {
            events.add("track_start:" + ((TrackStartEvent) event).track.getIdentifier());
          } else if (event instanceof TrackEndEvent) {
            TrackEndEvent end = (TrackEndEvent) event;
            events.add("track_end:" + end.track.getIdentifier() + ":" + end.endReason.name());
          } else if (event instanceof PlayerPauseEvent) {
            events.add("player_pause");
          } else if (event instanceof PlayerResumeEvent) {
            events.add("player_resume");
          } else if (event instanceof TrackExceptionEvent) {
            TrackExceptionEvent exception = (TrackExceptionEvent) event;
            events.add("track_exception:" + exception.exception.severity.name());
          } else if (event instanceof TrackStuckEvent) {
            events.add("track_stuck");
          }
        });
        emit(id, "player", "created", true, "volume", player.getVolume(), "paused", player.isPaused());
        break;
      case "observe_configuration":
        emit(id, "configuration",
            "frame_buffer_ms", manager.getFrameBufferDuration(),
            "seek_ghosting", manager.isUsingSeekGhosting(),
            "resampling", manager.getConfiguration().getResamplingQuality().name(),
            "opus_quality", manager.getConfiguration().getOpusEncodingQuality(),
            "filter_hot_swap", manager.getConfiguration().isFilterHotSwapEnabled(),
            "channels", manager.getConfiguration().getOutputFormat().channelCount,
            "sample_rate", manager.getConfiguration().getOutputFormat().sampleRate,
            "chunk_samples", manager.getConfiguration().getOutputFormat().chunkSampleCount);
        break;
      case "load":
        load(id, text(fields[2]), text(fields[3]), fields[4].equals("-") ? null : text(fields[4]),
            Boolean.parseBoolean(fields[5]));
        break;
      case "observe_track": {
        AudioTrack track = track(text(fields[2]));
        AudioTrackInfo info = track.getInfo();
        emit(id, "track",
            "identifier", track.getIdentifier(), "title", info.title, "author", info.author,
            "length_ms", info.length, "stream", info.isStream, "uri", info.uri,
            "seekable", track.isSeekable(), "position_ms", track.getPosition(),
            "duration_ms", track.getDuration(), "state", track.getState().name());
        break;
      }
      case "set_user_data": {
        AudioTrack track = track(text(fields[2]));
        String value = text(fields[3]);
        track.setUserData(value);
        emit(id, "user_data", "value", track.getUserData(String.class),
            "object_identity", track.getUserData() == value,
            "typed_mismatch_null", track.getUserData(Integer.class) == null);
        break;
      }
      case "set_marker": {
        String trackId = text(fields[2]);
        String markerId = text(fields[3]);
        long position = Long.parseLong(fields[4]);
        TrackMarker marker = new TrackMarker(position,
            state -> markerEvents.add(markerId + ":" + state.name()));
        markers.put(markerId, marker);
        track(trackId).setMarker(marker);
        emit(id, "marker", "marker", markerId, "position_ms", position,
            "events", drain(markerEvents));
        break;
      }
      case "remove_marker": {
        String markerId = text(fields[3]);
        track(text(fields[2])).removeMarker(required(markers, markerId));
        emit(id, "marker", "marker", markerId, "events", drain(markerEvents));
        break;
      }
      case "seek": {
        AudioTrack track = track(text(fields[2]));
        long position = Long.parseLong(fields[3]);
        track.setPosition(position);
        emit(id, "seek", "position_ms", track.getPosition(), "marker_events", drain(markerEvents));
        break;
      }
      case "play": {
        AudioTrack track = track(text(fields[2]));
        boolean started = player.startTrack(track, Boolean.parseBoolean(fields[3]));
        emit(id, "play", "started", started,
            "active", player.getPlayingTrack() == null ? null : player.getPlayingTrack().getIdentifier(),
            "events", drain(events));
        break;
      }
      case "set_paused": {
        boolean paused = Boolean.parseBoolean(fields[2]);
        player.setPaused(paused);
        emit(id, "pause", "paused", player.isPaused(), "events", drain(events));
        break;
      }
      case "provide_frame": {
        long timeout = Long.parseLong(fields[2]);
        com.sedmelluq.discord.lavaplayer.track.playback.AudioFrame frame =
            player.provide(timeout, TimeUnit.MILLISECONDS);
        if (frame == null) {
          emit(id, "frame", "available", false, "events", drain(events));
        } else {
          emit(id, "frame", "available", true, "timecode_ms", frame.getTimecode(),
              "volume", frame.getVolume(), "data", bytes(frame.getData()),
              "data_length", frame.getDataLength(), "terminator", frame.isTerminator(),
              "events", drain(events));
        }
        break;
      }
      case "stop":
        player.stopTrack();
        emit(id, "stop", "active", player.getPlayingTrack() != null, "events", drain(events),
            "marker_events", drain(markerEvents));
        break;
      case "encode_track": {
        AudioTrack track = track(text(fields[2]));
        String encodingId = text(fields[3]);
        byte[] data = manager.encodeTrackDetails(track);
        encodings.put(encodingId, new Encoding(track.getInfo(), data));
        emit(id, "serialization", "encoding", encodingId, "bytes", bytes(data), "length", data.length);
        break;
      }
      case "decode_track": {
        String encodingId = text(fields[2]);
        String trackId = text(fields[3]);
        Encoding encoding = required(encodings, encodingId);
        AudioTrack decoded = manager.decodeTrackDetails(encoding.info, encoding.data);
        tracks.put(trackId, decoded);
        emit(id, "deserialization", "track", trackId,
            "identifier", decoded == null ? null : decoded.getIdentifier(),
            "position_ms", decoded == null ? null : decoded.getPosition());
        break;
      }
      case "shutdown":
        if (player != null) player.destroy();
        manager.shutdown();
        manager = null;
        emit(id, "shutdown", "complete", true, "events", drain(events),
            "marker_events", drain(markerEvents));
        break;
      default:
        throw new IllegalArgumentException("unknown operation " + fields[1]);
    }
  }

  private static void load(String actionId, String identifier, String trackId, String orderedKey,
                           boolean cancel) throws Exception {
    AtomicReference<String> result = new AtomicReference<>("none");
    AtomicReference<AudioTrack> loaded = new AtomicReference<>();
    CountDownLatch callback = new CountDownLatch(1);
    AudioLoadResultHandler handler = new AudioLoadResultHandler() {
      public void trackLoaded(AudioTrack track) { loaded.set(track); result.set("track"); callback.countDown(); }
      public void playlistLoaded(AudioPlaylist playlist) { result.set("playlist"); callback.countDown(); }
      public void noMatches() { result.set("no_matches"); callback.countDown(); }
      public void loadFailed(FriendlyException error) {
        result.set("failed:" + error.severity.name()); callback.countDown();
      }
    };

    OracleSourceManager.Blocker blocker = cancel ? source.block(identifier) : null;
    Future<Void> future = orderedKey == null
        ? manager.loadItem(identifier, handler)
        : manager.loadItemOrdered(orderedKey, identifier, handler);
    if (cancel) {
      if (!blocker.entered.await(5, TimeUnit.SECONDS)) throw new AssertionError("load did not enter source");
      boolean accepted = future.cancel(true);
      blocker.release.countDown();
      emit(actionId, "load", "identifier", identifier, "cancel_requested", true,
          "cancel_accepted", accepted, "done", future.isDone(), "cancelled", future.isCancelled());
      return;
    }
    future.get(5, TimeUnit.SECONDS);
    if (!callback.await(5, TimeUnit.SECONDS)) throw new AssertionError("load callback timeout");
    if (loaded.get() != null) tracks.put(trackId, loaded.get());
    emit(actionId, "load", "identifier", identifier, "result", result.get(),
        "track", loaded.get() == null ? null : trackId, "done", future.isDone(),
        "cancelled", future.isCancelled());
  }

  private static AudioTrack track(String id) { return required(tracks, id); }

  private static <T> T required(Map<String, T> values, String id) {
    T value = values.get(id);
    if (value == null) throw new IllegalArgumentException("unknown object " + id);
    return value;
  }

  private static String text(String hex) {
    byte[] bytes = new byte[hex.length() / 2];
    for (int index = 0; index < bytes.length; index++) {
      bytes[index] = (byte) Integer.parseInt(hex.substring(index * 2, index * 2 + 2), 16);
    }
    return new String(bytes, StandardCharsets.UTF_8);
  }

  private static String bytes(byte[] data) {
    StringBuilder value = new StringBuilder(data.length * 2);
    for (byte item : data) value.append(String.format(Locale.ROOT, "%02x", item & 0xff));
    return value.toString();
  }

  private static String drain(List<String> values) {
    synchronized (values) {
      String joined = String.join(",", values);
      values.clear();
      return joined;
    }
  }

  private static void emit(String actionId, String kind, Object... fields) {
    StringBuilder line = new StringBuilder();
    line.append("{\"action_id\":").append(quote(actionId));
    line.append(",\"kind\":").append(quote(kind)).append(",\"data\":{");
    for (int index = 0; index < fields.length; index += 2) {
      if (index > 0) line.append(',');
      line.append(quote((String) fields[index])).append(':').append(json(fields[index + 1]));
    }
    line.append("}}");
    System.out.println(line);
  }

  private static String json(Object value) {
    if (value == null) return "null";
    if (value instanceof Number || value instanceof Boolean) return value.toString();
    return quote(value.toString());
  }

  private static String quote(String value) {
    StringBuilder output = new StringBuilder("\"");
    for (int index = 0; index < value.length(); index++) {
      char character = value.charAt(index);
      switch (character) {
        case '\\': output.append("\\\\"); break;
        case '"': output.append("\\\""); break;
        case '\n': output.append("\\n"); break;
        case '\r': output.append("\\r"); break;
        case '\t': output.append("\\t"); break;
        default:
          if (character < 0x20) output.append(String.format(Locale.ROOT, "\\u%04x", (int) character));
          else output.append(character);
      }
    }
    return output.append('"').toString();
  }

  private static final class Encoding {
    final AudioTrackInfo info;
    final byte[] data;
    Encoding(AudioTrackInfo info, byte[] data) { this.info = info; this.data = data; }
  }

  private static final class OracleSourceManager implements AudioSourceManager {
    private final ConcurrentMap<String, Blocker> blockers = new ConcurrentHashMap<>();

    Blocker block(String identifier) {
      Blocker blocker = new Blocker();
      blockers.put(identifier, blocker);
      return blocker;
    }

    public String getSourceName() { return "mantle-oracle"; }

    public AudioItem loadItem(com.sedmelluq.discord.lavaplayer.player.AudioPlayerManager ignored,
                              AudioReference reference) {
      Blocker blocker = blockers.remove(reference.identifier);
      if (blocker != null) {
        blocker.entered.countDown();
        try {
          blocker.release.await();
        } catch (InterruptedException interrupted) {
          Thread.currentThread().interrupt();
          return null;
        }
      }
      return new OracleTrack(new AudioTrackInfo(
          "Synthetic title", "Synthetic author", 1000, reference.identifier, false,
          "oracle://" + reference.identifier, "oracle://artwork", "ORACLE000001"), this);
    }

    public boolean isTrackEncodable(AudioTrack track) { return true; }
    public void encodeTrack(AudioTrack track, DataOutput output) throws IOException {
      output.writeUTF("oracle-v1");
    }
    public AudioTrack decodeTrack(AudioTrackInfo info, DataInput input) throws IOException {
      if (!"oracle-v1".equals(input.readUTF())) throw new IOException("unknown oracle encoding");
      return new OracleTrack(info, this);
    }
    public void shutdown() { blockers.clear(); }

    private static final class Blocker {
      final CountDownLatch entered = new CountDownLatch(1);
      final CountDownLatch release = new CountDownLatch(1);
    }
  }

  private static final class OracleTrack extends BaseAudioTrack {
    private final OracleSourceManager source;
    OracleTrack(AudioTrackInfo info, OracleSourceManager source) { super(info); this.source = source; }
    public void process(LocalAudioTrackExecutor executor) throws Exception {
      executor.getAudioBuffer().consume(new ImmutableAudioFrame(
          0, new byte[] {1, 2, 3, 4}, 100, StandardAudioDataFormats.DISCORD_OPUS));
      executor.getAudioBuffer().setTerminateOnEmpty();
      executor.getAudioBuffer().waitForTermination();
    }
    public AudioSourceManager getSourceManager() { return source; }
    protected AudioTrack makeShallowClone() { return new OracleTrack(trackInfo, source); }
  }
}
"#;

pub const MANTLE: &str = r#"
import com.sedmelluq.discord.lavaplayer.player.AudioLoadResultHandler;
import com.sedmelluq.discord.lavaplayer.player.AudioPlayer;
import com.sedmelluq.discord.lavaplayer.player.DefaultAudioPlayerManager;
import com.sedmelluq.discord.lavaplayer.player.event.*;
import com.sedmelluq.discord.lavaplayer.tools.FriendlyException;
import com.sedmelluq.discord.lavaplayer.track.*;
import com.sedmelluq.discord.lavaplayer.track.playback.AudioFrame;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicReference;

public final class OracleMantle {
  private static DefaultAudioPlayerManager manager;
  private static AudioPlayer player;
  private static final Map<String, AudioTrack> tracks = new HashMap<>();
  private static final Map<String, TrackMarker> markers = new HashMap<>();
  private static final Map<String, Encoding> encodings = new HashMap<>();
  private static final List<String> events = new ArrayList<>();
  private static final List<String> markerEvents = new ArrayList<>();

  public static void main(String[] args) throws Exception {
    System.load(args[0]);
    try (Scanner input = new Scanner(System.in, StandardCharsets.UTF_8)) {
      while (input.hasNextLine()) {
        String line = input.nextLine();
        if (line.isEmpty()) continue;
        String[] fields = line.split("\\t", -1);
        try {
          execute(fields);
        } catch (Throwable failure) {
          Throwable cause = failure instanceof java.lang.reflect.InvocationTargetException
              ? failure.getCause() : failure;
          emit(fields[0], cause instanceof UnsupportedOperationException ? "unsupported" : "error",
              "type", cause.getClass().getSimpleName());
        }
      }
    } finally {
      if (manager != null) manager.shutdown();
    }
  }

  private static void execute(String[] fields) throws Exception {
    String id = fields[0];
    switch (fields[1]) {
      case "create_manager":
        manager = new DefaultAudioPlayerManager();
        emit(id, "manager", "created", true);
        break;
      case "create_player":
        player = manager.createPlayer();
        player.addListener(event -> {
          if (event instanceof TrackStartEvent) {
            events.add("track_start:" + ((TrackStartEvent) event).track.getIdentifier());
          } else if (event instanceof TrackEndEvent) {
            TrackEndEvent end = (TrackEndEvent) event;
            events.add("track_end:" + end.track.getIdentifier() + ":" + end.endReason.name());
          } else if (event instanceof PlayerPauseEvent) {
            events.add("player_pause");
          } else if (event instanceof PlayerResumeEvent) {
            events.add("player_resume");
          }
        });
        emit(id, "player", "created", true, "volume", player.getVolume(), "paused", player.isPaused());
        break;
      case "observe_configuration":
        emit(id, "configuration",
            "frame_buffer_ms", manager.getFrameBufferDuration(),
            "seek_ghosting", manager.isUsingSeekGhosting(),
            "resampling", manager.getConfiguration().getResamplingQuality().name(),
            "opus_quality", manager.getConfiguration().getOpusEncodingQuality(),
            "filter_hot_swap", manager.getConfiguration().isFilterHotSwapEnabled(),
            "channels", manager.getConfiguration().getOutputFormat().channelCount,
            "sample_rate", manager.getConfiguration().getOutputFormat().sampleRate,
            "chunk_samples", manager.getConfiguration().getOutputFormat().chunkSampleCount);
        break;
      case "load":
        load(id, text(fields[2]), text(fields[3]), fields[4].equals("-") ? null : text(fields[4]),
            Boolean.parseBoolean(fields[5]));
        break;
      case "observe_track": {
        AudioTrack track = track(text(fields[2]));
        AudioTrackInfo info = track.getInfo();
        emit(id, "track",
            "identifier", track.getIdentifier(), "title", info.title, "author", info.author,
            "length_ms", info.length, "stream", info.isStream, "uri", info.uri,
            "seekable", track.isSeekable(), "position_ms", track.getPosition(),
            "duration_ms", track.getDuration(), "state", track.getState().name());
        break;
      }
      case "set_user_data": {
        AudioTrack track = track(text(fields[2]));
        String value = text(fields[3]);
        track.setUserData(value);
        emit(id, "user_data", "value", track.getUserData(String.class),
            "object_identity", track.getUserData() == value,
            "typed_mismatch_null", track.getUserData(Integer.class) == null);
        break;
      }
      case "set_marker": {
        String trackId = text(fields[2]);
        String markerId = text(fields[3]);
        long position = Long.parseLong(fields[4]);
        TrackMarker marker = new TrackMarker(position,
            state -> markerEvents.add(markerId + ":" + state.name()));
        markers.put(markerId, marker);
        track(trackId).setMarker(marker);
        emit(id, "marker", "marker", markerId, "position_ms", position,
            "events", drain(markerEvents));
        break;
      }
      case "remove_marker": {
        String markerId = text(fields[3]);
        track(text(fields[2])).removeMarker(required(markers, markerId));
        emit(id, "marker", "marker", markerId, "events", drain(markerEvents));
        break;
      }
      case "seek": {
        AudioTrack track = track(text(fields[2]));
        long position = Long.parseLong(fields[3]);
        track.setPosition(position);
        emit(id, "seek", "position_ms", track.getPosition(), "marker_events", drain(markerEvents));
        break;
      }
      case "play": {
        AudioTrack track = track(text(fields[2]));
        boolean started = player.startTrack(track, Boolean.parseBoolean(fields[3]));
        emit(id, "play", "started", started,
            "active", player.getPlayingTrack() == null ? null : player.getPlayingTrack().getIdentifier(),
            "events", drain(events));
        break;
      }
      case "set_paused": {
        boolean paused = Boolean.parseBoolean(fields[2]);
        player.setPaused(paused);
        emit(id, "pause", "paused", player.isPaused(), "events", drain(events));
        break;
      }
      case "provide_frame": {
        AudioFrame frame = player.provide(Long.parseLong(fields[2]), TimeUnit.MILLISECONDS);
        if (frame == null) emit(id, "frame", "available", false, "events", drain(events));
        else emit(id, "frame", "available", true, "timecode_ms", frame.getTimecode(),
            "volume", frame.getVolume(), "data", bytes(frame.getData()),
            "data_length", frame.getDataLength(), "terminator", frame.isTerminator(),
            "events", drain(events));
        break;
      }
      case "stop":
        player.stopTrack();
        emit(id, "stop", "active", player.getPlayingTrack() != null,
            "events", drain(events), "marker_events", drain(markerEvents));
        break;
      case "encode_track": {
        AudioTrack track = track(text(fields[2]));
        String encodingId = text(fields[3]);
        byte[] data = manager.encodeTrackDetails(track);
        encodings.put(encodingId, new Encoding(track.getInfo(), data));
        emit(id, "serialization", "encoding", encodingId, "bytes", bytes(data), "length", data.length);
        break;
      }
      case "decode_track": {
        String encodingId = text(fields[2]);
        String trackId = text(fields[3]);
        Encoding encoding = required(encodings, encodingId);
        AudioTrack decoded = manager.decodeTrackDetails(encoding.info, encoding.data);
        tracks.put(trackId, decoded);
        emit(id, "deserialization", "track", trackId,
            "identifier", decoded == null ? null : decoded.getIdentifier(),
            "position_ms", decoded == null ? null : decoded.getPosition());
        break;
      }
      case "shutdown":
        if (player != null) player.destroy();
        manager.shutdown();
        manager = null;
        emit(id, "shutdown", "complete", true, "events", drain(events),
            "marker_events", drain(markerEvents));
        break;
      default:
        throw new IllegalArgumentException("unknown operation " + fields[1]);
    }
  }

  private static void load(String actionId, String identifier, String trackId, String orderedKey,
                           boolean cancel) throws Exception {
    AtomicReference<String> result = new AtomicReference<>("none");
    AtomicReference<AudioTrack> loaded = new AtomicReference<>();
    AudioLoadResultHandler handler = new AudioLoadResultHandler() {
      public void trackLoaded(AudioTrack track) { loaded.set(track); result.set("track"); }
      public void playlistLoaded(AudioPlaylist playlist) { result.set("playlist"); }
      public void noMatches() { result.set("no_matches"); }
      public void loadFailed(FriendlyException error) { result.set("failed"); }
    };
    Future<Void> future = orderedKey == null
        ? manager.loadItem(identifier, handler)
        : manager.loadItemOrdered(orderedKey, identifier, handler);
    if (cancel) {
      boolean accepted = future.cancel(true);
      emit(actionId, "load", "identifier", identifier, "cancel_requested", true,
          "cancel_accepted", accepted, "done", future.isDone(), "cancelled", future.isCancelled());
      return;
    }
    future.get(5, TimeUnit.SECONDS);
    if (loaded.get() != null) tracks.put(trackId, loaded.get());
    emit(actionId, "load", "identifier", identifier, "result", result.get(),
        "track", loaded.get() == null ? null : trackId, "done", future.isDone(),
        "cancelled", future.isCancelled());
  }

  private static AudioTrack track(String id) { return required(tracks, id); }
  private static <T> T required(Map<String, T> values, String id) {
    T value = values.get(id);
    if (value == null) throw new IllegalArgumentException("unknown object " + id);
    return value;
  }
  private static String text(String hex) {
    byte[] bytes = new byte[hex.length() / 2];
    for (int index = 0; index < bytes.length; index++)
      bytes[index] = (byte) Integer.parseInt(hex.substring(index * 2, index * 2 + 2), 16);
    return new String(bytes, StandardCharsets.UTF_8);
  }
  private static String bytes(byte[] data) {
    StringBuilder value = new StringBuilder(data.length * 2);
    for (byte item : data) value.append(String.format(Locale.ROOT, "%02x", item & 0xff));
    return value.toString();
  }
  private static String drain(List<String> values) {
    String joined = String.join(",", values);
    values.clear();
    return joined;
  }
  private static void emit(String actionId, String kind, Object... fields) {
    StringBuilder line = new StringBuilder();
    line.append("{\"action_id\":").append(quote(actionId));
    line.append(",\"kind\":").append(quote(kind)).append(",\"data\":{");
    for (int index = 0; index < fields.length; index += 2) {
      if (index > 0) line.append(',');
      line.append(quote((String) fields[index])).append(':').append(json(fields[index + 1]));
    }
    System.out.println(line.append("}}").toString());
  }
  private static String json(Object value) {
    if (value == null) return "null";
    if (value instanceof Number || value instanceof Boolean) return value.toString();
    return quote(value.toString());
  }
  private static String quote(String value) {
    StringBuilder output = new StringBuilder("\"");
    for (int index = 0; index < value.length(); index++) {
      char character = value.charAt(index);
      if (character == '\\') output.append("\\\\");
      else if (character == '"') output.append("\\\"");
      else if (character == '\n') output.append("\\n");
      else if (character == '\r') output.append("\\r");
      else if (character == '\t') output.append("\\t");
      else if (character < 0x20) output.append(String.format(Locale.ROOT, "\\u%04x", (int) character));
      else output.append(character);
    }
    return output.append('"').toString();
  }
  private static final class Encoding {
    final AudioTrackInfo info;
    final byte[] data;
    Encoding(AudioTrackInfo info, byte[] data) { this.info = info; this.data = data; }
  }
}
"#;
