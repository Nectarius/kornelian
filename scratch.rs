use genpdf::{elements::Paragraph, Document, fonts::{FontFamily, FontData}};

fn main() {
    let font_bytes = include_bytes!("src/Roboto-Regular.ttf").to_vec();
    let font = FontFamily {
        regular: FontData::new(font_bytes.clone(), None).unwrap(),
        bold: FontData::new(font_bytes.clone(), None).unwrap(),
        italic: FontData::new(font_bytes.clone(), None).unwrap(),
        bold_italic: FontData::new(font_bytes.clone(), None).unwrap(),
    };
    let mut doc = Document::new(font);
    doc.push(Paragraph::new("Hello World"));
    doc.render_to_file("test.pdf").unwrap();
}
