use std::fs;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use caesium::parameters::CSParameters;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use img_parts::{DynImage, ImageEXIF, ImageICC};
use oxipng::{InFile, Options, OutFile};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::output_publish_transaction::{
    cleanup_staged_output_family, publish_verified_file, staged_output_path, PublishError,
};

const MAX_DECODED_PIXELS: u64 = 100_000_000;
const MAX_ENCODED_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImageCompressionMode {
    Lossy,
    Lossless,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageCompressionRequest {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub mode: ImageCompressionMode,
    pub quality: u8,
    pub target_format: ImageFileFormat,
    pub max_dimensions: Option<ImageDimensions>,
    pub preserve_metadata: bool,
    pub only_if_smaller: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum ImageCompressionOutcome {
    Published(ImageCompressionFacts),
    KeptSourceBecauseOutputWasNotSmaller {
        input_bytes: u64,
        encoded_bytes: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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
    #[error("lossless JPEG is only available when optimizing an existing JPEG")]
    LosslessJpegConversion,
    #[error("resize limits must both be between 1 and 32768 pixels")]
    InvalidResize,
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
    if !destination_extension_matches(&request.destination, request.target_format) {
        return Err(ImageCompressionError::DestinationFormatMismatch(
            request.target_format,
        ));
    }
    if request.target_format == ImageFileFormat::Png && request.mode == ImageCompressionMode::Lossy
    {
        return Err(ImageCompressionError::LossyPng);
    }
    if request.target_format == ImageFileFormat::Jpeg
        && input.format != ImageFileFormat::Jpeg
        && request.mode == ImageCompressionMode::Lossless
    {
        return Err(ImageCompressionError::LosslessJpegConversion);
    }

    let target_visible = target_visible_dimensions(&input, request.max_dimensions);
    let normalizes_orientation = request.target_format != input.format && input.orientation != 1;
    let source_metadata = read_metadata(&request.source, true)?;

    let input_bytes = fs::metadata(&request.source)?.len();
    let staged = staged_output_path(&request.destination, "image-compress")?;
    let guard = StagedOutputGuard(staged.clone());
    encode(request, &input, target_visible, &staged)?;
    let expected_metadata = apply_metadata_policy(
        &staged,
        &source_metadata,
        request.preserve_metadata,
        if normalizes_orientation {
            1
        } else {
            input.orientation
        },
    )?;
    check_cancelled(cancelled)?;

    let output = decode_facts(&staged, false)
        .map_err(|error| ImageCompressionError::Verification(error.to_string()))?;
    verify_output(
        &input,
        &output,
        request.target_format,
        target_visible,
        if normalizes_orientation {
            1
        } else {
            input.orientation
        },
    )?;
    let output_metadata = read_metadata(&staged, false)?;
    if output_metadata != expected_metadata {
        return Err(ImageCompressionError::Verification(
            "EXIF/ICC metadata policy was not preserved by the encoded container".into(),
        ));
    }
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
    if request.max_dimensions.is_some_and(|value| {
        value.width == 0 || value.height == 0 || value.width > 32_768 || value.height > 32_768
    }) {
        return Err(ImageCompressionError::InvalidResize);
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
    input: &ImageCompressionFacts,
    target_visible: ImageDimensions,
    staged: &Path,
) -> Result<(), ImageCompressionError> {
    let resized = target_visible.width != input.visible_width
        || target_visible.height != input.visible_height;
    match request.target_format {
        ImageFileFormat::Png => {
            if input.format == ImageFileFormat::Png && !resized {
                let mut options = Options::from_preset(3);
                options.optimize_alpha = false;
                options.max_decompressed_size = Some(MAX_DECODED_PIXELS as usize * 8);
                return oxipng::optimize(
                    &InFile::Path(request.source.clone()),
                    &OutFile::Path {
                        path: Some(staged.to_path_buf()),
                        preserve_attrs: false,
                    },
                    &options,
                )
                .map(|_| ())
                .map_err(|error| ImageCompressionError::Encoder(error.to_string()));
            }

            let mut image = ImageReader::open(&request.source)
                .and_then(|reader| reader.with_guessed_format())
                .map_err(|error| ImageCompressionError::Encoder(error.to_string()))?
                .decode()
                .map_err(|error| ImageCompressionError::Encoder(error.to_string()))?;
            image.apply_orientation(
                image::metadata::Orientation::from_exif(input.orientation)
                    .unwrap_or(image::metadata::Orientation::NoTransforms),
            );
            if resized {
                image = image.resize_exact(
                    target_visible.width,
                    target_visible.height,
                    image::imageops::FilterType::Lanczos3,
                );
            }
            let mut encoded = Vec::new();
            image
                .write_to(&mut Cursor::new(&mut encoded), ImageFormat::Png)
                .map_err(|error| ImageCompressionError::Encoder(error.to_string()))?;
            let mut options = Options::from_preset(3);
            options.optimize_alpha = false;
            options.max_decompressed_size = Some(MAX_DECODED_PIXELS as usize * 8);
            let optimized = oxipng::optimize_from_memory(&encoded, &options)
                .map_err(|error| ImageCompressionError::Encoder(error.to_string()))?;
            fs::write(staged, optimized).map_err(ImageCompressionError::Io)
        }
        ImageFileFormat::Jpeg | ImageFileFormat::WebP => {
            let mut parameters = CSParameters::new();
            parameters.keep_metadata = true;
            // Orientation is visual-integrity data and is preserved independently of the
            // user's optional metadata policy.
            parameters.keep_rotation = true;
            parameters.jpeg.quality = u32::from(request.quality);
            parameters.jpeg.optimize = request.mode == ImageCompressionMode::Lossless;
            parameters.webp.quality = u32::from(request.quality);
            parameters.webp.lossless = request.mode == ImageCompressionMode::Lossless;
            if resized {
                parameters.width = target_visible.width;
                parameters.height = target_visible.height;
            }
            let result = if input.format == request.target_format {
                caesium::compress(
                    request.source.to_string_lossy().into_owned(),
                    staged.to_string_lossy().into_owned(),
                    &parameters,
                )
            } else {
                caesium::convert(
                    request.source.to_string_lossy().into_owned(),
                    staged.to_string_lossy().into_owned(),
                    &parameters,
                    match request.target_format {
                        ImageFileFormat::Jpeg => caesium::SupportedFileTypes::Jpeg,
                        ImageFileFormat::WebP => caesium::SupportedFileTypes::WebP,
                        ImageFileFormat::Png => unreachable!(),
                    },
                )
            };
            result.map_err(|error| ImageCompressionError::Encoder(error.to_string()))
        }
    }
}

fn verify_output(
    input: &ImageCompressionFacts,
    output: &ImageCompressionFacts,
    expected_format: ImageFileFormat,
    expected_visible: ImageDimensions,
    expected_orientation: u8,
) -> Result<(), ImageCompressionError> {
    if output.format != expected_format {
        return Err(ImageCompressionError::Verification(
            "encoded format does not match the requested output format".into(),
        ));
    }
    if output.frame_count != 1 {
        return Err(ImageCompressionError::Verification(format!(
            "expected one frame, decoded {}",
            output.frame_count
        )));
    }
    let expected_encoded = if matches!(expected_orientation, 5..=8) {
        ImageDimensions {
            width: expected_visible.height,
            height: expected_visible.width,
        }
    } else {
        expected_visible
    };
    if output.encoded_width != expected_encoded.width
        || output.encoded_height != expected_encoded.height
        || output.visible_width != expected_visible.width
        || output.visible_height != expected_visible.height
        || output.orientation != expected_orientation
    {
        return Err(ImageCompressionError::Verification(
            "encoded matrix, orientation, or visible dimensions changed".into(),
        ));
    }
    if expected_format != ImageFileFormat::Jpeg && output.has_alpha != input.has_alpha {
        return Err(ImageCompressionError::Verification(
            "alpha-channel semantics changed unexpectedly".into(),
        ));
    }
    Ok(())
}

fn target_visible_dimensions(
    input: &ImageCompressionFacts,
    limit: Option<ImageDimensions>,
) -> ImageDimensions {
    let original = ImageDimensions {
        width: input.visible_width,
        height: input.visible_height,
    };
    let Some(limit) = limit else {
        return original;
    };
    if original.width <= limit.width && original.height <= limit.height {
        return original;
    }
    let width_limited_height =
        u64::from(original.height) * u64::from(limit.width) / u64::from(original.width);
    if width_limited_height <= u64::from(limit.height) {
        ImageDimensions {
            width: limit.width,
            height: width_limited_height.max(1) as u32,
        }
    } else {
        ImageDimensions {
            width: (u64::from(original.width) * u64::from(limit.height)
                / u64::from(original.height))
            .max(1) as u32,
            height: limit.height,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MetadataPayload {
    exif: Option<Vec<u8>>,
    icc: Option<Vec<u8>>,
}

fn read_metadata(path: &Path, input: bool) -> Result<MetadataPayload, ImageCompressionError> {
    let bytes = fs::read(path)?;
    let image = DynImage::from_bytes(img_parts::Bytes::from(bytes))
        .map_err(|error| invalid_image(input, error))?
        .ok_or_else(|| invalid_image(input, "metadata container format is unsupported"))?;
    Ok(MetadataPayload {
        exif: image.exif().map(|value| value.to_vec()),
        icc: image.icc_profile().map(|value| value.to_vec()),
    })
}

fn apply_metadata_policy(
    path: &Path,
    source: &MetadataPayload,
    preserve_metadata: bool,
    expected_orientation: u8,
) -> Result<MetadataPayload, ImageCompressionError> {
    let expected = if preserve_metadata {
        MetadataPayload {
            exif: source.exif.as_ref().map(|value| {
                if expected_orientation == 1 {
                    set_exif_orientation(value, 1)
                } else {
                    value.clone()
                }
            }),
            icc: source.icc.clone(),
        }
    } else {
        MetadataPayload {
            exif: (expected_orientation != 1).then(|| build_orientation_exif(expected_orientation)),
            icc: None,
        }
    };

    let bytes = fs::read(path)?;
    let mut image = DynImage::from_bytes(img_parts::Bytes::from(bytes))
        .map_err(|error| ImageCompressionError::Encoder(error.to_string()))?
        .ok_or_else(|| {
            ImageCompressionError::Encoder("encoded metadata container is unsupported".into())
        })?;
    image.set_exif(expected.exif.clone().map(img_parts::Bytes::from));
    image.set_icc_profile(expected.icc.clone().map(img_parts::Bytes::from));
    let mut output = Vec::new();
    image
        .encoder()
        .write_to(&mut output)
        .map_err(|error| ImageCompressionError::Encoder(error.to_string()))?;
    fs::write(path, output)?;
    Ok(expected)
}

fn build_orientation_exif(orientation: u8) -> Vec<u8> {
    let mut exif = Vec::with_capacity(26);
    exif.extend_from_slice(b"II*\0");
    exif.extend_from_slice(&8u32.to_le_bytes());
    exif.extend_from_slice(&1u16.to_le_bytes());
    exif.extend_from_slice(&0x0112u16.to_le_bytes());
    exif.extend_from_slice(&3u16.to_le_bytes());
    exif.extend_from_slice(&1u32.to_le_bytes());
    exif.extend_from_slice(&u16::from(orientation).to_le_bytes());
    exif.extend_from_slice(&[0, 0]);
    exif.extend_from_slice(&0u32.to_le_bytes());
    exif
}

fn set_exif_orientation(exif: &[u8], orientation: u8) -> Vec<u8> {
    let Some(header) = exif.get(..8) else {
        return exif.to_vec();
    };
    let big_endian = match &header[..2] {
        b"MM" => true,
        b"II" => false,
        _ => return exif.to_vec(),
    };
    let read_u16 = |value: &[u8]| {
        if big_endian {
            u16::from_be_bytes([value[0], value[1]])
        } else {
            u16::from_le_bytes([value[0], value[1]])
        }
    };
    let read_u32 = |value: &[u8]| {
        if big_endian {
            u32::from_be_bytes([value[0], value[1], value[2], value[3]])
        } else {
            u32::from_le_bytes([value[0], value[1], value[2], value[3]])
        }
    };
    let ifd0 = read_u32(&header[4..8]) as usize;
    let Some(count) = exif.get(ifd0..ifd0 + 2).map(read_u16) else {
        return exif.to_vec();
    };
    for index in 0..usize::from(count) {
        let offset = ifd0 + 2 + index * 12;
        let Some(entry) = exif.get(offset..offset + 12) else {
            return exif.to_vec();
        };
        if read_u16(&entry[..2]) == 0x0112
            && read_u16(&entry[2..4]) == 3
            && read_u32(&entry[4..8]) == 1
        {
            let mut patched = exif.to_vec();
            let bytes = if big_endian {
                u16::from(orientation).to_be_bytes()
            } else {
                u16::from(orientation).to_le_bytes()
            };
            patched[offset + 8..offset + 10].copy_from_slice(&bytes);
            return patched;
        }
    }
    exif.to_vec()
}

fn decode_facts(path: &Path, input: bool) -> Result<ImageCompressionFacts, ImageCompressionError> {
    let encoded_bytes = fs::metadata(path)?.len();
    if encoded_bytes == 0 || encoded_bytes > MAX_ENCODED_BYTES {
        return Err(invalid_image(
            input,
            format!("encoded size {encoded_bytes} exceeds the 512 MiB safety boundary"),
        ));
    }
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
        let target_format = match destination
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "jpg" | "jpeg" => ImageFileFormat::Jpeg,
            "png" => ImageFileFormat::Png,
            "webp" => ImageFileFormat::WebP,
            _ => ImageFileFormat::Jpeg,
        };
        ImageCompressionRequest {
            source,
            destination,
            mode: ImageCompressionMode::Lossy,
            quality: 80,
            target_format,
            max_dimensions: None,
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
        let mut wrong_request = request(disguised_source, wrong_destination.clone());
        wrong_request.target_format = ImageFileFormat::Jpeg;
        let error = compress_single_image(&wrong_request, &AtomicBool::new(false)).unwrap_err();
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

    #[test]
    fn oriented_jpeg_converts_to_png_with_normalized_orientation_and_metadata() {
        let source = fixture("exif-orientation.jpg");
        if !source.exists() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("converted.png");
        let mut request = request(source.clone(), destination);
        request.mode = ImageCompressionMode::Lossless;
        request.target_format = ImageFileFormat::Png;
        let outcome = compress_single_image(&request, &AtomicBool::new(false)).unwrap();
        let ImageCompressionOutcome::Published(facts) = outcome else {
            panic!("converted output should publish");
        };
        assert_eq!(facts.format, ImageFileFormat::Png);
        assert_eq!(facts.orientation, 1);
        assert_eq!((facts.visible_width, facts.visible_height), (360, 640));
        let source_metadata = read_metadata(&source, true).unwrap();
        let output_metadata = read_metadata(&request.destination, false).unwrap();
        assert_eq!(
            output_metadata.exif,
            source_metadata
                .exif
                .map(|value| set_exif_orientation(&value, 1))
        );
    }

    #[test]
    fn resize_limit_preserves_aspect_ratio_and_strip_keeps_only_orientation() {
        let source = fixture("exif-orientation.jpg");
        if !source.exists() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("resized.jpg");
        let mut request = request(source, destination);
        request.preserve_metadata = false;
        request.max_dimensions = Some(ImageDimensions {
            width: 180,
            height: 320,
        });
        let outcome = compress_single_image(&request, &AtomicBool::new(false)).unwrap();
        let ImageCompressionOutcome::Published(facts) = outcome else {
            panic!("resized output should publish");
        };
        assert_eq!((facts.visible_width, facts.visible_height), (180, 320));
        assert_eq!(facts.orientation, 6);
        let metadata = read_metadata(&request.destination, false).unwrap();
        assert_eq!(metadata.exif, Some(build_orientation_exif(6)));
        assert_eq!(metadata.icc, None);
    }

    #[test]
    fn transparent_png_converts_to_jpeg_with_explicit_alpha_removal() {
        let source = fixture("transparent.png");
        if !source.exists() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("opaque.jpg");
        let mut request = request(source, destination);
        request.target_format = ImageFileFormat::Jpeg;
        let outcome = compress_single_image(&request, &AtomicBool::new(false)).unwrap();
        let ImageCompressionOutcome::Published(facts) = outcome else {
            panic!("JPEG conversion should publish");
        };
        assert_eq!(facts.format, ImageFileFormat::Jpeg);
        assert!(!facts.has_alpha);
    }
}
