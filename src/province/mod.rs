// src/province/mod.rs

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

#[cfg(test)]
mod tests;
