#![allow(clippy::unreadable_literal)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::needless_range_loop)]

#[cfg(target_arch = "wasm32")]
extern crate wasm_bindgen;
#[cfg(feature = "aes256")]
mod aes256sha256;

#[cfg(all(test,feature="aes256",feature="additional-codecs"))]
mod ppmd_preview_1628_tests;
#[cfg(all(test,feature="aes256",feature="additional-codecs"))]
mod ppmd_solid_preview_1630_tests;
#[cfg(all(test,feature="aes256",feature="additional-codecs"))]
mod ppmd_prefix_1631_tests;
#[cfg(all(test,feature="aes256",feature="additional-codecs"))]
mod ppmd_solid_prefix_1632_tests;
#[cfg(all(test,feature="additional-codecs"))]
mod deflate_preview_1633_tests;
mod bcj2;
#[cfg(feature = "aes256")]
pub use aes256sha256::*;
#[cfg(target_arch = "wasm32")]
mod wasm;
extern crate filetime_creation as ft;
pub(crate) mod archive;
mod bcj;
#[cfg(not(target_arch = "wasm32"))]
mod de_funcs;
pub(crate) mod decoders;
mod delta;
#[cfg(feature = "compress")]
mod en_funcs;
#[cfg(feature = "compress")]
mod encoders;
mod error;
pub(crate) mod folder;
mod method_options;
pub use method_options::*;
mod password;
mod reader;
mod preview;
pub use preview::with_preview_limits;
#[cfg(feature = "compress")]
mod writer;
pub use archive::*;
#[cfg(not(target_arch = "wasm32"))]
pub use de_funcs::*;
#[cfg(feature = "compress")]
pub use en_funcs::*;
pub use error::Error;
pub use lzma_rust as lzma;
pub use nt_time;
pub use password::Password;
pub use reader::BlockDecoder;
pub use reader::SevenZReader;
#[cfg(feature = "compress")]
pub use writer::*;
