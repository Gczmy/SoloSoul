package org.solosoul.rf312frames;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.pm.ApplicationInfo;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.os.Process;
import android.os.SystemClock;
import android.view.View;
import android.view.ViewGroup;
import android.view.ViewTreeObserver;
import android.webkit.WebView;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashSet;
import java.util.Iterator;
import java.util.Set;
import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;
import java.util.function.LongConsumer;
import org.json.JSONArray;
import org.json.JSONObject;

/** Explicit target instrumentation. No UI input, invalidation or business IPC. */
public final class FrameCollector extends Instrumentation {
  private static final String APP = "com.solosoul.app";
  private static final long STAGE_NS = 45000000000L;
  private static final long READ_NS = 3000000000L;
  private static final String[] STATE_FIELDS = {"schemaVersion", "mainFrame", "childFrames", "localOrigin",
      "loginRoute", "passwordInputs", "submitButtons", "enabled", "visible", "hitPassword", "hitSubmit",
      "documentFocused", "documentVisible", "timeOriginMs", "atMs"};
  private static final String STATE_SCRIPT = "(() => {"
      + "const visible=e=>{const r=e.getBoundingClientRect(),s=getComputedStyle(e);return r.width>0&&r.height>0"
      + "&&r.left>=0&&r.top>=0&&r.right<=innerWidth&&r.bottom<=innerHeight&&s.display!=='none'"
      + "&&s.visibility==='visible'&&Number(s.opacity)>0;};"
      + "const p=[...document.querySelectorAll('input[type=password]')].filter(visible);"
      + "const b=p.length===1?[...p[0].closest('form')?.querySelectorAll('button[type=submit]')||[]].filter(visible):[];"
      + "const hit=e=>{if(!e)return false;const r=e.getBoundingClientRect(),h=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return !!h&&(h===e||e.contains(h));};"
      + "return JSON.stringify({schemaVersion:2,mainFrame:top===window,childFrames:document.querySelectorAll('iframe,frame').length,"
      + "localOrigin:location.hostname==='tauri.localhost'&&['http:','https:'].includes(location.protocol),"
      + "loginRoute:location.pathname==='/login',passwordInputs:p.length,submitButtons:b.length,"
      + "enabled:p.length===1&&b.length===1&&!p[0].disabled&&!b[0].disabled,visible:p.length===1&&b.length===1,"
      + "hitPassword:hit(p[0]),hitSubmit:hit(b[0]),documentFocused:document.hasFocus(),documentVisible:document.visibilityState==='visible',"
      + "timeOriginMs:performance.timeOrigin,atMs:performance.now()});})()";

  private Bundle arguments;
  private WindowFrameTrace windowTrace;
  private File root;
  private String run;
  private JSONObject process;
  private ActivityMonitor monitor;
  private final Handler main = new Handler(Looper.getMainLooper());
  private final AtomicReference<String> failure = new AtomicReference<>();
  private final AtomicInteger commitCount = new AtomicInteger();
  private final ConcurrentLinkedQueue<JSONObject> frames = new ConcurrentLinkedQueue<>();
  private volatile boolean closed;
  private volatile JSONObject lastDocument;
  private volatile String nativeStatus = "not-observed";
  private Activity activity;
  private WebView view;
  private ViewTreeObserver tree;
  private ViewTreeObserver.OnDrawListener drawListener;
  private ViewTreeObserver.OnPreDrawListener preDrawListener;
  private FrameCommitSequence frameSequence;
  private Runnable pendingCallback;
  private long fenceRequest;
  private long fencePostedNs;
  private long fenceCompleteNs;
  private long traceStartedNs;
  private int width;
  private int height;
  private String step = "arguments";

  private static final class Fence extends WebView.VisualStateCallback {
    private final LongConsumer completion;
    Fence(LongConsumer completion) { this.completion = completion; }
    @Override public void onComplete(long requestId) { completion.accept(requestId); }
  }

  private static void require(boolean condition) {
    if (!condition) throw new IllegalStateException("frame-contract-rejected");
  }

  private static String sha(byte[] bytes) throws Exception {
    byte[] digest = MessageDigest.getInstance("SHA-256").digest(bytes);
    StringBuilder value = new StringBuilder();
    for (byte part : digest) value.append(String.format(java.util.Locale.ROOT, "%02x", part & 255));
    return value.toString();
  }

  private static String fileSha(File file) throws Exception {
    MessageDigest digest = MessageDigest.getInstance("SHA-256");
    try (FileInputStream input = new FileInputStream(file)) {
      byte[] block = new byte[65536]; int count;
      while ((count = input.read(block)) >= 0) if (count > 0) digest.update(block, 0, count);
    }
    StringBuilder value = new StringBuilder();
    for (byte part : digest.digest()) value.append(String.format(java.util.Locale.ROOT, "%02x", part & 255));
    return value.toString();
  }

  private static JSONObject identity() throws Exception {
    byte[] raw;
    try (FileInputStream input = new FileInputStream("/proc/self/stat")) {
      java.io.ByteArrayOutputStream output = new java.io.ByteArrayOutputStream();
      byte[] block = new byte[1024]; int count;
      while ((count = input.read(block)) >= 0) {
        if (count == 0) continue;
        require(output.size() + count <= 16384); output.write(block, 0, count);
      }
      raw = output.toByteArray();
    }
    String stat = new String(raw, StandardCharsets.UTF_8);
    String[] fields = stat.substring(stat.lastIndexOf(')') + 2).trim().split("\\s+");
    require(fields.length > 19 && Process.myUid() >= 10000);
    long birth = Long.parseLong(fields[19]); require(birth > 0 && Process.myPid() > 0);
    return new JSONObject().put("processId", Process.myPid()).put("uid", Process.myUid()).put("startTicks", birth);
  }

  private void write(String name, JSONObject value) throws Exception {
    require(name.equals("ready.json") || name.equals("frames.json") || name.equals("failure.json")
        || (windowTrace != null && name.equals("window-trace.json")));
    File file = new File(root, name); require(file.createNewFile());
    byte[] raw = (value.toString() + "\n").getBytes(StandardCharsets.UTF_8);
    try (FileOutputStream stream = new FileOutputStream(file)) { stream.write(raw); stream.getFD().sync(); }
    File marker = new File(root, name + ".sha256"); require(marker.createNewFile());
    try (FileOutputStream stream = new FileOutputStream(marker)) {
      stream.write((sha(raw) + "\n").getBytes(StandardCharsets.UTF_8)); stream.getFD().sync();
    }
  }

  private static void views(View parent, ArrayList<WebView> result) {
    if (parent instanceof WebView) result.add((WebView) parent);
    if (parent instanceof ViewGroup) {
      ViewGroup group = (ViewGroup) parent;
      for (int index = 0; index < group.getChildCount(); index++) views(group.getChildAt(index), result);
    }
  }

  private static boolean flag(JSONObject value, String key) throws Exception {
    Object part = value.get(key); require(part instanceof Boolean); return (Boolean) part;
  }

  private static boolean ready(JSONObject value) throws Exception {
    Set<String> names = new HashSet<>(); Iterator<String> keys = value.keys();
    while (keys.hasNext()) names.add(keys.next());
    require(names.equals(new HashSet<>(Arrays.asList(STATE_FIELDS))));
    for (String key : new String[]{"schemaVersion", "childFrames", "passwordInputs", "submitButtons"}) {
      Object part = value.get(key); require(part instanceof Integer || part instanceof Long);
      require(((Number) part).longValue() >= 0 && ((Number) part).longValue() <= 100000);
    }
    require(value.getInt("schemaVersion") == 2);
    for (String key : new String[]{"timeOriginMs", "atMs"}) {
      Object part = value.get(key); require(part instanceof Number);
      double number = ((Number) part).doubleValue(); require(Double.isFinite(number) && number >= 0);
    }
    require(value.getDouble("timeOriginMs") > 0);
    flag(value, "documentFocused"); // Diagnostic only; do not focus the page to obtain a frame.
    boolean accepted = value.getInt("childFrames") == 0 && value.getInt("passwordInputs") == 1 && value.getInt("submitButtons") == 1;
    for (String key : new String[]{"mainFrame", "localOrigin", "loginRoute", "enabled", "visible", "hitPassword", "hitSubmit", "documentVisible"}) {
      accepted = flag(value, key) && accepted;
    }
    return accepted;
  }

  private void boundView() {
    require(activity != null && view != null && monitor.getLastActivity() == activity && !activity.isFinishing());
    ArrayList<WebView> current = new ArrayList<>(); views(activity.getWindow().getDecorView(), current);
    require(current.size() == 1 && current.get(0) == view && view.isAttachedToWindow() && view.isShown()
        && view.hasWindowFocus() && view.isHardwareAccelerated() && view.getWidth() == width && view.getHeight() == height);
  }

  private JSONObject state(long deadline) throws Exception {
    CountDownLatch done = new CountDownLatch(1);
    AtomicReference<JSONObject> result = new AtomicReference<>();
    main.post(() -> {
      try {
        require(!closed);
        Activity candidate = monitor.getLastActivity();
        if (candidate == null) { nativeStatus = "no-activity"; done.countDown(); return; }
        nativeStatus = "activity-identity";
        require("com.solosoul.app.MainActivity".equals(candidate.getClass().getName()) && !candidate.isFinishing());
        ArrayList<WebView> found = new ArrayList<>(); views(candidate.getWindow().getDecorView(), found);
        require(found.size() <= 1);
        if (found.isEmpty()) { nativeStatus = "no-webview"; require(view == null); done.countDown(); return; }
        WebView next = found.get(0);
        if (!next.isAttachedToWindow() || !next.isShown() || !next.hasWindowFocus()
            || !next.isHardwareAccelerated() || next.getWidth() <= 0 || next.getHeight() <= 0) {
          nativeStatus = !next.isAttachedToWindow() ? "view-detached" : !next.isShown() ? "view-hidden"
              : !next.hasWindowFocus() ? "window-unfocused" : !next.isHardwareAccelerated() ? "software-rendering" : "unmeasured-view";
          // Before admission this is pending initialization, including a 0x0
          // WebView attached before its first layout. After binding it remains
          // a hard invariant violation. No draw or focus request is injected.
          require(view == null); done.countDown(); return;
        }
        if (view != null) boundView();
        nativeStatus = "readonly-script-queued";
        next.evaluateJavascript(STATE_SCRIPT, raw -> {
          try {
            require(!closed && raw != null && raw.length() <= 4096);
            if ("null".equals(raw)) { require(view == null); return; }
            Object parsed = new JSONArray("[" + raw + "]").get(0); require(parsed instanceof String);
            JSONObject value = new JSONObject((String) parsed);
            boolean eligible = ready(value); lastDocument = value;
            nativeStatus = eligible ? "document-ready" : "document-not-ready";
            if (eligible) {
              if (view == null) { activity = candidate; view = next; width = next.getWidth(); height = next.getHeight(); }
              boundView(); result.set(value);
            } else require(view == null);
          } catch (Throwable error) { failure.compareAndSet(null, "document-or-view-rejected"); }
          finally { done.countDown(); }
        });
      } catch (Throwable error) { failure.compareAndSet(null, "native-view-rejected"); done.countDown(); }
    });
    long left = Math.min(READ_NS, deadline - System.nanoTime());
    require(left > 0 && done.await(left, TimeUnit.NANOSECONDS) && System.nanoTime() < deadline);
    require(failure.get() == null);
    return result.get();
  }

  private void arm(long deadline) throws Exception {
    CountDownLatch installed = new CountDownLatch(1);
    main.post(() -> {
      try {
        boundView(); tree = view.getViewTreeObserver(); require(tree.isAlive());
        drawListener = () -> {
          if (closed || failure.get() != null || frameSequence == null || commitCount.get() >= 2) return;
          try {
            boundView();
            if (frameSequence.observeDraw(System.nanoTime())) nativeStatus = "native-draw-observed";
          } catch (Throwable error) { failure.compareAndSet(null, "draw-observation-rejected"); }
        };
        preDrawListener = () -> {
          if (closed || failure.get() != null || frameSequence == null || commitCount.get() >= 2) return true;
          try {
            boundView(); final FrameCommitSequence.Ticket ticket = frameSequence.reserve(System.nanoTime());
            if (ticket == null) return true;
            pendingCallback = () -> {
              final long commitNs = System.nanoTime();
              main.post(() -> {
                try {
                  if (closed) return;
                  boundView(); frameSequence.commit(ticket, commitNs);
                  nativeStatus = "native-commit-observed";
                  frames.add(new JSONObject().put("drawOrdinal", ticket.ordinal).put("drawStartedNs", ticket.drawStartedNs)
                      .put("committedNs", ticket.committedNs).put("callbackSource", "ViewTreeObserver.registerFrameCommitCallback"));
                  require(commitCount.incrementAndGet() == frameSequence.committedCount());
                } catch (Throwable error) { failure.compareAndSet(null, "frame-commit-rejected"); }
              });
            };
            // API31 snapshots this list before dispatching OnDraw. Register now,
            // while the actual OnDraw and actual native callback remain required.
            tree.registerFrameCommitCallback(pendingCallback);
            nativeStatus = "native-pre-draw-commit-armed";
          } catch (Throwable error) { failure.compareAndSet(null, "pre-draw-registration-rejected"); }
          return true;
        };
        tree.addOnDrawListener(drawListener); tree.addOnPreDrawListener(preDrawListener);
        nativeStatus = "native-listeners-armed";
        fencePostedNs = System.nanoTime(); fenceRequest = fencePostedNs;
        nativeStatus = "visual-fence-pending";
        view.postVisualStateCallback(fenceRequest, new Fence(request -> {
          try {
            require(!closed && request == fenceRequest && System.nanoTime() < deadline); boundView();
            fenceCompleteNs = windowTrace == null ? System.nanoTime() : windowTrace.fenceCompleted(view);
            frameSequence = new FrameCommitSequence(fenceCompleteNs, deadline);
            nativeStatus = "native-draw-listener-armed";
          } catch (Throwable error) { failure.compareAndSet(null, "visual-fence-rejected"); }
        }));
      } catch (Throwable error) { failure.compareAndSet(null, "visual-fence-registration-rejected"); }
      finally { installed.countDown(); }
    });
    long left = Math.min(READ_NS, deadline - System.nanoTime());
    require(left > 0 && installed.await(left, TimeUnit.NANOSECONDS) && failure.get() == null);
  }

  private boolean detach(boolean verifyBoundWindow) {
    long stopBegin = System.nanoTime(), stopDeadline = stopBegin + READ_NS;
    closed = true;
    if (windowTrace != null) windowTrace.beginClose(stopBegin);
    CountDownLatch done = new CountDownLatch(1);
    AtomicBoolean clean = new AtomicBoolean();
    main.post(() -> {
      try {
        if (verifyBoundWindow) boundView();
        if (tree != null && tree.isAlive()) {
          if (drawListener != null) tree.removeOnDrawListener(drawListener);
          if (preDrawListener != null) tree.removeOnPreDrawListener(preDrawListener);
          if (pendingCallback != null) tree.unregisterFrameCommitCallback(pendingCallback);
        }
        if (windowTrace != null) windowTrace.removeOnMain();
        clean.set(true);
      } catch (Throwable error) { failure.compareAndSet(null, "frame-listener-cleanup-rejected"); }
      finally { done.countDown(); }
    });
    try {
      long left = stopDeadline - System.nanoTime();
      boolean base = left > 0 && done.await(left, TimeUnit.NANOSECONDS) && clean.get();
      return windowTrace == null ? base : windowTrace.finishClose(stopDeadline) && base;
    } catch (InterruptedException error) { Thread.currentThread().interrupt(); return false; }
  }

  @Override public void onCreate(Bundle args) { super.onCreate(args); arguments = args; start(); }

  @Override public void onStart() {
    Bundle terminal = new Bundle(); JSONObject before = null; long deadline = 0;
    try {
      require(Build.VERSION.SDK_INT >= 29 && arguments != null);
      String traceOption = arguments.getString("windowFrameTrace");
      require(traceOption == null || "true".equals(traceOption));
      run = arguments.getString("runId"); String expected = arguments.getString("targetApkSha256");
      require(run != null && run.matches("[a-f0-9]{32}") && expected != null && expected.matches("[a-f0-9]{64}"));
      require("true".equals(arguments.getString("privateDataBackedUp")) && APP.equals(getTargetContext().getPackageName()));
      require((getTargetContext().getApplicationInfo().flags & ApplicationInfo.FLAG_DEBUGGABLE) == 0);
      require(fileSha(new File(getTargetContext().getApplicationInfo().sourceDir)).equals(expected));
      process = identity(); File parent = getTargetContext().getExternalFilesDir(null); require(parent != null);
      root = new File(parent, "rf312-frames-" + run); require(!root.exists() && root.mkdir());
      monitor = addMonitor("com.solosoul.app.MainActivity", null, false);
      traceStartedNs = System.nanoTime(); deadline = traceStartedNs + STAGE_NS;
      String collectorIdentity = fileSha(new File(getContext().getApplicationInfo().sourceDir));
      if (traceOption != null) {
        step = "window-trace-hooks";
        windowTrace = new WindowFrameTrace(run, process, expected, collectorIdentity, traceStartedNs, deadline, main);
        windowTrace.start(getTargetContext().getApplicationContext());
      }
      write("ready.json", new JSONObject().put("schemaVersion", 1).put("scope", "native-login-frame-commit-probe")
          .put("runId", run).put("process", process).put("targetApkSha256", expected)
          .put("collectorApkSha256", collectorIdentity)
          .put("startedNs", traceStartedNs).put("deadlineNs", deadline));
      step = "usable-login-document";
      while (System.nanoTime() < deadline && before == null) { before = state(deadline); if (before == null) SystemClock.sleep(25); }
      require(before != null && ready(before));
      step = "two-native-commits"; arm(deadline);
      while (System.nanoTime() < deadline && commitCount.get() < 2 && failure.get() == null) SystemClock.sleep(5);
      require(failure.get() == null && commitCount.get() == 2 && frames.size() == 2);
      step = "final-document-identity";
      JSONObject after = state(deadline); require(after != null && ready(after));
      require(before.getDouble("timeOriginMs") == after.getDouble("timeOriginMs") && after.getDouble("atMs") >= before.getDouble("atMs"));
      require(System.nanoTime() < deadline && fenceCompleteNs >= fencePostedNs && fencePostedNs >= traceStartedNs);
      ArrayList<JSONObject> captured = new ArrayList<>(frames); require(captured.size() == 2);
      require(captured.get(1).getLong("drawOrdinal") > captured.get(0).getLong("drawOrdinal")
          && captured.get(1).getLong("drawStartedNs") > captured.get(0).getLong("drawStartedNs")
          && captured.get(1).getLong("committedNs") > captured.get(0).getLong("committedNs"));
      require(detach(true));
      long completed = System.nanoTime(); require(completed < deadline);
      JSONObject result = new JSONObject().put("schemaVersion", 2).put("scope", "native-login-frame-commits-after-visual-fence")
          .put("runId", run).put("process", process).put("targetApkSha256", expected).put("api", Build.VERSION.SDK_INT)
          .put("startedNs", traceStartedNs).put("deadlineNs", deadline).put("completedNs", completed)
          .put("fenceRequest", fenceRequest).put("fencePostedNs", fencePostedNs).put("fenceCompleteNs", fenceCompleteNs)
          .put("hardwareAccelerated", true).put("nativeWindowFocused", true).put("width", width).put("height", height).put("explicitDrawRequestsDispatched", false)
          .put("nativeClock", "System.nanoTime; same target process; not converted to document clock")
          .put("forcedInvalidation", false).put("inputDispatched", false).put("businessIpcDispatched", false)
          .put("documentBefore", before).put("documentAfter", after).put("frames", new JSONArray(captured))
          .put("callbacksDetached", true).put("physicalDisplayPresentationProven", false).put("applicationOnlyLatencyAccepted", false);
      if (windowTrace != null) write("window-trace.json", windowTrace.snapshot());
      write("frames.json", result);
      // Keep the instrumented window alive for external native screenshots.
      // The acknowledgement consumes the SAME original stage budget.
      step = "external-frame-evidence-acknowledgement";
      File accept = new File(root, "accept");
      while (System.nanoTime() < deadline && !accept.isFile()) SystemClock.sleep(5);
      require(System.nanoTime() < deadline && accept.isFile() && accept.length() == 0
          && accept.getCanonicalFile().equals(accept.getAbsoluteFile()) && identity().toString().equals(process.toString()));
      terminal.putBoolean("frameProbePassed", true); terminal.putInt("committedFrames", 2);
      finish(Activity.RESULT_OK, terminal);
    } catch (Throwable error) {
      boolean detached = detach(false);
      try {
        if (root != null && windowTrace != null) write("window-trace.json", windowTrace.snapshot());
      } catch (Throwable ignored) { terminal.putBoolean("windowTraceUnavailable", true); }
      try {
        if (root != null) write("failure.json", new JSONObject().put("schemaVersion", 2).put("runId", run)
            .put("scope", "native-login-frame-commit-refusal").put("step", step)
            .put("errorClass", error.getClass().getName()).put("reason", failure.get() == null ? "frame-contract-rejected" : failure.get())
            .put("nativeStatus", nativeStatus).put("lastDocument", lastDocument == null ? JSONObject.NULL : lastDocument)
            .put("frames", new JSONArray(new ArrayList<>(frames))).put("callbacksDetached", detached)
            .put("nativeFramesAccepted", false).put("performanceSampleAccepted", false));
      } catch (Throwable ignored) { terminal.putBoolean("failureReceiptUnavailable", true); }
      terminal.putBoolean("frameProbePassed", false); terminal.putString("failedStep", step);
      terminal.putInt("committedFrames", commitCount.get()); finish(Activity.RESULT_CANCELED, terminal);
    } finally {
      if (!closed) detach(false);
      if (monitor != null) removeMonitor(monitor);
    }
  }
}
