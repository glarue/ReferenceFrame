//! Version information for the ReferenceFrame core library
//!
//! Provides compile-time version constants from Cargo.toml

use serde::{Deserialize, Serialize};

/// Core library version from Cargo.toml
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Core library name
pub const CORE_NAME: &str = env!("CARGO_PKG_NAME");

/// Version information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// Core library version (e.g., "1.0.0")
    pub core: String,
    /// Platform wrapper version (e.g., "1.0.0" for mobile, "1.0.0" for WASM)
    pub platform: String,
    /// Platform name (e.g., "ios", "android", "web")
    pub platform_name: String,
    /// Build identifier (optional, e.g., build number or git hash)
    pub build: Option<String>,
}

impl VersionInfo {
    /// Create version info with core version and platform details
    pub fn new(platform: &str, platform_name: &str, build: Option<&str>) -> Self {
        Self {
            core: CORE_VERSION.to_string(),
            platform: platform.to_string(),
            platform_name: platform_name.to_string(),
            build: build.map(|s| s.to_string()),
        }
    }

    /// Get just the core version
    pub fn core_only() -> Self {
        Self {
            core: CORE_VERSION.to_string(),
            platform: String::new(),
            platform_name: String::new(),
            build: None,
        }
    }

    /// Format as display string (e.g., "Core 1.0.0 | iOS 1.2.0 (build 15)")
    pub fn display(&self) -> String {
        let mut parts = vec![format!("Core {}", self.core)];

        if !self.platform.is_empty() {
            let platform_str = if let Some(ref build) = self.build {
                format!("{} {} ({})", self.platform_name, self.platform, build)
            } else {
                format!("{} {}", self.platform_name, self.platform)
            };
            parts.push(platform_str);
        }

        parts.join(" | ")
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Get just the core version string
pub fn get_core_version() -> &'static str {
    CORE_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_version_not_empty() {
        assert!(!CORE_VERSION.is_empty());
        assert!(CORE_VERSION.contains('.'));
    }

    #[test]
    fn test_version_info_display() {
        let info = VersionInfo::new("1.2.0", "iOS", Some("15"));
        let display = info.display();
        assert!(display.contains("Core"));
        assert!(display.contains("iOS"));
        assert!(display.contains("1.2.0"));
    }
}
