// src/quote/pdf.rs: a quote as a PDF file (the text is `print.rs`; the math is `totals.rs`)
//
// A quote is only text and a few lines, so this writes the PDF itself instead of pulling in a big
// library. It uses the two standard fonts every PDF reader already has (Helvetica and
// Helvetica Bold), so nothing is embedded and the file stays small. The same quote on the same
// day always gives the same bytes (no creation time inside).
//
// Limits, by design: the standard fonts hold Latin-1 only (English and French letters), so any
// other character is shown as `?`. Letter paper, one column of lines, any number of pages.

use super::Quote;
use crate::money::format_money;
use chrono::NaiveDate;

const HELVETICA: [u16; 224] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667,
    611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500,
    222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 278, 333, 556, 556, 556, 556, 260, 556, 333, 737, 370, 556, 584, 333, 737, 333, 400, 584,
    333, 333, 333, 556, 537, 278, 333, 333, 365, 556, 834, 834, 834, 611, 667, 667, 667, 667, 667,
    667, 1000, 722, 667, 667, 667, 667, 278, 278, 278, 278, 722, 722, 778, 778, 778, 778, 778, 584,
    778, 722, 722, 722, 722, 667, 667, 611, 556, 556, 556, 556, 556, 556, 889, 500, 556, 556, 556,
    556, 278, 278, 278, 278, 556, 556, 556, 556, 556, 556, 556, 584, 611, 556, 556, 556, 556, 500,
    556, 500,
];

const HELVETICA_BOLD: [u16; 224] = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611, 975, 722, 722, 722, 722, 667,
    611, 778, 722, 278, 556, 722, 611, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 333, 278, 333, 584, 556, 333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556,
    278, 889, 611, 611, 611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 278, 333, 556, 556, 556, 556, 280, 556, 333, 737, 370, 556, 584, 333, 737, 333, 400, 584,
    333, 333, 333, 611, 556, 278, 333, 333, 365, 556, 834, 834, 834, 611, 722, 722, 722, 722, 722,
    722, 1000, 722, 667, 667, 667, 667, 278, 278, 278, 278, 722, 722, 778, 778, 778, 778, 778, 584,
    778, 722, 722, 722, 722, 667, 667, 611, 556, 556, 556, 556, 556, 556, 889, 556, 556, 556, 556,
    556, 278, 278, 278, 278, 611, 611, 611, 611, 611, 611, 611, 584, 611, 611, 611, 611, 611, 556,
    611, 556,
];

const PAGE_WIDTH: f64 = 612.0; // Letter, in points (1/72 inch)
const PAGE_HEIGHT: f64 = 792.0;
const MARGIN: f64 = 50.0;
const LEFT: f64 = MARGIN;
const RIGHT: f64 = PAGE_WIDTH - MARGIN;
const BOTTOM: f64 = 70.0; // nothing but the footer goes below this
const MIDDLE: f64 = 312.0; // where the second column of a two-column block starts

// The lines table: where each column sits.
const KIND_X: f64 = LEFT;
const DESCRIPTION_X: f64 = 112.0;
const DESCRIPTION_WIDTH: f64 = 200.0;
const QUANTITY_RIGHT: f64 = 372.0;
const UNIT_RIGHT: f64 = 452.0;
const TOTAL_RIGHT: f64 = RIGHT;

const BODY: f64 = 9.0;
const LEADING: f64 = 12.0; // the height of one line of body text

#[derive(Clone, Copy, PartialEq)]
enum Face {
    Regular,
    Bold,
}

impl Face {
    fn resource(self) -> &'static str {
        match self {
            Face::Regular => "F1",
            Face::Bold => "F2",
        }
    }
}

/// The byte a character is written as, or `?` if the standard fonts cannot show it.
fn byte_of(c: char) -> u8 {
    match c as u32 {
        0x20..=0x7E | 0xA0..=0xFF => c as u32 as u8,
        _ => b'?',
    }
}

fn bytes_of(text: &str) -> Vec<u8> {
    text.chars().map(byte_of).collect()
}

fn width_of(text: &str, face: Face, size: f64) -> f64 {
    let table = match face {
        Face::Regular => &HELVETICA,
        Face::Bold => &HELVETICA_BOLD,
    };
    let thousandths: u32 = bytes_of(text)
        .iter()
        .map(|&byte| u32::from(table[usize::from(byte) - 0x20]))
        .sum();
    f64::from(thousandths) * size / 1000.0
}

/// A PDF string in brackets: `\`, `(` and `)` are escaped, and every byte above 127 is written
/// as an octal escape so the file itself is plain ASCII.
fn pdf_string(text: &str) -> String {
    let mut out = String::from("(");
    for byte in bytes_of(text) {
        match byte {
            b'\\' | b'(' | b')' => {
                out.push('\\');
                out.push(char::from(byte));
            }
            0x20..=0x7E => out.push(char::from(byte)),
            _ => out.push_str(&format!("\\{byte:03o}")),
        }
    }
    out.push(')');
    out
}

/// Cuts text into lines no wider than `width`: at spaces when it can, in the middle of a word
/// when one word alone is wider than a line.
fn wrap(text: &str, face: Face, size: f64, width: f64) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let mut word = word.to_string();
        loop {
            let candidate = if current.is_empty() {
                word.clone()
            } else {
                format!("{current} {word}")
            };
            if width_of(&candidate, face, size) <= width {
                current = candidate;
                break;
            }
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                continue; // try the word again on a fresh line
            }
            // one word wider than a whole line: cut it where it stops fitting
            let mut cut = 0;
            for (index, c) in word.char_indices() {
                if width_of(&word[..index + c.len_utf8()], face, size) > width {
                    break;
                }
                cut = index + c.len_utf8();
            }
            if cut == 0 {
                cut = word.chars().next().map_or(0, char::len_utf8); // a line too narrow for a letter
            }
            lines.push(word[..cut].to_string());
            word = word[cut..].to_string();
            if word.is_empty() {
                break;
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// The pages being drawn. `y` is the next free height on the last page (it counts up from the
/// bottom edge, as PDF does).
struct Document {
    pages: Vec<String>,
    y: f64,
    in_table: bool,
}

impl Document {
    fn new() -> Self {
        Document {
            pages: vec![String::new()],
            y: PAGE_HEIGHT - MARGIN,
            in_table: false,
        }
    }

    fn page(&mut self) -> &mut String {
        self.pages.last_mut().expect("there is always one page")
    }

    fn text(&mut self, x: f64, y: f64, text: &str, face: Face, size: f64) {
        let op = format!(
            "BT /{} {size} Tf {x:.2} {y:.2} Td {} Tj ET\n",
            face.resource(),
            pdf_string(text)
        );
        self.page().push_str(&op);
    }

    fn text_right(&mut self, right: f64, y: f64, text: &str, face: Face, size: f64) {
        let x = right - width_of(text, face, size);
        self.text(x, y, text, face, size);
    }

    fn line(&mut self, x1: f64, x2: f64, y: f64, thickness: f64) {
        let op = format!("{thickness} w {x1:.2} {y:.2} m {x2:.2} {y:.2} l S\n");
        self.page().push_str(&op);
    }

    fn shade(&mut self, x: f64, y: f64, width: f64, height: f64, gray: f64) {
        let op = format!("{gray} g {x:.2} {y:.2} {width:.2} {height:.2} re f 0 g\n");
        self.page().push_str(&op);
    }

    /// Makes room for `height` more points; starts a new page if the last one is too full.
    fn room_for(&mut self, height: f64) {
        if self.y - height < BOTTOM {
            self.pages.push(String::new());
            self.y = PAGE_HEIGHT - MARGIN;
            if self.in_table {
                self.table_head();
            }
        }
    }

    fn table_head(&mut self) {
        self.shade(LEFT, self.y - 4.0, RIGHT - LEFT, 16.0, 0.9);
        let y = self.y;
        self.text(KIND_X + 4.0, y, "Type", Face::Bold, BODY);
        self.text(DESCRIPTION_X, y, "Description", Face::Bold, BODY);
        self.text_right(QUANTITY_RIGHT, y, "Qty", Face::Bold, BODY);
        self.text_right(UNIT_RIGHT, y, "Unit price", Face::Bold, BODY);
        self.text_right(TOTAL_RIGHT - 4.0, y, "Total", Face::Bold, BODY);
        self.y -= 20.0;
    }

    /// Draws `lines` one under the other from `(x, y)`; returns the height they took.
    fn block(&mut self, x: f64, mut y: f64, lines: &[(String, Face)]) -> f64 {
        let start = y;
        for (line, face) in lines {
            self.text(x, y, line, *face, BODY);
            y -= LEADING;
        }
        start - y
    }
}

impl Quote {
    /// The quote as a PDF file. `today` only decides whether the "EXPIRED" mark is shown (the
    /// quote never reads the clock itself).
    pub fn to_pdf(&self, today: NaiveDate) -> Vec<u8> {
        let mut doc = Document::new();
        self.draw_head(&mut doc, today);
        self.draw_parties(&mut doc);
        self.draw_lines(&mut doc);
        self.draw_totals(&mut doc);
        assemble(&doc.pages, &self.number.to_string())
    }

    fn draw_head(&self, doc: &mut Document, today: NaiveDate) {
        let top = doc.y;
        doc.text(LEFT, top - 14.0, "QUOTE", Face::Bold, 22.0);
        doc.text_right(RIGHT, top - 8.0, &self.number.to_string(), Face::Bold, 14.0);
        let mut mark = self.status.label().to_uppercase();
        if self.is_expired(today) {
            mark.push_str(" - EXPIRED");
        }
        doc.text_right(RIGHT, top - 24.0, &mark, Face::Bold, 10.0);
        doc.y = top - 40.0;
        doc.line(LEFT, RIGHT, doc.y, 1.0);
        doc.y -= 16.0;

        // The branch on the left, the dates on the right.
        let branch = &self.branch;
        let left = vec![
            (branch.name().to_string(), Face::Bold),
            (branch.address().to_string(), Face::Regular),
            (format!("Phone: {}", branch.phone()), Face::Regular),
        ];
        let promise = match self.promise_date {
            Some(date) => date.to_string(),
            None => String::from("to be confirmed"),
        };
        let right = [
            (
                format!("Date: {}", self.created_at.format("%Y-%m-%d")),
                Face::Regular,
            ),
            (format!("Valid until: {}", self.expires_on), Face::Regular),
            (format!("Promise date: {promise}"), Face::Regular),
        ];
        let y = doc.y;
        let used = doc.block(LEFT, y, &wrapped(&left, MIDDLE - LEFT - 12.0));
        let used_right = doc.block(MIDDLE, y, &right);
        doc.y = y - used.max(used_right) - 10.0;
    }

    fn draw_parties(&self, doc: &mut Document) {
        let y = doc.y;
        let mut customer = vec![(String::from("Prepared for"), Face::Bold)];
        match &self.customer {
            Some(person) => {
                customer.push((person.name().to_string(), Face::Regular));
                if let Some(company) = person.company() {
                    customer.push((company.to_string(), Face::Regular));
                }
                customer.push((person.address().to_string(), Face::Regular));
                customer.push((
                    format!("Phone: {}", person.contact().phone()),
                    Face::Regular,
                ));
                if let Some(email) = person.contact().email() {
                    customer.push((format!("Email: {email}"), Face::Regular));
                }
            }
            None => customer.push((String::from("to be added"), Face::Regular)),
        }
        let mut machine = vec![(String::from("Equipment"), Face::Bold)];
        match &self.equipment {
            Some(unit) => {
                machine.extend(
                    unit.to_string()
                        .lines()
                        .map(|line| (line.to_string(), Face::Regular)),
                );
            }
            None => machine.push((String::from("to be added"), Face::Regular)),
        }
        let left = doc.block(LEFT, y, &wrapped(&customer, MIDDLE - LEFT - 12.0));
        let right = doc.block(MIDDLE, y, &wrapped(&machine, RIGHT - MIDDLE));
        doc.y = y - left.max(right) - 6.0;
        doc.line(LEFT, RIGHT, doc.y, 0.5);
        doc.y -= 20.0;
    }

    fn draw_lines(&self, doc: &mut Document) {
        doc.in_table = true;
        doc.table_head();
        if self.items.is_empty() {
            doc.text(DESCRIPTION_X, doc.y, "No lines yet.", Face::Regular, BODY);
            doc.y -= LEADING;
        }
        for item in &self.items {
            let description = wrap(item.description(), Face::Regular, BODY, DESCRIPTION_WIDTH);
            let height = LEADING * description.len().max(1) as f64 + 4.0;
            doc.room_for(height);
            let y = doc.y;
            doc.text(KIND_X + 4.0, y, item.kind().label(), Face::Regular, BODY);
            for (index, part) in description.iter().enumerate() {
                doc.text(
                    DESCRIPTION_X,
                    y - LEADING * index as f64,
                    part,
                    Face::Regular,
                    BODY,
                );
            }
            doc.text_right(
                QUANTITY_RIGHT,
                y,
                &item.quantity().to_string(),
                Face::Regular,
                BODY,
            );
            doc.text_right(
                UNIT_RIGHT,
                y,
                &format_money(item.signed_unit_price_cents()),
                Face::Regular,
                BODY,
            );
            doc.text_right(
                TOTAL_RIGHT - 4.0,
                y,
                &format_money(item.line_total()),
                Face::Regular,
                BODY,
            );
            doc.y -= height;
            doc.line(LEFT, RIGHT, doc.y + 10.5, 0.25); // a hairline between rows
        }
        doc.in_table = false;
        doc.y -= 6.0;
    }

    fn draw_totals(&self, doc: &mut Document) {
        let gst = format!("GST {}%", self.province().gst_rate());
        let mut rows: Vec<(String, i64, Face)> = vec![
            (String::from("Labour"), self.labour_total(), Face::Regular),
            (String::from("Parts"), self.parts_total(), Face::Regular),
            (String::from("Fees"), self.fees_total(), Face::Regular),
            (String::from("Credits"), self.credit_total(), Face::Regular),
            (String::from("Subtotal"), self.subtotal(), Face::Bold),
            (gst, self.gst_amount(), Face::Regular),
        ];
        if let Some(rate) = self.province().pst_rate() {
            rows.push((format!("PST {rate}%"), self.pst_amount(), Face::Regular));
        }
        rows.push((String::from("Total"), self.total(), Face::Bold));
        // The totals stay together on one page.
        doc.room_for(LEADING * rows.len() as f64 + 20.0);
        let label_x = 380.0;
        for (label, cents, face) in rows {
            let y = doc.y;
            if label == "Total" {
                doc.line(label_x, RIGHT, y + LEADING - 3.0, 0.75);
                doc.text(label_x, y - 2.0, &label, Face::Bold, 11.0);
                doc.text_right(
                    TOTAL_RIGHT - 4.0,
                    y - 2.0,
                    &format_money(cents),
                    Face::Bold,
                    11.0,
                );
                doc.y -= LEADING + 2.0;
            } else {
                doc.text(label_x, y, &label, face, BODY);
                doc.text_right(TOTAL_RIGHT - 4.0, y, &format_money(cents), face, BODY);
                doc.y -= LEADING;
            }
        }
        doc.y -= 12.0;
        let note = format!(
            "Prices are good until {}. Taxes follow the province of the branch ({}).",
            self.expires_on,
            self.province().label()
        );
        for part in wrap(&note, Face::Regular, 8.0, RIGHT - LEFT) {
            doc.room_for(LEADING);
            let y = doc.y;
            doc.text(LEFT, y, &part, Face::Regular, 8.0);
            doc.y -= LEADING;
        }
    }
}

/// Wraps each `(text, face)` line to `width`, keeping its face.
fn wrapped(lines: &[(String, Face)], width: f64) -> Vec<(String, Face)> {
    lines
        .iter()
        .flat_map(|(text, face)| {
            wrap(text, *face, BODY, width)
                .into_iter()
                .map(|part| (part, *face))
        })
        .collect()
}

/// Puts the pages into a PDF file: the fixed objects (catalog, page list, the two fonts, the
/// title), then a page and its content for each page, the cross-reference table, and the trailer.
fn assemble(pages: &[String], title: &str) -> Vec<u8> {
    let count = pages.len();
    let mut objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        {
            let kids: Vec<String> = (0..count).map(|i| format!("{} 0 R", 6 + 2 * i)).collect();
            format!(
                "<< /Type /Pages /Kids [{}] /Count {count} /MediaBox [0 0 {PAGE_WIDTH} {PAGE_HEIGHT}] \
                 /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> >>",
                kids.join(" ")
            )
            .into_bytes()
        },
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>"
            .to_vec(),
        format!(
            "<< /Title {} /Creator (Quoto) /Producer (Quoto) >>",
            pdf_string(title)
        )
        .into_bytes(),
    ];
    for (index, content) in pages.iter().enumerate() {
        let footer = format!(
            "BT /F1 8 Tf {LEFT:.2} 40 Td {} Tj ET\n\
             BT /F1 8 Tf {:.2} 40 Td {} Tj ET\n",
            pdf_string(&format!("Quote {title}")),
            RIGHT
                - width_of(
                    &format!("Page {} of {count}", index + 1),
                    Face::Regular,
                    8.0
                ),
            pdf_string(&format!("Page {} of {count}", index + 1)),
        );
        let stream = format!("{content}{footer}");
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /Contents {} 0 R >>",
                7 + 2 * index
            )
            .into_bytes(),
        );
        let mut body = format!("<< /Length {} >>\nstream\n", stream.len()).into_bytes();
        body.extend_from_slice(stream.as_bytes());
        body.extend_from_slice(b"endstream");
        objects.push(body);
    }

    let mut file = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(file.len());
        file.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        file.extend_from_slice(body);
        file.extend_from_slice(b"\nendobj\n");
    }
    let xref = file.len();
    file.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    file.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R /Info 5 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    file
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_match_helvetica() {
        // 1000 units = the font size; "A" is 667, a space 278, "i" 222 in Helvetica
        assert!((width_of("A", Face::Regular, 10.0) - 6.67).abs() < 1e-9);
        assert!((width_of(" ", Face::Regular, 10.0) - 2.78).abs() < 1e-9);
        assert!((width_of("i", Face::Regular, 10.0) - 2.22).abs() < 1e-9);
        // bold is wider
        assert!(width_of("Total", Face::Bold, 10.0) > width_of("Total", Face::Regular, 10.0));
        // anything the font cannot show is measured as the `?` that is written
        assert_eq!(
            width_of("日", Face::Regular, 10.0),
            width_of("?", Face::Regular, 10.0)
        );
    }

    #[test]
    fn wrapping_never_goes_past_the_width_and_loses_nothing() {
        let text =
            "Replace the left front hydraulic hose, the clamp and the o-ring, then test under load";
        for width in [60.0, 120.0, 200.0, 500.0] {
            let lines = wrap(text, Face::Regular, 9.0, width);
            assert!(
                lines
                    .iter()
                    .all(|l| width_of(l, Face::Regular, 9.0) <= width),
                "{width}"
            );
            assert_eq!(lines.join(" "), text, "{width}");
        }
        // a word wider than a line is cut, and every letter is kept
        let word = "W".repeat(60);
        let lines = wrap(&word, Face::Regular, 9.0, 100.0);
        assert!(lines.len() > 1);
        assert!(
            lines
                .iter()
                .all(|l| width_of(l, Face::Regular, 9.0) <= 100.0)
        );
        assert_eq!(lines.concat(), word);
        // nothing to wrap, and a width too small for any letter does not loop forever
        assert!(wrap("   ", Face::Regular, 9.0, 100.0).is_empty());
        assert_eq!(wrap("abc", Face::Regular, 9.0, 1.0).concat(), "abc");
    }

    #[test]
    fn a_pdf_string_is_plain_ascii_with_brackets_and_backslashes_escaped() {
        assert_eq!(pdf_string("a (b) \\ c"), "(a \\(b\\) \\\\ c)");
        assert_eq!(pdf_string("é"), "(\\351)");
        assert_eq!(pdf_string("日本"), "(??)");
        assert!(pdf_string("tab\tnew\nline").is_ascii());
    }
}
