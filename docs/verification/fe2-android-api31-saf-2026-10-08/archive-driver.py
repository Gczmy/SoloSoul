from pathlib import Path
import json, hashlib, shutil, xml.etree.ElementTree as ET

ROOT = Path('/Users/zzc/PycharmProjects/SoloSoul')
DEST = ROOT / 'docs/verification/fe2-android-api31-saf-2026-10-08'
CP = ROOT / 'docs/verification/fe2-frontend-checkpoint-2026-10-07.json'
def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def write(path, value): path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')
checkpoint = json.loads(CP.read_text())
sources = checkpoint['source_sha256']
assert len(sources) == 107 and all(sha(ROOT / p) == h for p, h in sources.items())
DEST.mkdir(exist_ok=False)
for name in ['solosoul-fe2-android-saf-native-runner-20261008.py', 'solosoul-fe2-android-saf-api31-independent-restore-20261008.py']:
    shutil.copy2(Path('/tmp') / name, DEST / name)
shutil.copy2(Path('/tmp/solosoul-fe2-android-saf-api31-device-ready-20261008.json'), DEST / 'device-ready.json')
builds = []
for suffix in ['api31', 'api31-pixels', 'api31-filter-ab', 'api31-dark-ab', 'api31-surface', 'api31-parent', 'api31-meta-layer']:
    name = 'solosoul-fe2-android-saf-test-build-' + suffix + '-20261008'
    source = Path('/tmp') / name
    report = json.loads((source / 'report.json').read_text())
    assert report['passed'] and report['exit_code'] == 0 and not report['source_differences_after_restore']
    assert len(report['protected_inputs_restored']) == 10 and all(report['protected_inputs_restored'].values())
    assert json.loads((source / 'source.json').read_text())['source_sha256'] == sources
    assert sha(source / 'AndroidHomeInstrumentedTest.kt') == report['temporary_helper_sha256']
    assert sha(Path(report['frozen_apk'])) == report['apk_sha256']
    target = DEST / ('helper-' + suffix)
    target.mkdir()
    for filename in ['report.json', 'source.json', 'build.log', 'AndroidHomeInstrumentedTest.kt']:
        shutil.copy2(source / filename, target / filename)
    shutil.copy2(Path('/tmp') / (name + '.py'), target / 'driver.py')
    builds.append({'group': suffix, 'passed': True, 'apk_sha256': report['apk_sha256'], 'protected_inputs_restored': 10})
groups = []
for label, suffix in [('first-security-failure', 'supported'), ('search-channel-corrected', 'final-supported'),
                      ('pixel-diagnostic', 'diagnostic-supported'), ('direct-fixture-comparison', 'direct-comparison'),
                      ('filter-comparison', 'filter-ab-supported'), ('darkening-comparison', 'dark-ab-supported'),
                      ('surface-comparison', 'surface-supported'), ('parent-comparison', 'parent-supported'),
                      ('meta-layer-comparison', 'meta-layer-supported')]:
    source = Path('/tmp') / ('solosoul-fe2-android-saf-api31-' + suffix + '-20261008')
    report = json.loads((source / 'report.json').read_text())
    native = json.loads((source / 'device-files/report.json').read_text())
    restored = json.loads((source / 'independent-restoration.json').read_text())
    assert report['restored'] and report['blurRestored'] and restored['passed'] and restored['actual_private_entries'] == 2
    assert report['artifacts'][0]['sha256'] == checkpoint['android_saf_path_repair']['production_apk_after']
    assert native['runtime']['api'] == 31 and native['runtime']['webViewVersion'] == '91.0.4472.114'
    suite = ET.parse(source / 'junit.xml').getroot()
    assert suite.attrib['tests'] == '1' and suite.attrib['failures'] == '1' and suite.attrib['skipped'] == '0'
    target = DEST / label
    target.mkdir()
    for filename in ['report.json', 'native.log', 'independent-restoration.json', 'junit.xml']:
        shutil.copy2(source / filename, target / filename)
    files = target / 'device-files'
    files.mkdir()
    for file in (source / 'device-files').iterdir():
        assert file.is_file() and file.suffix in ['.json', '.png', '.txt']
        shutil.copy2(file, files / file.name)
    rows = native['records']
    imports = [r for r in rows if r['stage'] == 'preview-fixtures-encrypted' and r.get('actualSafPicker')]
    if label not in ['first-security-failure', 'direct-fixture-comparison']:
        assert len(imports) == 1 and imports[0]['encrypted'] and imports[0]['savedCount'] == 2
        for index in [1, 2]:
            selected = next(r for r in rows if r['stage'] == 'saf-picker-selected-' + str(index))
            assert selected['actualNativeSelection'] and selected['nodeResource'] == 'android:id/title'
    if label != 'direct-fixture-comparison':
        cleanup = next(r for r in rows if r['stage'] == 'saf-public-fixtures-cleaned')
        assert cleanup['count'] == 2 and all(r['deleted'] == 1 for r in cleanup['entries'])
    groups.append({'group': label, 'native_passed': False, 'tests': 1, 'failures': 1, 'skipped': 0,
                   'actual_saf_import_encrypted': bool(imports), 'actual_private_entries_restored': 2,
                   'records': len(rows), 'pngs': len(list(files.glob('*.png'))),
                   'failure': next(r['reason'] for r in rows if r['stage'] == 'failure'), 'runtime': native['runtime']})
for filename in ['pixel-analysis.json', 'installed-package-check.json', 'owned-emulator-stop.json']:
    shutil.copy2(Path('/tmp/solosoul-fe2-android-saf-api31-meta-layer-supported-20261008') / filename, DEST / filename)
write(DEST / 'acceptance.json', {'passed': False, 'status': 'api31_real_import_verified_preview_color_pending',
    'production_apk_sha256': checkpoint['android_saf_path_repair']['production_apk_after'],
    'source_sha256': sources, 'helper_builds': builds, 'native_groups': groups,
    'scope': 'Actual DocumentsUI TXT/PNG selection, returned original names/content URIs, SOLC encrypted files and actual decrypted text verified after search helper correction. No complete API31 preview pass; diagnostic interventions are not production evidence.',
    'remaining': 'API31 actual picker dark preview color differs from computed theme; fallback/full preview matrix not accepted. Full macOS window/a11y/continuous restore and physical mobile performance still pending.'})
checkpoint['android_api31_saf_followup'] = {'status': 'real_import_verified_preview_color_pending',
    'production_apk_sha256': checkpoint['android_saf_path_repair']['production_apk_after'],
    'source_entries': 107, 'native_groups': groups,
    'evidence_sha256': {str(p.relative_to(ROOT)): sha(p) for p in sorted(DEST.rglob('*')) if p.is_file()}}
checkpoint['remaining_validation'][1] = 'Android当前38482124包在API34/WebView113真实DocumentsUI导入与模糊支持/关闭两组预览通过。API31/WebView91真实TXT/PNG选择、原名/URI和SOLC加密及实际解密文本补证，但选择器返回后暗色预览实际RGB与主题声明不同，严格像素检查未通过；原生9组失败全部保留，含首次助手跨应用按键权限失败和直接夹具通知到期。各组实际2项沙盒和根权限/blur恢复。诊断干预不计产品通过，API31完整预览/fallback、其他API、多图/PDF/大附件、完整字体/IME/生命周期、连续动画和实体性能仍待验；历史包矩阵保持来源。'
write(CP, checkpoint)
print(json.dumps({'public_files': len(checkpoint['android_api31_saf_followup']['evidence_sha256']), 'source_entries': 107, 'native_groups': len(groups)}))
