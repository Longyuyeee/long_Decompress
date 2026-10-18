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

Build on x64 Windows with Visual Studio C++: `./scripts/build-zpaq-engine.ps1`.
Uses /O2 /MT /EHsc, advapi32 and reproducible-link /Brepro. No extra runtime DLL.
Reading and fresh journaling creation are exposed, including native passwords.
No incremental updates, revision rollback or multipart creation UI is exposed.
