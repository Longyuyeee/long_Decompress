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

2026-10-23: v1.6.23 adds read_single_entry_prefix for a uniquely mapped,
unencrypted, single-coder LZMA/LZMA2 non-solid stream larger than 1 MiB.
It keeps only the requested <=1 MiB, checks cancellation per 32 KiB output,
and fails if the decoder ends before the prefix. It deliberately does not
read/verify the remaining tail, CRC or codec end. add_prefix_decoder keeps
scoped dictionary/shape checks without treating declared total output as an
allocation; complete add_decoder and entry readers preserve their prior checks.
The application retains aggregate input, metadata, source and time bounds and
explicitly labels partial text; AES/shared prefixes are not enabled here.

The v1.6.23 actual integrity check also exposed a pre-existing terminal bug:
file/folder size and CRC wrappers stopped before the final LZMA2 control byte.
The complete stack now wraps bare LZMA2 and AES+LZMA2 in Lzma2EndReader before
the folder CRC wrapper, checking the codec end before returning the last output
chunk. Prefix reads intentionally bypass this complete stack. A new application
regression covers the actual integrity route, bad terminal and valid encrypted
solid integrity/image continuity; other codec/default choices are unchanged.

2026-10-23: v1.6.24 permits the explicit non-solid selected prefix to chain AES then LZMA/LZMA2. PreviewAes::new_prefix uses a private unregistered completion receipt; it never disables header/full-entry receipts. CBC exact short reads, KDF/cancel/limits and end checks when reached remain shared. The caller confirms AES aligned packed length; all complete readers retain full consumption/CRC/end checks. Added partial-vs-full receipt and short-read/cancellation unit coverage.

2026-10-23: v1.6.25 adds read_solid_entry_prefix for plaintext LZMA/LZMA2 shared blocks. It checks selected stream mapping/order/sizes, bounds skipped output plus prefix to 64 MiB, discards dependencies in cancellable chunks and retains only 1 MiB. It intentionally does not verify preceding CRCs or unread tail/codec end; the complete shared reader and AES receipt guards are unchanged. Covered by original two-tool readback and selected-range/mapping/cancel/short-output/full-vs-partial regressions in the application.

2026-10-23: v1.6.26 allows ordered AES + LZMA/LZMA2 in read_solid_entry_prefix, checking aligned AES packed length and reusing the explicit partial PreviewAes constructor from v1.6.24. Shared mapping and skipped-output-plus-prefix limits are retained. Header and complete-entry AES consumption receipts, full shared CRC/end conditions and defaults are unchanged; original content/header-encrypted numeric volumes independently read back and covered by bounded partial/full/password/source regressions.

2026-10-23: v1.6.27 adds read_ppmd_entry for complete plaintext non-solid PPMd7 previews. It checks unique stream mapping, single coder/pack/substream, order/dictionary bounds, declared output, exact compressed coverage and available file/folder/pack CRCs. It reads only the declared output and then calls the additive finish_7z API in local ppmd-rust 1.4.0 (see its LONG_PATCHES.md) to require the size-delimited range coder finish. It never requests a spurious EOS symbol. Existing generic Read/extraction, full/prefix LZMA paths, AES and defaults are unchanged; ZIP still resolves registry ppmd-rust. Original 7-Zip archives and readable-body/bad-flush/parameter/mapping/cancel regressions cover the new entry point.

2026-10-23: v1.6.28 extends only the complete non-solid PPMd entry point to ordered AES → PPMd. It validates method order and AES declared plaintext/aligned ciphertext sizes, then uses the existing full PreviewAes constructor with its registered completion receipt. The size-limited PPMd input must be fully consumed, followed by raw/decrypted EOF, range-code finish, full physical coverage and available CRCs. No extra PPMd symbol is decoded, no prefix AES constructor or completion waiver is used. Existing AES/KDF/default decoder and local ppmd-rust code are unchanged. Two constructed tests cover short physical reads, full receipt, padding/extra plaintext/alignment/method-order/KDF bounds and changed PPMd flush; original password/header-encrypted numeric-volume application fixtures independently read back.

2026-10-23: v1.6.29 adds read_solid_ppmd_entry for complete plaintext PPMd shared blocks, independently of the non-solid/AES and LZMA paths. It confirms mapped stream count/order/sizes, selected output cap, per-file or folder CRC coverage, one plain PPMd coder/packed stream and bounded order/dictionary. Packed/decoded totals are capped at 64 MiB. Stream through every dependency in cancellable chunks, retain only the selected file, verify available file/folder/pack CRCs and exact packed consumption, then require size-defined range coder finish without an extra symbol. No default PPMd/AES/ZIP decoder changes. Original two-tool solid/numeric-volume samples and readable-body/bad-flush, before/after CRC, folder-only CRC, mapping/limits/cancel regressions cover the new entry point.

v1.6.30 extends only the complete selected shared PPMd reader to an ordered AES → PPMd chain. It reuses full PreviewAes (including its registered completion receipt), bounds declared plaintext and aligned ciphertext, consumes exactly the PPMd plaintext and size-defined finish, then checks raw/decrypted EOF, full pack coverage, shared file/block/pack CRCs. It never uses partial AES for complete previews. Plain shared PPMd, non-solid PPMd, generic extraction and default codecs retain their scope. Two new constructed shared adapter tests reuse the existing test-only AES/PPMd producer helper; original encrypted solid samples and app/UI coverage are in the v1.6.30 fixture and freeze records. No dependency or license changes.

v1.6.31 extends the explicit non-solid prefix codec allowlist to PPMd7, with unconditional order 2–64 and dictionary 2048 bytes–64 MiB checks before allocation. It reuses the existing bounded prefix loop and explicit partial PreviewAes constructor, returns only the requested prefix without an extra symbol, finish or CRC claims, and never waives full AES receipts created by headers/complete reads. Complete PPMd, solid prefix allowlists, default codecs and dependencies retain their scope. Two new adapter checks and original 26.02/independent 26.04 fixtures, app/UI evidence are recorded in the v1.6.31 freeze document. No dependency or license changes.

v1.6.32 extends the existing explicit shared prefix allowlist to PPMd7, reusing the bounded selected-offset loop, PPMd property checks and explicit partial PreviewAes constructor. Only selected prefix bytes are retained; preceding bytes are discarded within 64 MiB, unread CRCs/tail/codec end are not claimed, and existing full AES receipts remain mandatory. Complete PPMd/default codecs/dependencies and licenses are unchanged. Two new adapter checks, original 26.02/independent 26.04 shared fixtures and app/UI results are recorded in the v1.6.32 freeze document.

v1.6.33 adds an explicit bounded bare non-solid Deflate complete-entry preview reader using the existing flate2 Decompress API. It requires raw StreamEnd, exact input/output totals and available file/folder/pack CRCs; fixed chunks handle short reads and cancellation without scanning unrelated blocks. No other codecs, AES/shared/prefix defaults, dependencies or licenses change. Two new adapter checks plus original 26.02/independent 26.04 input and app/UI evidence are recorded in the v1.6.33 freeze document.

v1.6.34 extends the explicit non-solid Deflate complete-entry reader to ordered AES -> Deflate. It reuses full PreviewAes receipts, verifies declared plaintext/ciphertext alignment and EOF/padding as well as raw Deflate StreamEnd, exact totals and available CRCs. Fixed chunks retain existing input/time/cancel/source limits; other full AES receipts cannot be waived. Complete plaintext Deflate gets one continuity check; no default codecs, dependencies or licenses change. Two new adapter checks plus original 26.02/independent 26.04 input and app/UI evidence are recorded in the v1.6.34 freeze document.

v1.6.35 adds a separate bounded solid Deflate complete-entry reader, optionally preceded by full AES. Fixed 32 KiB streaming routes every mapped file CRC and retains only the selected bytes, including empty/boundary cases. It requires explicit raw StreamEnd, exact plaintext/output totals, full AES EOF/padding receipts, and available file/folder/pack CRCs; shared compressed and decoded totals each stay within 64 MiB. Non-solid/default readers and dependencies/licenses are unchanged. Two adapter groups and four independently read-back original-tool groups plus backend/native UI evidence are recorded in the v1.6.35 freeze.

v1.6.36 adds an explicit non-solid Deflate prefix reader, optionally preceded by existing partial PreviewAes. It retains only a capped prefix with fixed 32 KiB buffers, confirms mapping/methods/alignment, and rejects observed short codec endings or stalled/truncated input before the prefix. Huge declarations do not become allocation/read targets; unread tail/CRCs/complete payload consumption are deliberately unverified. Other full AES receipts remain mandatory. Complete/default readers and dependencies/licenses are unchanged. Two adapter groups, independently read-back originals and backend/native UI evidence are recorded in the v1.6.36 freeze.

v1.6.37 adds a separate shared Deflate prefix reader, optionally preceded by partial PreviewAes. It validates all mapped stream sizes/order/counts and coder/binding shape, bounds dependencies plus selected prefix to 64 MiB, discards dependencies through fixed 32 KiB chunks and retains only the capped selected prefix. Known early endings, stalled input and cancellation fail; unread dependencies/tail/CRCs remain explicitly unverified. Full AES receipts from headers/other complete streams remain mandatory. Non-solid/complete/default readers and dependencies/licenses are unchanged. Three adapter groups and original-tool/backend/native UI evidence are in the v1.6.37 freeze.

v1.6.38 adds an independent complete ordinary non-solid Deflate64 preview reader. It validates mapping/coder/stream/pack shape, bounded declarations, actual end/drained output, exact compressed coverage excluding Huffman-prefetched whole bytes and every available CRC. Only this dependency uses the locked local deflate64 0.1.12 with an additive terminal-extent accessor; ZIP/default decoding is unchanged. MIT originals, upstream hashes and notices are retained. Three adapter, six backend and five native UI groups are recorded in the v1.6.38 freeze.

v1.6.39 extends only complete non-solid read_deflate64_entry with ordered full AES. It validates declarations/alignment/coder mapping, observes real Deflate64 finish and precise plaintext extent excluding Huffman prefetch, then completes AES EOF/zero padding/cipher consumption and all available CRCs. Other full AES receipts remain mandatory. The additive Deflate64 accessor, default codecs, ZIP registry copy and dependencies/licenses are unchanged. Three AES adapter, six backend and five native UI groups are in the v1.6.39 freeze.

v1.6.40 adds independent ordinary read_solid_deflate64_entry. Validate all mapped file sizes/order/counts and coder/stream/pack shape, bound packed and shared decoded totals separately to 64 MiB, stream fixed 32 KiB chunks and retain only the selected bounded file. Verify real terminal/drained output, exact input extent excluding Huffman-prefetched whole bytes and every available file/block/pack CRC. Non-solid plain/AES and default codec/dependency/license paths are unchanged. Three adapter, six backend and five native UI groups are in the v1.6.40 freeze.

v1.6.41 extends only complete read_solid_deflate64_entry with ordered full AES. Retain all shared mapping/size/CRC and terminal checks; validate coder/stream/binding shape, AES declarations/alignment, exact plaintext extent excluding Huffman prefetch, complete AES EOF/zero padding/cipher consumption and other full-stream receipts. Stream fixed chunks and retain only the selected bounded file. Ordinary shared, non-solid/default codecs, dependencies and licenses are unchanged. Three AES shared adapter, six backend and five native UI groups are in the v1.6.41 freeze.

v1.6.42 adds only read_deflate64_entry_prefix for non-solid selected text. Confirm mapping/coder/AES declarations, use fixed chunks and explicit partial AES, drain buffered output before refill and reject observed terminal size conflicts including pending output. Return only the capped prefix; do not certify unread CRC/tail/AES completion or waive any full-stream receipt. Complete codecs, dependencies, licenses and the Deflate64 query fork are unchanged. Three adapter units, six backend and four native groups are recorded in the v1.6.42 freeze.

v1.6.43 adds only read_solid_deflate64_entry_prefix. Confirm shared mapping/order/count/sum and coder/AES declarations; bound dependencies plus prefix to 64 MiB, discard dependencies and retain at most 1 MiB. Drain fixed buffered output before refill; reject observed bad/short streams including known terminal size conflicts with pending output. Stop at the selected target without certifying CRC/unread tails/full AES consumption or waiving any full-stream receipt. Existing non-solid and complete readers, dependencies, licenses and the Deflate64 query fork are unchanged. Four adapter units, six backend and four native groups are recorded in the v1.6.43 freeze.
