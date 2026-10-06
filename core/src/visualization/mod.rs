// Visualization module for generating professional frame diagrams
//
// This module provides SVG generation for frame diagrams with:
// - Technical-drawing style with palette-coded dimensions (light, dark, PDF styles)
// - Adaptive callout placement to avoid overlap
// - Plan view (front-on) and section view (cross-section)
// - Consistent output for both in-app display and PDF export

// Internal modules. The public API is only the re-exports below (used by the
// WASM/mobile bindings, core/tests, and core/examples); everything else is
// crate-private so dead-code analysis covers it.
mod types;
mod style;
mod geometry;
mod callouts;
mod layout;
mod overlays;
mod collision;
mod svg_util;
mod section_svg;
mod plan_svg;
mod svg;
mod wood;
// Layout snapshot harness (file I/O against core/tests/snapshots) — unit-test only.
#[cfg(test)]
mod snapshot;

// Public API
pub use types::{ViewOption, DetailMode, DiagramOptions, DiagramResult};
pub use style::{DiagramStyle, MaterialPatterns, FillPattern};
pub use svg::{generate_diagram, generate_diagram_with_style};
pub use wood::{
    board_svg, frame_face_svg, wood_appearance, DepthCues, FaceDepths, Figure, FrameFace, FrameFaceSvg, GrainStats,
    LineMode, WoodAppearance, WoodLod, WoodPalette, WoodParams, WoodStructure,
};

/// Shared test utilities for visualization tests.
#[cfg(test)]
pub(crate) mod test_helpers {
    use crate::frame::FrameDesign;

    /// Standard test design: 12×16 artwork, 2" mat, 1" frame.
    pub fn test_design() -> FrameDesign {
        let mut design = FrameDesign::new(12.0, 16.0);
        design.mat_width_top_bottom = 2.0;
        design.mat_width_sides = 2.0;
        design.frame_material_width = 1.0;
        design
    }
}
