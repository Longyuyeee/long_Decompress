pub mod zip_edit;
pub mod archive_engine;
pub mod archive_browser;
pub(crate) mod standard_zip_reader;
pub mod standard_zip_writer;
pub(crate) mod zip_recovery_reader;
pub mod zip_recovery_outcome;
#[cfg(test)]
mod zip_partial_rescue_tests;
pub mod archive_diagnostics;
pub mod rar_recovery;
pub mod archive_preview;
pub mod archive_entry_open;
pub mod universal_engine;
pub mod compression_service;
pub mod compression_analysis;
pub(crate) mod compression_verification;
pub mod archive_format;
pub mod zpaq_engine;
pub mod zpaq_multipart;
pub mod zpaq_update;
pub mod zpaq_diff;
pub mod alz_engine;
pub mod egg_engine;
pub mod egg_volumes;
pub mod alz_volumes;
pub mod compression_format;
pub(crate) mod compression_entries;
pub mod extraction_transaction;
pub(crate) mod mark_of_web;
pub(crate) mod native_compression;
pub(crate) mod native_extraction;
pub(crate) mod tar_compatibility;
pub(crate) mod progress_gate;
pub(crate) mod output_publish_transaction;
pub mod compression_profile_service;
pub mod decompression_profile_service;
pub mod encrypted_password_service;
pub mod file_service;
pub mod file_manager;
pub mod file_integrity_service;
pub mod io_buffer_pool;
pub mod io_buffer_pool_benchmark;
pub mod image_compression_service;
pub mod parallel_extraction;
pub mod password_service;
pub mod password_book_service;
pub mod password_category_service;
pub mod password_strength_service;
pub mod password_query_service;
pub mod password_attempt_service;
pub mod password_dictionary_service;
pub mod password_generator_service;
#[cfg(test)]
mod performance_regression;
#[cfg(test)]
mod native_zip_baseline;
pub mod split_compression;
pub(crate) mod numeric_volume_reader;
pub mod zip_password;
pub mod zip_method;
pub(crate) mod zip_selection;
pub(crate) mod sfx_archive;
pub(crate) mod sfx_volumes;
pub mod split_archive_detector;
pub(crate) mod archive_volumes;
pub(crate) mod codec_stream;
pub mod rar_support;
pub(crate) mod rar_preflight;
pub mod tar_aes_engine;
pub mod aes_wrapper;
pub mod aes_stream_v2;
pub mod system_service;
pub mod storage_preflight;
pub(crate) mod source_recycle;
pub mod task_template;
pub(crate) mod task_control;
pub mod watch_folder_service;
pub mod video_engine;
pub mod video_compression_plan;
pub mod video_encoding;
pub mod video_output_validation;
pub mod video_publish;
pub mod video_probe;
pub mod pdf_engine;
pub mod pdf_analysis;
pub mod pdf_transform;
pub mod pdf_output_validation;
pub mod pdf_publish;
#[cfg(test)]
mod standard_zipx_tests;
pub(crate) mod archive_io;

#[cfg(all(test, windows))]
mod archive_io_disk_tests;

#[cfg(test)]
mod multistream_tests;
#[cfg(test)]
mod aes_volume_tests;
pub(crate) mod aes_container;
#[cfg(test)]
mod checksum_details_tests;
pub(crate) mod checksum_manifest;

#[cfg(test)]
mod aes_codecs_1521_tests;

pub mod checksum_progress;
pub(crate) mod checksum_export;
#[cfg(test)]
mod checksum_export_1525_tests;

#[cfg(test)]
mod checksum_progress_tests;

#[cfg(test)]
mod lzma_1523_tests;

#[cfg(test)]
mod tar_lzma_1524_tests;
#[cfg(test)]
mod zpaq_1526_tests;

pub(crate) mod wim_preview;
pub(crate) mod swm_volumes;
#[cfg(test)]
mod swm_1529_tests;
#[cfg(test)]
mod zpaq_1530_tests;

#[cfg(test)]
mod wim_1527_tests;

pub mod wim_writer;

pub mod zip_edit_drafts;

pub mod archive_library;

pub mod library_organization;

pub mod workflows;

pub mod workflow_recovery;

pub mod workflow_triggers;

pub mod workflow_schedules;

pub mod source_comparison;

pub mod source_comparison_content;

pub mod comparison_package;

#[cfg(test)] mod comparison_package_tests;

pub mod image_reader;

#[cfg(test)]
mod zip_bzip2_compatibility_tests;

#[cfg(test)]
mod zip_lzma_compatibility_tests;

#[cfg(test)]
mod zip_zstd_compatibility_tests;

#[cfg(test)]
mod zip_ppmd_compatibility_tests;

#[cfg(test)]
mod zip_xz_compatibility_tests;

#[cfg(test)] mod zpaq_1564_tests;
