# Long EGG basic reader changes

Upstream unegg 0.2.1 at 9929009ca4d010e8cd88bb796f7a1868162026d4, MIT, copyright Sungjoon Moon.

Only archive parsing, filename decoding, ZipCrypto and Store/Deflate/Bzip2/LZMA are compiled in this version. The original CLI, extraction, volume discovery, AES/LEA/AZO sources remain for traceability but are not compiled or exposed. Native .volN.egg framing and strict two-way IDs use the app's shared egg_volumes.rs. No EGG creation.

Deflate requires StreamEnd and exact compressed end. Metadata skips cannot seek past input; parser rejects trailing bytes, unsupported global/name encryption, unknown versions, duplicate/missing/cyclic relative IDs, excessive metadata/entries. Relative names resolve fully without silently losing parent prefixes. UTF-8 and declared legacy filenames decode strictly; paths are never silently repaired. Full block size/CRC and entry-size sums are required in long-egg before success or password acceptance.
