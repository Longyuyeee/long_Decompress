# Long ALZ adapter changes

This is the MIT-licensed unalz 0.2.1 crate by Sungjoon Moon, copied from the
crates.io package, with its original LICENSE preserved. Local changes add two
explicit-volume reader constructors and reject a Deflate stream which reaches
input EOF without its end marker or leaves bytes in its declared compressed
range after the end marker. The latter has a typed trailing-data error so the
adapter can distinguish fully CRC-validated content from uncertain decryption.
See `src-tauri/resources/alz-engine/SOURCE.md`
and `alz-engine.lock.json` at the project root for the build and byte inventory.
The upstream CLI is not shipped or invoked; credentials do not enter argv.

v1.6.5 also requires the actual ALZ Bzip2 EOS marker and rejects whole trailing
bytes inside its declared compressed range, permitting only sub-byte alignment.
Bzip2 reads no further than the supplied entry length; short input and output
I/O errors retain their original type. The adapter distinguishes an incomplete
Bzip2 terminator after fully size/CRC-verified content from uncertain decryption.
The existing 256 MiB compressed-input bound is unchanged. Method 3 is not added.

v1.6.8 replaces whole-entry Bzip2 collection with a 32 KiB read / 4 MiB
compressed-block window. Consumed whole bytes are discarded without losing
bit alignment or cipher state. Candidate probes and their work budget are
per block and charged against actual buffered input. The 256 MiB entry limit
is removed; termination, tail, declared-range, I/O and output limits remain.
The large stream used for this boundary is controlled Python/libbzip2 data,
independently decoded, not historical ALZip producer evidence. Method 3 and
new producer password/volume combinations are not claimed.
