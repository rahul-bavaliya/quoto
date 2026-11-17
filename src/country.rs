// src/country.rs: the countries Quoto works in

/// The country is never typed in: it follows from the province (see `Province::country`),
/// so a branch can't say "Alberta, Germany". More countries arrive with their provinces or states.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Country {
    Canada,
}

impl Country {
    pub fn label(&self) -> &str {
        match self {
            Country::Canada => "Canada",
        }
    }

    /// The two-letter code (ISO 3166), e.g. "CA".
    pub fn code(&self) -> &str {
        match self {
            Country::Canada => "CA",
        }
    }
}
