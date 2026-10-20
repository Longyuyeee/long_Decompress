"""Read-only before/after fingerprint of this user's Long Explorer menu entries."""
import hashlib, json, sys, winreg
from pathlib import Path
rows = []
def children(path):
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, path) as key:
            return [winreg.EnumKey(key, n) for n in range(winreg.QueryInfoKey(key)[0])]
    except FileNotFoundError: return []
def collect(path):
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, path) as key:
            rows.append((path, sorted(winreg.EnumValue(key, n) for n in range(winreg.QueryInfoKey(key)[1]))))
    except FileNotFoundError: return
    for child in children(path): collect(path+'\\'+child)
classes = r'Software\Classes'
shells = [classes+r'\*\shell', classes+r'\Directory\shell', classes+r'\Directory\Background\shell',
    classes+r'\Folder\shell', classes+r'\Drive\shell', classes+r'\AllFilesystemObjects\shell',
    r'Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell']
shells.extend(classes+'\\SystemFileAssociations\\'+ext+'\\shell' for ext in children(classes+r'\SystemFileAssociations'))
for shell in shells:
    for name in children(shell):
        if name.lower().startswith('longdecompress'): collect(shell+'\\'+name)
for suffix in ['4A', '4B', '4C']: collect(classes+r'\CLSID\{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D'+suffix+'}')
result = dict(sha256=hashlib.sha256(json.dumps(sorted(rows), ensure_ascii=True, sort_keys=True, default=str).encode()).hexdigest(),
    keys=len(rows), values=sum(len(row[1]) for row in rows), readOnly=True)
target = Path(sys.argv[1]);target.parent.mkdir(parents=True, exist_ok=True)
target.write_text(json.dumps(result, indent=2)+'\n', encoding='utf-8')
if len(sys.argv)>2: assert result == json.loads(Path(sys.argv[2]).read_text(encoding='utf-8')), 'Long menus changed'
print(json.dumps(result))
