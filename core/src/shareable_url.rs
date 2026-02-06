// Shareable URL generation for frame designs
//
// Ported from Python shareable_url.py with identical behavior

use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

/// Parameters for shareable URL encoding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareableParams {
    pub artwork_height: f64,
    pub artwork_width: f64,
    pub mat_width: f64,
    pub frame_width: f64,
    pub frame_depth: f64,
    pub glazing_thickness: f64,
    pub matboard_thickness: f64,
    pub artwork_thickness: f64,
    pub backing_thickness: f64,
    pub rabbet_width: f64,  // Horizontal lip overlap
    pub rabbet_depth: f64,  // Z-axis cutout depth
    pub blade_width: f64,
    pub include_mat: bool,
    pub unit_mm: bool,
}

/// Error type for URL decoding failures
#[derive(Debug)]
pub enum DecodeError {
    InvalidUrl,
    InvalidBase64,
    TruncatedData,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::InvalidUrl => write!(f, "Invalid URL format"),
            DecodeError::InvalidBase64 => write!(f, "Invalid base64 encoding"),
            DecodeError::TruncatedData => write!(f, "Truncated data (expected 28 or 30 bytes)"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Pack a value as big-endian uint24 (3 bytes)
fn pack_uint24(val: f64) -> [u8; 3] {
    let v = (val * 10000.0) as u32;
    [
        ((v >> 16) & 0xFF) as u8,
        ((v >> 8) & 0xFF) as u8,
        (v & 0xFF) as u8,
    ]
}

/// Unpack a big-endian uint24 (3 bytes) to f64
fn unpack_uint24(bytes: &[u8]) -> f64 {
    let v = ((bytes[0] as u32) << 16) | ((bytes[1] as u32) << 8) | (bytes[2] as u32);
    v as f64 / 10000.0
}

/// Pack a value as big-endian uint16 (2 bytes)
fn pack_uint16(val: f64) -> [u8; 2] {
    let v = (val * 10000.0) as u16;
    [
        ((v >> 8) & 0xFF) as u8,
        (v & 0xFF) as u8,
    ]
}

/// Unpack a big-endian uint16 (2 bytes) to f64
fn unpack_uint16(bytes: &[u8]) -> f64 {
    let v = ((bytes[0] as u16) << 8) | (bytes[1] as u16);
    v as f64 / 10000.0
}

/// Generate a compact shareable URL encoding all frame settings
///
/// Binary format (30 bytes → ~40 chars base64):
///     5 × uint24: h, w, mw, fw, fd (×10000 for 4 decimal precision)
///     7 × uint16: gt, mt, at, bt, rw, rd, bw (×10000 for 4 decimal precision)
///     1 × byte: flags (bit 0 = mat, bit 1 = unit_mm)
///
/// All values stored in inches internally.
pub fn generate_shareable_url(params: &ShareableParams) -> String {
    let mut packed = Vec::with_capacity(30);

    // uint24 fields: h, w, mw, fw, fd (15 bytes total)
    packed.extend_from_slice(&pack_uint24(params.artwork_height));
    packed.extend_from_slice(&pack_uint24(params.artwork_width));
    packed.extend_from_slice(&pack_uint24(params.mat_width));
    packed.extend_from_slice(&pack_uint24(params.frame_width));
    packed.extend_from_slice(&pack_uint24(params.frame_depth));

    // uint16 fields: gt, mt, at, bt, rw, rd, bw (14 bytes total)
    packed.extend_from_slice(&pack_uint16(params.glazing_thickness));
    packed.extend_from_slice(&pack_uint16(params.matboard_thickness));
    packed.extend_from_slice(&pack_uint16(params.artwork_thickness));
    packed.extend_from_slice(&pack_uint16(params.backing_thickness));
    packed.extend_from_slice(&pack_uint16(params.rabbet_width));
    packed.extend_from_slice(&pack_uint16(params.rabbet_depth));
    packed.extend_from_slice(&pack_uint16(params.blade_width));

    // Flags byte (1 byte total)
    let flags = (params.include_mat as u8) | ((params.unit_mm as u8) << 1);
    packed.push(flags);

    // Base64 encode (URL-safe, no padding)
    // Return just the encoded string, not a full URL
    // The caller can construct the full URL based on their deployment location
    URL_SAFE_NO_PAD.encode(&packed)
}

/// Decode a shareable URL back to parameters
pub fn decode_shareable_url(url: &str) -> Result<ShareableParams, DecodeError> {
    // Extract base64 parameter
    let b64 = url
        .split("?d=")
        .nth(1)
        .ok_or(DecodeError::InvalidUrl)?;

    // Decode base64
    let bytes = URL_SAFE_NO_PAD
        .decode(b64)
        .map_err(|_| DecodeError::InvalidBase64)?;

    // Verify length (30 bytes for new format, 28 for legacy without rabbet_width)
    if bytes.len() != 30 && bytes.len() != 28 {
        return Err(DecodeError::TruncatedData);
    }

    // Unpack uint24 fields (15 bytes)
    let artwork_height = unpack_uint24(&bytes[0..3]);
    let artwork_width = unpack_uint24(&bytes[3..6]);
    let mat_width = unpack_uint24(&bytes[6..9]);
    let frame_width = unpack_uint24(&bytes[9..12]);
    let frame_depth = unpack_uint24(&bytes[12..15]);

    // Unpack uint16 fields (14 bytes for new format, 12 for legacy)
    let glazing_thickness = unpack_uint16(&bytes[15..17]);
    let matboard_thickness = unpack_uint16(&bytes[17..19]);
    let artwork_thickness = unpack_uint16(&bytes[19..21]);
    let backing_thickness = unpack_uint16(&bytes[21..23]);

    // Handle both legacy (28 bytes) and new (30 bytes) formats
    let (rabbet_width, rabbet_depth, blade_width, flags) = if bytes.len() == 30 {
        // New format: includes rabbet_width
        let rw = unpack_uint16(&bytes[23..25]);
        let rd = unpack_uint16(&bytes[25..27]);
        let bw = unpack_uint16(&bytes[27..29]);
        let f = bytes[29];
        (rw, rd, bw, f)
    } else {
        // Legacy format: rabbet_width defaults to rabbet_depth (square rabbet)
        let rd = unpack_uint16(&bytes[23..25]);
        let bw = unpack_uint16(&bytes[25..27]);
        let f = bytes[27];
        (rd, rd, bw, f)  // rabbet_width = rabbet_depth for backwards compatibility
    };

    let include_mat = (flags & 0x01) != 0;
    let unit_mm = (flags & 0x02) != 0;

    Ok(ShareableParams {
        artwork_height,
        artwork_width,
        mat_width,
        frame_width,
        frame_depth,
        glazing_thickness,
        matboard_thickness,
        artwork_thickness,
        backing_thickness,
        rabbet_width,
        rabbet_depth,
        blade_width,
        include_mat,
        unit_mm,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_unpack_uint24() {
        let val = 12.5;
        let packed = pack_uint24(val);
        let unpacked = unpack_uint24(&packed);
        assert!((unpacked - val).abs() < 0.0001);
    }

    #[test]
    fn test_pack_unpack_uint16() {
        let val = 0.375;
        let packed = pack_uint16(val);
        let unpacked = unpack_uint16(&packed);
        assert!((unpacked - val).abs() < 0.0001);
    }

    #[test]
    fn test_basic_url_generation() {
        let params = ShareableParams {
            artwork_height: 12.5,
            artwork_width: 18.75,
            mat_width: 2.0,
            frame_width: 0.75,
            frame_depth: 0.75,
            glazing_thickness: 0.093,
            matboard_thickness: 0.055,
            artwork_thickness: 0.008,
            backing_thickness: 0.125,
            rabbet_width: 0.375,
            rabbet_depth: 0.375,
            blade_width: 0.125,
            include_mat: true,
            unit_mm: false,
        };

        let encoded = generate_shareable_url(&params);
        // generate_shareable_url returns raw base64, not a full URL
        assert!(!encoded.is_empty());
        // Verify roundtrip by constructing a URL for decode
        let url = format!("https://example.com/?d={}", encoded);
        let decoded = decode_shareable_url(&url).unwrap();
        assert!((decoded.artwork_height - 12.5).abs() < 0.0001);
        assert!((decoded.artwork_width - 18.75).abs() < 0.0001);
    }

    #[test]
    fn test_non_square_rabbet_roundtrip() {
        let params = ShareableParams {
            artwork_height: 12.5,
            artwork_width: 18.75,
            mat_width: 2.0,
            frame_width: 0.75,
            frame_depth: 0.75,
            glazing_thickness: 0.093,
            matboard_thickness: 0.055,
            artwork_thickness: 0.008,
            backing_thickness: 0.125,
            rabbet_width: 0.25,   // Different from depth
            rabbet_depth: 0.375,
            blade_width: 0.125,
            include_mat: true,
            unit_mm: false,
        };

        let encoded = generate_shareable_url(&params);
        let url = format!("https://example.com/?d={}", encoded);
        let decoded = decode_shareable_url(&url).unwrap();

        assert!((decoded.rabbet_width - 0.25).abs() < 0.0001);
        assert!((decoded.rabbet_depth - 0.375).abs() < 0.0001);
    }
}
