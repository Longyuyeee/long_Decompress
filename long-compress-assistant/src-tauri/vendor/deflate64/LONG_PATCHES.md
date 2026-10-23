# Selected complete 7z preview extent

Based on the locked crates.io deflate64 0.1.12 MIT package. UPSTREAM_FILES.json
records the retained originals. Only local sevenz-rust resolves to this copy;
ZIP keeps its registry version. No upstream decode/default Read behaviour changes.

v1.6.38 adds finished_input_bytes(), available only at a drained, non-error
terminal stream. It subtracts whole Huffman-prefetched bytes from the existing
loaded-byte count; the final fractional byte is format padding. The caller uses
InflaterManaged::new(), never with_uncompressed_size(), so declared output cannot
masquerade as BFINAL/EOB. It separately verifies actual packed extent, output,
all available CRCs, cancellation and budgets. Constructed stored/fixed-Huffman
terminal tests and real-tool Deflate64 fixtures exercise this accessor.
