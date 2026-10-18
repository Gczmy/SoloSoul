package org.solosoul.rf312frames;

import java.util.ArrayList;

/** Host-side order regression. No native/rendering success is asserted here. */
public final class FrameCommitSequenceTest {
  private static int tests;
  private static void check(boolean ok) { if (!ok) throw new AssertionError(); }
  private static void rejects(Runnable action) {
    try { action.run(); } catch (IllegalStateException expected) { return; }
    throw new AssertionError("Expected rejection");
  }
  // Model only the independently inspected API31 traversal order:
  // preDraw -> capture callbacks -> OnDraw -> actual native commit notification.
  private static final class Traversal {
    final ArrayList<Runnable> pending = new ArrayList<>();
    void frame(Runnable preDraw, Runnable draw) {
      preDraw.run();
      ArrayList<Runnable> captured = new ArrayList<>(pending); pending.clear();
      draw.run();
      for (Runnable callback : captured) callback.run();
    }
  }

  public static void main(String[] ignored) {
    {
      Traversal t = new Traversal(); int[] commits = {0};
      t.frame(() -> {}, () -> t.pending.add(() -> commits[0]++));
      check(commits[0] == 0 && t.pending.size() == 1); tests++;
    }
    {
      Traversal t = new Traversal(); FrameCommitSequence s = new FrameCommitSequence(100, 1000);
      FrameCommitSequence.Ticket[] ticket = {null};
      t.frame(() -> { ticket[0] = s.reserve(110); t.pending.add(() -> s.commit(ticket[0], 130)); },
          () -> check(s.observeDraw(120)));
      check(s.committedCount() == 1 && ticket[0].drawStartedNs == 120 && ticket[0].committedNs == 130); tests++;
    }
    {
      FrameCommitSequence s = new FrameCommitSequence(100, 1000);
      FrameCommitSequence.Ticket first = s.reserve(110);
      check(s.reserve(111) == null && s.committedCount() == 0);
      rejects(() -> s.commit(first, 120)); check(s.committedCount() == 0); tests++;
    }
    {
      FrameCommitSequence s = new FrameCommitSequence(100, 1000);
      rejects(() -> s.reserve(99)); rejects(() -> s.reserve(1000));
      check(s.committedCount() == 0); tests++;
    }
    {
      FrameCommitSequence s = new FrameCommitSequence(100, 1000);
      FrameCommitSequence.Ticket first = s.reserve(110); check(s.observeDraw(120)); s.commit(first, 130);
      check(s.committedCount() == 1); rejects(() -> s.reserve(130));
      FrameCommitSequence.Ticket second = s.reserve(140);
      check(second.ordinal == 2); check(s.observeDraw(150)); s.commit(second, 160);
      check(s.committedCount() == 2 && s.reserve(170) == null && second.drawStartedNs > first.committedNs); tests++;
    }
    {
      FrameCommitSequence s = new FrameCommitSequence(100, 1000);
      FrameCommitSequence.Ticket first = s.reserve(110); s.observeDraw(120); s.commit(first, 130);
      rejects(() -> s.commit(first, 140));
      FrameCommitSequence.Ticket second = s.reserve(150); s.observeDraw(160);
      rejects(() -> s.commit(first, 170)); s.commit(second, 180); check(s.committedCount() == 2); tests++;
    }
    {
      FrameCommitSequence s = new FrameCommitSequence(100, 1000);
      FrameCommitSequence.Ticket first = s.reserve(110); s.observeDraw(120);
      rejects(() -> s.commit(first, 1000)); check(s.committedCount() == 0); tests++;
    }
    {
      FrameCommitSequence s = new FrameCommitSequence(100, 1000);
      FrameCommitSequence.Ticket first = s.reserve(110);
      // A cancelled pre-draw retains one reservation, never invents an OnDraw.
      check(s.reserve(120) == null && first.drawStartedNs == 0);
      s.observeDraw(130); s.commit(first, 140); check(s.committedCount() == 1); tests++;
    }
    {
      FrameCommitSequence s = new FrameCommitSequence(100, 1000);
      FrameCommitSequence.Ticket first = s.reserve(110);
      check(s.observeDraw(120)); check(!s.observeDraw(130));
      s.commit(first, 140); check(first.drawStartedNs == 120 && s.committedCount() == 1); tests++;
    }
    {
      rejects(() -> new FrameCommitSequence(0, 1000));
      rejects(() -> new FrameCommitSequence(100, 100));
      rejects(() -> new FrameCommitSequence(100, 45000000101L)); tests++;
    }
    {
      FrameCommitSequence s = new FrameCommitSequence(100, 1000);
      FrameCommitSequence.Ticket first = s.reserve(150);
      rejects(() -> s.observeDraw(140)); check(first.drawStartedNs == 0 && s.committedCount() == 0); tests++;
    }
    check(tests == 11);
    System.out.println("FrameCommitSequence: 11 tests passed");
  }
}
