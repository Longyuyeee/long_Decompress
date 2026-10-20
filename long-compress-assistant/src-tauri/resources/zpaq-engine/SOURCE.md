# ZPAQ adapter

Upstream: Matt Mahoney, ZPAQ 7.15 (2016-08-17).
Source: https://www.mattmahoney.net/dc/zpaq715.zip
Package SHA-256: e85ec2529eb0ba22ceaeabd461e55357ef099b80f61c14f377b429ea3d49d418
Documentation: https://www.mattmahoney.net/dc/zpaq.html

Sources in `vendor/zpaq`; preserve the accompanying COPYING (public-domain
ZPAQ/libzpaq and MIT libdivsufsort portions). Algorithms are unchanged.
Long changes in zpaq.cpp add length-prefixed binary stdin credentials and exact
selection, JSON current-file metadata and strict metadata error propagation.
The creation manifest uses the shared archive-entry policy, preserving relative
names without recursive CLI scanning. Windows I/O failures and short writes
terminate the isolated process with a typed operation and Win32 error code.
The fault-test hook is compiled only by `build-zpaq-engine.ps1 -TestIO`, never
into the shipped executable. Output files are flushed before closing.
No password is passed to this adapter in command arguments.

The v1.5.26 read adapter accepts an explicit length-prefixed numeric-member list
through stdin and reuses upstream cross-file seeking without a joined archive.
Reading the 32-byte salt spans even very small members; .1000 and non-uniform
slices use actual member names. Bounded entry previews limit reads/shared
decoded blocks to 64 MiB, decoder memory to 256 MiB and one worker; unknown
entry sizes require extraction. Ordinary reads and compression algorithms
retain their existing behavior.

Build on x64 Windows with Visual Studio C++: `./scripts/build-zpaq-engine.ps1`.
Uses /O2 /MT /EHsc, advapi32 and reproducible-link /Brepro. No extra runtime DLL.
Reading and fresh journaling creation are exposed, including native passwords.
Fixed-size numeric byte slices are exposed by the host; they are not native
journaling append parts. In-place revision rollback is not exposed.

The v1.5.30 read adapter lists complete journaling transaction identities and
freezes the file map at an explicit version/date pair while scanning the full
metadata catalog. Missing or changed identities and streaming/mixed history
requests fail explicitly; no silent latest-version fallback. Preview, selected
extraction, conversion and task history share this identity. Codecs and native
cryptography are unchanged.

The v1.5.36 adapter exposes `-long-update` on a host-owned full archive copy.
It loads the entire old file map (including unselected paths), preserves those
entries, and forces content comparison before appending changed/new fragments.
No-change updates truncate the empty transaction as upstream does and keep the
copy byte-identical. A JSON summary reports updated entries/new fragments;
stderr reports actual processed input bytes for pause/cancellation and progress.
The host verifies the complete updated archive before publishing another file
or fresh numeric slices. Fresh `-long-create` still refuses an existing file.
The user original is never an append target. Codec and cryptography are unchanged.
