// Visualization module for generating professional frame diagrams
//
// This module provides SVG generation for frame diagrams with:
// - Warm, woodworking-plan aesthetic
// - Adaptive callout placement to avoid overlap
// - Plan view (front-on) and section view (cross-section)
// - Consistent output for both in-app display and PDF export

pub mod types;
pub mod style;
pub mod geometry;
pub mod callouts;
pub mod layout;
pub mod collision;
pub mod svg;
pub mod snapshot;

// Re-export commonly used types
pub use types::{
    Point, Rect, Side, DimensionType, DimensionCallout,
    PositionedCallout, ViewType, ViewOption, DetailMode, DiagramOptions, DiagramResult,
    TextAnchor, AnnotationBounds, ThumbnailLabelPosition,
};
pub use style::{DiagramStyle, MaterialPatterns, FillPattern, ThumbnailMetrics};
pub use geometry::{PlanViewGeometry, SectionViewGeometry, estimate_text_width, effective_label_width};
pub use callouts::{generate_plan_callouts, generate_section_callouts};
pub use layout::{layout_plan_callouts, LayoutResult};
pub use svg::{generate_diagram, generate_diagram_with_style};
