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
    /// Format version (top 3 bits of flags byte) is newer than this decoder understands
    UnsupportedVersion(u8),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::InvalidUrl => write!(f, "Invalid URL format"),
            DecodeError::InvalidBase64 => write!(f, "Invalid base64 encoding"),
            DecodeError::TruncatedData => write!(f, "Truncated data (expected 28 or 30 bytes)"),
            DecodeError::UnsupportedVersion(v) => {
                write!(f, "Unsupported format version {} (decoder supports version {})", v, FORMAT_VERSION)
            }
        }
    }
}

impl std::error::Error for DecodeError {}

// ============================================================================
// Format versioning
// ============================================================================
//
// The flags byte (last byte of the payload) is laid out as:
//
//     bit 7..5: format version (currently 0)
//     bit 4..2: reserved (must be 0)
//     bit 1:    unit_mm
//     bit 0:    include_mat
//
// Version 0 covers both the 30-byte format and the legacy 28-byte format
// (distinguished by payload length). All URLs generated before versioning
// was introduced already have zero in the top bits, so they decode as
// version 0 with no wire change. A future incompatible format must bump
// the version so old payloads can't be misparsed; this decoder rejects
// any version it doesn't recognize.

/// Current binary format version (stored in the top 3 bits of the flags byte)
const FORMAT_VERSION: u8 = 0;

/// Bit position of the version within the flags byte
const VERSION_SHIFT: u8 = 5;

// Field ranges: values are stored as fixed-point ×10000.
//   uint24 fields max: 0xFFFFFF / 10000 = 1677.7215"
//   uint16 fields max: 0xFFFF / 10000 = 6.5535"
// Out-of-range values are clamped on encode (never silently wrapped).
const MAX_UINT24: f64 = 0xFF_FFFF as f64;
const MAX_UINT16: f64 = 0xFFFF as f64;

/// Pack a value as big-endian uint24 (3 bytes), clamped to [0, 1677.7215]
fn pack_uint24(val: f64) -> [u8; 3] {
    let v = (val * 10000.0).clamp(0.0, MAX_UINT24) as u32;
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

/// Pack a value as big-endian uint16 (2 bytes), clamped to [0, 6.5535]
fn pack_uint16(val: f64) -> [u8; 2] {
    let v = (val * 10000.0).clamp(0.0, MAX_UINT16) as u16;
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
/// Binary format version 0 (30 bytes → ~40 chars base64):
///     5 × uint24: h, w, mw, fw, fd (×10000 for 4 decimal precision, max 1677.7215")
///     7 × uint16: gt, mt, at, bt, rw, rd, bw (×10000 for 4 decimal precision, max 6.5535")
///     1 × byte: flags (bit 0 = mat, bit 1 = unit_mm, bits 5-7 = format version)
///
/// All values stored in inches internally. Out-of-range values are clamped
/// to each field's representable range rather than wrapping.
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

    // Flags byte (1 byte total): flag bits plus version in the top 3 bits.
    // Version 0 leaves the top bits zero, so the wire format is unchanged.
    let flags = (params.include_mat as u8)
        | ((params.unit_mm as u8) << 1)
        | (FORMAT_VERSION << VERSION_SHIFT);
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

    // Reject payloads from a future format version rather than misparsing them.
    // (Pre-versioning URLs have zero in the top bits, so they pass as version 0.)
    let version = flags >> VERSION_SHIFT;
    if version != FORMAT_VERSION {
        return Err(DecodeError::UnsupportedVersion(version));
    }

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

    // --- Legacy 28-byte format ---

    #[test]
    fn test_legacy_28_byte_decode() {
        // Manually construct a 28-byte legacy payload:
        //   5 × uint24: h=10.0, w=8.0, mw=2.0, fw=1.5, fd=0.75
        //   5 × uint16: gt=0.093, mt=0.055, at=0.008, bt=0.125, rd=0.375
        //   1 × uint16: bw=0.125
        //   1 × byte:   flags=0x01 (include_mat=true, unit_mm=false)
        // (no separate rabbet_width field — legacy format)
        let mut payload: Vec<u8> = Vec::with_capacity(28);
        payload.extend_from_slice(&pack_uint24(10.0));
        payload.extend_from_slice(&pack_uint24(8.0));
        payload.extend_from_slice(&pack_uint24(2.0));
        payload.extend_from_slice(&pack_uint24(1.5));
        payload.extend_from_slice(&pack_uint24(0.75));
        payload.extend_from_slice(&pack_uint16(0.093));
        payload.extend_from_slice(&pack_uint16(0.055));
        payload.extend_from_slice(&pack_uint16(0.008));
        payload.extend_from_slice(&pack_uint16(0.125));
        // Legacy: rd then bw then flags (no rw field)
        payload.extend_from_slice(&pack_uint16(0.375));
        payload.extend_from_slice(&pack_uint16(0.125));
        payload.push(0x01);
        assert_eq!(payload.len(), 28);

        let b64 = URL_SAFE_NO_PAD.encode(&payload);
        let url = format!("https://example.com/?d={}", b64);
        let decoded = decode_shareable_url(&url).unwrap();

        assert!((decoded.artwork_height - 10.0).abs() < 0.0001);
        assert!((decoded.artwork_width - 8.0).abs() < 0.0001);
        // Legacy: rabbet_width should equal rabbet_depth
        assert!((decoded.rabbet_width - 0.375).abs() < 0.0001);
        assert!((decoded.rabbet_depth - 0.375).abs() < 0.0001);
        assert!((decoded.blade_width - 0.125).abs() < 0.0001);
        assert!(decoded.include_mat);
        assert!(!decoded.unit_mm);
    }

    // --- Invalid base64 / URL inputs ---

    #[test]
    fn test_decode_empty_string() {
        let result = decode_shareable_url("");
        assert!(matches!(result, Err(DecodeError::InvalidUrl)));
    }

    #[test]
    fn test_decode_missing_query_param() {
        let result = decode_shareable_url("https://example.com/");
        assert!(matches!(result, Err(DecodeError::InvalidUrl)));
    }

    #[test]
    fn test_decode_garbage_base64() {
        let result = decode_shareable_url("https://example.com/?d=not-base64!!!");
        assert!(matches!(result, Err(DecodeError::InvalidBase64)));
    }

    #[test]
    fn test_decode_valid_base64_too_short() {
        // 4 bytes — valid base64, wrong length
        let b64 = URL_SAFE_NO_PAD.encode(&[0u8; 4]);
        let url = format!("https://example.com/?d={}", b64);
        let result = decode_shareable_url(&url);
        assert!(matches!(result, Err(DecodeError::TruncatedData)));
    }

    #[test]
    fn test_decode_valid_base64_too_long() {
        let b64 = URL_SAFE_NO_PAD.encode(&[0u8; 40]);
        let url = format!("https://example.com/?d={}", b64);
        let result = decode_shareable_url(&url);
        assert!(matches!(result, Err(DecodeError::TruncatedData)));
    }

    #[test]
    fn test_decode_valid_base64_29_bytes() {
        // 29 bytes — between the two valid sizes
        let b64 = URL_SAFE_NO_PAD.encode(&[0u8; 29]);
        let url = format!("https://example.com/?d={}", b64);
        let result = decode_shareable_url(&url);
        assert!(matches!(result, Err(DecodeError::TruncatedData)));
    }

    // --- Format versioning ---

    #[test]
    fn test_encoder_writes_version_zero() {
        let params = ShareableParams {
            artwork_height: 8.0,
            artwork_width: 12.0,
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
            unit_mm: true,
        };
        let encoded = generate_shareable_url(&params);
        let bytes = URL_SAFE_NO_PAD.decode(&encoded).unwrap();
        let flags = bytes[29];
        // Top 3 bits (version) must be zero; flag bits intact
        assert_eq!(flags >> VERSION_SHIFT, FORMAT_VERSION);
        assert_eq!(flags & 0x03, 0x03);
    }

    #[test]
    fn test_decode_unknown_version_rejected() {
        // Valid 30-byte payload, but with a future version in the flags byte
        let mut payload = vec![0u8; 30];
        payload[29] = 0x01 | (1 << VERSION_SHIFT); // version 1, include_mat set
        let b64 = URL_SAFE_NO_PAD.encode(&payload);
        let url = format!("https://example.com/?d={}", b64);
        let result = decode_shareable_url(&url);
        assert!(matches!(result, Err(DecodeError::UnsupportedVersion(1))));
    }

    #[test]
    fn test_decode_legacy_unknown_version_rejected() {
        // The version check also applies to the legacy 28-byte format
        let mut payload = vec![0u8; 28];
        payload[27] = 7 << VERSION_SHIFT; // version 7
        let b64 = URL_SAFE_NO_PAD.encode(&payload);
        let url = format!("https://example.com/?d={}", b64);
        let result = decode_shareable_url(&url);
        assert!(matches!(result, Err(DecodeError::UnsupportedVersion(7))));
    }

    // --- Encode clamping (out-of-range values must not wrap) ---

    #[test]
    fn test_encode_clamps_oversized_uint16_fields() {
        // 7.0" exceeds the uint16 max of 6.5535" — must clamp, not wrap
        let params = ShareableParams {
            artwork_height: 10.0,
            artwork_width: 10.0,
            mat_width: 2.0,
            frame_width: 0.75,
            frame_depth: 0.75,
            glazing_thickness: 7.0,
            matboard_thickness: 0.055,
            artwork_thickness: 0.008,
            backing_thickness: 0.125,
            rabbet_width: 100.0,
            rabbet_depth: 0.375,
            blade_width: 0.125,
            include_mat: true,
            unit_mm: false,
        };
        let encoded = generate_shareable_url(&params);
        let url = format!("https://example.com/?d={}", encoded);
        let decoded = decode_shareable_url(&url).unwrap();
        assert!((decoded.glazing_thickness - 6.5535).abs() < 0.0001);
        assert!((decoded.rabbet_width - 6.5535).abs() < 0.0001);
        // In-range neighbors are untouched
        assert!((decoded.rabbet_depth - 0.375).abs() < 0.0001);
    }

    #[test]
    fn test_encode_clamps_oversized_uint24_fields() {
        // 2000" exceeds the uint24 max of 1677.7215" — must clamp, not wrap
        let params = ShareableParams {
            artwork_height: 2000.0,
            artwork_width: 12.0,
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
        let url = format!("https://example.com/?d={}", encoded);
        let decoded = decode_shareable_url(&url).unwrap();
        assert!((decoded.artwork_height - 1677.7215).abs() < 0.0001);
        assert!((decoded.artwork_width - 12.0).abs() < 0.0001);
    }

    #[test]
    fn test_encode_clamps_negative_values_to_zero() {
        let params = ShareableParams {
            artwork_height: -5.0,
            artwork_width: 12.0,
            mat_width: 2.0,
            frame_width: 0.75,
            frame_depth: 0.75,
            glazing_thickness: -0.1,
            matboard_thickness: 0.055,
            artwork_thickness: 0.008,
            backing_thickness: 0.125,
            rabbet_width: 0.375,
            rabbet_depth: 0.375,
            blade_width: 0.125,
            include_mat: false,
            unit_mm: false,
        };
        let encoded = generate_shareable_url(&params);
        let url = format!("https://example.com/?d={}", encoded);
        let decoded = decode_shareable_url(&url).unwrap();
        assert_eq!(decoded.artwork_height, 0.0);
        assert_eq!(decoded.glazing_thickness, 0.0);
    }

    // --- Precision round-trip at boundaries ---

    #[test]
    fn test_roundtrip_very_small_value() {
        let params = ShareableParams {
            artwork_height: 0.0001,
            artwork_width: 0.0001,
            mat_width: 0.0,
            frame_width: 0.0001,
            frame_depth: 0.0001,
            glazing_thickness: 0.0001,
            matboard_thickness: 0.0001,
            artwork_thickness: 0.0001,
            backing_thickness: 0.0001,
            rabbet_width: 0.0001,
            rabbet_depth: 0.0001,
            blade_width: 0.0001,
            include_mat: false,
            unit_mm: false,
        };
        let encoded = generate_shareable_url(&params);
        let url = format!("https://example.com/?d={}", encoded);
        let decoded = decode_shareable_url(&url).unwrap();
        assert!((decoded.artwork_height - 0.0001).abs() < 0.00015);
        assert!((decoded.glazing_thickness - 0.0001).abs() < 0.00015);
    }

    #[test]
    fn test_roundtrip_zero_values() {
        let params = ShareableParams {
            artwork_height: 0.0,
            artwork_width: 0.0,
            mat_width: 0.0,
            frame_width: 0.0,
            frame_depth: 0.0,
            glazing_thickness: 0.0,
            matboard_thickness: 0.0,
            artwork_thickness: 0.0,
            backing_thickness: 0.0,
            rabbet_width: 0.0,
            rabbet_depth: 0.0,
            blade_width: 0.0,
            include_mat: false,
            unit_mm: false,
        };
        let encoded = generate_shareable_url(&params);
        let url = format!("https://example.com/?d={}", encoded);
        let decoded = decode_shareable_url(&url).unwrap();
        assert_eq!(decoded.artwork_height, 0.0);
        assert_eq!(decoded.mat_width, 0.0);
        assert_eq!(decoded.rabbet_width, 0.0);
        assert!(!decoded.include_mat);
        assert!(!decoded.unit_mm);
    }

    #[test]
    fn test_roundtrip_max_uint24() {
        // uint24 max: 0xFFFFFF = 16777215, /10000 = 1677.7215
        let max_u24 = 16_777_215.0 / 10000.0; // 1677.7215
        let params = ShareableParams {
            artwork_height: max_u24,
            artwork_width: max_u24,
            mat_width: max_u24,
            frame_width: max_u24,
            frame_depth: max_u24,
            glazing_thickness: 0.0,
            matboard_thickness: 0.0,
            artwork_thickness: 0.0,
            backing_thickness: 0.0,
            rabbet_width: 0.0,
            rabbet_depth: 0.0,
            blade_width: 0.0,
            include_mat: true,
            unit_mm: true,
        };
        let encoded = generate_shareable_url(&params);
        let url = format!("https://example.com/?d={}", encoded);
        let decoded = decode_shareable_url(&url).unwrap();
        assert!((decoded.artwork_height - max_u24).abs() < 0.0001);
        assert!((decoded.artwork_width - max_u24).abs() < 0.0001);
    }

    #[test]
    fn test_roundtrip_max_uint16() {
        // uint16 max: 0xFFFF = 65535, /10000 = 6.5535
        let max_u16 = 65_535.0 / 10000.0; // 6.5535
        let params = ShareableParams {
            artwork_height: 10.0,
            artwork_width: 10.0,
            mat_width: 0.0,
            frame_width: 1.0,
            frame_depth: 1.0,
            glazing_thickness: max_u16,
            matboard_thickness: max_u16,
            artwork_thickness: max_u16,
            backing_thickness: max_u16,
            rabbet_width: max_u16,
            rabbet_depth: max_u16,
            blade_width: max_u16,
            include_mat: false,
            unit_mm: false,
        };
        let encoded = generate_shareable_url(&params);
        let url = format!("https://example.com/?d={}", encoded);
        let decoded = decode_shareable_url(&url).unwrap();
        assert!((decoded.glazing_thickness - max_u16).abs() < 0.0001);
        assert!((decoded.rabbet_width - max_u16).abs() < 0.0001);
        assert!((decoded.blade_width - max_u16).abs() < 0.0001);
    }

    // --- All-fields non-default round-trip ---

    #[test]
    fn test_all_fields_roundtrip() {
        let params = ShareableParams {
            artwork_height: 24.333,
            artwork_width: 36.777,
            mat_width: 3.125,
            frame_width: 1.875,
            frame_depth: 2.5,
            glazing_thickness: 0.118,
            matboard_thickness: 0.067,
            artwork_thickness: 0.012,
            backing_thickness: 0.25,
            rabbet_width: 0.312,
            rabbet_depth: 0.437,
            blade_width: 0.093,
            include_mat: true,
            unit_mm: true,
        };

        let encoded = generate_shareable_url(&params);
        let url = format!("https://example.com/?d={}", encoded);
        let decoded = decode_shareable_url(&url).unwrap();

        let tol = 0.0001;
        assert!((decoded.artwork_height - params.artwork_height).abs() < tol);
        assert!((decoded.artwork_width - params.artwork_width).abs() < tol);
        assert!((decoded.mat_width - params.mat_width).abs() < tol);
        assert!((decoded.frame_width - params.frame_width).abs() < tol);
        assert!((decoded.frame_depth - params.frame_depth).abs() < tol);
        assert!((decoded.glazing_thickness - params.glazing_thickness).abs() < tol);
        assert!((decoded.matboard_thickness - params.matboard_thickness).abs() < tol);
        assert!((decoded.artwork_thickness - params.artwork_thickness).abs() < tol);
        assert!((decoded.backing_thickness - params.backing_thickness).abs() < tol);
        assert!((decoded.rabbet_width - params.rabbet_width).abs() < tol);
        assert!((decoded.rabbet_depth - params.rabbet_depth).abs() < tol);
        assert!((decoded.blade_width - params.blade_width).abs() < tol);
        assert_eq!(decoded.include_mat, true);
        assert_eq!(decoded.unit_mm, true);
    }
}
