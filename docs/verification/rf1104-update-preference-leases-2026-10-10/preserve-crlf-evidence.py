"""Recognize retained CRLF evidence without changing any diagnostic bytes."""
from pathlib import Path
import hashlib
import json

root = Path('D:/SoloSoul')
stage = Path(__file__).parent
archive = root/'docs/verification/rf1104-update-preference-leases-2026-10-10'
index_path = archive.with_suffix('.json')
index = json.loads(index_path.read_text(encoding='utf-8'))
assert index['task'] == 'RF-1104' and index['validationAccepted']
records = index['records']
sha = lambda raw: hashlib.sha256(raw).hexdigest()

def record_for(path):
    name = path.relative_to(root/'docs').as_posix()
    matches = [record for record in records if record['path'] == name]
    assert len(matches) == 1
    record = matches[0]
    raw = path.read_bytes()
    assert sha(raw) == record['storedSha256'] and len(raw) == record['storedBytes']
    assert record['originalSha256'] == record['storedSha256']
    return record, raw

def metadata(path, raw, source):
    return {'path':path.relative_to(root/'docs').as_posix(),'source':source,
        'originalBytes':len(raw),'originalSha256':sha(raw),
        'storedBytes':len(raw),'storedSha256':sha(raw)}

def preserve(path, raw, source):
    assert path.resolve().is_relative_to(archive.resolve())
    assert not path.exists()
    path.parent.mkdir(parents=True,exist_ok=True)
    with path.open('xb') as stream:
        stream.write(raw)
    assert path.read_bytes() == raw
    records.append(metadata(path,raw,source))

attributes = archive/'.gitattributes'
attribute_record, old_attributes = record_for(attributes)
assert old_attributes == b'# Preserve original evidence bytes across Windows/Unix checkouts.\n* -text\n'
old_package_record, old_package = record_for(archive/'package-evidence.py')
preserve(archive/'policy-before/evidence-attributes.txt',old_attributes,
    'generated: initial evidence byte-preservation rule, before CRLF whitespace recognition')
preserve(archive/'policy-before/package-evidence.py',old_package,
    'package-evidence.py before CRLF whitespace recognition')
new_attributes = (b'# Preserve original evidence bytes across Windows/Unix checkouts.\n'
    b'* -text whitespace=blank-at-eol,blank-at-eof,space-before-tab,cr-at-eol\n')
attributes.write_bytes(new_attributes)
attribute_record.update(metadata(attributes,new_attributes,attribute_record['source']))
new_package = (stage/'package-evidence.py').read_bytes()
assert b'whitespace=blank-at-eol,blank-at-eof,space-before-tab,cr-at-eol' in new_package
(archive/'package-evidence.py').write_bytes(new_package)
old_package_record.update(metadata(archive/'package-evidence.py',new_package,'package-evidence.py'))
preserve(archive/'preserve-crlf-evidence.py',Path(__file__).read_bytes(),'preserve-crlf-evidence.py')
index['evidenceBytePolicy'] = {
    'textConversionDisabled':True,'crlfRecognizedAsLineEnding':True,
    'whitespaceRules':['blank-at-eol','blank-at-eof','space-before-tab','cr-at-eol'],
    'originalDiagnosticFilesUnmodified':True,
    'initialGeneratedPolicyAndArchiveHelperPreservedUnder':'policy-before/',
}
index_path.write_text(json.dumps(index,indent=2,ensure_ascii=False)+'\n',encoding='utf-8',newline='\n')
report = root/'docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md'
title = '### RF-1104 归档 CRLF 暂存检查（2026-10-10）'
data = report.read_bytes()
assert title.encode('utf-8') not in data
entry = '\n\n'+title+'\n\n'
entry += '- 首次git diff --cached --check将保留原始CRLF的JSON证据行尾CR识别为尾空白；原始诊断数据不作换行转换。仅本项证据目录的.gitattributes补充cr-at-eol，同时保留blank-at-eol、blank-at-eof和space-before-tab检查，继续禁用text转换；初始生成规则与归档脚本保留于policy-before。随后重新暂存并核对Git对象中的实际原始字节。\n'
report.write_bytes(data+entry.replace('\n','\r\n').encode('utf-8'))
print(json.dumps({'task':'RF-1104','records':len(records),'diagnosticBytesChanged':False,
    'whitespacePolicyScopedToEvidence':True,'originalGeneratedPolicyPreserved':True}))
