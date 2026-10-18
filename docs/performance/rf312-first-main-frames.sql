-- Use only after the original capture, process-identity, clock and UI readers pass.
-- A main process must be unique; these frames do not prove business readiness.
WITH target AS (
  SELECT MIN(upid) AS upid FROM process
  WHERE name='com.solosoul.app' AND uid>=10000 HAVING COUNT(*)=1
), app AS (
  SELECT a.*,p.pid,p.uid,p.start_ts AS process_birth,
    EXTRACT_ARG(a.arg_set_id,'Is Buffer?') AS is_buffer,
    (SELECT COUNT(*) FROM expected_frame_timeline_slice e
      WHERE e.upid=a.upid AND e.surface_frame_token=a.surface_frame_token) AS expected_matches
  FROM actual_frame_timeline_slice a JOIN process p USING(upid) JOIN target USING(upid)
  WHERE a.layer_name='TX - com.solosoul.app/com.solosoul.app.MainActivity#0'
), sf AS (
  SELECT s.* FROM actual_frame_timeline_slice s JOIN process p USING(upid)
  WHERE p.name='/system/bin/surfaceflinger' AND p.uid=1000 AND s.layer_name IS NULL
)
SELECT a.id,a.upid,a.pid,a.uid,a.process_birth,a.ts,a.dur,a.surface_frame_token,a.display_frame_token,
  a.present_type,a.prediction_type,a.jank_type,a.is_buffer,a.expected_matches,
  (SELECT COUNT(*) FROM sf WHERE sf.display_frame_token=a.display_frame_token) AS display_matches,
  (SELECT MIN(sf.ts) FROM sf WHERE sf.display_frame_token=a.display_frame_token) AS display_start,
  (SELECT MIN(sf.dur) FROM sf WHERE sf.display_frame_token=a.display_frame_token) AS display_duration,
  (SELECT MIN(sf.ts+sf.dur) FROM sf WHERE sf.display_frame_token=a.display_frame_token AND sf.dur>=0) AS display_end
FROM app a ORDER BY a.ts,a.id LIMIT 20;

SELECT p.upid,p.pid,p.uid,p.start_ts,p.name FROM process p
WHERE p.name='com.solosoul.app' AND p.uid>=10000;

SELECT clock_id,COUNT(*) AS samples,MIN(ts-clock_value) AS minimum_offset,
  MAX(ts-clock_value) AS maximum_offset FROM clock_snapshot WHERE clock_id=6 GROUP BY clock_id;

SELECT s.surface_frame_token,COUNT(*) AS frames FROM actual_frame_timeline_slice s
JOIN process p USING(upid) WHERE p.name='/system/bin/surfaceflinger'
GROUP BY s.surface_frame_token;
