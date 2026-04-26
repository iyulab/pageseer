//! pageseer — document-to-page-image rasterizer.
//!
//! See the design spec at `claudedocs/specs/` for architecture.

#![warn(missing_docs)]

pub mod error;
pub mod report;

pub use error::PageseerError;
