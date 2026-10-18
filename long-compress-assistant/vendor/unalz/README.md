# unalz

[![crates.io](https://img.shields.io/crates/v/unalz.svg)](https://crates.io/crates/unalz)
[![docs.rs](https://img.shields.io/docsrs/unalz)](https://docs.rs/unalz)
[![CI](https://github.com/alkegi/unalz/actions/workflows/ci.yml/badge.svg)](https://github.com/alkegi/unalz/actions/workflows/ci.yml)

ALZ archive extractor written in Rust.

## Usage

```
unalz archive.alz                 # extract all files
unalz archive.alz file.txt        # extract a specific file
unalz -d output/ archive.alz      # extract into a directory
unalz -P SECRET archive.alz       # extract an encrypted archive
unalz -l archive.alz              # list contents
unalz -p archive.alz file.txt     # extract to stdout
cat archive.alz | unalz -l -      # read from stdin
```

| Option | | Description |
|--------|--|-------------|
| `-l` | `--list` | list contents instead of extracting |
| `-d` | `--output-dir DIR` | extract into DIR (default: current directory) |
| `-p` | `--pipe` | extract to stdout |
| `-P` | `--password PW` | decryption password |
| `-q` | `--quiet` | suppress progress messages |
| `-h` | `--help` | show help and exit |
| `-V` | `--version` | show version and exit |

## Install

```
cargo install unalz
```

## Supported Features

### Compression
- Store (no compression)
- Deflate
- Bzip2 (ALZ-modified)

### Encryption
- ZipCrypto (traditional PKWARE)

### Archive types
- Split (multi-volume) archives (`.alz`, `.a00`, `.a01`, ...)

### Other
- CP949/EUC-KR filename decoding to UTF-8
- CRC32 verification
- DOS timestamp preservation
- Truncated archives (a missing volume, an interrupted download) still yield
  every complete file, with a warning

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | everything extracted |
| 1 | extracted what was there, archive truncated |
| 2 | failed |

## Docs

[https://github.com/alkegi/docs](https://github.com/alkegi/docs)

## References

- [kippler/unalz](https://github.com/kippler/unalz)

---

Part of the [alkegi (알깨기)](https://github.com/alkegi) project.
