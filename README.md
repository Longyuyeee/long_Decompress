# LongDecompress (胧解压·方便助手)

A desktop compression/decompression tool built with **Rust + Tauri + Vue 3**, supporting 37+ archive formats with password management and batch processing.

## Features

### Format Support

| Category | Formats |
|----------|---------|
| **Extract (37+)** | ZIP, 7Z, RAR, TAR, GZ, BZ2, XZ, Zstd, TAR.GZ, TAR.BZ2, TAR.XZ, TAR.Zst, ISO, IMG, CAB, LZH, LHA, ARJ, DMG, WIM, VHD, VHDX, CHM, DEB, RPM, SQUASHFS, SFS, NSIS, MSI, XAR, CPIO, UDF, FAT, NTFS, HFS, LZMA, ALZ, ARC, APFS, EXT2/3/4 |
| **Compress (16)** | ZIP, ZIP(pwd), 7Z, RAR(CLI), TAR, GZ, BZ2, XZ, Zstd, LZMA, TAR.GZ, TAR.BZ2, TAR.XZ, TAR.Zst, 7Z(pwd), ZIP(split) |
| **Password-protected** | ZIP, 7Z, RAR (both extract and compress) |

### Core
- Drag-and-drop file/folder selection
- Batch decompression with per-task progress
- Intelligent password matching from vault (`password-required` event → UI prompt)
- Task cancellation with proper cleanup
- Selective task execution (checkboxes)
- Split/ZIP password/Zstd/LZMA/ISO+ compression
- 7z CLI fallback for universal format support

### Password Vault
- AES-256 encrypted credential storage
- Per-task password entry with show/hide toggle
- Import/export (JSON format)
- Per-installation random master key
- Copy-to-clipboard, usage tracking

### UI
- 5 theme modes (Light, Dark, Cyberpunk, Twilight, Sepia)
- 13 accent colors
- Window position/size persistence
- Auto-dismissing notifications (5s errors, 3s success)
- Color-coded task status indicators
- Responsive layout with sidebar navigation

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Backend | Rust + Tauri 1.5 |
| Frontend | Vue 3.4 + TypeScript + Pinia + Tailwind CSS 3.4 |
| Icons | PrimeIcons |
| Archive libs | zip, sevenz-rust, unrar, flate2, tar, bzip2, xz2 |
| Database | SQLite (sqlx) |
| Crypto | AES-256-GCM, Argon2 |
| Testing | Vitest (frontend), cargo test (backend) |

## Project Structure

```
long-compress-assistant/
├── src/                    # Vue 3 frontend
│   ├── views/              # DecompressView, CompressionView, PasswordVaultView, SettingsView
│   ├── stores/             # Pinia stores (app, compression, task, password, ui, config)
│   ├── components/         # UI components (AeroTable, GlassCard, Modal, EnhancedFileDropzone, etc.)
│   ├── composables/        # useTauriCommands, useTheme
│   ├── i18n/               # zh-CN / en-US translations
│   └── styles/             # design-tokens.css, Tailwind utilities
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── commands/       # Tauri commands (compression, file, password, system, task_queue)
│   │   ├── services/       # Core logic (compression_service, universal_engine, rar_support, etc.)
│   │   ├── models/         # Data models (compression, file, password, system)
│   │   ├── database/       # SQLite connection, migrations, repositories
│   │   ├── crypto/         # AES-256-GCM encryption, Argon2 hashing, key management
│   │   ├── config/         # App configuration management
│   │   ├── task_queue/     # Async task scheduling and execution
│   │   └── utils/          # File utilities, error types, async helpers
│   └── tests/              # Integration tests (35 passing)
├── package.json
└── Cargo.toml
```

## Getting Started

### Prerequisites
- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) 1.70+
- [7-Zip](https://7-zip.org/) (optional, for universal format support)

### Development

```bash
# Clone
git clone https://github.com/Longyuyeee/long_Decompress.git
cd long_Decompress/long-compress-assistant

# Install frontend dependencies
npm install

# Run in dev mode
npm run tauri dev

# Build for production
npm run tauri build
```

### Running Tests

```bash
# Frontend unit tests
npm test

# Rust integration tests (35 tests across 6 suites)
cd src-tauri
cargo test --test compression_capabilities_regression
cargo test --test split_compression_test
cargo test --test fixes_validation_test
cargo test --test rar_support_test
cargo test --test password_zip_test
cargo test --test zip_compression_test
```

## Current Status

The project is in active development. A comprehensive audit and refactoring (56 commits, 17 rounds) has been completed, fixing 6 CRITICAL security issues, all crash risks, and major UX problems.

| Metric | Status |
|--------|--------|
| Integration tests | 35/35 passing |
| Frontend build | 0 errors |
| Rust lib build | 0 errors |
| Security CRITICAL | 0 remaining |
| Formats (extract) | 37+ |
| Formats (compress) | 16 |

For remaining work items, see [REMAINING_WORK.md](REMAINING_WORK.md).

## License

MIT
