use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use clap::Parser;
use colored::Colorize;
use mime::Mime;
use reqwest::{Client, Response, Url, header};
use std::time::Duration;
use std::{collections::HashMap, str::FromStr};
use syntect::parsing::SyntaxReference;
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, ThemeSet},
    parsing::SyntaxSet,
    util::{LinesWithEndings, as_24_bit_terminal_escaped},
};

/// A modern, user-friendly HTTP client written in Rust
///
/// Examples:
///   # GET request
///   httpie get https://httpbin.org/get
///
///   # POST with JSON body
///   httpie post https://httpbin.org/post name=alice age=30
///
///   # Custom headers
///   httpie get https://api.github.com/users/torvalds Authorization:"token YOUR_TOKEN"
///
///   # Upload file
///   httpie post https://httpbin.org/post photo@/path/to/image.jpg
///
///   # Download file
///   httpie get https://example.com/file.zip -d
#[derive(Parser, Debug)]
#[clap(version = "1.0", author = "Ethan Bai")]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
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
    fn method(&self) -> reqwest::Method {
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

    fn args(&self) -> &RequestArgs {
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
struct RequestArgs {
    /// Target URL
    #[arg(value_name = "URL", value_parser = parse_url)]
    url: String,

    /// Request items: headers (Key:Value), query params (key==value), body (key=value)
    #[arg(value_name = "REQUEST_ITEM", value_parser = parse_request_item)]
    items: Vec<RequestItem>,

    /// Authentication: username:password or token
    #[arg(short = 'a', long = "auth", value_parser = parse_auth)]
    auth: Option<Auth>,

    /// Verbose mode: print request details
    #[arg(short = 'v', long = "verbose")]
    verbose: bool,

    /// Request timeout in seconds
    #[arg(long = "timeout", default_value = "30")]
    timeout: u64,

    /// Follow redirects
    #[arg(short = 'F', long = "follow")]
    follow_redirects: bool,

    /// Maximum number of redirects
    #[arg(long = "max-redirects", default_value = "10")]
    max_redirects: usize,

    /// Print only response headers
    #[arg(long = "headers", conflicts_with = "body_only")]
    headers_only: bool,

    /// Print only response body
    #[arg(long = "body", conflicts_with = "headers_only")]
    body_only: bool,

    /// Download mode: save response body to a file
    #[arg(short = 'd', long = "download")]
    download: bool,

    /// Output file path
    #[arg(short = 'o', long = "output")]
    output: Option<String>,
}

impl RequestArgs {
    fn output_filter(&self) -> OutputFilter {
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
enum Auth {
    Basic {
        username: String,
        password: Option<String>,
    },
    Bearer(String),
}

impl FromStr for Auth {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        //check if Bearer Token format

        if s.starts_with("bearer:") || s.starts_with("Bearer:") {
            let token = s.split_once(':').map(|x| x.1).unwrap_or("");
            if token.is_empty() {
                return Err(anyhow!("Bearer token cannot be empty."));
            }
            return Ok(Auth::Bearer(token.to_string()));
        }

        if s.starts_with("ghp_")
            || s.starts_with("gho_")
            || s.starts_with("ghs_")
            || s.starts_with("ghu_")
            || s.starts_with("glpat-")
            || s.starts_with("sk_")
        {
            // Stripe
            return Ok(Auth::Bearer(s.to_string()));
        }

        let parts: Vec<&str> = s.splitn(2, ':').collect();

        match parts.len() {
            1 => {
                let username = parts[0].to_string();
                if username.is_empty() {
                    return Err(anyhow!("Username cannot be empty"));
                }
                Ok(Auth::Basic {
                    username,
                    password: None,
                })
            }
            2 => {
                let username = parts[0].to_string();
                let password = parts[1].to_string();
                if username.is_empty() {
                    return Err(anyhow!("Username cannot be empty"));
                }

                Ok(Auth::Basic {
                    username,
                    password: Some(password),
                })
            }
            _ => unreachable!("splitn(2) can only return 1 or 2 parts"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum RequestItem {
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
        //find operator is more efficient than split , it doesn't create iterator
        if let Some(pos) = s.find(':') {
            let key = s[..pos].trim().to_string();
            let value = s[pos + 1..].trim().to_string();
            if key.is_empty() {
                return Err(anyhow!("Header key cannot be empty: {}", s));
            }
            return Ok(RequestItem::Header(key, value));
        }

        if let Some(pos) = s.find('@') {
            let key = s[..pos].trim().to_string();
            let filepath = s[pos + 1..].trim().to_string();
            if key.is_empty() {
                return Err(anyhow!("Form file key cannot be empty: {}", s));
            }
            if filepath.is_empty() {
                return Err(anyhow!("File path cannot be empty: {}", s));
            }
            return Ok(RequestItem::FormFile(key, filepath));
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

#[derive(Debug, Clone, PartialEq)]
enum OutputFilter {
    All,
    HeadersOnly,
    BodyOnly,
}

#[derive(Debug, Default)]
struct VerboseInfo {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    query_params: Vec<(String, String)>,
    body: Option<String>,
    files: Vec<(String, String)>,
}

impl VerboseInfo {
    fn new(method: &str, url: &str) -> Self {
        Self {
            method: method.to_string(),
            url: url.to_string(),
            headers: Vec::new(),
            query_params: Vec::new(),
            body: None,
            files: Vec::new(),
        }
    }
    fn add_header(&mut self, key: String, value: String) {
        self.headers.push((key, value));
    }

    fn add_query_param(&mut self, key: String, value: String) {
        self.query_params.push((key, value));
    }

    fn set_body(&mut self, body: String) {
        self.body = Some(body);
    }

    fn add_file(&mut self, key: String, filepath: String) {
        self.files.push((key, filepath));
    }

    fn print(&self) {
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

        // ✨ 显示文件信息
        if !self.files.is_empty() {
            println!("{}", ">".cyan().bold());
            println!("{} {}", ">".cyan().bold(), "Files:".yellow());
            for (key, filepath) in &self.files {
                // 提取文件名
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

// ============================================================================
// HTTP 请求处理
// ============================================================================

async fn execute_request(cli: &Cli, client: &Client) -> Result<()> {
    let command = &cli.command;
    let args = command.args();
    let method = command.method();

    //TODO: Why we need clone reqwest_method
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
                // If it is GET/HEAD/OPTIONS，Warnings
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

    //TODO:

    let resp = match body_type {
        Some(BodyType::Multipart) => {
            // ✨ Multipart 表单（文件上传）
            use reqwest::multipart;

            let mut form = multipart::Form::new();

            // 添加文本字段
            for (key, value) in form_fields {
                form = form.text(key, value);
            }

            // 添加文件
            for (key, filepath) in files {
                let file_content = tokio::fs::read(&filepath)
                    .await
                    .map_err(|e| anyhow!("Failed to read file '{}': {}", filepath, e))?;

                // 猜测 MIME 类型
                let mime_type = mime_guess::from_path(&filepath)
                    .first_or_octet_stream()
                    .to_string();

                // 提取文件名
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
            // ✨ application/json
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
            // 无 body
            if let Some(info) = verbose_info {
                info.print();
            }

            req_builder.send().await?
        }
    };

    // handle download pattern
    if args.download || args.output.is_some() {
        let filename = determine_filename(args, &resp);
        return download_file(resp, &filename).await;
    }

    // print response
    print_resp(resp, args.output_filter()).await
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
// Request build helper function
// ============================================================================

fn apply_auth(
    builder: reqwest::RequestBuilder,
    auth: &Option<Auth>,
    verbose_info: &mut Option<VerboseInfo>,
) -> reqwest::RequestBuilder {
    match auth {
        Some(Auth::Basic { username, password }) => {
            if let Some(info) = verbose_info {
                let credentials = match password {
                    Some(pwd) => format!("{}:{}", username, pwd),
                    None => username.clone(),
                };
                let auth_value = format!("Basic {}", general_purpose::STANDARD.encode(credentials));
                info.add_header("Authorization".to_string(), auth_value);
            }

            builder.basic_auth(username, password.as_ref())
        }
        Some(Auth::Bearer(token)) => {
            let auth_value = format!("Bearer {}", token);

            if let Some(info) = verbose_info {
                info.add_header("Authorization".to_string(), auth_value.clone());
            }

            builder.header("Authorization", auth_value)
        }
        None => builder,
    }
}

// ============================================================================
// Output Response
// ============================================================================

fn print_status(resp: &Response) {
    let status = format!("{:?} {}", resp.version(), resp.status()).blue();
    println!("{}\n", status);
}

fn print_headers(resp: &Response) {
    for (name, value) in resp.headers() {
        println!("{}: {:?}", name.to_string().green(), value);
    }
    println!();
}
fn print_body(m: Option<Mime>, body: &str) {
    match m {
        Some(v) if v == mime::APPLICATION_JSON => print_syntect(body, "json"),
        Some(v) if v == mime::TEXT_HTML => print_syntect(body, "html"),
        _ => println!("{}", body),
    }
}

async fn print_resp(resp: Response, filter: OutputFilter) -> Result<()> {
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

fn get_content_type(resp: &Response) -> Option<Mime> {
    resp.headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
}

fn print_syntect(s: &str, ext: &str) {
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
// File download handle
// ============================================================================

fn extract_filename_from_url(url: &str) -> String {
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

fn extract_filename_from_header(resp: &Response) -> Option<String> {
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

fn determine_filename(args: &RequestArgs, resp: &Response) -> String {
    if let Some(ref output) = args.output {
        return output.clone();
    }
    if let Some(filename) = extract_filename_from_header(resp) {
        return filename;
    }
    extract_filename_from_url(&args.url)
}

async fn download_file(resp: Response, filename: &str) -> Result<()> {
    use futures_util::StreamExt;
    use indicatif::{ProgressBar, ProgressStyle};
    use tokio::io::AsyncWriteExt;

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
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{msg} {spinner} {bytes}")?,
        );
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
// Content-Type Inference
// ============================================================================
#[derive(Debug, Clone, Copy, PartialEq)]
enum BodyType {
    Json,
    Multipart,
}

fn detect_body_type(items: &[RequestItem]) -> Option<BodyType> {
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

// ============================================================================
// 主函数
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let args = cli.command.args();

    let mut headers = header::HeaderMap::new();
    headers.insert("X-POWERED-BY", "Rust".parse()?);

    let mut client_builder = Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(args.timeout));

    if args.follow_redirects {
        client_builder =
            client_builder.redirect(reqwest::redirect::Policy::limited(args.max_redirects));
    } else {
        client_builder = client_builder.redirect(reqwest::redirect::Policy::none());
    }

    let client = client_builder.build()?;

    if let Err(e) = execute_request(&cli, &client).await {
        eprintln!("\n{} {}\n", "Error:".red().bold(), e);

        let error_msg = e.to_string();

        if error_msg.contains("dns error") || error_msg.contains("failed to lookup") {
            eprintln!("{}", "💡 Possible causes:".yellow());
            eprintln!("   - Check if the domain name is correct");
            eprintln!("   - Check your network connection");
            eprintln!("   - Try using IP address instead");
        } else if error_msg.contains("timed out") {
            eprintln!("{}", "💡 Suggestion:".yellow());
            eprintln!("   - Increase timeout with --timeout <seconds>");
            eprintln!("   - Check if the server is responsive");
        } else if error_msg.contains("connection refused") {
            eprintln!("{}", "💡 Possible causes:".yellow());
            eprintln!("   - Server is not running");
            eprintln!("   - Wrong port number");
            eprintln!("   - Firewall blocking the connection");
        } else if error_msg.contains("No such file") {
            eprintln!("{}", "💡 File not found:".yellow());
            eprintln!("   - Check if the file path is correct");
            eprintln!("   - Use absolute path or relative to current directory");
        }
        std::process::exit(1);
    };

    Ok(())
}

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
    fn parse_auth_basic() {
        // Basic Auth: username:password
        assert_eq!(
            parse_auth("alice:secret123").unwrap(),
            Auth::Basic {
                username: "alice".into(),
                password: Some("secret123".into()),
            }
        );

        // Basic Auth: 只有 username
        assert_eq!(
            parse_auth("bob").unwrap(),
            Auth::Basic {
                username: "bob".into(),
                password: None,
            }
        );

        // 密码中包含冒号
        assert_eq!(
            parse_auth("user:pass:with:colons").unwrap(),
            Auth::Basic {
                username: "user".into(),
                password: Some("pass:with:colons".into()),
            }
        );
    }

    #[test]
    fn parse_auth_bearer() {
        // 显式 Bearer 格式
        assert_eq!(
            parse_auth("bearer:ghp_xxxxx").unwrap(),
            Auth::Bearer("ghp_xxxxx".into())
        );

        // 自动识别 GitHub token
        assert_eq!(
            parse_auth("ghp_1234567890abcdef").unwrap(),
            Auth::Bearer("ghp_1234567890abcdef".into())
        );

        // 自动识别 GitLab token
        assert_eq!(
            parse_auth("glpat-xxxxx").unwrap(),
            Auth::Bearer("glpat-xxxxx".into())
        );

        // 自动识别 Stripe token
        assert_eq!(
            parse_auth("sk_test_xxxxx").unwrap(),
            Auth::Bearer("sk_test_xxxxx".into())
        );
    }

    #[test]
    fn parse_auth_errors() {
        // 空 username
        assert!(parse_auth("").is_err());
        assert!(parse_auth(":password").is_err());

        // 空 Bearer token
        assert!(parse_auth("bearer:").is_err());
    }
}
