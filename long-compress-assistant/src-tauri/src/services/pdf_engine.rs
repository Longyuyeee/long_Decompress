use anyhow::{bail, Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

const RUNTIME_DIRECTORY: &str = "pdf-engine";

pub fn bundled_pdf_resource_root(app_resource_dir: &Path) -> PathBuf {
    app_resource_dir.join("resources")
}

#[derive(Clone, Copy)]
struct ExpectedResource {
    relative_path: &'static str,
    bytes: u64,
    sha256: &'static str,
}

const EXPECTED_RESOURCES: [ExpectedResource; 10] = [
    ExpectedResource {
        relative_path: "qpdf.exe",
        bytes: 114_927,
        sha256: "832b73b371db31908f4dc7a5c1411c066d3e030121807377d739803f4d211b24",
    },
    ExpectedResource {
        relative_path: "qpdf30.dll",
        bytes: 9_645_568,
        sha256: "970ead48587b3fcd2651706c846597a4ff212357da2f2dd54787cfeacc4837cc",
    },
    ExpectedResource {
        relative_path: "libgcc_s_seh-1.dll",
        bytes: 150_998,
        sha256: "b37c1770c8ca092700875845b34918803ee6311573eba1c32ff4b1166e4a0e1e",
    },
    ExpectedResource {
        relative_path: "libstdc++-6.dll",
        bytes: 2_661_299,
        sha256: "887c21dbe2a211ac4d1a790e4f608b7dee27fae12352856963004e7a715d2e6c",
    },
    ExpectedResource {
        relative_path: "libwinpthread-1.dll",
        bytes: 64_419,
        sha256: "d54ed5baa6d339e28fe18c0106caffd110ac42612908593e07211d7bb48f5e79",
    },
    ExpectedResource {
        relative_path: "SOURCE.txt",
        bytes: 642,
        sha256: "1e61a4b304ed7dd7947e5dbfafb4f995c508966617da60d3a201236a9668646b",
    },
    ExpectedResource {
        relative_path: "licenses/qpdf-LICENSE.txt",
        bytes: 11_358,
        sha256: "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30",
    },
    ExpectedResource {
        relative_path: "licenses/qpdf-NOTICE.md",
        bytes: 2_729,
        sha256: "b207f65a9e5491195ded63b2941199b19a4d30148871f2742c88eae7bfc513a6",
    },
    ExpectedResource {
        relative_path: "licenses/GCC-MinGW-runtime-copyright.txt",
        bytes: 75_729,
        sha256: "a481f772f7a53335f13b32c6c54eb1c8577ce97704edd3757ab7ed4287a8e96a",
    },
    ExpectedResource {
        relative_path: "licenses/MinGW-w64-copyright.txt",
        bytes: 37_808,
        sha256: "af23297b7d17e8e31817a5d58088de6fdafe34705bb5a7d1d330f08711b31314",
    },
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PdfEngineFileIdentity {
    pub relative_path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PdfEngineStatus {
    pub version: String,
    pub license: String,
    pub crypto_providers: Vec<String>,
    pub supports_json_v2: bool,
    pub supports_image_optimization: bool,
    pub files: Vec<PdfEngineFileIdentity>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPdfEnginePreflightReport {
    pub schema_version: u32,
    pub executable_path: String,
    pub resource_root: String,
    pub passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<PdfEngineStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn hash_file(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .with_context(|| format!("cannot read {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn inspect_resources(resource_root: &Path) -> Result<(PathBuf, Vec<PdfEngineFileIdentity>)> {
    let runtime_root = resource_root.join(RUNTIME_DIRECTORY);
    let mut identities = Vec::with_capacity(EXPECTED_RESOURCES.len());
    for expected in EXPECTED_RESOURCES {
        let path = runtime_root.join(expected.relative_path);
        let metadata = std::fs::metadata(&path)
            .with_context(|| format!("PDF_ENGINE_RESOURCE_MISSING: {}", path.display()))?;
        if !metadata.is_file() {
            bail!("PDF_ENGINE_RESOURCE_NOT_FILE: {}", path.display());
        }
        if metadata.len() != expected.bytes {
            bail!(
                "PDF_ENGINE_RESOURCE_SIZE_MISMATCH: {} expected={} actual={}",
                path.display(),
                expected.bytes,
                metadata.len()
            );
        }
        let sha256 = hash_file(&path)?;
        if sha256 != expected.sha256 {
            bail!(
                "PDF_ENGINE_RESOURCE_HASH_MISMATCH: {} expected={} actual={}",
                path.display(),
                expected.sha256,
                sha256
            );
        }
        identities.push(PdfEngineFileIdentity {
            relative_path: expected.relative_path.replace('/', "\\"),
            bytes: metadata.len(),
            sha256,
        });
    }
    Ok((runtime_root, identities))
}

fn run_probe(executable: &Path, arguments: &[&str], label: &str) -> Result<String> {
    let output = Command::new(executable)
        .args(arguments)
        .output()
        .with_context(|| {
            format!(
                "PDF_ENGINE_LAUNCH_FAILED: {label}: {}",
                executable.display()
            )
        })?;
    if !output.status.success() {
        bail!(
            "PDF_ENGINE_PROBE_FAILED: {label}: exit={:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

pub fn validate_pdf_engine(resource_root: &Path) -> Result<PdfEngineStatus> {
    let (runtime_root, files) = inspect_resources(resource_root)?;
    let qpdf = runtime_root.join("qpdf.exe");

    let version = run_probe(&qpdf, &["--version"], "qpdf-version")?;
    if !version.contains("qpdf version 12.4.0") {
        bail!("PDF_ENGINE_VERSION_POLICY_MISMATCH");
    }

    let crypto = run_probe(&qpdf, &["--show-crypto"], "qpdf-crypto")?;
    let crypto_lower = crypto.to_lowercase();
    if !crypto_lower.lines().any(|line| line.trim() == "openssl")
        || !crypto_lower.lines().any(|line| line.trim() == "native")
    {
        bail!("PDF_ENGINE_CRYPTO_PROVIDER_MISMATCH");
    }

    let json_help = run_probe(&qpdf, &["--help=--json"], "qpdf-json-help")?;
    if !json_help.contains("--json[=version]") {
        bail!("PDF_ENGINE_JSON_V2_CAPABILITY_MISSING");
    }
    let image_help = run_probe(
        &qpdf,
        &["--help=--optimize-images"],
        "qpdf-image-optimization-help",
    )?;
    if !image_help.contains("DCT (JPEG)")
        || !image_help.contains("--oi-min-width")
        || !image_help.contains("--oi-min-height")
        || !image_help.contains("--oi-min-area")
    {
        bail!("PDF_ENGINE_IMAGE_OPTIMIZATION_CAPABILITY_MISSING");
    }

    Ok(PdfEngineStatus {
        version: "12.4.0".to_string(),
        license: "Apache-2.0".to_string(),
        crypto_providers: vec!["openssl".to_string(), "native".to_string()],
        supports_json_v2: true,
        supports_image_optimization: true,
        files,
    })
}

pub fn write_installed_pdf_engine_preflight_report(
    executable_path: &Path,
    report_path: &Path,
) -> Result<bool> {
    let install_root = executable_path.parent().with_context(|| {
        format!(
            "PDF_ENGINE_INSTALL_ROOT_UNAVAILABLE: {}",
            executable_path.display()
        )
    })?;
    let resource_root = install_root.join("resources");
    let validation = validate_pdf_engine(&resource_root);
    let (passed, status, error) = match validation {
        Ok(status) => (true, Some(status), None),
        Err(error) => (false, None, Some(error.to_string())),
    };
    let report = InstalledPdfEnginePreflightReport {
        schema_version: 1,
        executable_path: executable_path.display().to_string(),
        resource_root: resource_root.display().to_string(),
        passed,
        status,
        error,
    };
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create report directory {}", parent.display()))?;
    }
    std::fs::write(report_path, serde_json::to_vec_pretty(&report)?)
        .with_context(|| format!("cannot write report {}", report_path.display()))?;
    Ok(passed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repository_resource_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources")
    }

    #[test]
    fn packaged_resource_root_preserves_the_configured_prefix() {
        assert_eq!(
            bundled_pdf_resource_root(Path::new("install-root")),
            Path::new("install-root").join("resources")
        );
    }

    #[cfg(windows)]
    #[test]
    fn bundled_candidate_has_frozen_identity_and_capabilities() {
        let status = validate_pdf_engine(&repository_resource_root()).unwrap();
        assert_eq!(status.version, "12.4.0");
        assert_eq!(status.license, "Apache-2.0");
        assert!(status.supports_json_v2);
        assert!(status.supports_image_optimization);
        assert_eq!(status.files.len(), EXPECTED_RESOURCES.len());
    }

    #[test]
    fn missing_runtime_is_refused_before_process_launch() {
        let directory = tempfile::tempdir().unwrap();
        let error = validate_pdf_engine(directory.path())
            .unwrap_err()
            .to_string();
        assert!(error.contains("PDF_ENGINE_RESOURCE_MISSING"));
    }

    #[test]
    fn replaced_executable_is_refused_by_byte_identity() {
        let directory = tempfile::tempdir().unwrap();
        let target_root = directory.path().join(RUNTIME_DIRECTORY);
        std::fs::create_dir_all(target_root.join("licenses")).unwrap();
        for expected in EXPECTED_RESOURCES {
            let source = repository_resource_root()
                .join(RUNTIME_DIRECTORY)
                .join(expected.relative_path);
            let target = target_root.join(expected.relative_path);
            std::fs::copy(source, target).unwrap();
        }
        let executable = target_root.join("qpdf.exe");
        let mut bytes = std::fs::read(&executable).unwrap();
        bytes[0] ^= 0xff;
        std::fs::write(&executable, bytes).unwrap();

        let error = validate_pdf_engine(directory.path())
            .unwrap_err()
            .to_string();
        assert!(error.contains("PDF_ENGINE_RESOURCE_HASH_MISMATCH"));
    }
}
