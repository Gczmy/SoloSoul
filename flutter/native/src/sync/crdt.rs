//! CRDT document for profile data synchronization
//!
//! Maps ProfileData to a Yrs (Yjs-compatible) CRDT document for
//! conflict-free replication between devices.
//!
//! Strategy: each top-level section (identity, travel, financial, professional,
//! preferences) is stored as a JSON string in a Yrs Map. This avoids mapping
//! every individual field to CRDT types while still getting automatic conflict
//! resolution at the section level.

use serde::{Deserialize, Serialize};
use yrs::{
    updates::{decoder::Decode, encoder::Encode},
    Any, Doc, Map, MapRef, Out, ReadTxn, Transact,
};

use crate::vault::ProfileData;

/// Top-level map keys for profile sections
const KEY_IDENTITY: &str = "identity";
const KEY_TRAVEL: &str = "travel";
const KEY_FINANCIAL: &str = "financial";
const KEY_PROFESSIONAL: &str = "professional";
const KEY_PREFERENCES: &str = "preferences";
const KEY_UNIFIED_OBJECTS: &str = "unifiedObjects";
const KEY_META: &str = "_meta";

/// Metadata stored alongside profile data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocMeta {
    pub profile_id: String,
    pub version: u32,
    pub last_modified: String,
}

/// A CRDT document wrapping ProfileData
pub struct SoloDoc {
    doc: Doc,
    root: MapRef,
}

impl SoloDoc {
    /// Create a new SoloDoc from a ProfileData
    pub fn from_profile(profile: &ProfileData, meta: &DocMeta) -> Self {
        let doc = Doc::new();
        let root = doc.get_or_insert_map("profile");

        {
            let mut txn = doc.transact_mut();

            // Store each section as serialized JSON
            if let Some(ref identity) = profile.identity {
                let json = serde_json::to_string(identity).unwrap_or_default();
                root.insert(&mut txn, KEY_IDENTITY, json);
            }
            if let Some(ref travel) = profile.travel {
                let json = serde_json::to_string(travel).unwrap_or_default();
                root.insert(&mut txn, KEY_TRAVEL, json);
            }
            if let Some(ref financial) = profile.financial {
                let json = serde_json::to_string(financial).unwrap_or_default();
                root.insert(&mut txn, KEY_FINANCIAL, json);
            }
            if let Some(ref professional) = profile.professional {
                let json = serde_json::to_string(professional).unwrap_or_default();
                root.insert(&mut txn, KEY_PROFESSIONAL, json);
            }
            if let Some(ref preferences) = profile.preferences {
                let json = serde_json::to_string(preferences).unwrap_or_default();
                root.insert(&mut txn, KEY_PREFERENCES, json);
            }
            if let Some(ref unified_objects) = profile.unified_objects {
                let json = serde_json::to_string(unified_objects).unwrap_or_default();
                root.insert(&mut txn, KEY_UNIFIED_OBJECTS, json);
            }

            // Store metadata
            let meta_json = serde_json::to_string(meta).unwrap_or_default();
            root.insert(&mut txn, KEY_META, meta_json);
        }

        Self { doc, root }
    }

    /// Create a SoloDoc from an existing Yrs update (for receiving remote state)
    pub fn from_update(update: &[u8]) -> Result<Self, String> {
        let doc = Doc::new();
        let root = doc.get_or_insert_map("profile");
        {
            let mut txn = doc.transact_mut();
            let update = yrs::Update::decode_v2(update)
                .map_err(|e| format!("Failed to decode update: {}", e))?;
            txn.apply_update(update);
        }
        Ok(Self { doc, root })
    }

    /// Convert back to ProfileData
    pub fn to_profile(&self) -> Result<ProfileData, String> {
        let txn = self.doc.transact();

        let identity = self.read_section(&txn, KEY_IDENTITY)?;
        let travel = self.read_section(&txn, KEY_TRAVEL)?;
        let financial = self.read_section(&txn, KEY_FINANCIAL)?;
        let professional = self.read_section(&txn, KEY_PROFESSIONAL)?;
        let preferences = self.read_section(&txn, KEY_PREFERENCES)?;
        let unified_objects = self.read_section::<serde_json::Value>(&txn, KEY_UNIFIED_OBJECTS)?;

        Ok(ProfileData {
            identity,
            travel,
            financial,
            professional,
            preferences,
            unified_objects,
        })
    }

    /// Read metadata from the document
    pub fn meta(&self) -> Result<Option<DocMeta>, String> {
        let txn = self.doc.transact();
        match self.root.get(&txn, KEY_META) {
            Some(Out::Any(Any::String(s))) => {
                let meta: DocMeta = serde_json::from_str(s.as_ref())
                    .map_err(|e| format!("Meta parse error: {}", e))?;
                Ok(Some(meta))
            }
            _ => Ok(None),
        }
    }

    /// Apply a remote update to this document
    pub fn apply_update(&mut self, update: &[u8]) -> Result<(), String> {
        let mut txn = self.doc.transact_mut();
        let update = yrs::Update::decode_v2(update)
            .map_err(|e| format!("Failed to decode update: {}", e))?;
        txn.apply_update(update);
        Ok(())
    }

    /// Encode the full document state as an update
    pub fn encode_state_as_update(&self) -> Vec<u8> {
        let txn = self.doc.transact();
        let empty_sv = yrs::StateVector::default();
        txn.encode_state_as_update_v2(&empty_sv)
    }

    /// Encode a differential update relative to a remote state vector
    pub fn encode_diff(&self, remote_sv: &[u8]) -> Result<Vec<u8>, String> {
        let txn = self.doc.transact();
        let sv = yrs::StateVector::decode_v2(remote_sv)
            .map_err(|e| format!("Failed to decode state vector: {}", e))?;
        let update = txn.encode_state_as_update_v2(&sv);
        Ok(update)
    }

    /// Get the current state vector (for diff computation)
    pub fn state_vector(&self) -> Vec<u8> {
        let txn = self.doc.transact();
        txn.state_vector().encode_v2()
    }

    /// Read a single section from the map, deserializing from JSON
    fn read_section<T: for<'de> Deserialize<'de>>(
        &self,
        txn: &impl ReadTxn,
        key: &str,
    ) -> Result<Option<T>, String> {
        match self.root.get(txn, key) {
            Some(Out::Any(Any::String(s))) => {
                let val: T = serde_json::from_str(s.as_ref())
                    .map_err(|e| format!("Section '{}' parse error: {}", key, e))?;
                Ok(Some(val))
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::*;

    fn make_test_profile() -> ProfileData {
        ProfileData {
            identity: Some(IdentityData {
                full_name: Some("Alice Smith".to_string()),
                given_name: Some("Alice".to_string()),
                family_name: Some("Smith".to_string()),
                date_of_birth: Some("1990-01-15".to_string()),
                gender: Some("female".to_string()),
                nationality: Some("US".to_string()),
                id_cards: vec![],
                contact: None,
                addresses: vec![],
            }),
            travel: None,
            financial: None,
            professional: None,
            preferences: None,
        }
    }

    fn make_meta() -> DocMeta {
        DocMeta {
            profile_id: "test-profile-1".to_string(),
            version: 1,
            last_modified: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_roundtrip_basic() {
        let profile = make_test_profile();
        let meta = make_meta();
        let doc = SoloDoc::from_profile(&profile, &meta);

        let restored = doc.to_profile().unwrap();
        assert!(restored.identity.is_some());
        let identity = restored.identity.unwrap();
        assert_eq!(identity.full_name, Some("Alice Smith".to_string()));
        assert_eq!(identity.given_name, Some("Alice".to_string()));
        assert_eq!(identity.gender, Some("female".to_string()));
    }

    #[test]
    fn test_roundtrip_empty_profile() {
        let profile = ProfileData {
            identity: None,
            travel: None,
            financial: None,
            professional: None,
            preferences: None,
        };
        let meta = make_meta();
        let doc = SoloDoc::from_profile(&profile, &meta);

        let restored = doc.to_profile().unwrap();
        assert!(restored.identity.is_none());
        assert!(restored.travel.is_none());
    }

    #[test]
    fn test_meta_roundtrip() {
        let profile = make_test_profile();
        let meta = make_meta();
        let doc = SoloDoc::from_profile(&profile, &meta);

        let restored_meta = doc.meta().unwrap().unwrap();
        assert_eq!(restored_meta.profile_id, "test-profile-1");
        assert_eq!(restored_meta.version, 1);
    }

    #[test]
    fn test_state_vector_and_encode() {
        let profile = make_test_profile();
        let meta = make_meta();
        let doc = SoloDoc::from_profile(&profile, &meta);

        let sv = doc.state_vector();
        assert!(!sv.is_empty());

        let full_update = doc.encode_state_as_update();
        assert!(!full_update.is_empty());
    }

    #[test]
    fn test_apply_remote_update_merge() {
        // Device A creates profile with identity
        let mut profile_a = make_test_profile();
        profile_a.travel = None;
        let meta_a = make_meta();
        let doc_a = SoloDoc::from_profile(&profile_a, &meta_a);

        // Device B starts with empty profile, gets A's state
        let profile_b = ProfileData {
            identity: None,
            travel: None,
            financial: None,
            professional: None,
            preferences: None,
        };
        let meta_b = DocMeta {
            profile_id: "test-profile-1".to_string(),
            version: 1,
            last_modified: "2026-01-01T00:00:00Z".to_string(),
        };
        let mut doc_b = SoloDoc::from_profile(&profile_b, &meta_b);

        // Apply A's full state to B
        let update_from_a = doc_a.encode_state_as_update();
        doc_b.apply_update(&update_from_a).unwrap();

        // B should now have A's identity
        let restored = doc_b.to_profile().unwrap();
        assert!(restored.identity.is_some());
        assert_eq!(
            restored.identity.unwrap().full_name,
            Some("Alice Smith".to_string())
        );
    }

    #[test]
    fn test_bidirectional_merge() {
        // A has identity, B has travel — after sync both should have both
        let mut profile_a = make_test_profile();
        profile_a.travel = None;
        let meta = make_meta();
        let doc_a = SoloDoc::from_profile(&profile_a, &meta);

        let profile_b = ProfileData {
            identity: None,
            travel: Some(TravelData {
                passports: vec![PassportData {
                    number: Some("AB123456".to_string()),
                    country: Some("US".to_string()),
                    issue_date: Some("2020-01-01".to_string()),
                    expiry_date: Some("2030-06-01".to_string()),
                    holder_name: Some("Alice Smith".to_string()),
                    is_deleted: false,
                    deleted_at: None,
                }],
                visas: vec![],
                travel_history: vec![],
            }),
            financial: None,
            professional: None,
            preferences: None,
        };
        let mut doc_b = SoloDoc::from_profile(&profile_b, &meta);

        // Exchange updates
        let sv_b = doc_b.state_vector();
        let update_a_for_b = doc_a.encode_diff(&sv_b).unwrap();

        let sv_a = doc_a.state_vector();
        let update_b_for_a = doc_b.encode_diff(&sv_a).unwrap();

        // Apply to each other
        doc_b.apply_update(&update_a_for_b).unwrap();
        let mut doc_a_mut = doc_a; // need mut for apply
        doc_a_mut.apply_update(&update_b_for_a).unwrap();

        // Both should have identity AND travel
        let restored_a = doc_a_mut.to_profile().unwrap();
        let restored_b = doc_b.to_profile().unwrap();

        assert!(restored_a.identity.is_some(), "A should have identity");
        assert!(
            restored_a.travel.is_some(),
            "A should have travel after merge"
        );
        assert!(
            restored_b.identity.is_some(),
            "B should have identity after merge"
        );
        assert!(restored_b.travel.is_some(), "B should have travel");
    }
}
