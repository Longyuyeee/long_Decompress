# WIM encoder — wimlib 1.14.5, upstream unmodified Windows x64

Only the independent command-line encoder is invoked for explicitly selected XPRESS, non-solid LZX/LZMS, or LZMS solid WIM/ESD creation. Reading and legacy store creation remain with the existing archive engine. No Windows component is installed and no elevation is requested.

Binary distribution: https://wimlib.net/downloads/wimlib-1.14.5-windows-x86_64-bin.zip
SHA256: 2f446d6fa3866582175f1a22a7be198eeee0aec7aba5b4e04ad25c99eae2d265
Corresponding source: https://wimlib.net/downloads/wimlib-1.14.5.tar.gz
SHA256: 84221a3abd5b91228f15f8e6065c335a336237b5738197b75bf419eea561a194

The exact unmodified source archive accompanies the binaries in this directory, including upstream build scripts. README.WINDOWS.txt documents the upstream MSYS2 MINGW64 build (tools/windows-build.sh --install-prerequisites). Full notices and GPLv3+, LGPL2.1+ and libdivsufsort-lite licenses accompany the binaries. See COPYING.txt for upstream's dual-license distinctions. The independent wimlib-imagex program is GPLv3+; it is executed as a separate process, not linked into this application. libwim Windows builds allow the upstream LGPL2.1+ option. No local upstream patches.

https://wimlib.net/man1/wimcapture.html documents source-list, XPRESS/LZX/LZMS, no-acls, configuration and failure semantics. The application supplies an explicit UTF-8 BOM source list and an empty exclusion configuration, so default system-image exclusions do not silently drop ordinary user files. This is a single-image content archive, not an operating-system deployment or backup-metadata guarantee.

Explicit solid mode uses LZMS with 64 MiB solid chunks, 128 KiB metadata chunks and one encoder thread (upstream documents approximately 640 MiB per thread). ESD creation always selects this solid mode, not a renamed non-solid WIM. See the accompanying source archive doc/man1/wimcapture.1 for --solid, --solid-compress and --solid-chunk-size. Selection and preview can require decoding other files in a shared solid block. Numeric byte slices are not native SWM parts.
