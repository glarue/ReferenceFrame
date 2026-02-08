//! Design History Module
//!
//! Provides data structures and CRUD operations for storing frame design history.
//! The core logic is platform-agnostic; storage persistence is handled by platform layers.

use serde::{Deserialize, Serialize};
use crate::frame::FrameDesign;

/// Default maximum number of history entries
pub const DEFAULT_MAX_ENTRIES: usize = 50;

/// A single history entry containing a design snapshot
///
/// Tracks multiple timestamps for when the same design was saved,
/// avoiding duplicate entries for identical designs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Complete design snapshot at time of save
    pub design: FrameDesign,
    /// Unix timestamps in seconds when this design was saved (newest first)
    pub timestamps: Vec<i64>,
    /// User-provided or auto-generated title
    pub title: String,
}

impl HistoryEntry {
    /// Create a new history entry with a single timestamp
    pub fn new(design: FrameDesign, timestamp: i64, title: String) -> Self {
        Self {
            design,
            timestamps: vec![timestamp],
            title,
        }
    }

    /// Create entry with auto-generated title based on artwork dimensions
    pub fn with_auto_title(design: FrameDesign, timestamp: i64) -> Self {
        let title = Self::generate_title(&design);
        Self::new(design, timestamp, title)
    }

    /// Generate a default title from design dimensions
    pub fn generate_title(design: &FrameDesign) -> String {
        format!(
            "{:.1}\" × {:.1}\" Frame",
            design.artwork_height,
            design.artwork_width
        )
    }

    /// Get the most recent timestamp
    pub fn latest_timestamp(&self) -> i64 {
        self.timestamps.first().copied().unwrap_or(0)
    }

    /// Get the original (oldest) timestamp
    pub fn original_timestamp(&self) -> i64 {
        self.timestamps.last().copied().unwrap_or(0)
    }

    /// Get number of times this design was saved
    pub fn save_count(&self) -> usize {
        self.timestamps.len()
    }

    /// Add a new timestamp (for duplicate saves)
    pub fn add_timestamp(&mut self, timestamp: i64) {
        self.timestamps.insert(0, timestamp);
    }
}

/// Collection of history entries with CRUD operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignHistory {
    /// History entries, newest first
    pub entries: Vec<HistoryEntry>,
    /// Maximum number of entries to retain
    pub max_entries: usize,
}

impl Default for DesignHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl DesignHistory {
    /// Create empty history with default max entries
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: DEFAULT_MAX_ENTRIES,
        }
    }

    /// Create empty history with custom max entries
    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries: max_entries.max(1), // At least 1 entry
        }
    }

    /// Add a new entry to the history
    ///
    /// If an identical design already exists and `force_new` is false, adds the
    /// timestamp to that entry and moves it to the front. Otherwise creates a new entry.
    /// If history exceeds max_entries, oldest entries are removed.
    ///
    /// Returns true if this was a new design (or forced new), false if it was a duplicate.
    pub fn add_entry(&mut self, design: FrameDesign, timestamp: i64, title: String, force_new: bool) -> bool {
        // Check for existing identical design (unless forcing new)
        if !force_new {
            if let Some(idx) = self.find_matching_design(&design) {
                // Add timestamp to existing entry
                self.entries[idx].add_timestamp(timestamp);
                // Move to front (most recently saved)
                let entry = self.entries.remove(idx);
                self.entries.insert(0, entry);
                return false;
            }
        }

        // New design - create new entry
        let entry = HistoryEntry::new(design, timestamp, title);
        self.entries.insert(0, entry);
        self.enforce_limit();
        true
    }

    /// Add entry with auto-generated title
    ///
    /// Returns true if this was a new design (or forced new), false if it was a duplicate.
    pub fn add_entry_auto_title(&mut self, design: FrameDesign, timestamp: i64, force_new: bool) -> bool {
        let title = HistoryEntry::generate_title(&design);
        self.add_entry(design, timestamp, title, force_new)
    }

    /// Find index of entry with matching design
    fn find_matching_design(&self, design: &FrameDesign) -> Option<usize> {
        self.entries.iter().position(|e| &e.design == design)
    }

    /// Get entry by index (0 = newest)
    pub fn get(&self, index: usize) -> Option<&HistoryEntry> {
        self.entries.get(index)
    }

    /// Remove entry at index
    ///
    /// Returns the removed entry if index was valid
    pub fn remove(&mut self, index: usize) -> Option<HistoryEntry> {
        if index < self.entries.len() {
            Some(self.entries.remove(index))
        } else {
            None
        }
    }

    /// Update title for entry at index
    pub fn update_title(&mut self, index: usize, title: String) -> bool {
        if let Some(entry) = self.entries.get_mut(index) {
            entry.title = title;
            true
        } else {
            false
        }
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| e.to_string())
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }

    /// Enforce max entries limit by removing oldest entries
    fn enforce_limit(&mut self) {
        while self.entries.len() > self.max_entries {
            self.entries.pop();
        }
    }

    /// Set new max entries limit (enforces immediately if over limit)
    pub fn set_max_entries(&mut self, max: usize) {
        self.max_entries = max.max(1);
        self.enforce_limit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_design() -> FrameDesign {
        FrameDesign::new(10.0, 8.0)
    }

    fn create_different_design() -> FrameDesign {
        FrameDesign::new(12.0, 10.0)
    }

    #[test]
    fn test_new_history() {
        let history = DesignHistory::new();
        assert!(history.is_empty());
        assert_eq!(history.max_entries, DEFAULT_MAX_ENTRIES);
    }

    #[test]
    fn test_add_entry() {
        let mut history = DesignHistory::new();
        let design = create_test_design();

        let is_new = history.add_entry(design.clone(), 1000, "Test Design".to_string(), false);

        assert!(is_new);
        assert_eq!(history.len(), 1);
        let entry = history.get(0).unwrap();
        assert_eq!(entry.title, "Test Design");
        assert_eq!(entry.latest_timestamp(), 1000);
        assert_eq!(entry.save_count(), 1);
    }

    #[test]
    fn test_add_duplicate_design() {
        let mut history = DesignHistory::new();
        let design = create_test_design();

        // Add first time
        let is_new1 = history.add_entry(design.clone(), 1000, "First Save".to_string(), false);
        assert!(is_new1);
        assert_eq!(history.len(), 1);

        // Add same design again
        let is_new2 = history.add_entry(design.clone(), 2000, "Second Save".to_string(), false);
        assert!(!is_new2); // Should be detected as duplicate
        assert_eq!(history.len(), 1); // Still only one entry

        let entry = history.get(0).unwrap();
        assert_eq!(entry.save_count(), 2);
        assert_eq!(entry.latest_timestamp(), 2000);
        assert_eq!(entry.original_timestamp(), 1000);
        // Title should remain from first save
        assert_eq!(entry.title, "First Save");
    }

    #[test]
    fn test_force_new_bypasses_duplicate_detection() {
        let mut history = DesignHistory::new();
        let design = create_test_design();

        // Add first time
        history.add_entry(design.clone(), 1000, "First Save".to_string(), false);
        assert_eq!(history.len(), 1);

        // Force new entry with same design
        let is_new = history.add_entry(design.clone(), 2000, "Forced New".to_string(), true);
        assert!(is_new);
        assert_eq!(history.len(), 2);

        // New entry should be at front
        assert_eq!(history.get(0).unwrap().title, "Forced New");
        assert_eq!(history.get(1).unwrap().title, "First Save");
    }

    #[test]
    fn test_duplicate_moves_to_front() {
        let mut history = DesignHistory::new();
        let design1 = create_test_design();
        let design2 = create_different_design();

        // Add two different designs
        history.add_entry(design1.clone(), 1000, "Design 1".to_string(), false);
        history.add_entry(design2.clone(), 2000, "Design 2".to_string(), false);

        // Design 2 should be at front
        assert_eq!(history.get(0).unwrap().title, "Design 2");
        assert_eq!(history.get(1).unwrap().title, "Design 1");

        // Save design 1 again - should move to front
        history.add_entry(design1.clone(), 3000, "Design 1 Again".to_string(), false);

        assert_eq!(history.len(), 2);
        assert_eq!(history.get(0).unwrap().title, "Design 1"); // Now at front
        assert_eq!(history.get(0).unwrap().save_count(), 2);
        assert_eq!(history.get(1).unwrap().title, "Design 2");
    }

    #[test]
    fn test_add_entry_auto_title() {
        let mut history = DesignHistory::new();
        let design = create_test_design();

        history.add_entry_auto_title(design, 1000, false);

        let entry = history.get(0).unwrap();
        assert!(entry.title.contains("10.0"));
        assert!(entry.title.contains("8.0"));
    }

    #[test]
    fn test_newest_first() {
        let mut history = DesignHistory::new();

        // Create different designs so they don't get merged
        let mut design1 = create_test_design();
        let mut design2 = create_test_design();
        let mut design3 = create_test_design();
        design1.artwork_height = 10.0;
        design2.artwork_height = 11.0;
        design3.artwork_height = 12.0;

        history.add_entry(design1, 1000, "First".to_string(), false);
        history.add_entry(design2, 2000, "Second".to_string(), false);
        history.add_entry(design3, 3000, "Third".to_string(), false);

        assert_eq!(history.get(0).unwrap().title, "Third");
        assert_eq!(history.get(1).unwrap().title, "Second");
        assert_eq!(history.get(2).unwrap().title, "First");
    }

    #[test]
    fn test_max_entries_enforcement() {
        let mut history = DesignHistory::with_max_entries(3);

        for i in 0..5 {
            let mut design = create_test_design();
            design.artwork_height = i as f64; // Make each unique
            history.add_entry(design, i as i64, format!("Design {}", i), false);
        }

        assert_eq!(history.len(), 3);
        // Newest entries should be kept
        assert_eq!(history.get(0).unwrap().title, "Design 4");
        assert_eq!(history.get(1).unwrap().title, "Design 3");
        assert_eq!(history.get(2).unwrap().title, "Design 2");
    }

    #[test]
    fn test_remove() {
        let mut history = DesignHistory::new();

        let mut design1 = create_test_design();
        let mut design2 = create_test_design();
        design1.artwork_height = 10.0;
        design2.artwork_height = 11.0;

        history.add_entry(design1, 1000, "First".to_string(), false);
        history.add_entry(design2, 2000, "Second".to_string(), false);

        let removed = history.remove(0).unwrap();
        assert_eq!(removed.title, "Second");
        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).unwrap().title, "First");
    }

    #[test]
    fn test_remove_invalid_index() {
        let mut history = DesignHistory::new();
        history.add_entry(create_test_design(), 1000, "Test".to_string(), false);

        assert!(history.remove(5).is_none());
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_update_title() {
        let mut history = DesignHistory::new();
        history.add_entry(create_test_design(), 1000, "Original".to_string(), false);

        assert!(history.update_title(0, "Updated".to_string()));
        assert_eq!(history.get(0).unwrap().title, "Updated");

        assert!(!history.update_title(5, "Invalid".to_string()));
    }

    #[test]
    fn test_clear() {
        let mut history = DesignHistory::new();

        let mut design1 = create_test_design();
        let mut design2 = create_test_design();
        design1.artwork_height = 10.0;
        design2.artwork_height = 11.0;

        history.add_entry(design1, 1000, "Test".to_string(), false);
        history.add_entry(design2, 2000, "Test 2".to_string(), false);

        history.clear();
        assert!(history.is_empty());
    }

    #[test]
    fn test_json_roundtrip() {
        let mut history = DesignHistory::new();
        let design = create_test_design();

        // Add same design twice to test timestamps array
        history.add_entry(design.clone(), 1000, "Test Design".to_string(), false);
        history.add_entry(design.clone(), 2000, "Test Design Again".to_string(), false);

        let json = history.to_json().unwrap();
        let restored = DesignHistory::from_json(&json).unwrap();

        assert_eq!(restored.len(), 1);
        assert_eq!(restored.get(0).unwrap().title, "Test Design");
        assert_eq!(restored.get(0).unwrap().save_count(), 2);
    }

    #[test]
    fn test_set_max_entries() {
        let mut history = DesignHistory::new();

        for i in 0..10 {
            let mut design = create_test_design();
            design.artwork_height = i as f64;
            history.add_entry(design, i as i64, format!("Design {}", i), false);
        }

        history.set_max_entries(5);
        assert_eq!(history.len(), 5);
        assert_eq!(history.max_entries, 5);
    }

    #[test]
    fn test_entry_timestamps() {
        let design = create_test_design();
        let mut entry = HistoryEntry::new(design, 1000, "Test".to_string());

        assert_eq!(entry.save_count(), 1);
        assert_eq!(entry.latest_timestamp(), 1000);
        assert_eq!(entry.original_timestamp(), 1000);

        entry.add_timestamp(2000);
        entry.add_timestamp(3000);

        assert_eq!(entry.save_count(), 3);
        assert_eq!(entry.latest_timestamp(), 3000);
        assert_eq!(entry.original_timestamp(), 1000);
    }
}
