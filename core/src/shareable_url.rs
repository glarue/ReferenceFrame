// Shareable URL generation for frame designs
//
// Ported from Python shareable_url.py with identical behavior

use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

/// Parameters for shareable URL encoding
///
/// `mat_width` is the top/bottom mat border; `mat_width_sides` is the left/right
/// border. The three fields added in format v1 (`mat_width_sides`, `mat_overlap`,
/// `assembly_margin`) carry `#[serde(default)]` so JSON produced before they
/// existed still deserializes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShareableParams {
    pub artwork_height: f64,
    pub artwork_width: f64,
    pub mat_width: f64, // top/bottom mat border (single value in format v0)
    #[serde(default)]
    pub mat_width_sides: f64, // left/right mat border (format v1)
    #[serde(default)]
    pub mat_overlap: f64, // format v1
    pub frame_width: f64,
    pub frame_depth: f64,
    pub glazing_thickness: f64,
    pub matboard_thickness: f64,
    pub artwork_thickness: f64,
    pub backing_thickness: f64,
    #[serde(default)]
    pub assembly_margin: f64, // format v1
    pub rabbet_width: f64,  // Horizontal lip overlap
    pub rabbet_depth: f64,  // Z-axis cutout depth
    pub blade_width: f64,
    pub include_mat: bool,
    pub unit_mm: bool,
}

/// Defaults applied to the format-v1 fields when decoding an older (v0) URL
/// that never carried them. These match the app's factory defaults so a shared
/// v0 link resolves to the same design it did before v1 existed.
const DEFAULT_MAT_OVERLAP: f64 = 0.125;
const DEFAULT_ASSEMBLY_MARGIN: f64 = 0.0625;

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
            DecodeError::TruncatedData => write!(f, "Truncated data (expected 28, 30, or 37 bytes)"),
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
// Version 0 covers the 30-byte format and the legacy 28-byte format
// (distinguished by payload length); version 1 adds 7 trailing bytes for
// mat_width_sides, mat_overlap, and assembly_margin (37 bytes total). All URLs
// generated before versioning have zero in the top bits, so they decode as
// version 0. The decoder reads BOTH v0 and v1; any other version is rejected.
// Payload length implies the version (28/30 → v0, 37 → v1) and is cross-checked
// against the version bits so a mismatched/future payload can't be misparsed.

/// Current binary format version (stored in the top 3 bits of the flags byte)
const FORMAT_VERSION: u8 = 1;

/// Byte length of a version-1 payload (30-byte v0 layout + 7 appended bytes)
const V1_LEN: usize = 37;

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
/// Binary format version 1 (37 bytes → ~50 chars base64):
///     5 × uint24: h, w, mw (top/bottom), fw, fd (×10000, max 1677.7215")
///     7 × uint16: gt, mt, at, bt, rw, rd, bw (×10000, max 6.5535")
///     1 × byte:   flags (bit 0 = mat, bit 1 = unit_mm, bits 5-7 = format version)
///     1 × uint24: mat_width_sides (left/right)
///     2 × uint16: mat_overlap, assembly_margin
///
/// The first 30 bytes are byte-identical to the v0 layout (so v0 URLs are a
/// prefix); v1 appends the last 7 bytes after the flags byte. All values are
/// stored in inches; out-of-range values are clamped rather than wrapped.
pub fn generate_shareable_url(params: &ShareableParams) -> String {
    let mut packed = Vec::with_capacity(V1_LEN);

    // uint24 fields: h, w, mw (top/bottom), fw, fd (15 bytes total)
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
    let flags = (params.include_mat as u8)
        | ((params.unit_mm as u8) << 1)
        | (FORMAT_VERSION << VERSION_SHIFT);
    packed.push(flags);

    // Version-1 appended fields (7 bytes total).
    packed.extend_from_slice(&pack_uint24(params.mat_width_sides));
    packed.extend_from_slice(&pack_uint16(params.mat_overlap));
    packed.extend_from_slice(&pack_uint16(params.assembly_margin));

    // Base64 encode (URL-safe, no padding). Returns just the encoded string;
    // the caller builds the full URL based on their deployment location.
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

    // Payload length selects the format: 28/30 = v0 (flags is the last byte),
    // 37 = v1 (the 30-byte v0 layout plus 7 appended bytes; flags at index 29).
    let flags = match bytes.len() {
        28 => bytes[27],
        30 => bytes[29],
        V1_LEN => bytes[29],
        _ => return Err(DecodeError::TruncatedData),
    };

    // Shared prefix: uint24 fields (15 bytes) then the first four uint16 fields.
    let artwork_height = unpack_uint24(&bytes[0..3]);
    let artwork_width = unpack_uint24(&bytes[3..6]);
    let mat_width = unpack_uint24(&bytes[6..9]);
    let frame_width = unpack_uint24(&bytes[9..12]);
    let frame_depth = unpack_uint24(&bytes[12..15]);
    let glazing_thickness = unpack_uint16(&bytes[15..17]);
    let matboard_thickness = unpack_uint16(&bytes[17..19]);
    let artwork_thickness = unpack_uint16(&bytes[19..21]);
    let backing_thickness = unpack_uint16(&bytes[21..23]);

    // rabbet/blade and the v1-only fields depend on the format.
    let (rabbet_width, rabbet_depth, blade_width, mat_width_sides, mat_overlap, assembly_margin) =
        match bytes.len() {
            28 => {
                // Legacy v0: no rabbet_width (square rabbet), no v1 fields.
                let rd = unpack_uint16(&bytes[23..25]);
                let bw = unpack_uint16(&bytes[25..27]);
                (rd, rd, bw, mat_width, DEFAULT_MAT_OVERLAP, DEFAULT_ASSEMBLY_MARGIN)
            }
            30 => {
                // v0: rabbet_width present; v1 fields fall back to defaults and
                // mat_width_sides mirrors mat_width (symmetric borders).
                let rw = unpack_uint16(&bytes[23..25]);
                let rd = unpack_uint16(&bytes[25..27]);
                let bw = unpack_uint16(&bytes[27..29]);
                (rw, rd, bw, mat_width, DEFAULT_MAT_OVERLAP, DEFAULT_ASSEMBLY_MARGIN)
            }
            _ => {
                // v1: appended mat_width_sides (uint24) + mat_overlap +
                // assembly_margin (uint16), after the flags byte at index 29.
                let rw = unpack_uint16(&bytes[23..25]);
                let rd = unpack_uint16(&bytes[25..27]);
                let bw = unpack_uint16(&bytes[27..29]);
                let mws = unpack_uint24(&bytes[30..33]);
                let mo = unpack_uint16(&bytes[33..35]);
                let am = unpack_uint16(&bytes[35..37]);
                (rw, rd, bw, mws, mo, am)
            }
        };

    // The version bits must match the version implied by the payload length,
    // so a corrupt or future payload is rejected rather than misparsed.
    let version = flags >> VERSION_SHIFT;
    let expected_version = if bytes.len() == V1_LEN { 1 } else { 0 };
    if version != expected_version {
        return Err(DecodeError::UnsupportedVersion(version));
    }

    let include_mat = (flags & 0x01) != 0;
    let unit_mm = (flags & 0x02) != 0;

    Ok(ShareableParams {
        artwork_height,
        artwork_width,
        mat_width,
        mat_width_sides,
        mat_overlap,
        frame_width,
        frame_depth,
        glazing_thickness,
        matboard_thickness,
        artwork_thickness,
        backing_thickness,
        assembly_margin,
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
            mat_width_sides: 0.0,
            mat_overlap: 0.0,
            assembly_margin: 0.0,
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
            mat_width_sides: 0.0,
            mat_overlap: 0.0,
            assembly_margin: 0.0,
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
            mat_width_sides: 0.0,
            mat_overlap: 0.0,
            assembly_margin: 0.0,
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
            mat_width_sides: 0.0,
            mat_overlap: 0.0,
            assembly_margin: 0.0,
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
            mat_width_sides: 0.0,
            mat_overlap: 0.0,
            assembly_margin: 0.0,
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
            mat_width_sides: 0.0,
            mat_overlap: 0.0,
            assembly_margin: 0.0,
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
            mat_width_sides: 0.0,
            mat_overlap: 0.0,
            assembly_margin: 0.0,
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
            mat_width_sides: 0.0,
            mat_overlap: 0.0,
            assembly_margin: 0.0,
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
            mat_width_sides: 0.0,
            mat_overlap: 0.0,
            assembly_margin: 0.0,
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
            mat_width_sides: 0.0,
            mat_overlap: 0.0,
            assembly_margin: 0.0,
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
            mat_width_sides: 0.0,
            mat_overlap: 0.0,
            assembly_margin: 0.0,
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

    #[test]
    fn test_v1_new_fields_roundtrip() {
        // Separate mat borders, custom overlap, and assembly margin must all
        // survive a v1 encode/decode round-trip.
        let params = ShareableParams {
            artwork_height: 11.0,
            artwork_width: 14.0,
            mat_width: 2.0,       // top/bottom
            mat_width_sides: 1.5, // left/right (intentionally != top/bottom)
            mat_overlap: 0.25,
            frame_width: 0.75,
            frame_depth: 0.75,
            glazing_thickness: 0.093,
            matboard_thickness: 0.055,
            artwork_thickness: 0.008,
            backing_thickness: 0.125,
            assembly_margin: 0.05,
            rabbet_width: 0.375,
            rabbet_depth: 0.3125,
            blade_width: 0.125,
            include_mat: true,
            unit_mm: false,
        };

        let encoded = generate_shareable_url(&params);
        // v1 payloads are 37 bytes.
        assert_eq!(URL_SAFE_NO_PAD.decode(&encoded).unwrap().len(), V1_LEN);

        let decoded = decode_shareable_url(&format!("?d={}", encoded)).unwrap();
        let tol = 0.0001;
        assert!((decoded.mat_width - 2.0).abs() < tol);
        assert!((decoded.mat_width_sides - 1.5).abs() < tol);
        assert!((decoded.mat_overlap - 0.25).abs() < tol);
        assert!((decoded.assembly_margin - 0.05).abs() < tol);
        assert!((decoded.rabbet_width - 0.375).abs() < tol);
        assert!((decoded.rabbet_depth - 0.3125).abs() < tol);
    }

    #[test]
    fn test_v0_30byte_decode_fills_v1_defaults() {
        // A hand-crafted v0 (30-byte, version 0) payload must still decode, with
        // the v1-only fields filled from defaults and mat_width_sides mirroring
        // mat_width (symmetric borders).
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&pack_uint24(8.0)); // artwork_height
        bytes.extend_from_slice(&pack_uint24(12.0)); // artwork_width
        bytes.extend_from_slice(&pack_uint24(2.0)); // mat_width
        bytes.extend_from_slice(&pack_uint24(0.75)); // frame_width
        bytes.extend_from_slice(&pack_uint24(0.75)); // frame_depth
        bytes.extend_from_slice(&pack_uint16(0.093)); // glazing
        bytes.extend_from_slice(&pack_uint16(0.055)); // matboard
        bytes.extend_from_slice(&pack_uint16(0.008)); // artwork
        bytes.extend_from_slice(&pack_uint16(0.125)); // backing
        bytes.extend_from_slice(&pack_uint16(0.375)); // rabbet_width
        bytes.extend_from_slice(&pack_uint16(0.375)); // rabbet_depth
        bytes.extend_from_slice(&pack_uint16(0.125)); // blade_width
        bytes.push(0b0000_0011); // flags: version 0, unit_mm + include_mat set
        assert_eq!(bytes.len(), 30);

        let url = format!("?d={}", URL_SAFE_NO_PAD.encode(&bytes));
        let p = decode_shareable_url(&url).unwrap();
        let tol = 0.0001;
        assert!((p.mat_width - 2.0).abs() < tol);
        assert!((p.mat_width_sides - 2.0).abs() < tol); // mirrors mat_width
        assert!((p.mat_overlap - DEFAULT_MAT_OVERLAP).abs() < tol);
        assert!((p.assembly_margin - DEFAULT_ASSEMBLY_MARGIN).abs() < tol);
        assert!(p.include_mat && p.unit_mm);
    }
}
