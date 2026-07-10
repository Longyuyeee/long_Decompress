use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use long_compress_assistant::database::migrations;
use long_compress_assistant::models::compression::DecompressOptions;
use long_compress_assistant::models::password::{PasswordCategory, PasswordEntry};
use long_compress_assistant::services::compression_service::{CompressionService, CompressionServiceConfig};
use long_compress_assistant::services::encrypted_password_service::EncryptedPasswordService;
use long_compress_assistant::services::io_buffer_pool::IOBufferPool;
use long_compress_assistant::services::password_query_service::PasswordQueryService;
use long_compress_assistant::services::rar_support::RarSupportService;
use long_compress_assistant::services::universal_engine::UniversalCliEngine;
use long_compress_assistant::utils::archive_tools::find_7z_command;
use sqlx::SqlitePool;
use tempfile::{tempdir, TempDir};

struct FlowHarness {
    _temp: TempDir,
    pool: SqlitePool,
    encrypted_service: Arc<EncryptedPasswordService>,
    service: CompressionService,
}

async fn build_harness() -> FlowHarness {
    let temp = tempdir().expect("temp dir");
    let pool = SqlitePool::connect("sqlite::memory:").await.expect("sqlite");
    migrations::init_tables(&pool).await.expect("migrations");
    ensure_usage_history_column(&pool).await;

    let data_dir = temp.path().join("password-book");
    let mut encrypted_service = EncryptedPasswordService::new(&data_dir);
    encrypted_service
        .initialize("test-master-password")
        .await
        .expect("initialize password service");
    let encrypted_service = Arc::new(encrypted_service);

    let query_service = Arc::new(PasswordQueryService::new(pool.clone(), encrypted_service.clone()));
    let service = CompressionService::new(
        CompressionServiceConfig::default(),
        Arc::new(IOBufferPool::default()),
        Arc::new(RarSupportService::new()),
        Arc::new(UniversalCliEngine::new()),
        query_service,
    );

    FlowHarness {
        _temp: temp,
        pool,
        encrypted_service,
        service,
    }
}

async fn ensure_usage_history_column(pool: &SqlitePool) {
    let columns: Vec<(String,)> = sqlx::query_as("PRAGMA table_info(password_entries)")
        .fetch_all(pool)
        .await
        .expect("password_entries pragma")
        .into_iter()
        .map(|row: (i64, String, String, i64, Option<String>, i64)| (row.1,))
        .collect();

    if !columns.iter().any(|(name,)| name == "usage_history") {
        sqlx::query("ALTER TABLE password_entries ADD COLUMN usage_history TEXT NOT NULL DEFAULT '{}'")
            .execute(pool)
            .await
            .expect("add usage_history");
    }
}

async fn seed_password_book(harness: &FlowHarness, name: &str, password: &str, use_count: i32) {
    let mut entry = PasswordEntry::new(
        name.to_string(),
        password.to_string(),
        PasswordCategory::Other,
    );
    entry.use_count = use_count as u32;

    let entry = harness
        .encrypted_service
        .add_password(entry)
        .await
        .expect("add password file entry");

    sqlx::query(
        r#"
        INSERT OR IGNORE INTO password_keys (
            id, key_type, algorithm, key_data, key_hash, key_size, key_version,
            created_at, active, archived, metadata
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind("test-key")
    .bind("Master")
    .bind("PLAINTEXT_TEST")
    .bind("test-key-data")
    .bind("test-key-hash")
    .bind(0_i32)
    .bind(1_i32)
    .bind(entry.created_at)
    .bind(true)
    .bind(false)
    .bind("{}")
    .execute(&harness.pool)
    .await
    .expect("insert password key");

    sqlx::query(
        r#"
        INSERT INTO password_entries (
            id, name, username, password, url, notes, tags, category, strength,
            key_id, encryption_algorithm, encryption_version, created_at, updated_at,
            last_used, use_count, expires_at, favorite, archived, deleted,
            usage_history, custom_fields
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&entry.id)
    .bind(&entry.name)
    .bind(&entry.username)
    .bind(&entry.password)
    .bind(&entry.url)
    .bind(&entry.notes)
    .bind(serde_json::to_string(&entry.tags).expect("tags json"))
    .bind(entry.category.to_string())
    .bind(entry.strength.to_string())
    .bind("test-key")
    .bind("PLAINTEXT_TEST")
    .bind(1_i32)
    .bind(entry.created_at)
    .bind(entry.updated_at)
    .bind(entry.last_used)
    .bind(entry.use_count as i32)
    .bind(entry.expires_at)
    .bind(entry.favorite)
    .bind(false)
    .bind(false)
    .bind("{}")
    .bind(serde_json::to_string(&entry.custom_fields).expect("fields json"))
    .execute(&harness.pool)
    .await
    .expect("insert password index row");
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
