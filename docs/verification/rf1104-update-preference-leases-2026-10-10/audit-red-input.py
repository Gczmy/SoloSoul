from pathlib import Path
import hashlib, json, subprocess, re

root = Path('D:/SoloSoul')
stage = Path(__file__).parent
source_path = 'tauri/src-tauri/src/commands/update_preferences.rs'
head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
base = subprocess.check_output(['git', 'show', 'HEAD:'+source_path], cwd=root).decode('utf-8').replace('\r\n','\n')
actual = (root/source_path).read_text(encoding='utf-8')
production = lambda text: text.split('#[cfg(test)]\nmod tests {', 1)[0]
assert production(base) == production(actual)
report = (root/'docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md').read_text(encoding='utf-8')
rows = re.findall(r'^\| (\d+) \| \[RF-(\d+)\].*$', report, re.M)
cards = re.findall(r'^### RF-(\d+)\s*$', report, re.M)
assert len(rows) == len(cards) == len(set(cards)) == 294
assert set(cards) == {i for _, i in rows}
output = stage/'red-input-audit.json'
assert not output.exists()
output.write_text(json.dumps({
    'head':head,
    'productionMatchesHead':True,
    'productionSha256NormalizedLf':hashlib.sha256(production(actual).encode('utf-8')).hexdigest(),
    'regressionSourceSha256':hashlib.sha256((root/source_path).read_bytes()).hexdigest(),
    'taskIndexAndCardCount':294,
    'reportSha256':hashlib.sha256((root/'docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md').read_bytes()).hexdigest(),
},indent=2)+'\n',encoding='utf-8')
print(output.read_text(encoding='utf-8'))
