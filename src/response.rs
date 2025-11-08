use anyhow::Result;
use colored::Colorize;
use mime::Mime;
use reqwest::{header, Response};
use syntect::parsing::SyntaxReference;
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, ThemeSet},
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};

use crate::cli::OutputFilter;

pub fn print_status(resp: &Response) {
    let status = format!("{:?} {}", resp.version(), resp.status()).blue();
    println!("{}\n", status);
}

pub fn print_headers(resp: &Response) {
    for (name, value) in resp.headers() {
        println!("{}: {:?}", name.to_string().green(), value);
    }
    println!();
}

pub fn print_body(m: Option<Mime>, body: &str) {
    match m {
        Some(v) if v == mime::APPLICATION_JSON => print_syntect(body, "json"),
        Some(v) if v == mime::TEXT_HTML => print_syntect(body, "html"),
        _ => println!("{}", body),
    }
}

pub async fn print_resp(resp: Response, filter: OutputFilter) -> Result<()> {
    match filter {
        OutputFilter::All => {
            print_status(&resp);
            print_headers(&resp);
            let mime = get_content_type(&resp);
            let body = resp.text().await?;
            print_body(mime, &body);
        }
        OutputFilter::HeadersOnly => {
            print_status(&resp);
            print_headers(&resp);
            //don't print body, but need consume response
            let _ = resp.text().await?;
        }
        OutputFilter::BodyOnly => {
            let mime = get_content_type(&resp);
            let body = resp.text().await?;
            print_body(mime, &body);
        }
    }
    Ok(())
}

pub fn get_content_type(resp: &Response) -> Option<Mime> {
    resp.headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
}

pub fn print_syntect(s: &str, ext: &str) {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax: &SyntaxReference = ps
        .find_syntax_by_extension(ext)
        .unwrap_or_else(|| ps.find_syntax_plain_text()); // fallback if not found

    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

    for line in LinesWithEndings::from(s) {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
        print!("{}", escaped);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_syntect_json() {
        let json = r#"{"name": "test", "value": 123}"#;
        // Just ensure it doesn't panic
        print_syntect(json, "json");
    }

    #[test]
    fn test_print_syntect_html() {
        let html = r#"<html><body><h1>Test</h1></body></html>"#;
        // Just ensure it doesn't panic
        print_syntect(html, "html");
    }

    #[test]
    fn test_print_syntect_unknown() {
        let text = "plain text";
        // Should fallback to plain text
        print_syntect(text, "unknown_extension");
    }
}
