// src/province.rs

pub enum Province {
    Saskatchewan,
    Alberta,
}

impl Province {
    pub fn label(&self) -> &str {
        match self {
            Province::Saskatchewan => "Saskatchewan",
            Province::Alberta => "Alberta",``
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

#[cfg(test)] // only build this room when running `cargo test`
mod province_tests {
    use super::*; // bring in everything from the file above

    #[test]
    fn saskatchewan_has_correct_pst_rate() {
        let saskatchewan = Province::Saskatchewan;
        assert_eq!(saskatchewan.pst_rate(), Some(6.0));
    }

    #[test]
    fn alberta_has_correct_pst_rate() {
        let alberta = Province::Alberta;
        assert_eq!(alberta.pst_rate(), None);
    }

    #[test]
    fn alberta_has_correct_gst_rate() {
        let alberta = Province::Alberta;
        assert_eq!(alberta.gst_rate(), 5.0);
    }
}
