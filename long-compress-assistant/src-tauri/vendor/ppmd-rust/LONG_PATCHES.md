# Local PPMd7 complete 7z preview adapter

Based on ppmd-rust 1.4.0 from the locked crates.io package; original file hashes
are in UPSTREAM_FILES.json. MIT-0 / CC0 originals and upstream README/changelog
are retained. ZIP continues to use its original registry dependency; only the
local sevenz-rust dependency resolves to this copy.

2026-10-23, v1.6.27: add Ppmd7Decoder::finish_7z, exposing the existing range
decoder Code == 0 state after exactly the declared output has been read. Reject
an already terminated/short stream. This is additive: default Read, allocation,
models, encoders and PPMd8 are unchanged. The 7z caller separately checks exact
compressed coverage, declared output and all available CRCs; it must not decode
an extra symbol because standard 7z PPMd is size-delimited without EOS.

Reference: [official 7-Zip PpmdDecoder.cpp](https://github.com/ip7z/7zip/blob/main/CPP/7zip/Compress/PpmdDecoder.cpp),
size-defined FinishStream handling. Two focused tests cover no-EOS completion,
premature finish, truncation and changed flush. Original two-tool archive samples
and a readable-body/bad-flush negative case live in the application fixture set.

The published registry package excludes the upstream test fixture. The retained
encoder roundtrip modules require that include_str path at compilation, so
tests/fixtures/text/apache2.txt contains the Apache-2.0 license text copied from
the existing sevenz-rust/LICENSE solely as roundtrip input. It is not an archive
compatibility fixture. The excluded reference_test target is removed from the
normalized Cargo.toml; the upstream Cargo.toml.orig is retained.
