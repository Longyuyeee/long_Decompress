use caesium::parameters::CSParameters;
use oxipng::{InFile, Options, OutFile};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

#[cfg(windows)]
fn peak_working_set_bytes() -> usize {
    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut core::ffi::c_void;
        fn K32GetProcessMemoryInfo(
            process: *mut core::ffi::c_void,
            counters: *mut ProcessMemoryCounters,
            size: u32,
        ) -> i32;
    }
    let mut counters = ProcessMemoryCounters {
        cb: std::mem::size_of::<ProcessMemoryCounters>() as u32,
        page_fault_count: 0,
        peak_working_set_size: 0,
        working_set_size: 0,
        quota_peak_paged_pool_usage: 0,
        quota_paged_pool_usage: 0,
        quota_peak_non_paged_pool_usage: 0,
        quota_non_paged_pool_usage: 0,
        pagefile_usage: 0,
        peak_pagefile_usage: 0,
    };
    let ok = unsafe {
        K32GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            std::mem::size_of::<ProcessMemoryCounters>() as u32,
        )
    };
    if ok == 0 {
        0
    } else {
        counters.peak_working_set_size
    }
}

#[cfg(not(windows))]
fn peak_working_set_bytes() -> usize {
    0
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn emit(
    kind: &str,
    input: &Path,
    output: Option<&Path>,
    started: Instant,
    result: Result<(), String>,
) -> bool {
    let input_bytes = fs::metadata(input).map(|facts| facts.len()).unwrap_or(0);
    let output_bytes = output
        .and_then(|path| fs::metadata(path).ok())
        .map(|facts| facts.len())
        .unwrap_or(0);
    match result {
        Ok(()) => {
            println!(
                "RESULT|{kind}|ok|{input_bytes}|{output_bytes}|{}",
                started.elapsed().as_micros()
            );
            true
        }
        Err(error) => {
            println!(
                "RESULT|{kind}|rejected|{input_bytes}|0|{}|{}",
                started.elapsed().as_micros(),
                error.replace('|', "/")
            );
            false
        }
    }
}

fn main() -> ExitCode {
    let arguments = env::args_os().collect::<Vec<_>>();
    if arguments.len() != 3 {
        eprintln!("usage: long-image-baseline <input-directory> <output-directory>");
        return ExitCode::from(2);
    }
    let input = PathBuf::from(&arguments[1]);
    let output = PathBuf::from(&arguments[2]);
    if let Err(error) = fs::create_dir_all(&output) {
        eprintln!("cannot create output directory: {error}");
        return ExitCode::FAILURE;
    }

    let jpeg_input = input.join("exif-orientation.jpg");
    let jpeg_output = output.join("exif-orientation.optimized.jpg");
    let mut jpeg_parameters = CSParameters::new();
    jpeg_parameters.keep_metadata = true;
    jpeg_parameters.keep_rotation = true;
    jpeg_parameters.jpeg.quality = 80;
    let jpeg_started = Instant::now();
    let jpeg_ok = emit(
        "jpeg",
        &jpeg_input,
        Some(&jpeg_output),
        jpeg_started,
        caesium::compress(
            path_string(&jpeg_input),
            path_string(&jpeg_output),
            &jpeg_parameters,
        )
        .map_err(|error| error.to_string()),
    );

    let webp_input = input.join("photo.webp");
    let webp_output = output.join("photo.optimized.webp");
    let mut webp_parameters = CSParameters::new();
    webp_parameters.keep_metadata = true;
    webp_parameters.webp.quality = 80;
    let webp_started = Instant::now();
    let webp_ok = emit(
        "webp",
        &webp_input,
        Some(&webp_output),
        webp_started,
        caesium::compress(
            path_string(&webp_input),
            path_string(&webp_output),
            &webp_parameters,
        )
        .map_err(|error| error.to_string()),
    );

    let png_input = input.join("transparent.png");
    let png_output = output.join("transparent.optimized.png");
    let png_started = Instant::now();
    let mut png_options = Options::from_preset(3);
    png_options.optimize_alpha = true;
    let png_ok = emit(
        "png-lossless",
        &png_input,
        Some(&png_output),
        png_started,
        oxipng::optimize(
            &InFile::Path(png_input.clone()),
            &OutFile::Path {
                path: Some(png_output.clone()),
                preserve_attrs: false,
            },
            &png_options,
        )
        .map(|_| ())
        .map_err(|error| error.to_string()),
    );

    let gif_input = input.join("animated.gif");
    let gif_output = output.join("animated.unsupported.gif");
    let gif_started = Instant::now();
    let gif_rejected = !emit(
        "gif-boundary",
        &gif_input,
        Some(&gif_output),
        gif_started,
        caesium::compress(
            path_string(&gif_input),
            path_string(&gif_output),
            &CSParameters::new(),
        )
        .map_err(|error| error.to_string()),
    );
    if gif_output.exists() {
        let _ = fs::remove_file(&gif_output);
    }

    println!("PROCESS|peakWorkingSetBytes|{}", peak_working_set_bytes());

    if jpeg_ok && webp_ok && png_ok && gif_rejected {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
