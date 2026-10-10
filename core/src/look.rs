//! A design's look: how the finished frame is shown, kept with the design in saved
//! configs and history so each record restores its mat colour, wood and photo.
//!
//! None of it changes a measurement. Platforms store the photo itself (a downscaled
//! JPEG) and refer to it here by id.

use serde::{Deserialize, Serialize};

use crate::visualization::{PhotoFit, WoodTone};

/// Mat colour, wood and photo for one design. Every field is optional; an empty
/// look serializes as `{}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DesignLook {
    /// A `presets.json` `mat_colors` key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mat_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wood: Option<WoodLook>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo: Option<PhotoRef>,
}

/// Species, tone and board (see `WoodRender`). Whether grain is drawn at all stays a
/// view setting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WoodLook {
    pub species: String,
    #[serde(default)]
    pub tone: WoodTone,
    #[serde(default)]
    pub reshuffle: u32,
}

/// A stored photo (by the platform's id), its pixel size and how it sits in the
/// artwork area.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotoRef {
    pub id: String,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub fit: PhotoFit,
}

impl DesignLook {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }

    /// Same mat colour, wood and photo; the photo's crop may differ.
    pub fn same_choices(&self, other: &Self) -> bool {
        self.mat_color == other.mat_color
            && self.wood == other.wood
            && self.photo.as_ref().map(|p| &p.id) == other.photo.as_ref().map(|p| &p.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_look_is_an_empty_object() {
        assert_eq!(serde_json::to_string(&DesignLook::default()).unwrap(), "{}");
        let l: DesignLook = serde_json::from_str("{}").unwrap();
        assert!(l.is_empty());
    }

    #[test]
    fn look_round_trips() {
        let json = r#"{"mat_color":"cream","wood":{"species":"cherry","tone":"dark","reshuffle":2},
            "photo":{"id":"a1","width":1200,"height":800,"fit":{"mode":"fit","zoom":1.5,"center_x":0.4,"center_y":0.6}}}"#;
        let l: DesignLook = serde_json::from_str(json).unwrap();
        assert_eq!(l.wood.as_ref().unwrap().tone, WoodTone::Dark);
        let back: DesignLook = serde_json::from_str(&serde_json::to_string(&l).unwrap()).unwrap();
        assert_eq!(back, l);
    }

    #[test]
    fn crop_changes_keep_the_same_choices() {
        let a: DesignLook = serde_json::from_str(r#"{"photo":{"id":"a","width":10,"height":10}}"#).unwrap();
        let mut b = a.clone();
        b.photo.as_mut().unwrap().fit.zoom = 2.0;
        assert!(a.same_choices(&b));
        b.photo.as_mut().unwrap().id = "b".into();
        assert!(!a.same_choices(&b));
    }
}
