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

## ZIP LZMA writing (v1.5.38)

Added optional method-14 writing using the existing encoder-enabled lzma-rust2
0.16.2 dependency: ZIP SDK/version and five-byte property header, raw LZMA1
stream with EOS, and matching bit 1 in local/central headers. Writer lifecycle,
AES/ZipCrypto wrapping, stream finalization and raw-copy codec flags are kept
consistent. Default codecs and strict CRC/password reading remain unchanged.
Modified additional upstream files: `src/write.rs` and `src/types.rs`.
No new product dependency or executable is added.

Format references: [PKWARE APPNOTE 5.8.8 and 4.4.4](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT),
[CPython ZIP LZMA framing](https://github.com/python/cpython/blob/3.13/Lib/zipfile/__init__.py).

## ZIP LZMA complete reading (v1.5.60)

The method-14 reader now receives the real ZIP flags in seekable and stream paths.
When bit 1 promises EOS it decodes to the marker, checking exact output size;
otherwise legitimate sized, non-EOS streams keep their existing behavior. This
prevents a directory-sized prefix or an incomplete EOS stream from being accepted
as complete. Original properties are validated before preserving the existing
small-file dictionary clamp, without adding a new dictionary/memory limit.
CRC, AES/ZipCrypto wrapping and method-14 writer framing remain unchanged.
Modified additional source: `src/compression.rs`; the private read helpers carry
flags also when `legacy-zip` is disabled. No product dependency is added.

## ZIP Zstandard error context (v1.5.61)

The existing method-93 streaming decoder continues through complete concatenated
frames, retaining its default window bound, frame/checksum validation and strict
CRC/AES wrappers. Read errors now include actionable context for missing external
dictionaries, oversized windows and incomplete frames, preserving the I/O kind
and original detail. Empty reads do not consume input. Method 20 is unchanged;
no new codec, memory limit or dependency is introduced. Modified source:
`src/compression.rs`.

## ZIP PPMd complete reading (v1.5.62)

Method 98 now checks the declared output size, real end marker and exact payload
end through the existing CRC/crypto wrappers. A ZIP-only input adapter prevents
ppmd-rust's sized-stream UnexpectedEof fallback from accepting truncated input.
Orders below 2 and unsupported restoration codes (including Freeze) are rejected
before decoder allocation; Restart/CutOff retain the original 1–256 MiB header
range. Empty reads do not initialize the decoder. No dependency, writer change
or new memory cap is introduced. Modified source: `src/compression.rs`.

Reference: [7-Zip PpmdZip full-file decoder](https://github.com/ip7z/7zip/blob/main/CPP/7zip/Compress/PpmdZip.cpp)
requires the end marker, finished range coder and exact input consumption.

## ZIP XZ complete reading (v1.5.63)

Method 95 retains its existing single-XZ-stream decoder and resource behavior.
It now verifies exact output size and consumes the remaining compressed payload
through the existing CRC/crypto wrappers. Extra payload, including zero padding
or a subsequent stream, is explicitly rejected rather than ignored: independent
7-Zip full ZIP decoding rejects a padded method-95 payload. Empty reads do not initialize the stream. No
writer or dependency change. Modified source: `src/compression.rs`.
