# Default values for frame design (all measurements in inches)

# Artwork dimensions
DEFAULT_ARTWORK_HEIGHT = 12.5
DEFAULT_ARTWORK_WIDTH = 18.75

# Mat settings
DEFAULT_INCLUDE_MAT = True
DEFAULT_MAT_WIDTH = 2.0
DEFAULT_MAT_OVERLAP = 0.125

# Frame dimensions
DEFAULT_FRAME_MATERIAL_WIDTH = 0.75  # Width of frame molding (face)
DEFAULT_FRAME_THICKNESS = 0.75       # Depth of frame stock (z-axis)
DEFAULT_RABBET_DEPTH = 0.375        # Rabbet extension (x/y plane)

# Material thicknesses (z-axis)
DEFAULT_GLAZING_THICKNESS = 0.093   # ~3/32" glass/acrylic
DEFAULT_MATBOARD_THICKNESS = 0.055  # ~1/16" 4-ply matboard
DEFAULT_ARTWORK_THICKNESS = 0.008   # Photo paper / thin print
DEFAULT_BACKING_THICKNESS = 0.125   # 1/8" foam core or hardboard

# Cutting/tool settings
DEFAULT_BLADE_WIDTH = 0.125         # 1/8" saw blade kerf

# Standard artwork size keys for consistent referencing
STANDARD_ARTWORK_SIZES = {
    "8x10": {"height": 8.0, "width": 10.0, "name": "8×10"},
    "10x15": {"height": 10.0, "width": 15.0, "name": "10×15"},
    "11x14": {"height": 11.0, "width": 14.0, "name": "11×14"},
    "16x20": {"height": 16.0, "width": 20.0, "name": "16×20"},
}

# Default starting artwork size key (can be referenced throughout the app)
DEFAULT_ARTWORK_SIZE_KEY = "10x15"
