# ALZ isolated read engine

`long-alz.exe` is built from this repository's `tools/alz-engine` and pinned
`vendor/unalz` 0.2.1 (MIT, Sungjoon Moon). The original crate is
https://crates.io/crates/unalz/0.2.1 and its project is
https://github.com/alkegi/unalz . The redistributed notices are under `licenses/`;
the complete resolved dependency inventory is `dependencies.json`.

Long adapter modifications to unalz are explicit `AlzArchive::from_reader`,
`MultiVolumeReader::from_paths` constructors, original filename-byte retention,
and rejecting unterminated Deflate/Bzip2 or unconsumed declared compressed bytes.
The 0.4.0 helper reports definitive termination/trailing-data corruption only after the
decoded content matches both its original length and CRC, including encrypted
entries. Obfuscated Deflate (method 3) remains unsupported pending original
producer evidence; no decoder from XADMaster or another project is incorporated.
Bzip2 requires its DLZ end marker, permits only sub-byte alignment afterward,
and reads within the declared input. Input is read in 32 KiB chunks using a 4 MiB compressed-block window;
consumed whole bytes are discarded while bit alignment and cipher state
continue. The former whole-entry 256 MiB limit is removed. Boundary probing
is bounded per actual block, and decoded output is still limited and CRC
checked. Controlled large streams are independently decoded regression
evidence, not historical ALZip producer originals. This is an
altered adapter, not an official ALZip or unalz release. Native volume framing
is shared with the application's `alz_volumes.rs`; credentials use stdin JSON.

Rebuild with `scripts/build-alz-engine.ps1` (Rust stable >=1.88, Windows x64).
The build updates the generated source/binary/notice inventory
`alz-engine.lock.json`; `node scripts/check-alz-engine.mjs` verifies the shipped
bytes. The main application runs this helper through its cancellable, pausable
archive child channel; input archives are not executed. No creation, EGG, ALZ
repair, incremental backup or generic renamed slices are provided here.
