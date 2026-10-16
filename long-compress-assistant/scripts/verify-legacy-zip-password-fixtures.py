"""Read-only independent verification using Python's ZIP implementation."""
import hashlib
import json
from pathlib import Path
import zipfile


def verify(root: Path) -> dict:
    expected = (root / "payload.txt").read_bytes()
    manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
    assert hashlib.sha256(expected).hexdigest() == manifest["payload_sha256"]
    originals = {case["encoding"]: case for case in manifest["cases"]}
    cases = [("cp932", "日本語パスワード+!1416"),
             ("cp936", "简体密码测试+!1416"),
             ("cp950", "繁體密碼測試+!1416")]
    results = []
    for encoding, password in cases:
        source = root / f"{encoding}.zip"
        assert hashlib.sha256(source.read_bytes()).hexdigest() == originals[encoding]["zip_sha256"]
        with zipfile.ZipFile(source) as archive:
            assert archive.namelist() == ["payload.txt"]
            entry = archive.getinfo("payload.txt")
            assert entry.flag_bits & 1 and entry.compress_type == zipfile.ZIP_DEFLATED
            assert entry.extra == b"", "platform-specific attributes are not fixture inputs"
            actual = archive.read(entry, pwd=password.encode(encoding))
            assert actual == expected, "complete content and CRC must match"
            rejected = []
            for other in ("utf8", "cp932", "cp936", "cp950", "wrong"):
                if other == encoding:
                    continue
                try:
                    candidate = b"not-the-password" if other == "wrong" else password.encode(other)
                except UnicodeEncodeError:
                    rejected.append(f"{other}:unrepresentable")
                    continue
                try:
                    archive.read(entry, pwd=candidate)
                except (RuntimeError, zipfile.BadZipFile):
                    rejected.append(other)
                else:
                    raise AssertionError(f"{encoding}: incorrect encoding/password accepted: {other}")
        results.append({"encoding": encoding, "password_hex": password.encode(encoding).hex(),
                        "zip_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
                        "zip_bytes": source.stat().st_size, "rejected": rejected})
    return {"verifier": "Python stdlib zipfile (independent of the application)",
            "payload_bytes": len(expected), "payload_sha256": hashlib.sha256(expected).hexdigest(),
            "cases": results}


if __name__ == "__main__":
    root = Path(__file__).resolve().parent.parent / "tests/fixtures/archives/legacy-zip-password-1416"
    print(json.dumps(verify(root), ensure_ascii=False, indent=2))
