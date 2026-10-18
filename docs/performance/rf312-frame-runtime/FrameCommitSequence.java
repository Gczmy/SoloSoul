package org.solosoul.rf312frames;

/** Orders actual native callbacks; this class never schedules or renders a frame. */
final class FrameCommitSequence {
  static final class Ticket {
    final int ordinal;
    final long reservedNs;
    long drawStartedNs;
    long committedNs;
    Ticket(int ordinal, long reservedNs) { this.ordinal = ordinal; this.reservedNs = reservedNs; }
  }

  private final long fenceCompleteNs;
  private final long deadlineNs;
  private Ticket pending;
  private int committed;
  private long previousCommitNs;

  FrameCommitSequence(long fenceCompleteNs, long deadlineNs) {
    require(fenceCompleteNs > 0 && deadlineNs > fenceCompleteNs
        && deadlineNs - fenceCompleteNs <= 45000000000L);
    this.fenceCompleteNs = fenceCompleteNs;
    this.deadlineNs = deadlineNs;
  }

  private static void require(boolean condition) {
    if (!condition) throw new IllegalStateException("native-frame-order-rejected");
  }

  private void clock(long now) {
    require(now >= fenceCompleteNs && now < deadlineNs && now > previousCommitNs);
  }

  // Called from OnPreDraw, before API31 captures the next frame's callback list.
  Ticket reserve(long now) {
    clock(now);
    if (pending != null || committed == 2) return null;
    pending = new Ticket(committed + 1, now);
    return pending;
  }

  // An actual OnDraw is still required. A pre-draw alone cannot become evidence.
  boolean observeDraw(long now) {
    clock(now);
    if (pending == null || pending.drawStartedNs != 0) return false;
    require(now >= pending.reservedNs);
    pending.drawStartedNs = now;
    return true;
  }

  // Called only by registerFrameCommitCallback's actual native callback.
  void commit(Ticket ticket, long now) {
    clock(now);
    require(ticket != null && ticket == pending && ticket.drawStartedNs >= fenceCompleteNs
        && ticket.drawStartedNs > previousCommitNs && now >= ticket.drawStartedNs
        && committed < 2 && ticket.ordinal == committed + 1);
    ticket.committedNs = now;
    previousCommitNs = now;
    committed++;
    pending = null;
  }

  int committedCount() { return committed; }
}
