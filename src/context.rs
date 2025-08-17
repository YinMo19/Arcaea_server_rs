//! Client context module for extracting client information
//!
//! This module provides the `ClientContext` request guard which extracts
//! client information such as IP address, headers, and cookies from incoming requests.

use rocket::outcome::Outcome;
use rocket::request::{self, FromRequest, Request};
use std::collections::HashMap;

/// Client context containing information about the requesting client
#[derive(Debug, Clone)]
pub struct ClientContext {
    /// Client IP address
    pub ip: Option<String>,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Cookies from the request
    pub cookies: HashMap<String, String>,
    /// User agent string
    pub user_agent: Option<String>,
    /// Forwarded for header (for proxied requests)
    pub forwarded_for: Option<String>,
    /// Real IP header (often set by reverse proxies)
    pub real_ip: Option<String>,
}

impl ClientContext {
    /// Create a new empty client context
    pub fn new() -> Self {
        Self {
            ip: None,
            headers: HashMap::new(),
            cookies: HashMap::new(),
            user_agent: None,
            forwarded_for: None,
            real_ip: None,
        }
    }

    /// Get the best available IP address for the client
    /// Priority: X-Real-IP > X-Forwarded-For > Remote Address
    pub fn get_client_ip(&self) -> Option<String> {
        // First try X-Real-IP header (most reliable for proxied requests)
        if let Some(real_ip) = &self.real_ip {
            if !real_ip.is_empty() {
                return Some(real_ip.clone());
            }
        }

        // Then try X-Forwarded-For header (take the first IP)
        if let Some(forwarded) = &self.forwarded_for {
            if !forwarded.is_empty() {
                // X-Forwarded-For can contain multiple IPs, take the first one
                let first_ip = forwarded.split(',').next().unwrap_or("").trim();
                if !first_ip.is_empty() {
                    return Some(first_ip.to_string());
                }
            }
        }

        // Finally fall back to the direct connection IP
        self.ip.clone()
    }

    /// Get a header value by name (case-insensitive)
    pub fn get_header(&self, name: &str) -> Option<&String> {
        let name_lower = name.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == name_lower)
            .map(|(_, v)| v)
    }

    /// Get a cookie value by name
    pub fn get_cookie(&self, name: &str) -> Option<&String> {
        self.cookies.get(name)
    }

    /// Check if the request came from a local address
    pub fn is_local_request(&self) -> bool {
        match self.get_client_ip() {
            Some(ip) => {
                ip == "127.0.0.1"
                    || ip == "::1"
                    || ip.starts_with("192.168.")
                    || ip.starts_with("10.")
                    || ip.starts_with("172.")
                    || ip == "localhost"
            }
            None => false,
        }
    }

    /// Get device ID from headers or cookies
    pub fn get_device_id(&self) -> Option<String> {
        // Try to get device ID from various sources
        if let Some(device_id) = self.get_header("X-Device-ID") {
            return Some(device_id.clone());
        }

        if let Some(device_id) = self.get_header("Device-ID") {
            return Some(device_id.clone());
        }

        if let Some(device_id) = self.get_cookie("device_id") {
            return Some(device_id.clone());
        }

        None
    }
}

impl Default for ClientContext {
    fn default() -> Self {
        Self::new()
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ClientContext {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let mut context = ClientContext::new();

        // Extract IP address from client connection
        if let Some(client_ip) = request.client_ip() {
            context.ip = Some(client_ip.to_string());
        }

        // Extract all headers
        for header in request.headers().iter() {
            context
                .headers
                .insert(header.name().to_string(), header.value().to_string());
        }

        // Extract specific important headers
        context.user_agent = request
            .headers()
            .get_one("User-Agent")
            .map(|s| s.to_string());

        context.forwarded_for = request
            .headers()
            .get_one("X-Forwarded-For")
            .map(|s| s.to_string());

        context.real_ip = request
            .headers()
            .get_one("X-Real-IP")
            .map(|s| s.to_string());

        // Extract cookies
        let cookies = request.cookies();
        for cookie in cookies.iter() {
            context
                .cookies
                .insert(cookie.name().to_string(), cookie.value().to_string());
        }

        Outcome::Success(context)
    }
}

/// A simplified context that only extracts IP address for performance
#[derive(Debug, Clone)]
pub struct IpContext {
    pub ip: Option<String>,
}

impl IpContext {
    /// Get the client IP address
    pub fn get_ip(&self) -> Option<&String> {
        self.ip.as_ref()
    }

    /// Get the client IP address as a string, with fallback
    pub fn get_ip_string(&self) -> String {
        self.ip
            .as_ref()
            .map(|s| s.clone())
            .unwrap_or_else(|| "unknown".to_string())
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for IpContext {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let mut ip = None;

        // Try X-Real-IP first (for reverse proxy setups)
        if let Some(real_ip) = request.headers().get_one("X-Real-IP") {
            if !real_ip.is_empty() {
                ip = Some(real_ip.to_string());
            }
        }

        // Try X-Forwarded-For if X-Real-IP is not available
        if ip.is_none() {
            if let Some(forwarded) = request.headers().get_one("X-Forwarded-For") {
                if !forwarded.is_empty() {
                    let first_ip = forwarded.split(',').next().unwrap_or("").trim();
                    if !first_ip.is_empty() {
                        ip = Some(first_ip.to_string());
                    }
                }
            }
        }

        // Fall back to client IP from connection
        if ip.is_none() {
            if let Some(client_ip) = request.client_ip() {
                ip = Some(client_ip.to_string());
            }
        }

        Outcome::Success(IpContext { ip })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_context_creation() {
        let ctx = ClientContext::new();
        assert!(ctx.ip.is_none());
        assert!(ctx.headers.is_empty());
        assert!(ctx.cookies.is_empty());
    }

    #[test]
    fn test_get_client_ip_priority() {
        let mut ctx = ClientContext::new();
        ctx.ip = Some("192.168.1.1".to_string());
        ctx.forwarded_for = Some("203.0.113.1, 192.168.1.1".to_string());
        ctx.real_ip = Some("203.0.113.2".to_string());

        // Should prefer X-Real-IP
        assert_eq!(ctx.get_client_ip(), Some("203.0.113.2".to_string()));

        // Without X-Real-IP, should use first IP from X-Forwarded-For
        ctx.real_ip = None;
        assert_eq!(ctx.get_client_ip(), Some("203.0.113.1".to_string()));

        // Without both headers, should fall back to direct IP
        ctx.forwarded_for = None;
        assert_eq!(ctx.get_client_ip(), Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_is_local_request() {
        let mut ctx = ClientContext::new();

        ctx.ip = Some("127.0.0.1".to_string());
        assert!(ctx.is_local_request());

        ctx.ip = Some("192.168.1.1".to_string());
        assert!(ctx.is_local_request());

        ctx.ip = Some("203.0.113.1".to_string());
        assert!(!ctx.is_local_request());
    }

    #[test]
    fn test_get_header_case_insensitive() {
        let mut ctx = ClientContext::new();
        ctx.headers
            .insert("Content-Type".to_string(), "application/json".to_string());

        assert_eq!(
            ctx.get_header("content-type"),
            Some(&"application/json".to_string())
        );
        assert_eq!(
            ctx.get_header("Content-Type"),
            Some(&"application/json".to_string())
        );
        assert_eq!(
            ctx.get_header("CONTENT-TYPE"),
            Some(&"application/json".to_string())
        );
    }
}
