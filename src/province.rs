// src/province.rs

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Province {
    Saskatchewan,
    Alberta,
}

impl Province {
    pub fn label(&self) -> &str {
        match self {
            Province::Saskatchewan => "Saskatchewan",
            Province::Alberta => "Alberta",
        }
    }

    /// The two-letter code used on addresses, e.g. "SK".
    pub fn code(&self) -> &str {
        match self {
            Province::Saskatchewan => "SK",
            Province::Alberta => "AB",
        }
    }

    pub fn gst_rate(&self) -> f64 {
        5.0
    }

    pub fn pst_rate(&self) -> Option<f64> {
        match self {
            Province::Saskatchewan => Some(6.0),
            Province::Alberta => None,
        }
    }
}
