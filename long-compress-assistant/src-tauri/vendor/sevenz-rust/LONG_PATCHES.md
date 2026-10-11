# Long-Decompress local patch

Source: crates.io sevenz-rust 0.6.1 (Apache-2.0; LICENSE preserved).
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
