# UnRAR MSVC exception cleanup

Vendored unrar_sys 0.5.8 from the published crate, upstream `muja/unrar.rs` commit `8fe139781b92d0678afecc568496f5805401a6c4`, path `unrar_sys`. Preserve the original UnRAR sources and license in `vendor/unrar/license.txt`.

Only build behavior is changed: request `/EHsc` when supported. The original build did not enable MSVC C++ stack unwinding, so UnRAR's cancellation exception could skip the stack-local output `File` destructor in `ExtractCurrentFile`, leaving a Windows sharing-locked partial file even after RARCloseArchive. The RAR decoder and format algorithms are unchanged. The Rust wrapper's data checkpoint provides pause/cancel; exception unwinding releases native file handles so the existing extraction transaction can clean its private staging directory normally.

UnRAR sources must not be used to recreate a RAR-compatible compression algorithm. This remains a read-only decoder, not a RAR encoder.
