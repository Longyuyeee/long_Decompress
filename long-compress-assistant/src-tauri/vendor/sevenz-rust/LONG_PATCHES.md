# Long-Decompress local patch

Source: crates.io sevenz-rust 0.6.1 (Apache-2.0; LICENSE preserved).

## v1.5.7 additional read codecs

The opt-in `additional-codecs` feature registers 7z PPMdH and uses the existing
locked ppmd-rust 1.4.0 Ppmd7Decoder, flate2 raw Deflate and deflate64 readers.
PPMd properties are validated before creating its arena and respect the caller's
memory limit. These readers stay inside the original bounded, CRC-checked solid
stream stack; no external fallback or encoder/default change. The application
also enables the existing bzip2 feature. Original 7-Zip 24.09 fixtures and an
independent 26.04 readback cover contents/header encryption and numeric volumes.
Registry package checksum: 26482cf1ecce4540dc782fc70019eba89ffc4d87b3717eb5ec524b5db6fdefef.
Original manifest and upstream VCS metadata are included. The source, examples,
manifest, README, changelog and license are copied from the registry package;
registry cache markers, upstream tests and editor/CI settings are omitted.

2026-10-11: writer.rs write_encoded_header must never fall back to a plaintext
header when its selected methods contain AES. Keep the existing size-saving
fallback only for unencrypted headers. This fixes short/empty-directory archives
silently exposing filenames despite set_encrypt_header(true).

Application regression: native_compression::seven_zip::tests::
encrypted_small_header_never_falls_back_to_plaintext (failed on upstream 0.6.1).
The product uses an explicit opt-in filename-encryption option, default false.
Do not change that default during upgrades.
Re-audit this patch and the application's K_ANTI workaround when updating.
Trailing whitespace in the upstream README and changelog was normalized for
the repository diff checks; no documentation meaning was changed.

2026-10-11: reader.rs Crc32VerifyingReader rejects decoder EOF while expected
unpacked bytes remain. Previously it returned normal EOF without checking CRC,
allowing a mixed encrypted volume to produce an empty file reported as success.
Empty caller buffers still return zero without consuming data. Regression tests
in reader::long_crc_tests fail before this fix and pass afterwards. Run with
`cargo test --manifest-path src-tauri/vendor/sevenz-rust/Cargo.toml --lib long_crc_tests`.
Keep the real desktop mixed-volume failure assertions unchanged.

2026-10-23: BlockDecoder::read_single_entry provides bounded complete non-solid
text for v1.6.17. It uses the existing decoder stack without an outer size/CRC
wrapper masking the terminal read, then checks exact size and both entry/folder
CRCs. No codec, writer, default, or general extraction behavior is changed.
The application limits methods, dictionaries, input reads, time and source scope.

2026-10-23: BlockDecoder::read_solid_entry streams the whole selected shared
block for v1.6.19 text. It uses file_folder_index plus has_stream rather than
a contiguous file slice (empty files/directories are not substreams), checks
stream count/summed sizes, each file CRC, folder CRC and optional packed CRC,
compressed coverage and codec end. Only selected output is retained; a caller
callback checks cancellation/deadline on every output chunk and before success.
The application allows only unencrypted LZMA/LZMA2 shared text <=1 MiB within
64 MiB compressed/unpacked shared blocks. General extraction is unchanged.

2026-10-23: v1.6.21 adds with_preview_limits, a synchronous thread-local RAII
scope used only by the application's 7Z metadata and preview reads. It bounds
header allocations/dictionaries and AES work (cycles power <=20, UTF16 bytes
<=4096), checks cancellation/deadline every 4096 KDF rounds, and restores the
previous scope on error/panic. Scoped AES-CBC uses exact aligned short-read-safe
input, declared plaintext length, zero padding and ciphertext EOF; every scoped
AES stream must be fully consumed before a successful return. Encoded headers
in scope read the actual codec end and verify folder CRC after exact output.
General unscoped extraction/creation defaults are preserved. Original producer
cases and five scoped vendor tests cover this path. The zero-padding/declared
size model follows the official source:
https://github.com/ip7z/7zip/blob/main/CPP/7zip/Common/FilterCoder.cpp
https://github.com/ip7z/7zip/blob/main/CPP/7zip/Crypto/7zAes.cpp
