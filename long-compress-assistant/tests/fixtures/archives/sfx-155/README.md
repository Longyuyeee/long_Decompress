# Original SFX fixtures

Self-authored payloads are CC0-1.0. The fifteen groups retain original producer bytes and names, including original EXE modules; archives are read as data and never executed. Source and member sizes/SHA256 are fixed in `manifest.json` after independent 7-Zip 26.04 extraction. Forced RAR/RAR5 selection avoids treating an EXE's PE resources as its archive; numeric 7Z input uses the original `.7z.001` entry and joins its actual members.

RAR4 groups were produced by original RAR 6.24 from the official installer; RAR5 groups by local original RAR 7.23. These are RAR4-format compatibility cases, not a claim of every historical SFX module. No RAR encoder is redistributed. RAR's license covers distribution of self-extracting archives/volumes; the producer license is retained alongside these test archives.

7-Zip 24.09 `-sfx -v64k` produces a separate GUI loader plus `.7z.001…` data, not an embedded first data chunk. Only the exact original GUI module (213504 bytes, SHA256 `97b304ccf831e83ca58b5c99a736f6525f81378125143e92b76baea0988ec5e2`) is recognized as this loader. It is LGPL-2.1-or-later, Copyright 1999–2024 Igor Pavlov; license retained here. Corresponding unmodified source: https://www.7-zip.org/a/7z2409-src.7z. No source EXE is run. The data members remain valid 7Z archives without the loader.
