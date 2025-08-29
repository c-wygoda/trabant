use crate::sgp4::elements::OrbitalElements;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OmmError {
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("Invalid epoch format: {0}")]
    InvalidEpoch(String),
    
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
}

/// OMM data structure matching the JSON format from fixtures
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OmmData {
    #[serde(rename = "OBJECT_NAME")]
    pub object_name: String,
    
    #[serde(rename = "OBJECT_ID")]
    pub object_id: String,
    
    #[serde(rename = "EPOCH")]
    pub epoch: String,
    
    #[serde(rename = "MEAN_MOTION")]
    pub mean_motion: f64,
    
    #[serde(rename = "ECCENTRICITY")]
    pub eccentricity: f64,
    
    #[serde(rename = "INCLINATION")]
    pub inclination: f64,
    
    #[serde(rename = "RA_OF_ASC_NODE")]
    pub ra_of_asc_node: f64,
    
    #[serde(rename = "ARG_OF_PERICENTER")]
    pub arg_of_pericenter: f64,
    
    #[serde(rename = "MEAN_ANOMALY")]
    pub mean_anomaly: f64,
    
    #[serde(rename = "BSTAR")]
    pub bstar: f64,
    
    #[serde(rename = "MEAN_MOTION_DOT")]
    pub mean_motion_dot: f64,
    
    #[serde(rename = "MEAN_MOTION_DDOT")]
    pub mean_motion_ddot: f64,
    
    #[serde(rename = "NORAD_CAT_ID")]
    pub norad_cat_id: u32,
}

pub struct OmmParser;

impl OmmParser {
    /// Parse OMM JSON data
    pub fn parse(json_str: &str) -> Result<OrbitalElements, OmmError> {
        let omm_data: OmmData = serde_json::from_str(json_str)?;
        Self::omm_to_elements(omm_data)
    }
    
    /// Convert OmmData to OrbitalElements
    fn omm_to_elements(omm: OmmData) -> Result<OrbitalElements, OmmError> {
        let epoch = Self::parse_epoch(&omm.epoch)?;
        
        let elements = OrbitalElements {
            name: omm.object_name,
            norad_id: omm.norad_cat_id,
            international_designator: omm.object_id,
            epoch,
            mean_motion_dot: omm.mean_motion_dot,
            mean_motion_ddot: omm.mean_motion_ddot,
            bstar: omm.bstar,
            ephemeris_type: 0, // Always 0 for SGP4
            element_set_number: 0, // Not provided in OMM format
            inclination: omm.inclination,
            raan: omm.ra_of_asc_node,
            eccentricity: omm.eccentricity,
            argument_of_perigee: omm.arg_of_pericenter,
            mean_anomaly: omm.mean_anomaly,
            mean_motion: omm.mean_motion,
            revolution_number: 0, // Not provided in OMM format
            classification: 'U', // Default to unclassified
        };
        
        elements.validate().map_err(|e| OmmError::ValidationError(e))?;
        Ok(elements)
    }
    
    /// Parse ISO 8601 timestamp to DateTime<Utc>
    fn parse_epoch(epoch_str: &str) -> Result<DateTime<Utc>, OmmError> {
        epoch_str.parse::<DateTime<Utc>>()
            .map_err(|_| OmmError::InvalidEpoch(epoch_str.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOTSAT1_OMM_JSON: &str = r#"{
        "OBJECT_NAME": "HOTSAT-1",
        "OBJECT_ID": "2023-084Y",
        "EPOCH": "2025-08-29T07:07:10.490112Z",
        "MEAN_MOTION": 15.21880160,
        "ECCENTRICITY": 0.0005478,
        "INCLINATION": 97.5868,
        "RA_OF_ASC_NODE": 7.4102,
        "ARG_OF_PERICENTER": 336.1866,
        "MEAN_ANOMALY": 23.9116,
        "BSTAR": 0.00031779927,
        "MEAN_MOTION_DOT": 0.00007163,
        "MEAN_MOTION_DDOT": 0.0,
        "NORAD_CAT_ID": 56954
    }"#;

    #[test]
    fn test_parse_hotsat1_omm() {
        let elements = OmmParser::parse(HOTSAT1_OMM_JSON).unwrap();
        
        assert_eq!(elements.name, "HOTSAT-1");
        assert_eq!(elements.norad_id, 56954);
        assert_eq!(elements.international_designator, "2023-084Y");
        assert!((elements.inclination - 97.5868).abs() < 1e-6);
        assert!((elements.eccentricity - 0.0005478).abs() < 1e-7);
        assert!((elements.mean_motion - 15.21880160).abs() < 1e-8);
        assert!((elements.bstar - 0.00031779927).abs() < 1e-10);
    }

    #[test]
    fn test_invalid_json() {
        let bad_json = r#"{"invalid": "json"#;
        assert!(OmmParser::parse(bad_json).is_err());
    }

    #[test]
    fn test_invalid_epoch() {
        let bad_epoch_json = r#"{
            "OBJECT_NAME": "TEST",
            "OBJECT_ID": "TEST",
            "EPOCH": "invalid-date",
            "MEAN_MOTION": 15.0,
            "ECCENTRICITY": 0.001,
            "INCLINATION": 90.0,
            "RA_OF_ASC_NODE": 0.0,
            "ARG_OF_PERICENTER": 0.0,
            "MEAN_ANOMALY": 0.0,
            "BSTAR": 0.0,
            "MEAN_MOTION_DOT": 0.0,
            "MEAN_MOTION_DDOT": 0.0,
            "NORAD_CAT_ID": 12345
        }"#;
        assert!(OmmParser::parse(bad_epoch_json).is_err());
    }
}