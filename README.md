# robots-server
A high-performance gRPC server for fetching, caching, and querying robots.txt files with RFC 9309 compliance.
# Features
- gRPC API: Fast, type-safe API using Tonic and Protocol Buffers
- Smart Caching: Built-in caching with Moka for high-performance repeated queries
- RFC 9309 Compliant: Full support for robots.txt parsing including wildcards, end-of-path anchors ($), and longest-match semantics
- Permission Checking: Check if a specific user-agent is allowed to crawl a URL
- Tracing: Comprehensive logging with the tracing crate
- Streaming Support: Efficient handling of large robots.txt files (up to 550KB) with proper truncation
- Redirect Following: Follows up to 5 redirects per RFC 9309
- Forked Parser: Uses a custom fork of robotstxt-rs for proper consecutive user-agent handling
Architecture
```
┌─────────────────┐     gRPC      ┌──────────────────┐
│   Client        │◄─────────────►│  robots-server   │
│  (grpcurl, etc) │               │                  │
└─────────────────┘               │  ┌────────────┐  │
                                  │  │   Cache    │  │
                                  │  │   (Moka)   │  │
                                  │  └─────┬──────┘  │
                                  │        │         │
                                  │  ┌─────┴──────┐  │
                                  │  │  Fetcher   │  │
                                  │  │ (reqwest)  │  │
                                  │  └─────┬──────┘  │
                                  │        │         │
                                  │  ┌─────┴──────┐  │
                                  │  │   Parser   │  │
                                  │  │(robotstxt- │  │
                                  │  │    rs)     │  │
                                  │  └────────────┘  │
                                  └──────────────────┘
```
# Quick Start
Building
```
# Clone the repository
git clone <repository-url>
cd robots-server
# Build the project
cargo build --release
# Or build in debug mode for development
cargo build
Running the Server
# Run the server (defaults to [::1]:50051)
cargo run --bin robots-server
# With custom log level
RUST_LOG=info cargo run --bin robots-server
# Debug logging
RUST_LOG=debug cargo run --bin robots-server
Using grpcurl
# Get robots.txt for a domain

grpcurl -plaintext \
  -d '{"url": "https://example.com"}' \
  -import-path ./proto \
  -proto robots.proto \
  localhost:50051 \
  robots.RobotsService/GetRobotsTxt
# Check if crawling is allowed
grpcurl -plaintext \
  -d '{"target_url": "https://example.com/page", "user_agent": "MyBot/1.0"}' \
  -import-path ./proto \
  -proto robots.proto \
  localhost:50051 \
  robots.RobotsService/IsAllowed
```
# API Reference
Services
 RobotsService
 GetRobotsTxt(GetRobotsRequest) -> GetRobotsResponse
Fetches and returns the parsed robots.txt for a given URL.
```
message GetRobotsRequest {
  string url = 1;  // Target URL (e.g., "https://example.com")
}
message GetRobotsResponse {
  string target_url = 1;
  string robots_txt_url = 2;
  AccessResult access_result = 3;
  uint32 http_status_code = 4;
  repeated Group groups = 5;
  repeated string sitemaps = 6;
  uint64 content_length_bytes = 7;
  bool truncated = 8;
}
IsAllowed(IsAllowedRequest) -> IsAllowedResponse
Checks if a specific user-agent is allowed to crawl a target URL.
message IsAllowedRequest {
  string target_url = 1;  // URL to check (e.g., "https://example.com/page")
  string user_agent = 2;  // User-agent string (e.g., "MyBot/1.0")
}
message IsAllowedResponse {
  bool allowed = 1;  // true = allowed, false = blocked
}
```
# Configuration
Environment Variables
- RUST_LOG: Set logging level (e.g., info, debug, trace)
Caching
The server uses Moka cache with a 24-hour TTL for all robots.txt entries. This ensures:
- Fast repeated queries
- Reduced network load
- RFC 9309 compliant freshness
# Testing
# Run all tests
cargo test
# Run specific test
cargo test test_is_allowed
# Run tests with output
cargo test -- --nocapture
# Run integration tests only
cargo test --test service_integration_tests
# Project Structure
```
robots-server/
├── Cargo.toml              # Project configuration
├── Cargo.lock              # Dependency lock file
├── build.rs                # Build script for protobuf
├── proto/                  # Protocol Buffer definitions
│   └── robots.proto        # gRPC service definitions
├── src/
│   ├── main.rs             # Server entry point
│   ├── lib.rs              # Library exports
│   ├── service.rs          # gRPC service implementation
│   ├── fetcher.rs          # HTTP fetching logic
│   ├── robots_data.rs      # Data structures and conversions
│   ├── cache.rs            # Caching trait and implementation
│   └── client.rs           # Example client
├── tests/                  # Integration tests
│   ├── service_integration_tests.rs
│   ├── fetcher_tests.rs
│   ├── cache_tests.rs
│   └── robots_url_tests.rs
└── AGENTS.md               # Guidelines for AI agents
```
# RFC 9309 Compliance
This implementation follows RFC 9309 (Robots Exclusion Protocol):
- Case-insensitive user-agent matching: Googlebot matches googlebot
- Longest match wins: /admin/private beats /admin
- Allow wins ties: When allow and disallow have equal length
- End-of-path anchor ($): /secret$ matches only exact /secret
- Wildcard (*): Matches any sequence of characters
- Empty disallow: Treated as "allow all" (empty patterns ignored)
- Query strings: Included in path matching per RFC 9309
Performance
- Concurrent: Async/await throughout with Tokio
- Cached: Sub-millisecond responses for cached entries
- Streaming: Efficient handling of large files (up to 550KB)
- Pooled: HTTP connection pooling via reqwest

# Acknowledgments
- Uses a forked version of robotstxt-rs (https://github.com/ChosunOne/robots-txt) for proper consecutive user-agent handling
- Built with Tonic (https://github.com/hyperium/tonic) for gRPC
- Caching powered by Moka (https://github.com/moka-rs/moka)
---
