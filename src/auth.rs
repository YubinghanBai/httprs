use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use std::str::FromStr;

use crate::request::VerboseInfo;

#[derive(Debug, Clone, PartialEq)]
pub enum Auth {
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

pub fn apply_auth(
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
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_auth_basic() {
        // Basic Auth: username:password
        assert_eq!(
            "alice:secret123".parse::<Auth>().unwrap(),
            Auth::Basic {
                username: "alice".into(),
                password: Some("secret123".into()),
            }
        );

        // Basic Auth: only username
        assert_eq!(
            "bob".parse::<Auth>().unwrap(),
            Auth::Basic {
                username: "bob".into(),
                password: None,
            }
        );

        // Password with colons
        assert_eq!(
            "user:pass:with:colons".parse::<Auth>().unwrap(),
            Auth::Basic {
                username: "user".into(),
                password: Some("pass:with:colons".into()),
            }
        );
    }

    #[test]
    fn parse_auth_bearer() {
        // Explicit Bearer format
        assert_eq!(
            "bearer:ghp_xxxxx".parse::<Auth>().unwrap(),
            Auth::Bearer("ghp_xxxxx".into())
        );

        // Auto-detect GitHub token
        assert_eq!(
            "ghp_1234567890abcdef".parse::<Auth>().unwrap(),
            Auth::Bearer("ghp_1234567890abcdef".into())
        );

        // Auto-detect GitLab token
        assert_eq!(
            "glpat-xxxxx".parse::<Auth>().unwrap(),
            Auth::Bearer("glpat-xxxxx".into())
        );

        // Auto-detect Stripe token
        assert_eq!(
            "sk_test_xxxxx".parse::<Auth>().unwrap(),
            Auth::Bearer("sk_test_xxxxx".into())
        );
    }

    #[test]
    fn parse_auth_errors() {
        // Empty username
        assert!("".parse::<Auth>().is_err());
        assert!(":password".parse::<Auth>().is_err());

        // Empty Bearer token
        assert!("bearer:".parse::<Auth>().is_err());
    }

    #[test]
    fn auth_basic_without_password() {
        let auth = "admin".parse::<Auth>().unwrap();
        assert_eq!(
            auth,
            Auth::Basic {
                username: "admin".into(),
                password: None
            }
        );
    }

    #[test]
    fn auth_github_tokens() {
        let tokens = vec![
            "ghp_test123",
            "gho_test456",
            "ghs_test789",
            "ghu_testabc",
        ];

        for token in tokens {
            let auth = token.parse::<Auth>().unwrap();
            assert_eq!(auth, Auth::Bearer(token.to_string()));
        }
    }

    #[test]
    fn auth_bearer_explicit_lowercase() {
        let auth = "bearer:test_token_123".parse::<Auth>().unwrap();
        assert_eq!(auth, Auth::Bearer("test_token_123".to_string()));
    }

    #[test]
    fn auth_bearer_explicit_uppercase() {
        let auth = "Bearer:TEST_TOKEN".parse::<Auth>().unwrap();
        assert_eq!(auth, Auth::Bearer("TEST_TOKEN".to_string()));
    }

    #[test]
    fn auth_basic_with_special_chars() {
        let auth = "user@example.com:p@ssw0rd!".parse::<Auth>().unwrap();
        assert_eq!(
            auth,
            Auth::Basic {
                username: "user@example.com".to_string(),
                password: Some("p@ssw0rd!".to_string())
            }
        );
    }

    #[test]
    fn auth_equality() {
        let auth1 = Auth::Bearer("token123".to_string());
        let auth2 = Auth::Bearer("token123".to_string());
        let auth3 = Auth::Bearer("different".to_string());

        assert_eq!(auth1, auth2);
        assert_ne!(auth1, auth3);

        let basic1 = Auth::Basic {
            username: "user".to_string(),
            password: Some("pass".to_string()),
        };
        let basic2 = Auth::Basic {
            username: "user".to_string(),
            password: Some("pass".to_string()),
        };

        assert_eq!(basic1, basic2);
        assert_ne!(auth1, basic1);
    }

    #[test]
    fn auth_clone() {
        let auth = Auth::Bearer("token".to_string());
        let cloned = auth.clone();
        assert_eq!(auth, cloned);
    }
}
