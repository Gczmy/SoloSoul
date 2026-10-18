SELECT 'count' AS category, 'process' AS name, COUNT(*) AS value FROM process
UNION ALL SELECT 'count','counter',COUNT(*) FROM counter
UNION ALL SELECT 'count','ftrace_event',COUNT(*) FROM ftrace_event
UNION ALL SELECT 'count','frame_timeline',COUNT(*) FROM actual_frame_timeline_slice
UNION ALL SELECT 'count','clock_snapshot',COUNT(*) FROM clock_snapshot;
SELECT name, COUNT(*) AS event_count FROM ftrace_event GROUP BY name ORDER BY name;
SELECT t.name, COUNT(*) AS samples FROM counter c JOIN process_counter_track t ON t.id=c.track_id WHERE t.name LIKE 'mem.%' GROUP BY t.name ORDER BY t.name;
SELECT clock_id, COUNT(*) AS samples FROM clock_snapshot GROUP BY clock_id ORDER BY clock_id;
SELECT start_ts, end_ts FROM trace_bounds;
SELECT name, severity, source, value FROM stats WHERE value != 0 ORDER BY severity, name, idx;
