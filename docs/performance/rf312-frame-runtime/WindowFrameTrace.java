package org.solosoul.rf312frames;

import android.app.Activity;
import android.app.Application;
import android.content.Context;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.HandlerThread;
import android.view.FrameMetrics;
import android.view.View;
import android.view.ViewTreeObserver;
import android.view.Window;
import java.util.ArrayList;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import org.json.JSONArray;
import org.json.JSONObject;

/** Optional passive diagnostics. It never contributes a native commit. */
final class WindowFrameTrace implements Application.ActivityLifecycleCallbacks,
    Window.OnFrameMetricsAvailableListener {
  static final int LIMIT = 64;
  private static final int COUNT_LIMIT = 1000000;
  private static final long READ_NS = 3000000000L;
  private final Object lock = new Object();
  private final Handler main;
  private final long startedNs;
  private final long deadlineNs;
  private final String run;
  private final JSONObject process;
  private final String targetSha;
  private final String collectorSha;
  private static final class TraceThread extends HandlerThread {
    final CountDownLatch prepared = new CountDownLatch(1);
    TraceThread() { super("rf312-window-frames"); }
    @Override protected void onLooperPrepared() { prepared.countDown(); }
  }
  private final TraceThread worker = new TraceThread();
  private Handler metricsHandler;
  private final ArrayList<JSONObject> metrics = new ArrayList<>();
  private Application application;
  private Activity activity;
  private Window window;
  private View decor;
  private ViewTreeObserver installedTree;
  private ViewTreeObserver.OnPreDrawListener preDraw;
  private ViewTreeObserver.OnDrawListener draw;
  private boolean closing, lifecycleInstalled, windowInstalled, detached, threadTerminated, cleanupBudgetMet;
  private boolean targetObserved, treeCompared, treeMatches;
  private boolean metricsAdded, preDrawAdded, drawAdded;
  private String hookFrom = "none", fault = "none";
  private long lifecycleNs, hookNs, fenceNs, detachStartedNs, stoppedNs, cleanupFinishedNs;
  private int preDrawCount, drawCount, preDrawAfterFence, drawAfterFence;
  private int callbackCount, callbacksAfterFence, dropCount, lateMetrics, lateUi;
  private long firstPreDrawNs, lastPreDrawNs, firstDrawNs, lastDrawNs;

  WindowFrameTrace(String run, JSONObject process, String targetSha, String collectorSha,
      long startedNs, long deadlineNs, Handler main) {
    this.run = run; this.process = process; this.targetSha = targetSha; this.collectorSha = collectorSha;
    this.startedNs = startedNs; this.deadlineNs = deadlineNs; this.main = main;
  }

  private void require(boolean condition) {
    if (!condition) throw new IllegalStateException("window-trace-contract-rejected");
  }
  private int increment(int value) { require(value < COUNT_LIMIT); return value + 1; }

  void start(Context context) throws Exception {
    require(context instanceof Application && !worker.isAlive());
    application = (Application) context;
    long startDeadline = Math.min(deadlineNs, System.nanoTime() + READ_NS);
    worker.start();
    long prepareLeft = startDeadline - System.nanoTime();
    require(prepareLeft > 0 && worker.prepared.await(prepareLeft, TimeUnit.NANOSECONDS));
    metricsHandler = new Handler(worker.getLooper());
    CountDownLatch done = new CountDownLatch(1);
    main.post(() -> {
      try {
        synchronized (lock) {
          require(!closing && System.nanoTime() < deadlineNs);
          application.registerActivityLifecycleCallbacks(this);
          lifecycleNs = System.nanoTime(); lifecycleInstalled = true;
        }
      } catch (Throwable error) { synchronized (lock) { fault = "setup-rejected"; } }
      finally { done.countDown(); }
    });
    long left = startDeadline - System.nanoTime();
    require(left > 0 && done.await(left, TimeUnit.NANOSECONDS));
    synchronized (lock) { require(lifecycleInstalled && fault.equals("none")); }
  }

  private void hook(Activity next, String origin) {
    if (!next.getClass().getName().equals("com.solosoul.app.MainActivity")) return;
    synchronized (lock) {
      if (closing || System.nanoTime() >= deadlineNs) return;
      try {
        require(activity == null || activity == next); activity = next; targetObserved = true;
        if (windowInstalled) return;
        Window candidate = next.getWindow();
        // Never request a Decor that the application has not created itself.
        if (candidate == null || candidate.peekDecorView() == null) return;
        window = candidate; decor = candidate.peekDecorView();
        installedTree = decor.getViewTreeObserver(); require(installedTree.isAlive());
        preDraw = () -> { observeUi(false); return true; };
        draw = () -> observeUi(true);
        installedTree.addOnPreDrawListener(preDraw); preDrawAdded = true;
        installedTree.addOnDrawListener(draw); drawAdded = true;
        window.addOnFrameMetricsAvailableListener(this, metricsHandler);
        metricsAdded = true;
        require(window.peekDecorView() == decor);
        hookNs = System.nanoTime(); hookFrom = origin; windowInstalled = true;
      } catch (Throwable error) { fault = "hook-rejected"; }
    }
  }

  private void observeUi(boolean drawing) {
    synchronized (lock) {
      if (closing) return;
      long now = System.nanoTime();
      try {
        if (now >= deadlineNs) { lateUi = increment(lateUi); return; }
        require(activity != null && activity.getWindow() == window && window.peekDecorView() == decor);
        if (drawing) {
          drawCount = increment(drawCount); if (firstDrawNs == 0) firstDrawNs = now; lastDrawNs = now;
          if (fenceNs > 0 && now >= fenceNs) drawAfterFence = increment(drawAfterFence);
        } else {
          preDrawCount = increment(preDrawCount); if (firstPreDrawNs == 0) firstPreDrawNs = now; lastPreDrawNs = now;
          if (fenceNs > 0 && now >= fenceNs) preDrawAfterFence = increment(preDrawAfterFence);
        }
      } catch (Throwable error) { fault = "ui-observation-rejected"; }
    }
  }

  long fenceCompleted(View webView) {
    synchronized (lock) {
      require(!closing && fenceNs == 0 && System.nanoTime() < deadlineNs);
      if (decor != null) {
        treeCompared = true;
        treeMatches = webView.getViewTreeObserver() == decor.getViewTreeObserver();
      }
      // Delivery and UI callback clocks share this exact synchronized boundary.
      fenceNs = System.nanoTime(); return fenceNs;
    }
  }

  @Override public void onFrameMetricsAvailable(Window observed, FrameMetrics reusable, int dropped) {
    synchronized (lock) {
      if (closing) return;
      long now = System.nanoTime();
      try {
        if (now >= deadlineNs) { lateMetrics = increment(lateMetrics); return; }
        require(observed == window && dropped >= 0 && dropped <= COUNT_LIMIT);
        require(dropCount <= COUNT_LIMIT - dropped); dropCount += dropped;
        callbackCount = increment(callbackCount);
        if (fenceNs > 0 && now >= fenceNs) callbacksAfterFence = increment(callbacksAfterFence);
        if (metrics.size() == LIMIT) return;
        FrameMetrics copy = new FrameMetrics(reusable);
        metrics.add(new JSONObject().put("ordinal", callbackCount).put("deliveredNs", now)
            .put("intendedVsyncNs", copy.getMetric(FrameMetrics.INTENDED_VSYNC_TIMESTAMP))
            .put("vsyncNs", copy.getMetric(FrameMetrics.VSYNC_TIMESTAMP))
            .put("totalDurationNs", copy.getMetric(FrameMetrics.TOTAL_DURATION))
            .put("drawDurationNs", copy.getMetric(FrameMetrics.DRAW_DURATION))
            .put("swapDurationNs", copy.getMetric(FrameMetrics.SWAP_BUFFERS_DURATION))
            .put("firstDrawFlag", copy.getMetric(FrameMetrics.FIRST_DRAW_FRAME))
            .put("droppedSincePrevious", dropped));
      } catch (Throwable error) { fault = "metrics-observation-rejected"; }
    }
  }

  void beginClose(long now) {
    synchronized (lock) { if (!closing) { closing = true; detachStartedNs = now; stoppedNs = System.nanoTime(); } }
  }

  void removeOnMain() {
    synchronized (lock) {
      try {
        if (application != null && lifecycleInstalled) application.unregisterActivityLifecycleCallbacks(this);
        if (window != null && metricsAdded) window.removeOnFrameMetricsAvailableListener(this);
        // Floating observers can be merged into the attached root; remove from
        // both surviving observers, without creating a new Decor or hierarchy.
        ViewTreeObserver current = decor == null ? null : decor.getViewTreeObserver();
        for (ViewTreeObserver observer : new ViewTreeObserver[] {installedTree, current}) {
          if (observer == null || !observer.isAlive()) continue;
          if (preDrawAdded) observer.removeOnPreDrawListener(preDraw);
          if (drawAdded) observer.removeOnDrawListener(draw);
        }
        detached = true;
      } catch (Throwable error) { fault = "listener-detach-rejected"; }
      finally { if (worker.isAlive()) worker.quitSafely(); }
    }
  }

  boolean finishClose(long stopDeadlineNs) {
    try {
      long left = stopDeadlineNs - System.nanoTime();
      if (worker.isAlive() && left > 0) TimeUnit.NANOSECONDS.timedJoin(worker, left);
    } catch (InterruptedException error) { Thread.currentThread().interrupt(); }
    synchronized (lock) {
      threadTerminated = !worker.isAlive(); cleanupFinishedNs = System.nanoTime();
      cleanupBudgetMet = stoppedNs > 0 && cleanupFinishedNs <= stopDeadlineNs;
      return detached && threadTerminated && cleanupBudgetMet;
    }
  }

  JSONObject snapshot() throws Exception {
    synchronized (lock) {
      return new JSONObject().put("schemaVersion", 1).put("scope", "passive-window-frame-diagnostics")
          .put("runId", run).put("process", process).put("targetApkSha256", targetSha).put("collectorApkSha256", collectorSha)
          .put("api", Build.VERSION.SDK_INT).put("startedNs", startedNs).put("deadlineNs", deadlineNs)
          .put("detachStartedNs", detachStartedNs).put("captureStoppedNs", stoppedNs).put("cleanupFinishedNs", cleanupFinishedNs)
          .put("lifecycleHookNs", lifecycleNs).put("windowHookNs", hookNs).put("fenceCompleteNs", fenceNs)
          .put("targetActivityObserved", targetObserved).put("lifecycleHookInstalled", lifecycleInstalled)
          .put("windowHookInstalled", windowInstalled).put("hookFrom", hookFrom).put("fault", fault)
          .put("callbacksDetached", detached).put("metricsThreadTerminated", threadTerminated).put("cleanupBudgetMet", cleanupBudgetMet)
          .put("webViewTreeCompared", treeCompared).put("webViewTreeMatchesDecorTree", treeMatches)
          .put("preDrawCount", preDrawCount).put("drawCount", drawCount).put("preDrawAfterFence", preDrawAfterFence)
          .put("drawAfterFence", drawAfterFence).put("metricsCallbackCount", callbackCount)
          .put("metricsCallbacksAfterFence", callbacksAfterFence).put("reportedMetricDrops", dropCount)
          .put("lateMetricsCallbacks", lateMetrics).put("lateUiCallbacks", lateUi)
          .put("firstPreDrawNs", firstPreDrawNs).put("lastPreDrawNs", lastPreDrawNs)
          .put("firstDrawNs", firstDrawNs).put("lastDrawNs", lastDrawNs)
          .put("storedMetricLimit", LIMIT).put("metricPrefixTruncated", callbackCount > LIMIT).put("metrics", new JSONArray(metrics))
          .put("nativeClock", "System.nanoTime; delivery and UI callbacks only; FrameMetrics clocks unconverted")
          .put("explicitDrawRequestsDispatched", false).put("forcedInvalidation", false).put("inputDispatched", false)
          .put("businessIpcDispatched", false).put("explicitDecorCreationRequested", false)
          .put("nativeFramesAccepted", false).put("applicationOnlyLatencyAccepted", false)
          .put("defaultPerformanceSampleAccepted", false).put("physicalDisplayPresentationProven", false).put("fullRF312Accepted", false);
    }
  }

  @Override public void onActivityPostCreated(Activity a, Bundle b) { hook(a, "post-created"); }
  @Override public void onActivityStarted(Activity a) { hook(a, "started"); }
  @Override public void onActivityResumed(Activity a) { hook(a, "resumed"); }
  @Override public void onActivityCreated(Activity a, Bundle b) { }
  @Override public void onActivityPaused(Activity a) { }
  @Override public void onActivityStopped(Activity a) { }
  @Override public void onActivitySaveInstanceState(Activity a, Bundle b) { }
  @Override public void onActivityDestroyed(Activity a) { }
}
