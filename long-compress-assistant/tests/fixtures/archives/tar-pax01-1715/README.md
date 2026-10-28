# GNU PAX sparse 0.1 original

Self-authored source files are extracted from the fixed-SHA v1.7.14 GNU original, including a 16 MiB file with 96 data extents. GNU tar 1.35 creates a real `--format=pax --sparse --sparse-version=0.1 --hole-detection=raw --no-recursion` archive; a fixed-SHA official lzop executable compresses it and round-trips all TAR bytes.

`manifest.json` records tool/source hashes, logical source/GNU/Python SHA values, physical member locations, local PAX controls and comma-map ranges. GNU/source paths establish real filenames; this Python version's sparse reader provides the independent logical-content check, with `GNU.sparse.name` used explicitly rather than treating a placeholder path as a filename oracle.

`scripts/prepare-tar-pax01-1715.py` generates and verifies the fixture and reuses committed bytes when hashes match. GNU's PID placeholders can change when regenerating; hashes describe the committed original. Invalid metadata and damage cases in native tests are separate constructions establishing stop boundaries, not additional producer compatibility.

This is an external producer output from project-authored files, not an application-writer fixture or third-party user's archive. Authored source/fixture content uses the project's MIT license.
