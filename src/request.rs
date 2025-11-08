use anyhow::{anyhow, Result};
use colored::Colorize;
use reqwest::{Client, Url};
use std::collections::HashMap;

use crate::auth::apply_auth;
use crate::cli::{Cli, RequestItem};
use crate::download::{determine_filename, download_file};
use crate::response::print_resp;
use crate::timing::RequestTimer;

#[derive(Debug, Default)]
pub struct VerboseInfo {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    query_params: Vec<(String, String)>,
    body: Option<String>,
    files: Vec<(String, String)>,
}

impl VerboseInfo {
    pub fn new(method: &str, url: &str) -> Self {
        Self {
            method: method.to_string(),
            url: url.to_string(),
            headers: Vec::new(),
            query_params: Vec::new(),
            body: None,
            files: Vec::new(),
        }
    }

    pub fn add_header(&mut self, key: String, value: String) {
        self.headers.push((key, value));
    }

    pub fn add_query_param(&mut self, key: String, value: String) {
        self.query_params.push((key, value));
    }

    pub fn set_body(&mut self, body: String) {
        self.body = Some(body);
    }

    pub fn add_file(&mut self, key: String, filepath: String) {
        self.files.push((key, filepath));
    }

    pub fn print(&self) {
        let full_url = if self.query_params.is_empty() {
            self.url.clone()
        } else {
            let query_string = self
                .query_params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            format!("{}?{}", self.url, query_string)
        };

        let parsed_url = Url::parse(&full_url).unwrap();
        let path = if parsed_url.query().is_some() {
            format!("{}?{}", parsed_url.path(), parsed_url.query().unwrap())
        } else {
            parsed_url.path().to_string()
        };

        println!(
            "{} {} {} {}",
            ">".cyan().bold(),
            self.method.cyan(),
            path.cyan(),
            "HTTP/1.1".cyan().dimmed()
        );

        if let Some(host) = parsed_url.host_str() {
            println!("{} {}: {}", ">".cyan().bold(), "Host".cyan(), host);
        }

        for (key, value) in &self.headers {
            if key.to_lowercase() == "authorization" {
                let masked_value = if value.len() > 20 {
                    format!("{}...{}", &value[..10], &value[value.len() - 5..])
                } else {
                    value.clone()
                };
                println!("{} {}: {}", ">".cyan().bold(), key.cyan(), masked_value);
            } else {
                println!("{} {}: {}", ">".cyan().bold(), key.cyan(), value);
            }
        }

        // Display file info
        if !self.files.is_empty() {
            println!("{}", ">".cyan().bold());
            println!("{} {}", ">".cyan().bold(), "Files:".yellow());
            for (key, filepath) in &self.files {
                // Extract filename
                let filename = std::path::Path::new(filepath)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(filepath);
                println!("{} {} @ {}", ">".cyan().bold(), key.cyan(), filename);
            }
        }

        println!("{}", ">".cyan().bold());

        if let Some(body) = &self.body {
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(body) {
                if let Ok(pretty_json) = serde_json::to_string_pretty(&json_value) {
                    for line in pretty_json.lines() {
                        println!("{} {}", ">".cyan().bold(), line.cyan());
                    }
                } else {
                    println!("{} {}", ">".cyan().bold(), body.cyan());
                }
            } else {
                println!("{} {}", ">".cyan().bold(), body.cyan());
            }
            println!();
        }

        println!();
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BodyType {
    Json,
    Multipart,
}

pub fn detect_body_type(items: &[RequestItem]) -> Option<BodyType> {
    let has_file = items
        .iter()
        .any(|item| matches!(item, RequestItem::FormFile(_, _)));
    let has_body = items
        .iter()
        .any(|item| matches!(item, RequestItem::Body(_, _)));

    if has_file {
        Some(BodyType::Multipart)
    } else if has_body {
        // Default to JSON for HTTPie compatibility
        Some(BodyType::Json)
    } else {
        None
    }
}

pub async fn execute_request(cli: &Cli, client: &Client) -> Result<()> {
    let command = &cli.command;
    let args = command.args();
    let method = command.method();

    let mut timer=if args.verbose{
        Some(RequestTimer::start())
    }else{
        None
    };

    let mut req_builder = client.request(method.clone(), &args.url);

    let mut verbose_info = if args.verbose {
        Some(VerboseInfo::new(method.as_str(), &args.url))
    } else {
        None
    };

    //apply auth
    req_builder = apply_auth(req_builder, &args.auth, &mut verbose_info);

    let mut body = HashMap::new();
    let mut form_fields = HashMap::new();
    let mut files: Vec<(String, String)> = Vec::new();
    let mut query_params: Vec<(String, String)> = Vec::new();

    for item in &args.items {
        match item {
            RequestItem::Header(key, value) => {
                req_builder = req_builder.header(key, value);
                if let Some(ref mut info) = verbose_info {
                    info.add_header(key.clone(), value.clone());
                }
            }
            RequestItem::QueryParam(key, value) => {
                query_params.push((key.clone(), value.clone()));
                if let Some(ref mut info) = verbose_info {
                    info.add_query_param(key.clone(), value.clone());
                }
            }
            RequestItem::Body(key, value) => {
                // If it is GET/HEAD/OPTIONS, Warnings
                if matches!(
                    method,
                    reqwest::Method::GET | reqwest::Method::HEAD | reqwest::Method::OPTIONS
                ) {
                    eprintln!(
                        "{}",
                        format!(
                            "⚠️  Warning: Ignoring body parameter '{}' in {} request",
                            key, method
                        )
                        .yellow()
                    );
                } else {
                    body.insert(key.clone(), value.clone());
                    form_fields.insert(key.clone(), value.clone());
                }
            }
            RequestItem::FormFile(key, filepath) => {
                files.push((key.clone(), filepath.clone()));
                if let Some(ref mut info) = verbose_info {
                    info.add_file(key.clone(), filepath.clone());
                }
            }
        }
    }

    // Add query params
    if !query_params.is_empty() {
        req_builder = req_builder.query(&query_params);
    }

    let body_type = detect_body_type(&args.items);

    if args.verbose {
        eprintln!("{} {:?}", "Detected body type:".yellow(), body_type);
        eprintln!(
            "{} {} files, {} body fields",
            "Request contains:".yellow(),
            files.len(),
            body.len()
        );
    }

    let resp = match body_type {
        Some(BodyType::Multipart) => {
            // Multipart form (file upload)
            use reqwest::multipart;

            let mut form = multipart::Form::new();

            // Add text fields
            for (key, value) in form_fields {
                form = form.text(key, value);
            }

            // Add files
            for (key, filepath) in files {
                let file_content = tokio::fs::read(&filepath)
                    .await
                    .map_err(|e| anyhow!("Failed to read file '{}': {}", filepath, e))?;

                // Guess MIME type
                let mime_type = mime_guess::from_path(&filepath)
                    .first_or_octet_stream()
                    .to_string();

                // Extract filename
                let filename = std::path::Path::new(&filepath)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file")
                    .to_string();

                let part = multipart::Part::bytes(file_content)
                    .file_name(filename)
                    .mime_str(&mime_type)?;

                form = form.part(key, part);
            }
            if let Some(ref mut info) = verbose_info {
                info.add_header(
                    "Content-Type".to_string(),
                    "multipart/form-data".to_string(),
                );
            }

            if let Some(info) = verbose_info {
                info.print();
            }

            req_builder.multipart(form).send().await?
        }

        Some(BodyType::Json) | None if !body.is_empty() => {
            // application/json
            let json_body = serde_json::to_string(&body)?;

            if let Some(ref mut info) = verbose_info {
                info.set_body(json_body.clone());
                info.add_header("Content-Type".to_string(), "application/json".to_string());
            }

            if let Some(info) = verbose_info {
                info.print();
            }

            req_builder.json(&body).send().await?
        }

        _ => {
            // Nobody
            if let Some(info) = verbose_info {
                info.print();
            }

            req_builder.send().await?
        }
    };

    if let Some(ref mut t)=timer{
        t.record_first_byte();
    }

    // handle download pattern
    if args.download || args.output.is_some() {
        let filename = determine_filename(args, &resp);
        let result =download_file(resp,&filename).await;

        if let Some(mut t) = timer {
            t.finish();
            t.print_summary();
        }

        return result;
    }

    // print response
    let result = print_resp(resp, args.output_filter()).await;
    if let Some(mut t) = timer {
        t.finish();
        t.print_summary();
    }
    result
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_body_type_json() {
        let items = vec![
            RequestItem::Body("name".to_string(), "alice".to_string()),
            RequestItem::Body("age".to_string(), "30".to_string()),
        ];
        assert_eq!(detect_body_type(&items), Some(BodyType::Json));
    }

    #[test]
    fn test_detect_body_type_multipart() {
        let items = vec![
            RequestItem::Body("title".to_string(), "test".to_string()),
            RequestItem::FormFile("file".to_string(), "/path/to/file".to_string()),
        ];
        assert_eq!(detect_body_type(&items), Some(BodyType::Multipart));
    }

    #[test]
    fn test_detect_body_type_none() {
        let items = vec![
            RequestItem::Header("Authorization".to_string(), "Bearer token".to_string()),
            RequestItem::QueryParam("page".to_string(), "1".to_string()),
        ];
        assert_eq!(detect_body_type(&items), None);
    }

    #[test]
    fn test_verbose_info_new() {
        let info = VerboseInfo::new("GET", "https://example.com");
        assert_eq!(info.method, "GET");
        assert_eq!(info.url, "https://example.com");
        assert!(info.headers.is_empty());
        assert!(info.query_params.is_empty());
        assert!(info.body.is_none());
        assert!(info.files.is_empty());
    }

    #[test]
    fn test_verbose_info_add_header() {
        let mut info = VerboseInfo::new("POST", "https://example.com");
        info.add_header("Content-Type".to_string(), "application/json".to_string());
        assert_eq!(info.headers.len(), 1);
        assert_eq!(
            info.headers[0],
            ("Content-Type".to_string(), "application/json".to_string())
        );
    }

    #[test]
    fn test_verbose_info_add_query_param() {
        let mut info = VerboseInfo::new("GET", "https://example.com");
        info.add_query_param("page".to_string(), "1".to_string());
        assert_eq!(info.query_params.len(), 1);
        assert_eq!(
            info.query_params[0],
            ("page".to_string(), "1".to_string())
        );
    }

    #[test]
    fn test_verbose_info_set_body() {
        let mut info = VerboseInfo::new("POST", "https://example.com");
        info.set_body(r#"{"name":"test"}"#.to_string());
        assert_eq!(info.body, Some(r#"{"name":"test"}"#.to_string()));
    }

    #[test]
    fn test_verbose_info_add_file() {
        let mut info = VerboseInfo::new("POST", "https://example.com");
        info.add_file("photo".to_string(), "/path/to/image.jpg".to_string());
        assert_eq!(info.files.len(), 1);
        assert_eq!(
            info.files[0],
            ("photo".to_string(), "/path/to/image.jpg".to_string())
        );
    }

    #[test]
    fn test_detect_body_type_mixed() {
        // Should prioritize multipart when both body and file present
        let items = vec![
            RequestItem::Body("title".to_string(), "test".to_string()),
            RequestItem::FormFile("file".to_string(), "/path/to/file".to_string()),
            RequestItem::Body("description".to_string(), "desc".to_string()),
        ];
        assert_eq!(detect_body_type(&items), Some(BodyType::Multipart));
    }

    #[test]
    fn test_detect_body_type_only_headers() {
        let items = vec![
            RequestItem::Header("Authorization".to_string(), "Bearer token".to_string()),
            RequestItem::Header("Accept".to_string(), "application/json".to_string()),
        ];
        assert_eq!(detect_body_type(&items), None);
    }

    #[test]
    fn test_detect_body_type_only_query_params() {
        let items = vec![
            RequestItem::QueryParam("page".to_string(), "1".to_string()),
            RequestItem::QueryParam("limit".to_string(), "10".to_string()),
        ];
        assert_eq!(detect_body_type(&items), None);
    }

    #[test]
    fn test_body_type_equality() {
        assert_eq!(BodyType::Json, BodyType::Json);
        assert_eq!(BodyType::Multipart, BodyType::Multipart);
        assert_ne!(BodyType::Json, BodyType::Multipart);
    }
}
