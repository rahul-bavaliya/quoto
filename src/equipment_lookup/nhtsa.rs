// src/equipment_lookup/nhtsa.rs: road vehicles, looked up with the US government's free vPIC service

use super::{EquipmentLookup, normalize_serial};
use crate::equipment::{EquipmentDraft, EquipmentType};
use crate::errors::{QuoteError, QuoteResult};
use serde::Deserialize;
use std::time::Duration;
use tracing::debug;

use crate::equipment::NHTSA_VPIC as PROVIDER;
const BASE_URL: &str = "https://vpic.nhtsa.dot.gov/api/vehicles/DecodeVinValues";
const TIMEOUT: Duration = Duration::from_secs(5);
/// A decoded VIN is a few kilobytes; anything near this size is not an answer we want.
const MAX_ANSWER_BYTES: u64 = 1024 * 1024;

/// Looks up cars, trucks and trailers by their 17-character VIN with NHTSA's
/// free vPIC service (no key needed). Only the VIN leaves the computer:
/// nothing about the customer is sent.
///
/// It only knows road vehicles. A serial number that is not a VIN (a tractor's
/// product identification number, say) is answered with `Ok(None)` without
/// calling the service, so the next lookup in a chain, or the service writer, takes over.
pub struct NhtsaLookup {
    fetch: Fetcher,
}

/// Gets the text the service answers for a URL. Real by default; tests hand in their own.
type Fetcher = Box<dyn Fn(&str) -> QuoteResult<String>>;

impl NhtsaLookup {
    /// Talks to the real service over the internet.
    pub fn new() -> Self {
        NhtsaLookup {
            fetch: Box::new(fetch_over_http),
        }
    }

    /// Uses your own way of getting the answer, so tests never touch the internet.
    pub fn with_fetcher(fetch: impl Fn(&str) -> QuoteResult<String> + 'static) -> Self {
        NhtsaLookup {
            fetch: Box::new(fetch),
        }
    }
}

impl Default for NhtsaLookup {
    fn default() -> Self {
        NhtsaLookup::new()
    }
}

impl EquipmentLookup for NhtsaLookup {
    fn lookup(&self, serial: &str) -> QuoteResult<Option<EquipmentDraft>> {
        let vin = normalize_serial(serial);
        if !looks_like_a_vin(&vin) {
            return Ok(None);
        }
        // Only letters and digits get this far, so nothing can change the URL's shape.
        let url = format!("{BASE_URL}/{vin}?format=json");
        let answer = (self.fetch)(&url)?;
        read_answer(&answer)
    }
}

/// A VIN is exactly 17 letters and digits, and never uses I, O or Q.
fn looks_like_a_vin(serial: &str) -> bool {
    serial.len() == 17
        && serial
            .chars()
            .all(|c| c.is_ascii_alphanumeric() && !matches!(c, 'I' | 'O' | 'Q'))
}

/// The answer wraps one row per VIN in `Results`.
#[derive(Deserialize)]
struct Answer {
    #[serde(rename = "Results")]
    results: Vec<Decoded>,
}

/// The fields we use, out of about 140. NHTSA fills unknown ones with
/// blanks, so every field may be missing, blank or null.
#[derive(Deserialize)]
struct Decoded {
    #[serde(rename = "ErrorCode")]
    error_code: Option<String>,
    #[serde(rename = "Make")]
    make: Option<String>,
    #[serde(rename = "Model")]
    model: Option<String>,
    #[serde(rename = "ModelYear")]
    model_year: Option<String>,
    #[serde(rename = "VehicleType")]
    vehicle_type: Option<String>,
}

/// Turns NHTSA's text into a draft.
/// NHTSA answers HTTP 200 even for a bad VIN, so the `ErrorCode` in the text
/// decides: only "0" (decoded clean) is trusted. A wrong guess here would put
/// the wrong machine on a quote, so anything else counts as "not found"
/// and the writer types the details in.
fn read_answer(text: &str) -> QuoteResult<Option<EquipmentDraft>> {
    let answer: Answer = serde_json::from_str(text).map_err(|cause| {
        debug!(%cause, "NHTSA answer could not be read");
        unavailable()
    })?;
    let Some(decoded) = answer.results.into_iter().next() else {
        debug!("NHTSA answer had no results");
        return Err(unavailable());
    };

    if decoded.error_code.as_deref().map(str::trim) != Some("0") {
        return Ok(None);
    }
    let make = clean(decoded.make);
    let model = clean(decoded.model);
    if make.is_empty() || model.is_empty() {
        return Ok(None);
    }
    let model_year = clean(decoded.model_year).parse::<i64>().ok();
    let equipment_type = equipment_type_for(&clean(decoded.vehicle_type));
    Ok(Some(EquipmentDraft::new(
        PROVIDER,
        make,
        model,
        model_year,
        equipment_type,
    )))
}

fn clean(value: Option<String>) -> String {
    value.unwrap_or_default().trim().to_string()
}

/// NHTSA's `VehicleType` is words like "PASSENGER CAR" or "TRUCK". Anything
/// that doesn't clearly match one of our kinds is `Other`; the writer can correct it.
fn equipment_type_for(vehicle_type: &str) -> EquipmentType {
    let vehicle_type = vehicle_type.to_uppercase();
    if vehicle_type.contains("TRAILER") {
        EquipmentType::Trailer
    } else if vehicle_type.contains("TRUCK") {
        EquipmentType::Truck
    } else if vehicle_type.contains("PASSENGER CAR") || vehicle_type.contains("MULTIPURPOSE") {
        EquipmentType::Car
    } else {
        EquipmentType::Other
    }
}

fn unavailable() -> QuoteError {
    QuoteError::LookupFailed { provider: PROVIDER }
}

/// The real request: a plain GET with a time limit and a size limit.
/// A web address that answers with an error status counts as a failure too.
fn fetch_over_http(url: &str) -> QuoteResult<String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .build()
        .into();
    let mut response = agent.get(url).call().map_err(failed)?;
    response
        .body_mut()
        .with_config()
        .limit(MAX_ANSWER_BYTES)
        .read_to_string()
        .map_err(failed)
}

fn failed(cause: ureq::Error) -> QuoteError {
    debug!(%cause, "NHTSA request failed");
    unavailable()
}
