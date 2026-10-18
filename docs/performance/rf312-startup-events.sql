INCLUDE PERFETTO MODULE android.startup.startups;

SELECT startup_id, ts, ts_end, dur, package, startup_type
FROM android_startups
ORDER BY ts, startup_id;

SELECT s.ts, s.dur, s.name, t.utid, t.tid, p.upid, p.pid, p.uid, p.name AS process_name
FROM slice s
JOIN thread_track tt ON tt.id = s.track_id
JOIN thread t ON t.utid = tt.utid
JOIN process p ON p.upid = t.upid
WHERE s.name GLOB 'MetricsLogger:*'
   OR s.name GLOB 'launching: *'
   OR s.name GLOB 'launchingActivity#*'
ORDER BY s.ts, s.id;

SELECT startup_id, upid, pid, startup_type
FROM android_startup_processes
ORDER BY startup_id, upid;
