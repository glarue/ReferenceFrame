//! ReferenceFrame Core Library
//!
//! Pure Rust business logic for picture frame design calculations.
//! This crate is platform-agnostic and can be used from:
//! - WebAssembly (via wasm_bindings wrapper)
//! - iOS/Android (via Flutter FFI bridge)
//! - Command-line tools
//! - Server-side applications
//!
//! All calculations are done in inches internally.

pub mod presets;
pub mod version;
pub mod conversions;
pub mod frame;
pub mod aspect_ratio;
pub mod shareable_url;
pub mod visualization;
pub mod validation;
pub mod constraints;
pub mod input_parser;
pub mod history;
pub mod joinery;
pub mod hanging;
pub mod weight;

// Re-export key types for convenience
pub use frame::{FrameDesign, FrameStyle};
pub use conversions::{Unit, format_value, inches_to_mm, mm_to_inches};
pub use aspect_ratio::{
    AspectLockState, get_aspect_ratio_display, get_aspect_ratio_display_from_ratio, invert_ratio
};
pub use shareable_url::{ShareableParams, generate_shareable_url, decode_shareable_url, DecodeError};
pub use validation::{ValidationConfig, ValidationResult, TypicalRanges, validate_design};
pub use constraints::{apply_input_constraints, ConstraintAdjustment, ConstraintOutcome};
pub use input_parser::{DimensionInput, decimal_to_fraction};
pub use history::{HistoryEntry, DesignHistory, DEFAULT_MAX_ENTRIES};
