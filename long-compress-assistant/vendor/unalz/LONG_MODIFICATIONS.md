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
