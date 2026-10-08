from pathlib import Path
import json, hashlib, shutil, xml.etree.ElementTree as ET
from datetime import datetime, timezone
from PIL import Image
from collections import Counter
ROOT=Path('/Users/zzc/PycharmProjects/SoloSoul')
DEST=ROOT/'docs/verification/fe2-android-api31-paint-2026-10-08'
CP=ROOT/'docs/verification/fe2-frontend-checkpoint-2026-10-07.json'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def write(p,v):p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n')
cp=json.loads(CP.read_text());sources=cp['source_sha256'];assert len(sources)==107
assert all(sha(ROOT/p)==h for p,h in sources.items())
assert not DEST.exists()
rows=[];builds=[]
for suffix in ['paint','underlying-blur','underlying-blur-restored']:
 b=Path('/tmp/solosoul-fe2-android-saf-test-build-api31-'+suffix+'-20261008')
 br=json.loads((b/'report.json').read_text());assert br['passed'] and br['exit_code']==0 and not br['source_differences_after_restore']
 assert all(br['protected_inputs_restored'].values()) and len(br['protected_inputs_restored'])==10
 assert json.loads((b/'source.json').read_text())['source_sha256']==sources
 assert sha(Path(br['frozen_apk']))==br['apk_sha256']
 n=Path('/tmp/solosoul-fe2-android-saf-api31-'+suffix+'-supported-20261008')
 nr=json.loads((n/'report.json').read_text());native=json.loads((n/'device-files/report.json').read_text())
 assert nr['restored'] and nr['blurRestored'] and json.loads((n/'independent-restoration.json').read_text())['passed']
 assert nr['artifacts'][0]['sha256']==cp['android_saf_path_repair']['production_apk_after']
 assert native['runtime']['api']==31 and native['runtime']['webViewVersion']=='91.0.4472.114'
 encrypted=[r for r in native['records'] if r['stage']=='preview-fixtures-encrypted' and r.get('actualSafPicker')];assert len(encrypted)==1 and encrypted[0]['encrypted'] and encrypted[0]['savedCount']==2
 clean=next(r for r in native['records'] if r['stage']=='saf-public-fixtures-cleaned');assert clean['count']==2 and all(r['deleted']==1 for r in clean['entries'])
 suite=ET.parse(n/'junit.xml').getroot();assert suite.attrib['tests']=='1' and suite.attrib['failures']=='1' and suite.attrib['skipped']=='0'
 fail=next(r['reason'] for r in native['records'] if r['stage']=='failure')
 rows.append({'group':suffix,'passed':False,'tests':1,'failures':1,'skipped':0,'failure':fail,'runtime':native['runtime'],'actual_saf_import_encrypted':True,'actual_private_entries_restored':2})
 builds.append({'group':suffix,'passed':True,'apk_sha256':br['apk_sha256'],'protected_inputs_restored':10})
DEST.mkdir()
for suffix in ['paint','underlying-blur','underlying-blur-restored']:
 b=Path('/tmp/solosoul-fe2-android-saf-test-build-api31-'+suffix+'-20261008');t=DEST/('helper-'+suffix);t.mkdir()
 for f in ['source.json','report.json','build.log','AndroidHomeInstrumentedTest.kt']:shutil.copy2(b/f,t/f)
 shutil.copy2(Path(str(b)+'.py'),t/'driver.py')
 n=Path('/tmp/solosoul-fe2-android-saf-api31-'+suffix+'-supported-20261008');t=DEST/suffix;t.mkdir()
 for f in ['report.json','native.log','junit.xml','independent-restoration.json']:shutil.copy2(n/f,t/f)
 (t/'device-files').mkdir()
 for p in (n/'device-files').iterdir():
  assert p.is_file() and p.suffix in ['.json','.png','.txt'];shutil.copy2(p,t/'device-files'/p.name)
for f in ['solosoul-fe2-android-saf-native-runner-20261008.py','solosoul-fe2-android-saf-api31-independent-restore-20261008.py']:shutil.copy2(Path('/tmp')/f,DEST/f)
for f in ['installed-package-check.json','owned-emulator-stop.json']:
 shutil.copy2(Path('/tmp/solosoul-fe2-android-saf-api31-underlying-blur-restored-supported-20261008')/f,DEST/f)
shutil.copy2(Path(__file__),DEST/'archive-driver.py')
analysis=[]
for suffix in ['paint','underlying-blur','underlying-blur-restored']:
 d=DEST/suffix/'device-files'
 for name in ['file-preview-text-dark-pixel-diagnostic.png','file-preview-text-dark-without-underlying-blur.png']:
  p=d/name
  if not p.exists():continue
  im=Image.open(p).convert('RGB');pixels=list(im.crop((36,89,163,216)).get_flattened_data());c=Counter(pixels)
  analysis.append({'group':suffix,'frame':name,'back_region':[36,89,163,216],'most_frequent_colors':c.most_common(2),'expected_foreground':[221,216,200],'expected_ink_pixels':sum(all(abs(rgb[i]-[221,216,200][i])<=8 for i in range(3)) for rgb in pixels)})
write(DEST/'pixel-analysis.json',{'scope':'Original Back button region; intervention frames diagnostic only, no product acceptance','frames':analysis})
write(DEST/'acceptance.json',{'passed':False,'status':'preview_color_pending_after_paint_coverage_diagnostics','source_sha256':sources,'production_apk_sha256':cp['android_saf_path_repair']['production_apk_after'],'helper_builds':builds,'native_groups':rows,'scope':'Read-only bounds/styles including pointer-events:none and before/after pseudo-elements; temporary underlying blur A/B. Original color gate unchanged. Each run real DocumentsUI import and SOLC verified, private data independently restored. No full API31 preview pass.'})
(DEST/'README.md').write_text('''# FE2 Android API31 绘制覆盖层诊断

2026-10-08；当前产品源码 107 项、生产 APK `384821244478eaa76ee9bd25c8902dd8344b123c747b1f5b27c6fdd6847af118`。没有修改产品代码。

- 实际 Android API31 / WebView 91.0.4472.114，SoloSoul_FE2_API31 / emulator-5588；三组原生诊断各 1 项失败、0 跳过，不计客户端验收通过。
- 真实 DocumentsUI 选择 TXT / PNG、原始名称及 content URI、非零附件、SOLC 加密和解密文本均再次取得证据。
- 绘制检查枚举覆盖返回按钮、标题、正文中心点的所有可见 DOM，包括不接受命中测试的节点，并记录所有有内容的 before/after 伪元素。未发现额外覆盖预览顶栏和正文的指针穿透遮罩；底部安全区伪元素仅位于底部。边界及 CSS 枚举不等于完整浏览器内部绘制跟踪。
- 下层 AppBar、底部导航、附件浮层模糊暂时关闭后的同区域仍为前景 RGB(205,200,185)、背景 RGB(39,35,30)，没有恢复声明的 RGB(221,216,200) / RGB(42,38,32)。该对照不支持修改导航玻璃来修复此问题。
- 第一版下层模糊对照的 style 原始字符串恢复检查失败，完整保留；后续标准化 CSS 属性恢复核验通过：HEADER / NAV 从无 style 属性变为空 style 属性，CSS 内容相同；三层实际 blur 回到原值。后续仍失败于原预览像素判据。各组实际测试沙盒与根权限均独立确认恢复，公开 MediaStore 夹具各删除 2 个。
- APK、完整私有目录 tar、受保护备份仅保留 /tmp。本目录仅公开合成夹具的日志、JSON、JUnit、PNG、脚本和助手源码。

仍需定位颜色差异；不提高文字颜色迎合截图、不放宽像素门槛、不把诊断干预计为产品通过。macOS 完整材质/窗口恢复与实体移动设备性能仍待验。
''')
cp['android_api31_paint_followup']={'status':'preview_color_pending_after_paint_coverage_diagnostics','source_entries':107,'production_apk_sha256':cp['android_saf_path_repair']['production_apk_after'],'native_groups':rows,'evidence_sha256':{str(p.relative_to(ROOT)):sha(p) for p in sorted(DEST.rglob('*')) if p.is_file()}}
write(CP,cp)
print(json.dumps({'public_files':len(cp['android_api31_paint_followup']['evidence_sha256']),'native_groups':rows},ensure_ascii=False))
