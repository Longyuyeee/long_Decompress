use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use caesium::parameters::CSParameters;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use oxipng::{InFile, Options, OutFile, StripChunks};
use thiserror::Error;

use super::output_publish_transaction::{
    cleanup_staged_output_family, publish_verified_file, staged_output_path, PublishError,
};

const MAX_DECODED_PIXELS: u64 = 100_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageCompressionMode {
    Lossy,
    Lossless,
}

#[derive(Debug, Clone)]
pub struct ImageCompressionRequest {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub mode: ImageCompressionMode,
    pub quality: u8,
    pub preserve_metadata: bool,
    pub only_if_smaller: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageCompressionOutcome {
    Published(ImageCompressionFacts),
    KeptSourceBecauseOutputWasNotSmaller {
        input_bytes: u64,
        encoded_bytes: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageCompressionFacts {
    pub format: ImageFileFormat,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub encoded_width: u32,
    pub encoded_height: u32,
    pub visible_width: u32,
    pub visible_height: u32,
    pub orientation: u8,
    pub frame_count: u32,
    pub has_alpha: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFileFormat {
    Jpeg,
    Png,
    WebP,
}

#[derive(Debug, Error)]
pub enum ImageCompressionError {
    #[error("image compression was cancelled")]
    Cancelled,
    #[error("source and destination must be different files")]
    SourceEqualsDestination,
    #[error("destination already exists and will not be overwritten: {0}")]
    DestinationExists(PathBuf),
    #[error("destination extension does not match the decoded {0:?} input format")]
    DestinationFormatMismatch(ImageFileFormat),
    #[error("unsupported or invalid image input: {0}")]
    InvalidInput(String),
    #[error("animated images are not supported by this image encoder (detected {0} frames)")]
    AnimatedInput(u32),
    #[error("lossy mode is not valid for PNG; choose lossless mode")]
    LossyPng,
    #[error("quality must be between 1 and 100")]
    InvalidQuality,
    #[error("encoded output verification failed: {0}")]
    Verification(String),
    #[error("image encoder failed: {0}")]
    Encoder(String),
    #[error("verified output could not be published: {0}")]
    Publish(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<PublishError> for ImageCompressionError {
    fn from(error: PublishError) -> Self {
        match error {
            PublishError::Cancelled => Self::Cancelled,
            other => Self::Publish(other.to_string()),
        }
    }
}

struct StagedOutputGuard(PathBuf);

impl Drop for StagedOutputGuard {
    fn drop(&mut self) {
        cleanup_staged_output_family(&self.0);
    }
}

pub fn compress_single_image(
    request: &ImageCompressionRequest,
    cancelled: &AtomicBool,
) -> Result<ImageCompressionOutcome, ImageCompressionError> {
    validate_request(request)?;
    check_cancelled(cancelled)?;

    let input = decode_facts(&request.source, true)?;
    if input.frame_count != 1 {
        return Err(ImageCompressionError::AnimatedInput(input.frame_count));
    }
    if !destination_extension_matches(&request.destination, input.format) {
        return Err(ImageCompressionError::DestinationFormatMismatch(
            input.format,
        ));
    }
    if input.format == ImageFileFormat::Png && request.mode == ImageCompressionMode::Lossy {
        return Err(ImageCompressionError::LossyPng);
    }

    let input_bytes = fs::metadata(&request.source)?.len();
    let staged = staged_output_path(&request.destination, "image-compress")?;
    let guard = StagedOutputGuard(staged.clone());
    encode(request, input.format, &staged)?;
    check_cancelled(cancelled)?;

    let output = decode_facts(&staged, false)
        .map_err(|error| ImageCompressionError::Verification(error.to_string()))?;
    verify_output(&input, &output)?;
    let encoded_bytes = fs::metadata(&staged)?.len();

    if request.only_if_smaller && encoded_bytes >= input_bytes {
        return Ok(
            ImageCompressionOutcome::KeptSourceBecauseOutputWasNotSmaller {
                input_bytes,
                encoded_bytes,
            },
        );
    }

    publish_verified_file(&staged, &request.destination, || {
        cancelled.load(Ordering::Acquire)
    })?;
    drop(guard);

    Ok(ImageCompressionOutcome::Published(ImageCompressionFacts {
        input_bytes,
        output_bytes: encoded_bytes,
        ..output
    }))
}

fn destination_extension_matches(path: &Path, format: ImageFileFormat) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match format {
        ImageFileFormat::Jpeg => matches!(extension.as_str(), "jpg" | "jpeg"),
        ImageFileFormat::Png => extension == "png",
        ImageFileFormat::WebP => extension == "webp",
    }
}

fn validate_request(request: &ImageCompressionRequest) -> Result<(), ImageCompressionError> {
    if request.quality == 0 || request.quality > 100 {
        return Err(ImageCompressionError::InvalidQuality);
    }
    if request.source == request.destination {
        return Err(ImageCompressionError::SourceEqualsDestination);
    }
    if request.destination.exists() {
        return Err(ImageCompressionError::DestinationExists(
            request.destination.clone(),
        ));
    }
    Ok(())
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<(), ImageCompressionError> {
    if cancelled.load(Ordering::Acquire) {
        Err(ImageCompressionError::Cancelled)
    } else {
        Ok(())
    }
}

fn encode(
    request: &ImageCompressionRequest,
    format: ImageFileFormat,
    staged: &Path,
) -> Result<(), ImageCompressionError> {
    match format {
        ImageFileFormat::Png => {
            let mut options = Options::from_preset(3);
            options.optimize_alpha = false;
            options.max_decompressed_size = Some(MAX_DECODED_PIXELS as usize * 8);
            if !request.preserve_metadata {
                options.strip = StripChunks::Safe;
            }
            oxipng::optimize(
                &InFile::Path(request.source.clone()),
                &OutFile::Path {
                    path: Some(staged.to_path_buf()),
                    preserve_attrs: false,
                },
                &options,
            )
            .map(|_| ())
            .map_err(|error| ImageCompressionError::Encoder(error.to_string()))
        }
        ImageFileFormat::Jpeg | ImageFileFormat::WebP => {
            let mut parameters = CSParameters::new();
            parameters.keep_metadata = request.preserve_metadata;
            // Orientation is visual-integrity data and is preserved independently of the
            // user's optional metadata policy.
            parameters.keep_rotation = true;
            parameters.jpeg.quality = u32::from(request.quality);
            parameters.jpeg.optimize = request.mode == ImageCompressionMode::Lossless;
            parameters.webp.quality = u32::from(request.quality);
            parameters.webp.lossless = request.mode == ImageCompressionMode::Lossless;
            caesium::compress(
                request.source.to_string_lossy().into_owned(),
                staged.to_string_lossy().into_owned(),
                &parameters,
            )
            .map_err(|error| ImageCompressionError::Encoder(error.to_string()))
        }
    }
}

fn verify_output(
    input: &ImageCompressionFacts,
    output: &ImageCompressionFacts,
) -> Result<(), ImageCompressionError> {
    if output.format != input.format {
        return Err(ImageCompressionError::Verification(
            "encoded format changed unexpectedly".into(),
        ));
    }
    if output.frame_count != 1 {
        return Err(ImageCompressionError::Verification(format!(
            "expected one frame, decoded {}",
            output.frame_count
        )));
    }
    if output.encoded_width != input.encoded_width
        || output.encoded_height != input.encoded_height
        || output.visible_width != input.visible_width
        || output.visible_height != input.visible_height
        || output.orientation != input.orientation
    {
        return Err(ImageCompressionError::Verification(
            "encoded matrix, orientation, or visible dimensions changed".into(),
        ));
    }
    Ok(())
}

fn decode_facts(path: &Path, input: bool) -> Result<ImageCompressionFacts, ImageCompressionError> {
    let reader = ImageReader::open(path)
        .and_then(|reader| reader.with_guessed_format())
        .map_err(|error| invalid_image(input, error))?;
    let format = match reader.format() {
        Some(ImageFormat::Jpeg) => ImageFileFormat::Jpeg,
        Some(ImageFormat::Png) => ImageFileFormat::Png,
        Some(ImageFormat::WebP) => ImageFileFormat::WebP,
        _ => {
            return Err(invalid_image(
                input,
                "only JPEG, PNG and WebP are supported",
            ))
        }
    };

    let frame_count = detect_frame_count(path, format)?;
    let mut decoder = reader
        .into_decoder()
        .map_err(|error| invalid_image(input, error))?;
    let (encoded_width, encoded_height) = decoder.dimensions();
    let pixels = u64::from(encoded_width) * u64::from(encoded_height);
    if pixels == 0 || pixels > MAX_DECODED_PIXELS {
        return Err(invalid_image(
            input,
            format!("decoded pixel count {pixels} exceeds the 100 MP safety boundary"),
        ));
    }
    let has_alpha = decoder.color_type().has_alpha();
    let orientation = decoder
        .orientation()
        .map_err(|error| invalid_image(input, error))?
        .to_exif();
    let decoded =
        DynamicImage::from_decoder(decoder).map_err(|error| invalid_image(input, error))?;
    if decoded.width() != encoded_width || decoded.height() != encoded_height {
        return Err(invalid_image(
            input,
            "decoder returned an inconsistent pixel matrix",
        ));
    }
    let (visible_width, visible_height) = if matches!(orientation, 5..=8) {
        (encoded_height, encoded_width)
    } else {
        (encoded_width, encoded_height)
    };
    Ok(ImageCompressionFacts {
        format,
        input_bytes: 0,
        output_bytes: 0,
        encoded_width,
        encoded_height,
        visible_width,
        visible_height,
        orientation,
        frame_count,
        has_alpha,
    })
}

fn invalid_image(input: bool, error: impl std::fmt::Display) -> ImageCompressionError {
    if input {
        ImageCompressionError::InvalidInput(error.to_string())
    } else {
        ImageCompressionError::Verification(error.to_string())
    }
}

fn detect_frame_count(path: &Path, format: ImageFileFormat) -> Result<u32, ImageCompressionError> {
    let count = match format {
        ImageFileFormat::Jpeg => 1,
        ImageFileFormat::Png => png_frame_count(path)?,
        ImageFileFormat::WebP => webp_frame_count(path)?,
    };
    Ok(count)
}

fn png_frame_count(path: &Path) -> Result<u32, std::io::Error> {
    let mut file = fs::File::open(path)?;
    file.seek(SeekFrom::Start(8))?;
    loop {
        let mut header = [0u8; 8];
        if file.read_exact(&mut header).is_err() {
            return Ok(1);
        }
        let length = u32::from_be_bytes(header[..4].try_into().unwrap());
        if &header[4..] == b"acTL" && length >= 4 {
            let mut frames = [0u8; 4];
            file.read_exact(&mut frames)?;
            return Ok(u32::from_be_bytes(frames).max(1));
        }
        file.seek(SeekFrom::Current(i64::from(length) + 4))?;
    }
}

fn webp_frame_count(path: &Path) -> Result<u32, std::io::Error> {
    let mut file = fs::File::open(path)?;
    let mut riff = [0u8; 12];
    if file.read_exact(&mut riff).is_err() || &riff[..4] != b"RIFF" || &riff[8..] != b"WEBP" {
        return Ok(1);
    }
    let mut frames = 0u32;
    loop {
        let mut header = [0u8; 8];
        if file.read_exact(&mut header).is_err() {
            return Ok(frames.max(1));
        }
        let length = u32::from_le_bytes(header[4..].try_into().unwrap());
        if &header[..4] == b"ANMF" {
            frames = frames.saturating_add(1);
        }
        file.seek(SeekFrom::Current(i64::from(
            length.saturating_add(length & 1),
        )))?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("test-results")
            .join("media-fixture-audit")
            .join("fixtures")
            .join("images")
            .join(name)
    }

    fn request(source: PathBuf, destination: PathBuf) -> ImageCompressionRequest {
        ImageCompressionRequest {
            source,
            destination,
            mode: ImageCompressionMode::Lossy,
            quality: 80,
            preserve_metadata: true,
            only_if_smaller: false,
        }
    }

    #[test]
    fn real_oriented_jpeg_is_redecoded_and_published_atomically() {
        let source = fixture("exif-orientation.jpg");
        if !source.exists() {
            eprintln!("fixture is absent; run npm run test:fixtures:media:images");
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("result.jpg");
        let outcome = compress_single_image(
            &request(source, destination.clone()),
            &AtomicBool::new(false),
        )
        .unwrap();
        let ImageCompressionOutcome::Published(facts) = outcome else {
            panic!("output should publish");
        };
        assert_eq!((facts.encoded_width, facts.encoded_height), (640, 360));
        assert_eq!((facts.visible_width, facts.visible_height), (360, 640));
        assert_eq!(facts.orientation, 6);
        assert_eq!(facts.frame_count, 1);
        assert!(destination.is_file());
        assert!(!fs::read_dir(temp.path())
            .unwrap()
            .flatten()
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with(".image-compress-")));
    }

    #[test]
    fn transparent_png_remains_lossless_and_keeps_alpha() {
        let source = fixture("transparent.png");
        if !source.exists() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("result.png");
        let mut request = request(source, destination);
        request.mode = ImageCompressionMode::Lossless;
        let outcome = compress_single_image(&request, &AtomicBool::new(false)).unwrap();
        let ImageCompressionOutcome::Published(facts) = outcome else {
            panic!("output should publish");
        };
        assert!(facts.has_alpha);
    }

    #[test]
    fn gif_is_rejected_before_staging() {
        let source = fixture("animated.gif");
        if !source.exists() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("result.jpg");
        let error = compress_single_image(
            &request(source, destination.clone()),
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert!(matches!(error, ImageCompressionError::InvalidInput(_)));
        assert!(!destination.exists());
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
    }

    #[test]
    fn input_extension_is_ignored_but_destination_extension_must_match_magic() {
        let fixture = fixture("exif-orientation.jpg");
        if !fixture.exists() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let disguised_source = temp.path().join("input.bin");
        fs::copy(fixture, &disguised_source).unwrap();
        let valid_destination = temp.path().join("output.jpeg");
        assert!(matches!(
            compress_single_image(
                &request(disguised_source.clone(), valid_destination),
                &AtomicBool::new(false),
            )
            .unwrap(),
            ImageCompressionOutcome::Published(_)
        ));

        let wrong_destination = temp.path().join("output.webp");
        let error = compress_single_image(
            &request(disguised_source, wrong_destination.clone()),
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            ImageCompressionError::DestinationFormatMismatch(ImageFileFormat::Jpeg)
        ));
        assert!(!wrong_destination.exists());
    }

    #[test]
    fn cancellation_and_size_policy_leave_no_output_or_staging() {
        let source = fixture("photo.webp");
        if !source.exists() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("cancelled.webp");
        let error = compress_single_image(
            &request(source.clone(), destination.clone()),
            &AtomicBool::new(true),
        )
        .unwrap_err();
        assert!(matches!(error, ImageCompressionError::Cancelled));
        assert!(!destination.exists());

        let destination = temp.path().join("larger.webp");
        let mut policy_request = request(source, destination.clone());
        policy_request.mode = ImageCompressionMode::Lossless;
        policy_request.only_if_smaller = true;
        let outcome = compress_single_image(&policy_request, &AtomicBool::new(false)).unwrap();
        assert!(matches!(
            outcome,
            ImageCompressionOutcome::KeptSourceBecauseOutputWasNotSmaller { .. }
        ));
        assert!(!destination.exists());
        assert!(!fs::read_dir(temp.path())
            .unwrap()
            .flatten()
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with(".image-compress-")));
    }

    #[test]
    fn target_race_never_overwrites_existing_bytes() {
        let source = fixture("photo.webp");
        if !source.exists() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("result.webp");
        fs::write(&destination, b"existing").unwrap();
        let error = compress_single_image(
            &request(source, destination.clone()),
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert!(matches!(error, ImageCompressionError::DestinationExists(_)));
        assert_eq!(fs::read(destination).unwrap(), b"existing");
    }
}
