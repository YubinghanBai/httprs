use anyhow::{anyhow, Result};
use clap::Parser;
use reqwest::Url;
use std::str::FromStr;

use crate::auth::Auth;

/// A modern, user-friendly HTTP client written in Rust
///
/// Examples:
///   # GET request
///   httprs get https://httpbin.org/get
///
///   # POST with JSON body
///   httprs post https://httpbin.org/post name=alice age=30
///
///   # Custom headers
///   httprs get https://api.github.com/users/torvalds Authorization:"token YOUR_TOKEN"
///
///   # Upload file
///   httprs post https://httpbin.org/post photo@/path/to/image.jpg
///
///   # Download file
///   httprs get https://example.com/file.zip -d
#[derive(Parser, Debug)]
#[clap(version = "1.0", author = "Ethan Bai")]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Parser, Debug)]
pub enum Command {
    /// Make a GET request
    Get(RequestArgs),

    /// Make a POST request
    Post(RequestArgs),

    /// Make a PUT request
    Put(RequestArgs),

    /// Make a PATCH request
    Patch(RequestArgs),

    /// Make a DELETE request
    Delete(RequestArgs),

    /// Make a HEAD request
    Head(RequestArgs),

    /// Make an OPTIONS request
    Options(RequestArgs),
}

impl Command {
    pub fn method(&self) -> reqwest::Method {
        match self {
            Command::Get(_) => reqwest::Method::GET,
            Command::Post(_) => reqwest::Method::POST,
            Command::Put(_) => reqwest::Method::PUT,
            Command::Patch(_) => reqwest::Method::PATCH,
            Command::Delete(_) => reqwest::Method::DELETE,
            Command::Head(_) => reqwest::Method::HEAD,
            Command::Options(_) => reqwest::Method::OPTIONS,
        }
    }

    pub fn args(&self) -> &RequestArgs {
        match self {
            Command::Get(args) => args,
            Command::Post(args) => args,
            Command::Put(args) => args,
            Command::Patch(args) => args,
            Command::Delete(args) => args,
            Command::Head(args) => args,
            Command::Options(args) => args,
        }
    }
}

#[derive(Parser, Debug, Clone)]
pub struct RequestArgs {
    /// Target URL
    #[arg(value_name = "URL", value_parser = parse_url)]
    pub url: String,

    /// Request items: headers (Key:Value), query params (key==value), body (key=value)
    #[arg(value_name = "REQUEST_ITEM", value_parser = parse_request_item)]
    pub items: Vec<RequestItem>,

    /// Authentication: username:password or token
    #[arg(short = 'a', long = "auth", value_parser = parse_auth)]
    pub auth: Option<Auth>,

    /// Verbose mode: print request details
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Request timeout in seconds
    #[arg(long = "timeout", default_value = "30")]
    pub timeout: u64,

    /// Follow redirects
    #[arg(short = 'F', long = "follow")]
    pub follow_redirects: bool,

    /// Maximum number of redirects
    #[arg(long = "max-redirects", default_value = "10")]
    pub max_redirects: usize,

    /// Print only response headers
    #[arg(long = "headers", conflicts_with = "body_only")]
    pub headers_only: bool,

    /// Print only response body
    #[arg(long = "body", conflicts_with = "headers_only")]
    pub body_only: bool,

    /// Download mode: save response body to a file
    #[arg(short = 'd', long = "download")]
    pub download: bool,

    /// Output file path
    #[arg(short = 'o', long = "output")]
    pub output: Option<String>,
}

impl RequestArgs {
    pub fn output_filter(&self) -> OutputFilter {
        if self.headers_only {
            OutputFilter::HeadersOnly
        } else if self.body_only {
            OutputFilter::BodyOnly
        } else {
            OutputFilter::All
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OutputFilter {
    All,
    HeadersOnly,
    BodyOnly,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RequestItem {
    //HTTP Header: "Authorization:Bearer token"
    Header(String, String),
    //Query Parameter: "page==1"
    QueryParam(String, String),
    //JSON Body field: "name=alice"
    Body(String, String),
    //file upload: key@filepath
    FormFile(String, String),
}

impl FromStr for RequestItem {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {

        if let Some(pos) = s.find('@') {
            let before_at=&s[..pos];
            if !before_at.contains('=')&&!before_at.contains(':'){
                let key=before_at.trim().to_string();
                let filepath = s[pos + 1..].trim().to_string();
                if key.is_empty() {
                    return Err(anyhow!("Form file key cannot be empty: {}", s));
                }
                if filepath.is_empty() {
                    return Err(anyhow!("File path cannot be empty: {}", s));
                }
                return Ok(RequestItem::FormFile(key, filepath));
            }
        }

        //find operator is more efficient than split , it doesn't create iterator
        if let Some(pos) = s.find(':') {
            let key = s[..pos].trim().to_string();
            let value = s[pos + 1..].trim().to_string();
            if key.is_empty() {
                return Err(anyhow!("Header key cannot be empty: {}", s));
            }
            return Ok(RequestItem::Header(key, value));
        }


        //Longest Match First
        if let Some(pos) = s.find("==") {
            let key = s[..pos].trim().to_string();
            let value = s[pos + 2..].trim().to_string();
            if key.is_empty() {
                return Err(anyhow!("Query parameter key cannot be empty: {}", s));
            }
            return Ok(RequestItem::QueryParam(key, value));
        }

        if let Some(pos) = s.find('=') {
            let key = s[..pos].trim().to_string();
            let value = s[pos + 1..].trim().to_string();
            if key.is_empty() {
                return Err(anyhow!("Body key cannot be empty: {}", s));
            }
            return Ok(RequestItem::Body(key, value));
        }
        Err(anyhow!(
            "Invalid format: '{}'. Expected 'Header:Value','key@file', 'key==value', or 'key=value'",
            s
        ))
    }
}

// ============================================================================
// Parse Function
// ============================================================================

fn parse_url(s: &str) -> Result<String> {
    let _url: Url = s.parse()?;
    Ok(s.into())
}

fn parse_request_item(s: &str) -> Result<RequestItem> {
    s.parse()
}

fn parse_auth(s: &str) -> Result<Auth> {
    s.parse()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_url_works() {
        assert!(parse_url("abc").is_err());
        assert!(parse_url("http://abc.xyz").is_ok());
        assert!(parse_url("https://httpbin.org/post").is_ok());
    }

    #[test]
    fn parse_request_item_works() {
        assert_eq!(
            parse_request_item("Authorization:Bearer token").unwrap(),
            RequestItem::Header("Authorization".into(), "Bearer token".into())
        );

        assert_eq!(
            parse_request_item("name=alice").unwrap(),
            RequestItem::Body("name".into(), "alice".into())
        );

        assert_eq!(
            parse_request_item("page==1").unwrap(),
            RequestItem::QueryParam("page".into(), "1".into())
        );

        assert_eq!(
            parse_request_item("search==hello world").unwrap(),
            RequestItem::QueryParam("search".into(), "hello world".into())
        );

        assert_eq!(
            parse_request_item("token=abc=def").unwrap(),
            RequestItem::Body("token".into(), "abc=def".into())
        );

        assert_eq!(
            parse_request_item("formula==a==b").unwrap(),
            RequestItem::QueryParam("formula".into(), "a==b".into())
        );

        assert!(parse_request_item("invalid").is_err());
        assert!(parse_request_item(":no-key").is_err());
        assert!(parse_request_item("=no-key").is_err());
        assert!(parse_request_item("==no-key").is_err());
    }

    #[test]
    fn parse_query_param_edge_cases() {
        assert_eq!(
            parse_request_item("page==").unwrap(),
            RequestItem::QueryParam("page".into(), "".into())
        );

        assert_eq!(
            parse_request_item("query==hello&world").unwrap(),
            RequestItem::QueryParam("query".into(), "hello&world".into())
        );

        assert_eq!(
            parse_request_item("city==北京").unwrap(),
            RequestItem::QueryParam("city".into(), "北京".into())
        );
    }


    #[test]
    fn output_filter_works() {
        let args = RequestArgs {
            url: "http://example.com".to_string(),
            items: vec![],
            auth: None,
            verbose: false,
            timeout: 30,
            follow_redirects: false,
            max_redirects: 10,
            headers_only: true,
            body_only: false,
            download: false,
            output: None,
        };

        assert_eq!(args.output_filter(), OutputFilter::HeadersOnly);
    }

    #[test]
    fn output_filter_body_only() {
        let args = RequestArgs {
            url: "http://example.com".to_string(),
            items: vec![],
            auth: None,
            verbose: false,
            timeout: 30,
            follow_redirects: false,
            max_redirects: 10,
            headers_only: false,
            body_only: true,
            download: false,
            output: None,
        };

        assert_eq!(args.output_filter(), OutputFilter::BodyOnly);
    }

    #[test]
    fn output_filter_all() {
        let args = RequestArgs {
            url: "http://example.com".to_string(),
            items: vec![],
            auth: None,
            verbose: false,
            timeout: 30,
            follow_redirects: false,
            max_redirects: 10,
            headers_only: false,
            body_only: false,
            download: false,
            output: None,
        };

        assert_eq!(args.output_filter(), OutputFilter::All);
    }

    #[test]
    fn command_method_works() {
        let get_cmd = Command::Get(RequestArgs {
            url: "http://example.com".to_string(),
            items: vec![],
            auth: None,
            verbose: false,
            timeout: 30,
            follow_redirects: false,
            max_redirects: 10,
            headers_only: false,
            body_only: false,
            download: false,
            output: None,
        });

        assert_eq!(get_cmd.method(), reqwest::Method::GET);
    }

    #[test]
    fn parse_header_with_spaces() {
        assert_eq!(
            parse_request_item("  Content-Type  :  application/json  ").unwrap(),
            RequestItem::Header("Content-Type".into(), "application/json".into())
        );
    }

    #[test]
    fn parse_body_with_equals() {
        // Body value contains single equals sign (not double)
        assert_eq!(
            parse_request_item("encoded=abc=def").unwrap(),
            RequestItem::Body("encoded".into(), "abc=def".into())
        );
    }

    #[test]
    fn parse_empty_values() {
        assert_eq!(
            parse_request_item("key=").unwrap(),
            RequestItem::Body("key".into(), "".into())
        );

        assert_eq!(
            parse_request_item("Header:").unwrap(),
            RequestItem::Header("Header".into(), "".into())
        );
    }
    #[test]
    fn parse_email_address_in_body() {
        // ✅ email 地址中的 @ 应该被保留
        assert_eq!(

            parse_request_item("email=test@example.com").unwrap(),
            RequestItem::Body("email".into(),
                              "test@example.com".into())
        );

        assert_eq!(

            parse_request_item("contact=user@domain.org").unwrap(),
            RequestItem::Body("contact".into(),
                              "user@domain.org".into())
        );
    }

    #[test]
    fn parse_email_in_query_param() {
        // ✅ 查询参数中的 @ 也应该被保留
        assert_eq!(

            parse_request_item("email==admin@test.com").unwrap(),
            RequestItem::QueryParam("email".into(),
                                    "admin@test.com".into())
        );
    }

    #[test]
    fn parse_file_upload() {
        // ✅ 真正的文件上传：key@filepath
        assert_eq!(

            parse_request_item("photo@/path/to/image.jpg").unwrap(),
            RequestItem::FormFile("photo".into(),
                                  "/path/to/image.jpg".into())
        );

        assert_eq!(

            parse_request_item("document@../files/report.pdf").unwrap(),
            RequestItem::FormFile("document".into(),
                                  "../files/report.pdf".into())
        );
    }

    #[test]
    fn parse_file_upload_with_special_chars() {
        // ✅ 文件路径中可能包含特殊字符
        assert_eq!(

            parse_request_item("file@/home/user/file@backup.txt").unwrap(),
            RequestItem::FormFile("file".into(),
                                  "/home/user/file@backup.txt".into())
        );
    }

    #[test]
    fn parse_value_with_multiple_at_signs() {
        // ✅ 值中可能有多个 @
        assert_eq!(

            parse_request_item("mentions=@user1,@user2,@user3").unwrap(),
            RequestItem::Body("mentions".into(),
                              "@user1,@user2,@user3".into())
        );
    }

    #[test]
    fn parse_complex_email_scenarios() {
        // ✅ 各种复杂的 email 场景
        assert_eq!(
            parse_request_item("from=John Doe
  <john@example.com>").unwrap(),
            RequestItem::Body("from".into(), "John Doe
  <john@example.com>".into())
        );
    }

    #[test]
    fn parse_errors_on_invalid_file() {
        // ❌ 无效的文件上传格式
        assert!(parse_request_item("@no-key").is_err());
        assert!(parse_request_item("key@").is_err());
    }

}
