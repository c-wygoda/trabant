use crate::sgp4::elements::OrbitalElements;
use chrono::{DateTime, Utc, TimeZone};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TleError {
    #[error("Invalid TLE format: {0}")]
    InvalidFormat(String),
    
    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: u8, actual: u8 },
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Invalid epoch: {0}")]
    InvalidEpoch(String),
}

pub struct TleParser;

impl TleParser {
    /// Parse a three-line TLE set (name + two data lines)
    pub fn parse(tle_lines: &[&str]) -> Result<OrbitalElements, TleError> {
        if tle_lines.len() != 3 {
            return Err(TleError::InvalidFormat(
                "TLE must contain exactly 3 lines".to_string()
            ));
        }

        let name = tle_lines[0].trim();
        let line1 = tle_lines[1];
        let line2 = tle_lines[2];

        // Validate line lengths
        if line1.len() != 69 || line2.len() != 69 {
            return Err(TleError::InvalidFormat(
                "TLE lines must be exactly 69 characters".to_string()
            ));
        }

        // Validate checksums
        Self::validate_checksum(line1, 1)?;
        Self::validate_checksum(line2, 2)?;

        // Parse line 1
        let norad_id = Self::parse_int(&line1[2..7], "NORAD ID")? as u32;
        let classification = line1.chars().nth(7).unwrap_or('U');
        let international_designator = line1[9..17].trim().to_string();
        let epoch = Self::parse_epoch(&line1[18..32])?;
        let mean_motion_dot = Self::parse_float(&line1[33..43], "mean motion dot")?;
        let mean_motion_ddot = Self::parse_exponential(&line1[44..52], "mean motion ddot")?;
        let bstar = Self::parse_exponential(&line1[53..61], "B* drag term")?;
        let ephemeris_type = Self::parse_int(&line1[62..63], "ephemeris type")? as u32;
        let element_set_number = Self::parse_int(&line1[64..68], "element set number")? as u32;

        // Parse line 2
        let inclination = Self::parse_float(&line2[8..16], "inclination")?;
        let raan = Self::parse_float(&line2[17..25], "RAAN")?;
        let eccentricity = Self::parse_eccentricity(&line2[26..33])?;
        let argument_of_perigee = Self::parse_float(&line2[34..42], "argument of perigee")?;
        let mean_anomaly = Self::parse_float(&line2[43..51], "mean anomaly")?;
        let mean_motion = Self::parse_float(&line2[52..63], "mean motion")?;
        let revolution_number = Self::parse_int(&line2[63..68], "revolution number")? as u32;

        let elements = OrbitalElements {
            name: name.to_string(),
            norad_id,
            international_designator,
            epoch,
            mean_motion_dot,
            mean_motion_ddot,
            bstar,
            ephemeris_type,
            element_set_number,
            inclination,
            raan,
            eccentricity,
            argument_of_perigee,
            mean_anomaly,
            mean_motion,
            revolution_number,
            classification,
        };

        elements.validate().map_err(|e| TleError::ParseError(e))?;
        Ok(elements)
    }

    fn validate_checksum(line: &str, _line_number: u8) -> Result<(), TleError> {
        let expected = line.chars().last().unwrap().to_digit(10).unwrap() as u8;
        let calculated = Self::calculate_checksum(&line[..68]);
        
        if expected != calculated {
            return Err(TleError::ChecksumMismatch { expected, actual: calculated });
        }
        Ok(())
    }

    fn calculate_checksum(line: &str) -> u8 {
        line.chars()
            .map(|c| match c {
                '0'..='9' => c.to_digit(10).unwrap() as u8,
                '-' => 1,
                _ => 0,
            })
            .sum::<u8>() % 10
    }

    fn parse_int(s: &str, field: &str) -> Result<i32, TleError> {
        s.trim().parse()
            .map_err(|_| TleError::ParseError(format!("Invalid {}: '{}'", field, s)))
    }

    fn parse_float(s: &str, field: &str) -> Result<f64, TleError> {
        s.trim().parse()
            .map_err(|_| TleError::ParseError(format!("Invalid {}: '{}'", field, s)))
    }

    fn parse_exponential(s: &str, field: &str) -> Result<f64, TleError> {
        let s = s.trim();
        if s.is_empty() || s == "0" {
            return Ok(0.0);
        }

        // Handle format like " 31780-3" (means 0.31780e-3)
        let sign = if s.starts_with('-') { -1.0 } else { 1.0 };
        let s = s.trim_start_matches(['+', '-']);
        
        if s.len() < 2 {
            return Err(TleError::ParseError(format!("Invalid {} format: '{}'", field, s)));
        }

        let mantissa_str = &s[..s.len()-2];
        let exponent_str = &s[s.len()-2..];
        
        let mantissa: f64 = format!("0.{}", mantissa_str)
            .parse()
            .map_err(|_| TleError::ParseError(format!("Invalid {} mantissa: '{}'", field, mantissa_str)))?;
            
        let exponent: i32 = exponent_str
            .parse()
            .map_err(|_| TleError::ParseError(format!("Invalid {} exponent: '{}'", field, exponent_str)))?;

        Ok(sign * mantissa * 10.0_f64.powi(exponent))
    }

    fn parse_eccentricity(s: &str) -> Result<f64, TleError> {
        let s = s.trim();
        let value: f64 = format!("0.{}", s)
            .parse()
            .map_err(|_| TleError::ParseError(format!("Invalid eccentricity: '{}'", s)))?;
        Ok(value)
    }

    fn parse_epoch(s: &str) -> Result<DateTime<Utc>, TleError> {
        let year_str = &s[0..2];
        let day_str = &s[2..];

        let year: i32 = year_str.parse()
            .map_err(|_| TleError::InvalidEpoch(format!("Invalid year: '{}'", year_str)))?;
        
        let full_year = if year < 57 { 2000 + year } else { 1900 + year };
        
        let day_of_year: f64 = day_str.parse()
            .map_err(|_| TleError::InvalidEpoch(format!("Invalid day of year: '{}'", day_str)))?;

        let base_date = Utc.with_ymd_and_hms(full_year, 1, 1, 0, 0, 0)
            .single()
            .ok_or_else(|| TleError::InvalidEpoch(format!("Invalid base year: {}", full_year)))?;

        let days_to_add = (day_of_year - 1.0).floor() as i64;
        let fractional_day = day_of_year - day_of_year.floor();
        let seconds_to_add = (fractional_day * 86400.0) as i64;

        let epoch = base_date 
            + chrono::Duration::days(days_to_add)
            + chrono::Duration::seconds(seconds_to_add);

        Ok(epoch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOTSAT1_TLE: &[&str] = &[
        "HOTSAT-1",
        "1 56954U 23084Y   25241.29664323  .00007163  00000+0  31780-3 0  9993",
        "2 56954  97.5868   7.4102 0005478 336.1866  23.9116 15.21880160122699"
    ];

    #[test]
    fn test_parse_hotsat1_tle() {
        let elements = TleParser::parse(HOTSAT1_TLE).unwrap();
        
        assert_eq!(elements.name, "HOTSAT-1");
        assert_eq!(elements.norad_id, 56954);
        assert_eq!(elements.international_designator, "23084Y");
        assert!((elements.inclination - 97.5868).abs() < 1e-6);
        assert!((elements.eccentricity - 0.0005478).abs() < 1e-7);
        assert!((elements.mean_motion - 15.21880160).abs() < 1e-8);
    }

    #[test]
    fn test_checksum_validation() {
        // Valid checksum
        assert!(TleParser::validate_checksum(HOTSAT1_TLE[1], 1).is_ok());
        assert!(TleParser::validate_checksum(HOTSAT1_TLE[2], 2).is_ok());
        
        // Invalid checksum
        let bad_line = "1 56954U 23084Y   25241.29664323  .00007163  00000+0  31780-3 0  9990";
        assert!(TleParser::validate_checksum(bad_line, 1).is_err());
    }

    #[test]
    fn test_invalid_tle_format() {
        let bad_tle = &["HOTSAT-1", "invalid line"];
        assert!(TleParser::parse(bad_tle).is_err());
    }

    #[test]
    fn test_calculate_checksum() {
        let line1 = "1 56954U 23084Y   25241.29664323  .00007163  00000+0  31780-3 0  999";
        assert_eq!(TleParser::calculate_checksum(line1), 3);
    }
}