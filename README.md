# httprs 🦀

> A blazing fast HTTP client for the command line, written in Rust.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

Inspired by [HTTPie](https://httpie.io/), reimagined with Rust's speed and safety.

## ✨ Highlights

- 🚀 **Blazing Fast** - Built with Rust for maximum performance
- 🎨 **Syntax Highlighting** - Beautiful JSON and HTML output
- 📦 **Zero Config** - Works out of the box
- 🔐 **Auth Support** - Basic Auth and Bearer tokens
- 📤 **File Uploads** - Multipart form data made easy
- 📥 **Downloads** - Stream large files with progress
- 🌈 **Colorful** - Easy-to-read colored output

## 📦 Installation

### From source

```bash
git clone https://github.com/YubinghanBai/httprs.git
cd httprs
cargo build --release
cp target/release/httprs /usr/local/bin/
```

### Using Cargo

```bash
cargo install --path .
```

## 🚀 Quick Start

```bash
# Simple GET request
httprs get https://httpbin.org/get

# POST with JSON
httprs post https://httpbin.org/post name=alice email=alice@example.com

# Custom headers
httprs get https://api.github.com/users/torvalds \
    Authorization:"Bearer YOUR_TOKEN"

# File upload
httprs post https://httpbin.org/post photo@image.jpg description="My photo"

# Download file
httprs get https://httpbin.org/image/png --download
```

## 📖 Usage

### Basic Syntax

```
httprs <METHOD> <URL> [ITEMS...] [OPTIONS]
```

### Request Items

| Syntax | Type | Example |
|--------|------|---------|
| `Header:Value` | HTTP Header | `Authorization:"Bearer token"` |
| `key==value` | Query Parameter | `page==1 limit==20` |
| `key=value` | JSON Body | `name=alice age=30` |
| `key@file` | File Upload | `photo@/path/to/image.jpg` |

### HTTP Methods

Supported methods: `get`, `post`, `put`, `patch`, `delete`, `head`, `options`

### Authentication

```bash
# Basic Auth
httprs get https://api.example.com -a username:password

# Bearer Token (auto-detected)
httprs get https://api.example.com -a ghp_your_github_token

# Custom header
httprs get https://api.example.com Authorization:"Bearer YOUR_TOKEN"
```

### Query Parameters

```bash
# Automatically URL-encoded
httprs get https://api.example.com/search \
    q=="rust programming" \
    sort==stars \
    page==1
```

### Request Body

```bash
# JSON (default for key=value)
httprs post https://httpbin.org/post \
    name=alice \
    email=alice@example.com \
    age=30

# Result: {"name": "alice", "email": "alice@example.com", "age": "30"}
```

### File Upload

```bash
# Single file
httprs post https://httpbin.org/post \
    file@document.pdf

# Multiple files with form data
httprs post https://httpbin.org/post \
    title="My Upload" \
    photo@image1.jpg \
    document@file.pdf
```

### File Download

```bash
# Auto-detect filename
httprs get https://example.com/file.zip -d

# Specify output filename
httprs get https://httpbin.org/json -o data.json

# With progress (for large files)
httprs get https://example.com/large-file.zip --download
```

### Verbose Mode

```bash
# See request headers and body
httprs post https://httpbin.org/post name=test -v

# Output:
# > POST /post HTTP/1.1
# > Host: httpbin.org
# > Content-Type: application/json
# >
# > {"name": "test"}
```

### Options

```bash
# Timeout (default: 30s)
httprs get https://slow-api.com --timeout 10

# Follow redirects
httprs get http://github.com -F

# Max redirects
httprs get http://example.com -F --max-redirects 5

# Output filtering
httprs get https://httpbin.org/get --headers  # Only response headers
httprs get https://httpbin.org/get --body     # Only response body
```

## 🎯 Examples

### GitHub API

```bash
# Get user info
httprs get https://api.github.com/users/torvalds

# Create a gist (requires token)
httprs post https://api.github.com/gists \
    -a ghp_YOUR_TOKEN \
    description="My gist" \
    public=true
```

### REST API Testing

```bash
# GET with query params
httprs get https://api.example.com/users \
    role==admin \
    active==true

# POST with JSON
httprs post https://api.example.com/users \
    Authorization:"Bearer TOKEN" \
    name=alice \
    email=alice@example.com \
    role=admin

# PUT to update
httprs put https://api.example.com/users/123 \
    Authorization:"Bearer TOKEN" \
    name="Alice Smith" \
    active=true

# DELETE
httprs delete https://api.example.com/users/123 \
    Authorization:"Bearer TOKEN"
```

### File Operations

```bash
# Upload image
httprs post https://api.example.com/upload \
    Authorization:"Bearer TOKEN" \
    title="Sunset Photo" \
    tags="nature,photography" \
    image@sunset.jpg

# Download and save
httprs get https://api.example.com/reports/2024.pdf \
    Authorization:"Bearer TOKEN" \
    -o report.pdf
```

## 🆚 Comparison with HTTPie

| Feature | httprs | HTTPie |
|---------|--------|--------|
| Language | Rust 🦀 | Python 🐍 |
| Startup Time | < 10ms | ~100ms |
| Binary Size | ~5 MB | ~50 MB |
| Memory Usage | Low | Medium |
| Performance | ⚡ Very Fast | Fast |
| Syntax Highlighting | ✅ | ✅ |
| File Upload | ✅ | ✅ |
| File Download | ✅ | ✅ |
| Authentication | ✅ | ✅ |
| Sessions | ❌ | ✅ |
| Plugins | ❌ | ✅ |

## 🛠️ Development

```bash
# Clone repository
git clone https://github.com/YubinghanBai/httprs.git
cd httprs

# Run tests
cargo test

# Build
cargo build --release

# Run locally
cargo run -- get https://httpbin.org/get

# Format code
cargo fmt

# Lint
cargo clippy
```

## 🏗️ Built With

- [clap](https://github.com/clap-rs/clap) - Command line argument parser
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP client
- [tokio](https://tokio.rs/) - Async runtime
- [syntect](https://github.com/trishume/syntect) - Syntax highlighting
- [colored](https://github.com/mackwic/colored) - Terminal colors
- [serde_json](https://github.com/serde-rs/json) - JSON serialization

## 🤝 Contributing

Contributions are welcome! Here's how you can help:

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Please make sure to:
- Update tests as appropriate
- Follow the existing code style
- Update documentation

## 📋 TODO

- [ ] Add session/cookie persistence
- [ ] Support for `.netrc` authentication
- [ ] Custom color themes
- [ ] Configuration file support
- [ ] Plugin system
- [ ] Windows support improvements

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by [HTTPie](https://httpie.io/) by Jakub Roztočil
- Thanks to all [contributors](https://github.com/YubinghanBai/httprs/graphs/contributors)

## 📮 Author

**Yubinghan Bai**
- GitHub: [@YubinghanBai](https://github.com/YubinghanBai)

---

⭐ If you find this project useful, please consider giving it a star!

Made with ❤️ and Rust 🦀
