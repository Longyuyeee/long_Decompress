# Original encrypted RAR4 and RAR5 volumes

Twelve fixed original groups: RAR4 old/modern names, native/SFX, and RAR5 native/SFX; each has an independently generated original and another group with different payload, the same names and public test password `Rar159+中文123!`. Self-authored payload is CC0-1.0. Keep original volume names and bytes; never execute any input SFX.

The original tool versions are WinRAR 6.24 for RAR4 and RAR 7.23 for RAR5, the same authenticated producers and original module notices retained with `sfx-155`. Official sources: https://www.rarlab.com/rar/winrar-x64-624.exe and https://www.rarlab.com/download.htm. Exact producer and member hashes, arguments, original entry order and independent 7-Zip 26.04 readback are in manifest.json. Standalone RAR modules or producers are not redistributed.

The first nonempty encrypted text file has a complete CRC before the large cross-volume payload. Tests can distinguish a previously proven key from a later mixed/truncated volume. Missing-tail tests remove a real original tail, never fabricate a name or edit an archive header. The old large-first encrypted SFX fixtures remain separate regression controls for failures encountered during password verification itself.
