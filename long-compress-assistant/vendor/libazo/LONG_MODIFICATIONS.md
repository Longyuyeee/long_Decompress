# Long EGG integration

Vendored libazo 0.1.1, upstream commit 49b325c738cb0535d74ab2ea455f8dd696a78ad1, MIT copyright 2026 Sungjoon Moon.

The input adapter reads and decrypts one AZO block at a time rather than collecting the whole compressed stream. Compressed and expanded blocks are capped at 256 MiB, terminal/check fields and trailing input are checked, and output/IO failures propagate. Range decoding and x86 transformation remain upstream code. These bounds do not imply source authentication or EGG creation.
