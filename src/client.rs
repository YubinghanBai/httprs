use anyhow::Result;
use reqwest::{header, Client};
use std::time::Duration;

use crate::cli::RequestArgs;


pub fn build_client(args:&RequestArgs)->Result<Client>{
    let mut headers=header::HeaderMap::new();

    let user_agent=format!("httprs/{}", env!("CARGO_PKG_VERSION"));
    headers.insert(header::USER_AGENT,header::HeaderValue::from_str(&user_agent)?,);

    headers.insert("X-Powered-By",header::HeaderValue::from_static("Rust"),);

    let mut client_builder = Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(args.timeout));

    if args.follow_redirects{
        client_builder = client_builder.redirect(reqwest::redirect::Policy::limited(args.max_redirects));

    }else{
        client_builder=client_builder.redirect(reqwest::redirect::Policy::none());
    }
    Ok(client_builder.build()?)

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_client_basic() {
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

        let client = build_client(&args);
        assert!(client.is_ok());
    }

    #[test]
    fn test_build_client_with_redirects() {
        let args = RequestArgs {
            url: "http://example.com".to_string(),
            items: vec![],
            auth: None,
            verbose: false,
            timeout: 60,
            follow_redirects: true,
            max_redirects: 5,
            headers_only: false,
            body_only: false,
            download: false,
            output: None,
        };

        let client = build_client(&args);
        assert!(client.is_ok());
    }
}
