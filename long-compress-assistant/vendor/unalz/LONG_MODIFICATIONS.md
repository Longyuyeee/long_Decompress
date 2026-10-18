# Long ALZ adapter changes

This is the MIT-licensed unalz 0.2.1 crate by Sungjoon Moon, copied from the
crates.io package, with its original LICENSE preserved. Local changes add two
explicit-volume reader constructors and reject a Deflate stream which reaches
input EOF without its end marker. See `src-tauri/resources/alz-engine/SOURCE.md`
and `alz-engine.lock.json` at the project root for the build and byte inventory.
The upstream CLI is not shipped or invoked; credentials do not enter argv.
