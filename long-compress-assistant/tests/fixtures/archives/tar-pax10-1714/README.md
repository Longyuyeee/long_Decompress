# GNU PAX sparse 1.0 original

Self-authored files from the fixed-SHA v1.7.13 GNU original, plus one 16 MiB sparse file containing 96 data extents. GNU tar 1.35 creates the PAX 1.0 archive; a fixed-SHA official lzop executable compresses it and round-trips the full TAR bytes. This is a real external producer output, not an application-writer fixture or a third-party user's archive.

`manifest.json` records source/tool hashes, every logical member's source/GNU/Python SHA, metadata, physical record locations, inline map size and stored ranges. GNU extraction and source paths use `GNU.sparse.name`; Python's logical-content reader agrees on bytes, but this Python version exposes the producer's placeholder `path` for some names. It is therefore not used as a filename oracle.

`scripts/prepare-tar-pax10-1714.py` generates and verifies the original, reusing committed fixed-SHA bytes when present. Its producer uses `--format=pax --sparse --sparse-version=1.0 --hole-detection=raw --no-recursion`, with a fixed modification time. GNU-generated PID placeholders can change upon regeneration; hashes belong to the committed original.

Damage boundaries and invalid metadata in native tests are explicit constructions around this independently checked TAR. They establish stopping behavior, not additional producer compatibility. The source files and fixture content are authored for this project and distributed under its MIT license.
