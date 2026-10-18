# Long extraction controls

The existing UnRAR wrapper modifications preserve Unicode password handling, volume callbacks and cancellable testing. v1.5.5 adds `extract_to_with_control`: a synchronous borrowed checkpoint at native data/volume callbacks, used by the application for real in-file pause/resume/cancel. It does not change the RAR decoder or add creation capabilities.

The checkpoint reference lives only for the synchronous process call; the native callback is cleared before its stack userdata expires and on handle close. Checkpoint panics are caught and abort the native call, never unwinding through C. Existing `extract_to`, listing, and `test_with_cancel` retain their prior behavior. Source remains subject to the existing wrapper and UnRAR licenses; it must not be used to recreate a RAR-compatible compression algorithm.

v1.5.6 adds `extract_to_with_progress`, a synchronous borrowed hook reporting UCM_PROCESSDATA's decoded block length once per block. It does not copy the data or scan/read output files. UnRAR invokes this before writing/CRC, so it is decoded work, not committed/verified output. Progress-hook panics abort at the C boundary; the existing control hook still runs first. No decoder algorithm or native source changes.
