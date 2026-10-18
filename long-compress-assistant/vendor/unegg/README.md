# unegg

[![crates.io](https://img.shields.io/crates/v/unegg.svg)](https://crates.io/crates/unegg)
[![docs.rs](https://img.shields.io/docsrs/unegg)](https://docs.rs/unegg)
[![CI](https://github.com/alkegi/unegg/actions/workflows/ci.yml/badge.svg)](https://github.com/alkegi/unegg/actions/workflows/ci.yml)

EGG archive extractor written in Rust.

## Usage

```
unegg archive.egg                 # extract all files
unegg archive.egg file.txt        # extract a specific file
unegg -d output/ archive.egg      # extract into a directory
unegg -P SECRET archive.egg       # extract an encrypted archive
unegg -l archive.egg              # list contents
unegg -p archive.egg file.txt     # extract to stdout
cat archive.egg | unegg -l -      # read from stdin
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
cargo install unegg
```

## Supported Features

### Compression
- Store (no compression)
- Deflate
- Bzip2
- LZMA
- AZO

### Encryption
- ZipCrypto (32-bit variant)
- AES-128 / AES-256 (CTR mode, PBKDF2-HMAC-SHA1)
- LEA-128 / LEA-256 (CTR mode, PBKDF2-HMAC-SHA1)

### Archive types
- Solid archives (continuous compressed stream across files)
- Split (multi-volume) archives (automatic volume discovery)

## Docs

[https://github.com/alkegi/docs](https://github.com/alkegi/docs)

---

Part of the [alkegi (알깨기)](https://github.com/alkegi) project.
