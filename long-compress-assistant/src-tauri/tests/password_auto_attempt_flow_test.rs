use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use long_compress_assistant::models::compression::DecompressOptions;
use long_compress_assistant::models::password::{PasswordCategory, PasswordEntry};
use long_compress_assistant::services::compression_service::{CompressionService, CompressionServiceConfig};
use long_compress_assistant::services::encrypted_password_service::EncryptedPasswordService;
use long_compress_assistant::services::io_buffer_pool::IOBufferPool;
use long_compress_assistant::services::rar_support::RarSupportService;
use long_compress_assistant::services::universal_engine::UniversalCliEngine;
use long_compress_assistant::utils::archive_tools::find_7z_command;
use tempfile::{tempdir, TempDir};

struct FlowHarness {
    _temp: TempDir,
    encrypted_service: Arc<EncryptedPasswordService>,
    service: CompressionService,
}

async fn build_harness() -> FlowHarness {
    let temp = tempdir().expect("temp dir");
    let data_dir = temp.path().join("password-book");
    let mut encrypted_service = EncryptedPasswordService::new(&data_dir);
    encrypted_service
        .initialize("test-master-password")
        .await
        .expect("initialize password service");
    let encrypted_service = Arc::new(encrypted_service);

    let service = CompressionService::new(
        CompressionServiceConfig::default(),
        Arc::new(IOBufferPool::default()),
        Arc::new(RarSupportService::new()),
        Arc::new(UniversalCliEngine::new()),
        encrypted_service.clone(),
    );

    FlowHarness {
        _temp: temp,
        encrypted_service,
        service,
    }
}

async fn seed_password_book(harness: &FlowHarness, name: &str, password: &str, use_count: i32) {
    let mut entry = PasswordEntry::new(
        name.to_string(),
        password.to_string(),
        PasswordCategory::Other,
    );
    entry.use_count = use_count as u32;

    harness
        .encrypted_service
        .add_password(entry)
        .await
        .expect("add password file entry");
}

fn create_encrypted_7z(root: &Path, password: &str) -> std::path::PathBuf {
    let seven_zip = find_7z_command().expect("7za command available");
    let source = root.join("secret.txt");
    fs::write(&source, "password automation smoke").expect("fixture");
    let archive = root.join("secret.7z");

    let output = Command::new(seven_zip)
        .arg("a")
        .arg("-t7z")
        .arg("-mhe=on")
        .arg(format!("-p{}", password))
        .arg("-y")
        .arg(&archive)
        .arg(&source)
        .output()
        .expect("run 7za");

    assert!(
        output.status.success(),
        "7za failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    archive
}

fn create_plain_7z(root: &Path) -> std::path::PathBuf {
    let seven_zip = find_7z_command().expect("7za command available");
    let source = root.join("plain.txt");
    fs::write(&source, "plain archive").expect("fixture");
    let archive = root.join("plain.7z");

    let output = Command::new(seven_zip)
        .arg("a")
        .arg("-t7z")
        .arg("-y")
        .arg(&archive)
        .arg(&source)
        .output()
        .expect("run 7za");
    assert!(output.status.success(), "plain 7z creation failed");
    archive
}

fn create_content_encrypted_7z(root: &Path, password: &str) -> std::path::PathBuf {
    let seven_zip = find_7z_command().expect("7za command available");
    let source = root.join("content-secret.txt");
    fs::write(&source, "encrypted content with visible headers").expect("fixture");
    let archive = root.join("content-secret.7z");
    let output = Command::new(seven_zip)
        .arg("a")
        .arg("-t7z")
        .arg(format!("-p{}", password))
        .arg("-mhe=off")
        .arg("-y")
        .arg(&archive)
        .arg(&source)
        .output()
        .expect("run 7za");
    assert!(output.status.success(), "content-encrypted 7z creation failed");
    archive
}

#[tokio::test]
async fn rejects_arbitrary_passwords_for_unencrypted_7z() {
    let harness = build_harness().await;
    let archive = create_plain_7z(harness._temp.path());

    assert!(!harness
        .service
        .verify_archive_password_candidate(archive.to_str().unwrap(), "!@#$%^&*")
        .await
        .expect("plain 7z encryption check"));
    assert!(!harness
        .service
        .test_archive_password(archive.to_str().unwrap(), "!@#$%^&*")
        .await
        .expect("plain 7z password test"));
}

#[tokio::test]
async fn validates_7z_content_when_headers_are_not_encrypted() {
    let harness = build_harness().await;
    let archive = create_content_encrypted_7z(harness._temp.path(), "content-password");

    assert!(!harness
        .service
        .test_archive_password(archive.to_str().unwrap(), "wrong-password")
        .await
        .expect("wrong content password"));
    assert!(harness
        .service
        .test_archive_password(archive.to_str().unwrap(), "content-password")
        .await
        .expect("correct content password"));
}

#[tokio::test]
async fn resolves_encrypted_archive_password_from_password_book() {
    let harness = build_harness().await;
    seed_password_book(&harness, "wrong archive password", "not-this-one", 50).await;
    seed_password_book(&harness, "release bundle", "book-hit-2026", 10).await;

    let archive = create_encrypted_7z(harness._temp.path(), "book-hit-2026");
    let resolved = harness
        .service
        .resolve_archive_password_silent(archive.to_str().unwrap(), &DecompressOptions::default())
        .await;

    assert_eq!(resolved.as_deref(), Some("book-hit-2026"));
}

#[tokio::test]
async fn resolves_encrypted_archive_password_from_imported_wordlist() {
    let harness = build_harness().await;
    let archive = create_encrypted_7z(harness._temp.path(), "wordlist-hit-2026");
    let wordlist = harness._temp.path().join("passwords.txt");
    fs::write(&wordlist, "wrong\n\nwordlist-hit-2026\nwordlist-hit-2026\n").expect("wordlist");

    let options = DecompressOptions {
        enable_bruteforce: true,
        bruteforce_wordlists: vec![wordlist.to_string_lossy().to_string()],
        ..Default::default()
    };

    let resolved = harness
        .service
        .resolve_archive_password_silent(archive.to_str().unwrap(), &options)
        .await;

    assert_eq!(resolved.as_deref(), Some("wordlist-hit-2026"));
}

#[tokio::test]
async fn does_not_run_the_builtin_dictionary_without_explicit_authorization() {
    let harness = build_harness().await;
    let archive = create_encrypted_7z(harness._temp.path(), "123456");

    let resolved = harness
        .service
        .resolve_archive_password_silent(archive.to_str().unwrap(), &DecompressOptions::default())
        .await;

    assert_eq!(resolved, None);
}

#[tokio::test]
async fn runs_the_builtin_dictionary_after_explicit_authorization() {
    let harness = build_harness().await;
    let archive = create_encrypted_7z(harness._temp.path(), "123456");
    let options = DecompressOptions {
        enable_bruteforce: true,
        ..Default::default()
    };

    let resolved = harness
        .service
        .resolve_archive_password_silent(archive.to_str().unwrap(), &options)
        .await;

    assert_eq!(resolved.as_deref(), Some("123456"));
}
