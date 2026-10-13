"""Bind the reviewed RF-1104 behavior requirements to actual passing regressions."""
from pathlib import Path
import hashlib
import json
import re

root = Path('D:/SoloSoul')
stage = Path(__file__).parent
source_name = 'tauri/src-tauri/src/commands/update_preferences.rs'
source = (root/source_name).read_bytes()
expected = json.loads((stage/'regression-green.sources.json').read_text(encoding='utf-8'))
assert hashlib.sha256(source).hexdigest() == expected[source_name]
text = source.decode('utf-8')
tests = set(re.findall(r'^    #\[(?:tokio::)?test\]\r?\n    (?:async )?fn (\w+)\(', text, re.M))
output = (stage/'regression-green.stdout.log').read_text(encoding='utf-8')
passed = set(re.findall(r'^test commands::update_preferences::tests::(\w+) \.\.\. ok$', output, re.M))
assert len(tests) == len(passed) == 15 and tests == passed
receipt = json.loads((stage/'regression-green.receipt.json').read_text(encoding='utf-8'))
assert receipt['exitCode'] == 0 and receipt['sourceUnchanged']
requirements = [
    ('Network selection must not retain maintenance admission; same-account re-unlock rejects late writes even while the old Store remains alive',
        ['rf1104_pending_source_selection_does_not_block_password_reunlock']),
    ('Actual synchronous file work retains its activity permit until it finishes',
        ['rf1104_actual_cache_write_retains_activity_until_file_work_finishes']),
    ('Original service release, different root, same-path replacement and maintenance busy reject late cache writes',
        ['rf1104_pending_cache_does_not_keep_service_or_directory_owned',
         'rf1104_late_cache_write_rejects_maintenance_busy_and_replaced_root',
         'rf1104_same_path_replacement_cannot_revive_old_cache_request']),
    ('Account switching preserves both account profile bytes/versions and the public cache',
        ['rf1104_account_switch_rejects_late_account_and_cache_write']),
    ('Normal post-network account/cache persistence, login-before-account cache behavior and busy-load protection remain effective',
        ['rf905_network_delayed_preferences_reacquire_original_store_for_real_write',
         'rf905_locked_source_preferences_preserve_global_cache_with_owned_admission',
         'rf905_busy_preferences_never_fallback_to_unowned_cache_or_old_store']),
    ('Reopening preserves both channels and other settings; unchanged preferences do not advance the profile version; concurrent channels merge',
        ['preferences_survive_reopen_and_preserve_other_settings_and_account_isolation',
         'rf1104_concurrent_source_channels_preserve_account_and_cache_preferences']),
    ('Preferred/expired/disallowed source handling and bounded preferred head start remain unchanged',
        ['successful_preferred_source_skips_other_requests',
         'failed_or_stale_preferred_source_cannot_block_new_version',
         'stalled_preferred_source_has_bounded_head_start',
         'expired_and_disallowed_sources_are_not_prioritized']),
]
assert {test for _, names in requirements for test in names} == passed
value = {
    'task':'RF-1104', 'scope':'Reviewed requirement-to-test mapping for the actual passing targeted suite',
    'sourceFile':source_name, 'sourceSha256':hashlib.sha256(source).hexdigest(),
    'targetedReceiptSha256':hashlib.sha256((stage/'regression-green.receipt.json').read_bytes()).hexdigest(),
    'requirements':[{'requirement':requirement,'passingTests':names,
        'sourceTestLines':{name:len(text[:re.search(r'^    (?:async )?fn '+re.escape(name)+r'\(',text,re.M).start()].splitlines())+1
            for name in names}} for requirement,names in requirements],
    'targetedTestCount':15,
    'fullRustAndFreshNativeJourneyAcceptanceRequiredSeparately':True,
    'rf312TaskClosed':False,
}
with (stage/'acceptance-scope-review.json').open('x',encoding='utf-8',newline='\n') as stream:
    stream.write(json.dumps(value,indent=2)+'\n')
print(json.dumps({'reviewedRequirements':len(requirements),'actualPassingTests':len(passed),
    'fullAcceptanceClaimed':False}))
