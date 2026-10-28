# GNU PAX hardlink producer original

Project-authored source files include real Linux hardlinks to an ordinary file, an empty file and two sparse files. GNU tar 1.35 produces a PAX 1.0 sparse archive with ordinary and long link targets; the official fixed-SHA lzop executable compresses and round-trips the entire TAR.

GNU full extraction checks real hardlink relationships, names and content SHA against the source tree. Python independently reads each target's contents; it is not used to resolve links through sparse placeholder names. The manifest records actual link targets, prior target IDs, logical hashes, metadata and physical TAR positions. It is an external producer output from project-authored files, not an application-writer fixture or third-party user's archive.

`scripts/prepare-tar-hardlink-1717.py` generates and verifies the original and reuses committed bytes when hashes match. Invalid references and damage cases are separate explicit constructions, not additional producer compatibility claims. Authored source/fixture content uses the project's MIT license.
