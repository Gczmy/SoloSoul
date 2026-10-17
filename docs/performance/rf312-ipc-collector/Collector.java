package org.solosoul.rf312ipc;

import android.app.Activity;
import android.app.Instrumentation;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.os.Process;
import android.os.SystemClock;
import android.view.View;
import android.view.ViewGroup;
import android.webkit.WebView;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashSet;
import java.util.Iterator;
import java.util.Set;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
import org.json.JSONArray;
import org.json.JSONObject;

/** Non-default diagnostic only. Never acquires UiAutomation or sends UI input. */
public final class Collector extends Instrumentation {
  private static final String APP = "com.solosoul.app";
  private static final String[] PHASES = {"startup", "password-unlock", "first-ocr-user-flow", "workspace",
    "search-needle", "attachments-open", "image-preview", "text-preview", "pdf-text-native",
    "pdf-scanned-native", "application-lock", "password-reunlock"};
  private static final long READ_NS = 3000000000L;
  private Bundle arguments;
  private File root;
  private String token;
  private JSONObject process;
  private JSONObject previous;
  private ActivityMonitor monitor;
  private volatile boolean startupTraceActive;
  private Thread startupTracer;
  private int completed;
  private String step = "argument-admission";
  private String errorCode = "collector-contract-rejected";

  @Override public void onCreate(Bundle args) {
    super.onCreate(args);
    arguments = args;
    start();
  }

  private static void require(boolean value) {
    if (!value) throw new IllegalStateException("collector-contract-rejected");
  }

  private static void fields(JSONObject value, String... names) throws Exception {
    Set<String> actual = new HashSet<>();
    Iterator<String> keys = value.keys();
    while (keys.hasNext()) actual.add(keys.next());
    require(actual.equals(new HashSet<>(Arrays.asList(names))));
  }

  private static long integer(JSONObject value, String key) throws Exception {
    Object raw = value.get(key);
    require(raw instanceof Integer || raw instanceof Long);
    return ((Number) raw).longValue();
  }

  private static double number(JSONObject value, String key) throws Exception {
    Object raw = value.get(key);
    require(raw instanceof Number);
    double result = ((Number) raw).doubleValue();
    require(!Double.isNaN(result) && !Double.isInfinite(result));
    return result;
  }

  private static String sha(byte[] raw) throws Exception {
    byte[] hash = MessageDigest.getInstance("SHA-256").digest(raw);
    StringBuilder out = new StringBuilder();
    for (byte item : hash) out.append(String.format(java.util.Locale.ROOT, "%02x", item & 255));
    return out.toString();
  }

  private static byte[] bytes(File file, long limit) throws Exception {
    require(file.isFile() && file.length() <= limit);
    try (FileInputStream input = new FileInputStream(file)) {
      // procfs reports a zero stat size; bound the actual stream instead.
      java.io.ByteArrayOutputStream out = new java.io.ByteArrayOutputStream();
      byte[] block = new byte[1024]; int count;
      while ((count = input.read(block)) >= 0) {
        if (count == 0) continue;
        require(out.size() + count <= limit); out.write(block, 0, count);
      }
      require(out.size() > 0);
      return out.toByteArray();
    }
  }

  private static String fileSha(File file) throws Exception {
    MessageDigest digest = MessageDigest.getInstance("SHA-256");
    try (FileInputStream input = new FileInputStream(file)) {
      byte[] block = new byte[65536];
      int count;
      while ((count = input.read(block)) >= 0) if (count > 0) digest.update(block, 0, count);
    }
    StringBuilder result = new StringBuilder();
    for (byte item : digest.digest()) result.append(String.format(java.util.Locale.ROOT, "%02x", item & 255));
    return result.toString();
  }

  private JSONObject identity() throws Exception {
    String stat = new String(bytes(new File("/proc/self/stat"), 16384), StandardCharsets.UTF_8);
    String[] values = stat.substring(stat.lastIndexOf(')') + 2).trim().split("\\s+");
    require(values.length > 19);
    long ticks = Long.parseLong(values[19]);
    require(ticks > 0 && Process.myPid() > 0 && Process.myUid() >= 10000);
    return new JSONObject().put("processId", Process.myPid()).put("uid", Process.myUid()).put("startTicks", ticks);
  }

  private void write(String name, JSONObject value) throws Exception {
    File target = new File(root, name);
    require(target.getCanonicalFile().getParentFile().equals(root.getCanonicalFile()));
    require(target.createNewFile());
    byte[] raw = (value.toString() + "\n").getBytes(StandardCharsets.UTF_8);
    try (FileOutputStream stream = new FileOutputStream(target)) { stream.write(raw); stream.getFD().sync(); }
    File done = new File(root, name + ".sha256");
    require(done.createNewFile());
    try (FileOutputStream stream = new FileOutputStream(done)) {
      stream.write((sha(raw) + "\n").getBytes(StandardCharsets.UTF_8)); stream.getFD().sync();
    }
  }

  private static void views(View view, ArrayList<WebView> result) {
    if (view instanceof WebView) result.add((WebView) view);
    if (view instanceof ViewGroup) {
      ViewGroup group = (ViewGroup) view;
      for (int index = 0; index < group.getChildCount(); index++) views(group.getChildAt(index), result);
    }
  }

  private String script(boolean first) throws Exception {
    try (InputStream input = getContext().getAssets().open("snapshot.js")) {
      java.io.ByteArrayOutputStream out = new java.io.ByteArrayOutputStream();
      byte[] block = new byte[4096]; int count;
      while ((count = input.read(block)) >= 0) if (count > 0) out.write(block, 0, count);
      String value = out.toString("UTF-8");
      require(value.contains("__RUN_ID__") && value.contains("__BIND_FIRST__"));
      return value.replace("__RUN_ID__", token).replace("__BIND_FIRST__", first ? "true" : "false");
    }
  }

  private void validate(JSONObject doc) throws Exception {
    fields(doc, "present", "platform", "mainFrame", "frames", "invokeReadonly", "observer");
    errorCode = "observer-absent"; require(Boolean.TRUE.equals(doc.get("present")));
    errorCode = "invoke-not-protected"; require(Boolean.TRUE.equals(doc.get("invokeReadonly")));
    errorCode = "nonexclusive-document";
    require("android".equals(doc.get("platform")) && Boolean.TRUE.equals(doc.get("mainFrame")) && integer(doc, "frames") == 0);
    errorCode = "observer-metadata-invalid";
    JSONObject value = doc.getJSONObject("observer");
    fields(value, "schemaVersion", "scope", "runId", "valid", "invalidReasons", "installedAtMs",
      "timeOriginMs", "atMs", "maxEvents", "observedCount", "commands");
    require(integer(value, "schemaVersion") == 1 && token.equals(value.get("runId"))
      && "android-native-ipc-transport-observer".equals(value.get("scope"))
      && Boolean.TRUE.equals(value.get("valid")) && value.getJSONArray("invalidReasons").length() == 0
      && integer(value, "maxEvents") == 16384);
    JSONArray events = value.getJSONArray("commands");
    long total = integer(value, "observedCount");
    require(total > 0 && total == events.length() && total <= 16384);
    double time = number(value, "installedAtMs");
    double at = number(value, "atMs");
    require(time >= 0 && time <= at && number(value, "timeOriginMs") > 0);
    for (int index = 0; index < events.length(); index++) {
      JSONObject event = events.getJSONObject(index);
      fields(event, "command", "transport", "atMs");
      Object command = event.get("command"), transport = event.get("transport");
      require(command instanceof String && ((String) command).matches("[A-Za-z0-9_.:|/-]{1,128}"));
      require("android-java-bridge".equals(transport) || "android-channel-fetch".equals(transport));
      double timestamp = number(event, "atMs");
      require(timestamp >= time && timestamp <= at); time = timestamp;
      if (previous != null && index < previous.getJSONArray("commands").length()) {
        JSONObject old = previous.getJSONArray("commands").getJSONObject(index);
        require(old.get("command").equals(command) && old.get("transport").equals(transport)
          && Double.compare(number(old, "atMs"), timestamp) == 0);
      } else if (previous != null) require(timestamp >= number(previous, "atMs"));
    }
    if (previous != null) require(total >= integer(previous, "observedCount") && at >= number(previous, "atMs")
      && Double.compare(number(value, "timeOriginMs"), number(previous, "timeOriginMs")) == 0
      && Double.compare(number(value, "installedAtMs"), number(previous, "installedAtMs")) == 0);
    previous = value;
  }

  private JSONObject capture(final int index) throws Exception {
    errorCode = "activity-not-ready";
    final Activity activity = monitor.getLastActivity();
    require(activity != null && "com.solosoul.app.MainActivity".equals(activity.getClass().getName()) && !activity.isDestroyed());
    final String code = script(index == 0);
    final AtomicReference<String> raw = new AtomicReference<>();
    final AtomicReference<Throwable> error = new AtomicReference<>();
    final CountDownLatch done = new CountDownLatch(1);
    long start = System.nanoTime();
    boolean queued = new Handler(Looper.getMainLooper()).post(() -> {
      try {
        ArrayList<WebView> found = new ArrayList<>(); views(activity.getWindow().getDecorView(), found);
        require(found.size() == 1);
        found.get(0).evaluateJavascript(code, value -> { raw.set(value); done.countDown(); });
      } catch (Throwable problem) { error.set(problem); done.countDown(); }
    });
    long remaining = READ_NS - (System.nanoTime() - start);
    errorCode = "native-read-timeout";
    require(queued && remaining > 0 && done.await(remaining, TimeUnit.NANOSECONDS));
    long end = System.nanoTime();
    require(error.get() == null && end >= start && end - start <= READ_NS && raw.get() != null && raw.get().length() <= 4000000);
    errorCode = "metadata-decode-rejected";
    Object decoded = new JSONArray("[" + raw.get() + "]").get(0);
    require(decoded instanceof String);
    JSONObject document = new JSONObject((String) decoded); validate(document);
    errorCode = "process-replaced";
    require(identity().toString().equals(process.toString()));
    return new JSONObject().put("stage", PHASES[index]).put("process", process)
      .put("nativeReadStartedNs", start).put("nativeReadFinishedNs", end).put("document", document);
  }

  private void startStartupTrace() {
    startupTraceActive = true;
    startupTracer = new Thread(() -> {
      long traceStarted = System.nanoTime();
      long deadline = traceStarted + 8000000000L;
      for (int index = 1; index <= 80 && startupTraceActive && System.nanoTime() < deadline; index++) {
        try {
          long began = System.nanoTime();
          Thread main = Looper.getMainLooper().getThread();
          StackTraceElement[] stack = main.getStackTrace();
          JSONArray frames = new JSONArray();
          for (int frame = 0; frame < Math.min(stack.length, 128); frame++) {
            StackTraceElement item = stack[frame];
            require(item.getClassName().length() <= 512 && item.getMethodName().length() <= 256
              && (item.getFileName() == null || item.getFileName().length() <= 256));
            frames.put(new JSONObject().put("className", item.getClassName()).put("methodName", item.getMethodName())
              .put("fileName", item.getFileName() == null ? JSONObject.NULL : item.getFileName())
              .put("lineNumber", item.getLineNumber()).put("nativeMethod", item.isNativeMethod()));
          }
          long end = System.nanoTime();
          if (end >= deadline || !startupTraceActive) break;
          JSONObject value = new JSONObject().put("schemaVersion", 1).put("scope", "forensic-startup-main-thread-stack")
            .put("runId", token).put("sequence", index).put("process", identity()).put("traceStartedNs", traceStarted)
            .put("nativeReadStartedNs", began).put("nativeReadFinishedNs", end)
            .put("mainThreadId", main.getId()).put("threadState", main.getState().name())
            .put("truncated", stack.length > 128).put("frames", frames).put("performanceSampleAccepted", false);
          write(String.format(java.util.Locale.ROOT, "startup-stack-%03d.json", index), value);
          long remaining = deadline - System.nanoTime();
          if (remaining <= 0) break;
          SystemClock.sleep(Math.min(100, Math.max(1, remaining / 1000000L)));
        } catch (Throwable problem) {
          try {
            write("startup-stack-error.json", new JSONObject().put("schemaVersion", 1).put("runId", token)
              .put("errorClass", problem.getClass().getName()).put("performanceSampleAccepted", false));
          } catch (Throwable ignored) { /* The owned process may already be stopping. */ }
          break;
        }
      }
    }, "RF312StartupStack");
    startupTracer.setDaemon(true);
    startupTracer.start();
  }

  private void stopStartupTrace() {
    startupTraceActive = false;
    if (startupTracer != null) {
      try { startupTracer.join(500); }
      catch (InterruptedException interrupted) { Thread.currentThread().interrupt(); }
    }
  }

  @Override public void onStart() {
    Bundle result = new Bundle();
    try {
      token = arguments.getString("runId");
      String expected = arguments.getString("targetApkSha256");
      String startupStacks = arguments.getString("startupThreadStacks");
      require(startupStacks == null || "true".equals(startupStacks));
      require(token != null && token.matches("[a-f0-9]{32}") && expected != null && expected.matches("[a-f0-9]{64}"));
      step = "package-admission";
      require("true".equals(arguments.getString("privateDataBackedUp")) && APP.equals(getTargetContext().getPackageName()));
      require((getTargetContext().getApplicationInfo().flags & android.content.pm.ApplicationInfo.FLAG_DEBUGGABLE) == 0);
      step = "target-apk-hash";
      require(fileSha(new File(getTargetContext().getApplicationInfo().sourceDir)).equals(expected));
      step = "native-identity";
      process = identity();
      step = "evidence-root";
      File parent = getTargetContext().getExternalFilesDir(null); require(parent != null);
      root = new File(parent, "rf312-ipc-" + token); require(!root.exists() && root.mkdir());
      monitor = addMonitor("com.solosoul.app.MainActivity", null, false);
      step = "ready-publish";
      write("ready.json", new JSONObject().put("schemaVersion", 1).put("runId", token).put("process", process)
        .put("targetApkSha256", expected).put("collectorApkSha256", fileSha(new File(getContext().getApplicationInfo().sourceDir)))
        .put("phaseCount", PHASES.length));
      if ("true".equals(startupStacks)) startStartupTrace();
      long deadline = System.nanoTime() + 240000000000L;
      for (int index = 0; index < PHASES.length; index++) {
        step = "request-wait-" + PHASES[index]; errorCode = "collector-contract-rejected";
        String name = String.format(java.util.Locale.ROOT, "request-%03d.json", index + 1);
        File request = new File(root, name), marker = new File(root, name + ".sha256");
        while (!marker.isFile()) {
          require(System.nanoTime() < deadline && !new File(root, "abort").exists()); SystemClock.sleep(20);
        }
        byte[] data = bytes(request, 1024);
        step = "request-binding-" + PHASES[index];
        require(new String(bytes(marker, 128), StandardCharsets.UTF_8).trim().equals(sha(data)));
        JSONObject message = new JSONObject(new String(data, StandardCharsets.UTF_8));
        fields(message, "schemaVersion", "runId", "sequence", "stage");
        require(integer(message, "schemaVersion") == 1 && token.equals(message.get("runId"))
          && integer(message, "sequence") == index + 1 && PHASES[index].equals(message.get("stage")));
        step = "metadata-read-" + PHASES[index];
        JSONObject record = capture(index);
        if (index == 0) startupTraceActive = false;
        errorCode = "collector-contract-rejected";
        require(System.nanoTime() < deadline);
        step = "response-publish-" + PHASES[index];
        write(String.format(java.util.Locale.ROOT, "phase-%03d.json", index + 1), record);
        completed++;
        Bundle status = new Bundle(); status.putString("stage", PHASES[index]); status.putInt("completedStages", completed);
        sendStatus(0, status);
      }
      result.putBoolean("collectorPassed", true); result.putInt("completedStages", completed); result.putString("runId", token);
      finish(Activity.RESULT_OK, result);
    } catch (Throwable error) {
      result.putBoolean("collectorPassed", false); result.putInt("completedStages", completed);
      result.putString("errorClass", error.getClass().getName());
      result.putString("failedStep", step); result.putString("errorCode", errorCode);
      // Do not print raw JSON, arbitrary exception messages, request bodies or credentials.
      finish(Activity.RESULT_CANCELED, result);
    } finally {
      stopStartupTrace();
      if (monitor != null) removeMonitor(monitor);
    }
  }
}
