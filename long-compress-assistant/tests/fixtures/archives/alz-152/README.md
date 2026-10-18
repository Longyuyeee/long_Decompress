# Genuine ALZ originals

`manifest.json` fixes ten upstream CC0 archive cases (including original native
members, Store/Deflate/DLZ Bzip2 and ZipCrypto) to upstream commit
`30b4ad40c5452c686df69b3b45554de3073e57a1`. The independent Bandizip read uses
CP949, as identified by the upstream source table; automatic CP936 on this
computer produces mojibake and is not the expected filename evidence.
`scripts/prepare-alz-152-fixtures.mjs` records original member and expanded
payload SHA-256 values. No renamed ordinary byte slices stand in for volumes.

`original-alzip/cp936-provenance.json` fixes two additional self-authored genuine
archives from original ALZipCon: Chinese filenames, Chinese password and one
five-member native group. Both match independent Bandizip CP936 extraction.
The original producer was unpacked from the official installer without running
the installer; installer, executable, member and payload hashes are recorded.
Public test passwords are in manifests, never private user credentials.

`original-alzip/provenance.json` preserves the earlier original-producer cases
whose Korean filenames became literal question marks on the CP936 system.
They are negative evidence, not successful Unicode recovery or Korean-password
compatibility evidence. The application rejects those invalid Windows names;
we do not rename them or change expected data to manufacture success.

The 256 MiB real pause/resume/cancel archive is intentionally not redistributed.
`scripts/prepare-alz-152-control.mjs` creates it with the same original producer
in ignored test-results; the desktop result includes exact source/archive/tool
hashes and observes the actual decoder process. Input archive executables are
never executed.
