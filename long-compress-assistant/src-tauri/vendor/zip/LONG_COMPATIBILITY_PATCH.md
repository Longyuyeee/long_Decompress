# ZipCrypto descriptor CRC compatibility

Imported unchanged from the crates.io `zip` 8.6.0 package, upstream revision
`771dfc534d2614158af5497ea3dff4d4208d7db1`, then patched locally. Upstream source:
https://github.com/zip-rs/zip2/tree/771dfc534d2614158af5497ea3dff4d4208d7db1
The upstream MIT license is preserved in `LICENSE`.

HaoZipC 6.6.0.11316 creates spanning ZipCrypto entries with general-purpose
bit 3 set while the decrypted encryption header still uses the CRC high byte.
Unmodified zip 8.6.0 checks only the DOS-time high byte in that case and rejects
the correct password. Bandizip independently verifies the same generated files.

`ZipReadOptions::zipcrypto_crc_with_descriptor(true)` explicitly accepts either
header convention for descriptor-based ZipCrypto. The default remains strict;
AES, codecs, bytes, disk offsets and the final payload CRC check are unchanged.
The application opts in only through its shared ZIP password reader, retains
full content validation before accepting credentials, and never sets
`ignore_crc32`. A short header match alone is not proof of a correct password.

Modified upstream source files: `src/read.rs`, `src/read/zip_archive.rs`,
`src/read/readers.rs`, `src/read/stream.rs`, and `src/zipcrypto.rs`. The stream
reader explicitly retains strict behavior. No extra dependency was introduced.
