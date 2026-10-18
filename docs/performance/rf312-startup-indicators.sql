INCLUDE PERFETTO MODULE android.startup.startups;
SELECT COUNT(*) AS parsed_activity_startups FROM android_startups;
SELECT COUNT(*) AS target_activity_startups FROM android_startups WHERE package='com.solosoul.app';
SELECT COUNT(*) AS activity_indicator_slices FROM slice
WHERE name IN ('bindApplication','activityStart','activityResume')
   OR name GLOB 'launchingActivity#*:*' OR name GLOB 'MetricsLogger:*';
