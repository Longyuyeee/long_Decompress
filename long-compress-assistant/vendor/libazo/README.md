# libazo

[![crates.io](https://img.shields.io/crates/v/libazo.svg)](https://crates.io/crates/libazo)
[![docs.rs](https://img.shields.io/docsrs/libazo)](https://docs.rs/libazo)
[![CI](https://github.com/alkegi/libazo/actions/workflows/ci.yml/badge.svg)](https://github.com/alkegi/libazo/actions/workflows/ci.yml)

A decompression library for the AZO format.

```rust
let compressed = std::fs::read("data.azo")?;
let mut out = Vec::new();
let crc = libazo::extract_azo(&mut &compressed[..], &mut out, compressed.len() as u64, None, None)?;
println!("{} bytes, crc {crc:08x}", out.len());
```

See the [documentation](https://docs.rs/libazo) for the full API.

## Reference

- [`xunazo`](https://github.com/kippler/xunazo)
