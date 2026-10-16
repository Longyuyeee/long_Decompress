"""Observe real Shell -> UI queue -> verification/publication -> history flows.

No IPC test bridge, seeded task status, global settings or personal profile writes.
Win32 counters are logical process counters, not physical disk traffic. Sampled
simultaneous working sets double-count shared pages; never sum lifetime peaks.
"""
import argparse
import ctypes as C
from ctypes import wintypes as W
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import shutil
import sqlite3
import statistics
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent.parent
KERNEL = C.WinDLL("kernel32", use_last_error=True)
PSAPI = C.WinDLL("psapi", use_last_error=True)


class ProcessEntry(C.Structure):
    _fields_ = [("size", W.DWORD), ("usage", W.DWORD), ("pid", W.DWORD),
                ("heap", C.c_size_t), ("module", W.DWORD), ("threads", W.DWORD),
                ("parent", W.DWORD), ("priority", W.LONG), ("flags", W.DWORD),
                ("name", W.WCHAR * 260)]


class Memory(C.Structure):
    _fields_ = [("size", W.DWORD), ("faults", W.DWORD)] + [
        (name, C.c_size_t) for name in ["peak_ws", "ws", "peak_paged", "paged",
                                       "peak_nonpaged", "nonpaged", "pagefile",
                                       "peak_pagefile", "private"]]


class IO(C.Structure):
    _fields_ = [(name, C.c_ulonglong) for name in ["reads", "writes", "other",
                                                 "read_bytes", "write_bytes", "other_bytes"]]


KERNEL.CreateToolhelp32Snapshot.argtypes = [W.DWORD, W.DWORD]
KERNEL.CreateToolhelp32Snapshot.restype = W.HANDLE
KERNEL.Process32FirstW.argtypes = [W.HANDLE, C.POINTER(ProcessEntry)]
KERNEL.Process32NextW.argtypes = [W.HANDLE, C.POINTER(ProcessEntry)]
KERNEL.OpenProcess.argtypes = [W.DWORD, W.BOOL, W.DWORD]
KERNEL.OpenProcess.restype = W.HANDLE
KERNEL.CloseHandle.argtypes = [W.HANDLE]
KERNEL.GetProcessTimes.argtypes = [W.HANDLE] + [C.POINTER(W.FILETIME)] * 4
KERNEL.GetProcessIoCounters.argtypes = [W.HANDLE, C.POINTER(IO)]
PSAPI.GetProcessMemoryInfo.argtypes = [W.HANDLE, C.POINTER(Memory), W.DWORD]


def tree_ids(root_pid, entries):
    selected = {root_pid}
    while True:
        expanded = selected | {pid for pid, parent, _ in entries if parent in selected}
        if expanded == selected:
            return selected
        selected = expanded


def processes(root_pid):
    snapshot = KERNEL.CreateToolhelp32Snapshot(2, 0)
    if snapshot == C.c_void_p(-1).value:
        raise C.WinError(C.get_last_error())
    entries = []
    try:
        entry = ProcessEntry(size=C.sizeof(ProcessEntry))
        more = KERNEL.Process32FirstW(snapshot, C.byref(entry))
        while more:
            entries.append((int(entry.pid), int(entry.parent), entry.name))
            more = KERNEL.Process32NextW(snapshot, C.byref(entry))
    finally:
        KERNEL.CloseHandle(snapshot)
    wanted = tree_ids(root_pid, entries)
    result, unavailable = [], []
    ticks = lambda ft: (int(ft.dwHighDateTime) << 32) | int(ft.dwLowDateTime)
    for pid, parent, name in entries:
        if pid not in wanted:
            continue
        handle = KERNEL.OpenProcess(0x400 | 0x10, False, pid)
        if not handle:
            unavailable.append({"pid": pid, "error": C.get_last_error()})
            continue
        try:
            memory, io = Memory(size=C.sizeof(Memory)), IO()
            created, ended, kernel, user = [W.FILETIME() for _ in range(4)]
            if not (PSAPI.GetProcessMemoryInfo(handle, C.byref(memory), C.sizeof(memory))
                    and KERNEL.GetProcessIoCounters(handle, C.byref(io))
                    and KERNEL.GetProcessTimes(handle, C.byref(created), C.byref(ended), C.byref(kernel), C.byref(user))):
                unavailable.append({"pid": pid, "error": C.get_last_error()})
                continue
            result.append({"pid": pid, "parent": parent, "name": name,
                           "created": ticks(created), "working_set": memory.ws,
                           "private_bytes": memory.private,
                           "cpu_ms": (ticks(kernel) + ticks(user)) / 10000,
                           "read_bytes": io.read_bytes, "write_bytes": io.write_bytes})
        finally:
            KERNEL.CloseHandle(handle)
    if not any(p["pid"] == root_pid for p in result):
        raise RuntimeError(f"Measured root process {root_pid} disappeared / could not be read")
    return result, unavailable


def sha(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def temporary_bytes(case, diagnostics=None):
    total = 0
    def visit(directory, temporary=False):
        nonlocal total
        for path in directory.iterdir():
            # Exclude only the original top-level input/profile, not a same-
            # named source folder inside an extraction staging directory.
            if path in [case / "源资料", case / "profile", case / "webview"]:
                continue
            is_temp = temporary or path.name.startswith(".")
            try:
                if path.is_dir():
                    visit(path, is_temp)
                elif is_temp:
                    # Open handle metadata avoids stale Windows directory lengths.
                    try:
                        with path.open("rb") as handle:
                            total += os.fstat(handle.fileno()).st_size
                    except PermissionError:
                        # A publication/metadata handle can briefly deny reads.
                        # Path metadata is still a lower bound, not a fresh-
                        # handle guarantee; expose every fallback in the report.
                        total += path.stat().st_size
                        if diagnostics is not None:
                            diagnostics["attribute_fallbacks"] = diagnostics.get("attribute_fallbacks", 0) + 1
            except FileNotFoundError:
                pass  # A published/cleaned staging file disappeared between calls.
    visit(case)
    return total


class Observer:
    def __init__(self, pid):
        self.pid, self.latest = pid, {}

    def sample(self):
        rows, unavailable = processes(self.pid)
        for row in rows:
            self.latest[(row["pid"], row["created"])] = row
        backend = next(row for row in rows if row["pid"] == self.pid)
        children = [row for row in rows if row["pid"] != self.pid]
        return {"at": time.monotonic(), "backend_working_set": backend["working_set"],
                "backend_private_bytes": backend["private_bytes"],
                "children_working_set_sum": sum(p["working_set"] for p in children),
                "children_private_bytes_sum": sum(p["private_bytes"] for p in children),
                "tree_working_set_sum": sum(p["working_set"] for p in rows),
                "process_count": len(rows), "unavailable": unavailable,
                "cpu_ms": sum(p["cpu_ms"] for p in self.latest.values()),
                "read_bytes": sum(p["read_bytes"] for p in self.latest.values()),
                "write_bytes": sum(p["write_bytes"] for p in self.latest.values())}


def history(profile, source, task_type):
    database = profile / "data.db"
    if not database.exists():
        return None
    with sqlite3.connect(database.as_uri() + "?mode=ro", uri=True, timeout=0.1) as db:
        db.row_factory = sqlite3.Row
        try:
            rows = db.execute("SELECT * FROM task_operation_history WHERE task_type=? ORDER BY completed_at DESC", (task_type,)).fetchall()
        except sqlite3.OperationalError as error:
            if "no such table" in str(error):
                return None
            raise
    for row in rows:
        if any(Path(p) == source for p in json.loads(row["source_paths"])):
            return dict(row)
    return None


def activation(binary, env, flag, path):
    subprocess.run([str(binary), flag, str(path)], env=env, check=True,
                   timeout=15, creationflags=subprocess.CREATE_NO_WINDOW)


def phase(observer, binary, env, case, profile, flag, source, task_type):
    before = observer.sample()
    start = time.monotonic()
    activation(binary, env, flag, source)
    samples, space, space_diagnostics = [], [], {}
    next_space = 0
    while time.monotonic() - start < 180:
        sample = observer.sample()
        samples.append(sample)
        if sample["at"] >= next_space:
            space.append(temporary_bytes(case, space_diagnostics))
            next_space = sample["at"] + 0.5
        record = history(profile, source, task_type)
        if record:
            break
        time.sleep(0.1)
    else:
        raise TimeoutError(f"No real terminal history for {source}")
    elapsed = (time.monotonic() - start) * 1000
    assert record["status"] == "completed", record.get("error_message")
    assert len(samples) >= 2, "Too few samples to describe this workload"
    output = Path(record["output_path"])
    assert output.exists(), f"History must refer to a real published output: {output}"
    counters = {key: samples[-1][key] - before[key] for key in ["cpu_ms", "read_bytes", "write_bytes"]}
    gaps = [(b["at"] - a["at"]) * 1000 for a, b in zip(samples, samples[1:])]
    report = {"activation_to_saved_history_ms": elapsed, "task_duration_ms": record["duration_ms"],
              "status": record["status"], "processed_bytes": record["processed_bytes"],
              "total_bytes": record["total_bytes"], "format": record["format"],
              "execution_config": json.loads(record["execution_config"] or "null"),
              "completion_logs": [log["message"] for log in json.loads(record["logs"])
                                  if any(word in log["message"] for word in ["校验", "完成"])],
              "idle_before": before,
              "sampled_peak": {key: max(s[key] for s in samples) for key in [
                  "backend_working_set", "backend_private_bytes", "children_working_set_sum",
                  "children_private_bytes_sum", "tree_working_set_sum", "process_count"]},
              "logical_process_counters_delta": counters, "temporary_bytes_sampled_peak": max(space),
              "temporary_space_attribute_fallbacks": space_diagnostics.get("attribute_fallbacks", 0),
              "samples": len(samples), "interval_ms": {"requested": 100,
                  "median_observed": statistics.median(gaps), "max_observed": max(gaps)},
              "unavailable_samples": sum(bool(s["unavailable"]) for s in samples),
              "observations": [{**s, "at": round((s["at"] - start) * 1000, 3)} for s in samples]}
    return output, report


def fixtures(case, kind):
    source = case / "源资料"
    source.mkdir(parents=True)
    generator = random.Random(1415)
    if kind == "large":
        with (source / "大文件.bin").open("xb") as output:
            for _ in range(256):
                output.write(generator.randbytes(1024 * 1024))
    else:
        for index in range(5000):
            group = source / f"分类-{index // 100:02}"
            group.mkdir(exist_ok=True)
            (group / f"文件-{index:05}.bin").write_bytes(generator.randbytes(4096))
    expected = {p.relative_to(source).as_posix(): sha(p) for p in sorted(source.rglob("*")) if p.is_file()}
    size = sum(p.stat().st_size for p in source.rglob("*") if p.is_file())
    fingerprint = hashlib.sha256(json.dumps(expected, sort_keys=True, ensure_ascii=False).encode()).hexdigest()
    return source, expected, size, fingerprint


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--rounds", type=int, default=3, choices=range(1, 4))
    parser.add_argument("--kinds", nargs="+", choices=["large", "small"], default=["large", "small"])
    parser.add_argument("--keep-last", action="store_true", help="Leave only the last owned app for visual review")
    args = parser.parse_args()
    output = args.output.resolve()
    assert not output.exists(), "Use a new isolated output directory"
    assert output.is_relative_to(ROOT / "test-results"), "Measurement outputs belong under test-results"
    output.mkdir(parents=True)
    binary = ROOT / "src-tauri/target/release/long-compress-assistant.exe"
    binary_hash = sha(binary)
    git = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    report = {"git_commit": git, "binary_sha256": binary_hash, "python": sys.version,
              "machine": {"os": platform.platform(), "cpu": platform.processor(), "logical_cpus": os.cpu_count()},
              "scope": "Fresh isolated Release desktop E2E process per case; normal Shell actions, UI queue, final verification/publication and saved real history. Current implementation only, not A/B speedup evidence. OS cache unflushed; fixture preparation and output hash checks excluded from timings.",
              "counter_boundaries": "100ms target samples; actual gaps reported. Simultaneous process-tree working-set sums double-count shared pages. Logical process I/O includes cache/IPC, not physical disk traffic. Exited/short-lived children may be undercounted. Temporary space sampled every 500ms is a lower bound, not a guaranteed peak. Observer overhead included in observed timing. No claim of memory reduction.",
              "results": []}
    previous = {}
    for kind in args.kinds:
        for round_number in range(1, args.rounds + 1):
            case = output / f"{kind}-{round_number}"
            source, expected, size, fingerprint = fixtures(case, kind)
            assert previous.setdefault(kind, fingerprint) == fingerprint
            env = dict(os.environ, LONG_DECOMPRESS_E2E_DATA_DIR=str(case / "profile"),
                       LONG_DECOMPRESS_E2E_INSTANCE_ID=f"perf1415-{output.name}-{kind}-{round_number}"[:48],
                       WEBVIEW2_USER_DATA_FOLDER=str(case / "webview"))
            env.pop("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER", None)
            process = subprocess.Popen([str(binary)], env=env, creationflags=subprocess.CREATE_NO_WINDOW)
            keep = False
            try:
                time.sleep(2)  # Bounded idle warm-up; not part of task timings.
                assert process.poll() is None, "Isolated application exited during startup"
                observer = Observer(process.pid)
                archive, compression = phase(observer, binary, env, case, case / "profile", "--compress-zip", source, "compression")
                settings = compression["execution_config"]["settings"]
                assert settings["format"] == "zip" and settings["level"] == 6 and settings["verifyAfter"] is True
                assert any("校验通过" in line for line in compression["completion_logs"]), "Final content verification must run"
                downloaded = case / "下载"
                downloaded.mkdir()
                archive_copy = downloaded / "资料包.zip"
                shutil.copyfile(archive, archive_copy)  # Outside both timed task phases.
                restored, extraction = phase(observer, binary, env, case, case / "profile", "--extract-to", archive_copy, "decompression")
                actual = {p.relative_to(restored / source.name).as_posix(): sha(p)
                          for p in sorted((restored / source.name).rglob("*")) if p.is_file()}
                assert actual == expected, "Verify exact file set and every output SHA256"
                assert {p.relative_to(source).as_posix(): sha(p) for p in sorted(source.rglob("*")) if p.is_file()} == expected
                assert temporary_bytes(case) == 0, "Successful operations must release staging files"
                result = {"kind": kind, "round": round_number, "files": len(expected), "input_bytes": size,
                          "fixture_sha256": fingerprint, "compression": compression, "extraction": extraction,
                          "payload_hashes_verified": True, "source_unchanged": True, "temporary_bytes_after": 0,
                          "processes_last_observed": list(observer.latest.values())}
                report["results"].append(result)
                keep = args.keep_last and kind == args.kinds[-1] and round_number == args.rounds
                if keep:
                    report["kept_test_process"] = {"pid": process.pid, "binary": str(binary), "case": str(case)}
                print(f"{kind} {round_number}/{args.rounds}: {compression['task_duration_ms']} + {extraction['task_duration_ms']} ms; {len(expected)} hashes verified", flush=True)
            finally:
                if not keep and process.poll() is None:
                    process.terminate()  # Only this exact owned Popen process handle.
                    process.wait(timeout=15)
                (output / "report.json").write_text(json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8")
    assert sha(binary) == binary_hash, "Application binary changed during measurement"
    assert subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip() == git
    print(f"Saved {output / 'report.json'}", flush=True)


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        assert tree_ids(10, [(10, 1, "app"), (11, 10, "webview"), (12, 11, "renderer"), (20, 1, "unrelated")]) == {10, 11, 12}
        rows, unavailable = processes(os.getpid())
        assert not unavailable and any(row["pid"] == os.getpid() and row["working_set"] > 0 and row["private_bytes"] > 0 for row in rows)
        import tempfile
        with tempfile.TemporaryDirectory(prefix="observer-test-", dir=ROOT / "test-results") as temp:
            case = Path(temp)
            (case / "源资料").mkdir()
            (case / "源资料/input").write_bytes(b"not temporary")
            (case / ".long-extract-test/源资料").mkdir(parents=True)
            (case / ".long-extract-test/源资料/payload").write_bytes(b"temporary")
            assert temporary_bytes(case) == 9, "Nested same-named staging payload must be counted"
        print("Process-tree, real Win32 counters and staging-space checks passed")
    else:
        main()
