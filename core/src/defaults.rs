// Default values for frame design (all measurements in inches)

// Artwork dimensions
pub const DEFAULT_ARTWORK_HEIGHT: f64 = 12.5;
pub const DEFAULT_ARTWORK_WIDTH: f64 = 18.75;

// Mat settings
pub const DEFAULT_INCLUDE_MAT: bool = true;
pub const DEFAULT_MAT_WIDTH: f64 = 2.0;
pub const DEFAULT_MAT_OVERLAP: f64 = 0.125;

// Frame dimensions
pub const DEFAULT_FRAME_MATERIAL_WIDTH: f64 = 0.75;  // Width of frame molding (face)
pub const DEFAULT_FRAME_THICKNESS: f64 = 0.75;        // Depth of frame stock (z-axis)
pub const DEFAULT_RABBET_DEPTH: f64 = 0.375;         // Rabbet extension (x/y plane)

// Material thicknesses (z-axis)
pub const DEFAULT_GLAZING_THICKNESS: f64 = 0.093;    // ~3/32" glass/acrylic
pub const DEFAULT_MATBOARD_THICKNESS: f64 = 0.055;   // ~1/16" 4-ply matboard
pub const DEFAULT_ARTWORK_THICKNESS: f64 = 0.008;    // Photo paper / thin print
pub const DEFAULT_BACKING_THICKNESS: f64 = 0.125;    // 1/8" foam core or hardboard

// Assembly
pub const DEFAULT_ASSEMBLY_MARGIN: f64 = 0.0625;     // 1/16" clearance for assembly

// Cutting/tool settings
pub const DEFAULT_BLADE_WIDTH: f64 = 0.125;          // 1/8" saw blade kerf
