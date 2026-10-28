# GNU PAX sparse 0.0 original

GNU tar 1.35 produces this real `--format=pax --sparse --sparse-version=0.0 --hole-detection=raw --no-recursion` archive from project-authored files extracted from the fixed-SHA v1.7.14 original. Official fixed-SHA lzop compresses and round-trips the entire TAR. Source files, GNU extraction and Python sparse reading agree on logical contents and SHA.

`manifest.json` records the original/tool hashes, logical entries, physical offsets and ranges. `paxFields` and control `fields` preserve every ordered PAX attribute, including repeated offset/numbytes pairs; the `pax` object contains only non-pair metadata. The 96-data-extent sample includes an EOF range and spans several metadata blocks. Names use the ordinary/PAX path, without the later sparse.name placeholder mechanism.

`scripts/prepare-tar-pax00-1716.py` generates and verifies the original, reusing committed bytes when hashes match. Damage and invalid-metadata cases in native tests are separate explicit constructions, not additional producer compatibility claims.

This is an external producer output from self-authored files, not an application-writer fixture or a third-party user's archive. Authored source/fixture content uses the project's MIT license.
