use anyhow::Result;
use colored::Colorize;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{header, Response, Url};
use tokio::io::AsyncWriteExt;

use crate::cli::RequestArgs;

pub fn extract_filename_from_url(url: &str) -> String {
    let parsed = Url::parse(url).ok();
    if let Some(url) = parsed {
        let path = url.path();
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        if let Some(last) = segments.last()
            && !last.is_empty()
        {
            return last.to_string();
        }
    }
    "index.html".to_string()
}

pub fn extract_filename_from_header(resp: &Response) -> Option<String> {
    let content_disposition = resp
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())?;

    for part in content_disposition.split(';') {
        let part = part.trim();

        if let Some(name) = part.strip_prefix("filename=") {
            let name = name.trim_matches(|c| c == '"' || c == '\'');
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
        if let Some(name) = part.strip_prefix("filename*=")
            && let Some(idx) = name.rfind("''")
        {
            let name = &name[idx + 2..];
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

pub fn determine_filename(args: &RequestArgs, resp: &Response) -> String {
    if let Some(ref output) = args.output {
        return output.clone();
    }
    if let Some(filename) = extract_filename_from_header(resp) {
        return filename;
    }
    extract_filename_from_url(&args.url)
}

pub async fn download_file(resp: Response, filename: &str) -> Result<()> {
    let total_size = resp.content_length();

    let mut file = tokio::fs::File::create(filename).await?;

    let pb = if let Some(size) = total_size {
        let pb = ProgressBar::new(size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{msg}\n{spinner:.green} [{elapsed_precise}]\
            [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
                )?
                .progress_chars("#>-"),
        );
        pb.set_message(format!("Downloading {}", filename.cyan()));
        pb
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(ProgressStyle::default_spinner().template("{msg} {spinner} {bytes}")?);
        pb.set_message(format!("Downloading {}", filename.cyan()));
        pb
    };

    let mut stream = resp.bytes_stream();
    let mut downloaded = 0u64;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message(format!("{} {}", "Downloaded".green(), filename));

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_filename_from_url() {
        assert_eq!(
            extract_filename_from_url("https://example.com/file.zip"),
            "file.zip"
        );
        assert_eq!(
            extract_filename_from_url("https://example.com/path/to/document.pdf"),
            "document.pdf"
        );
        assert_eq!(
            extract_filename_from_url("https://example.com/"),
            "index.html"
        );
        assert_eq!(
            extract_filename_from_url("https://example.com"),
            "index.html"
        );
    }

    #[test]
    fn test_extract_filename_from_url_with_query() {
        assert_eq!(
            extract_filename_from_url("https://example.com/file.zip?version=1"),
            "file.zip"
        );
    }

    #[test]
    fn test_extract_filename_invalid_url() {
        assert_eq!(extract_filename_from_url("not a url"), "index.html");
    }

    #[test]
    fn test_extract_filename_with_trailing_slash() {
        // Trailing slash after a path segment still extracts the segment name
        assert_eq!(
            extract_filename_from_url("https://example.com/path/"),
            "path"
        );

        // Root path with trailing slash returns index.html
        assert_eq!(
            extract_filename_from_url("https://example.com/"),
            "index.html"
        );
    }

    #[test]
    fn test_extract_filename_with_fragment() {
        assert_eq!(
            extract_filename_from_url("https://example.com/file.pdf#section1"),
            "file.pdf"
        );
    }

    #[test]
    fn test_extract_filename_nested_path() {
        assert_eq!(
            extract_filename_from_url("https://example.com/a/b/c/d/file.txt"),
            "file.txt"
        );
    }

    #[test]
    fn test_extract_filename_no_extension() {
        assert_eq!(
            extract_filename_from_url("https://example.com/downloads/myfile"),
            "myfile"
        );
    }
}
