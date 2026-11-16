// src/quote/line_kind.rs

#[derive(Debug, PartialEq)]
pub enum LineKind {
    Labour,
    Part,
    Fee,
    Credit,
}

impl LineKind {
    pub fn label(&self) -> &str {
        match self {
            LineKind::Labour => "LABOUR",
            LineKind::Part => "PART",
            LineKind::Fee => "FEE",
            LineKind::Credit => "CREDIT",
        }
    }
}
