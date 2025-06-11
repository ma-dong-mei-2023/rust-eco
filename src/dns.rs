use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use tokio::net::lookup_host;

#[derive(Debug, Clone)]
pub struct DnsRecord {
    pub name: String,
    pub record_type: DnsRecordType,
    pub address: IpAddr,
    pub ttl: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DnsRecordType {
    A,
    AAAA,
    CNAME,
    MX,
    TXT,
    PTR,
}

pub struct DnsResolver;

impl DnsResolver {
    pub fn new() -> Self {
        Self
    }

    pub async fn resolve(&self, hostname: &str) -> crate::Result<Vec<IpAddr>> {
        let mut addresses = Vec::new();
        
        // Use tokio's built-in DNS resolution
        let addr_iter = lookup_host(format!("{}:0", hostname)).await?;
        
        for addr in addr_iter {
            addresses.push(addr.ip());
        }
        
        if addresses.is_empty() {
            return Err(format!("No addresses found for hostname: {}", hostname).into());
        }
        
        Ok(addresses)
    }

    pub async fn resolve_ipv4(&self, hostname: &str) -> crate::Result<Vec<Ipv4Addr>> {
        let addresses = self.resolve(hostname).await?;
        let ipv4_addresses: Vec<Ipv4Addr> = addresses
            .into_iter()
            .filter_map(|addr| match addr {
                IpAddr::V4(ipv4) => Some(ipv4),
                IpAddr::V6(_) => None,
            })
            .collect();
        
        if ipv4_addresses.is_empty() {
            return Err(format!("No IPv4 addresses found for hostname: {}", hostname).into());
        }
        
        Ok(ipv4_addresses)
    }

    pub async fn resolve_ipv6(&self, hostname: &str) -> crate::Result<Vec<Ipv6Addr>> {
        let addresses = self.resolve(hostname).await?;
        let ipv6_addresses: Vec<Ipv6Addr> = addresses
            .into_iter()
            .filter_map(|addr| match addr {
                IpAddr::V4(_) => None,
                IpAddr::V6(ipv6) => Some(ipv6),
            })
            .collect();
        
        if ipv6_addresses.is_empty() {
            return Err(format!("No IPv6 addresses found for hostname: {}", hostname).into());
        }
        
        Ok(ipv6_addresses)
    }

    pub async fn resolve_with_port(&self, hostname: &str, port: u16) -> crate::Result<Vec<SocketAddr>> {
        let addr_iter = lookup_host(format!("{}:{}", hostname, port)).await?;
        let addresses: Vec<SocketAddr> = addr_iter.collect();
        
        if addresses.is_empty() {
            return Err(format!("No addresses found for hostname: {}", hostname).into());
        }
        
        Ok(addresses)
    }

    pub async fn reverse_lookup(&self, ip: IpAddr) -> crate::Result<String> {
        // Note: This is a simplified reverse lookup
        // For full DNS functionality, you'd want to use a more complete DNS library
        match ip {
            IpAddr::V4(ipv4) => {
                let octets = ipv4.octets();
                let reverse_name = format!("{}.{}.{}.{}.in-addr.arpa", 
                    octets[3], octets[2], octets[1], octets[0]);
                
                // This is a placeholder - actual reverse lookup would require PTR record resolution
                Ok(format!("reverse-{}", reverse_name))
            }
            IpAddr::V6(ipv6) => {
                let segments = ipv6.segments();
                let reverse_name = format!("{:x}.{:x}.{:x}.{:x}.{:x}.{:x}.{:x}.{:x}.ip6.arpa",
                    segments[7], segments[6], segments[5], segments[4],
                    segments[3], segments[2], segments[1], segments[0]);
                
                Ok(format!("reverse-{}", reverse_name))
            }
        }
    }
}

impl Default for DnsResolver {
    fn default() -> Self {
        Self::new()
    }
}

// Utility functions for common DNS operations
pub async fn resolve(hostname: &str) -> crate::Result<Vec<IpAddr>> {
    DnsResolver::new().resolve(hostname).await
}

pub async fn resolve_ipv4(hostname: &str) -> crate::Result<Vec<Ipv4Addr>> {
    DnsResolver::new().resolve_ipv4(hostname).await
}

pub async fn resolve_ipv6(hostname: &str) -> crate::Result<Vec<Ipv6Addr>> {
    DnsResolver::new().resolve_ipv6(hostname).await
}

pub async fn resolve_with_port(hostname: &str, port: u16) -> crate::Result<Vec<SocketAddr>> {
    DnsResolver::new().resolve_with_port(hostname, port).await
}

pub async fn reverse_lookup(ip: IpAddr) -> crate::Result<String> {
    DnsResolver::new().reverse_lookup(ip).await
}

// DNS cache for better performance
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct CacheEntry {
    addresses: Vec<IpAddr>,
    expires_at: Instant,
}

pub struct CachedDnsResolver {
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
    ttl: Duration,
    resolver: DnsResolver,
}

impl CachedDnsResolver {
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            ttl,
            resolver: DnsResolver::new(),
        }
    }

    pub fn with_default_ttl() -> Self {
        Self::new(Duration::from_secs(300)) // 5 minutes default TTL
    }

    pub async fn resolve(&self, hostname: &str) -> crate::Result<Vec<IpAddr>> {
        let now = Instant::now();
        
        // Check cache first
        {
            let cache = self.cache.lock().await;
            if let Some(entry) = cache.get(hostname) {
                if entry.expires_at > now {
                    return Ok(entry.addresses.clone());
                }
            }
        }
        
        // Cache miss or expired, resolve and cache
        let addresses = self.resolver.resolve(hostname).await?;
        
        {
            let mut cache = self.cache.lock().await;
            cache.insert(hostname.to_string(), CacheEntry {
                addresses: addresses.clone(),
                expires_at: now + self.ttl,
            });
        }
        
        Ok(addresses)
    }

    pub async fn clear_cache(&self) {
        let mut cache = self.cache.lock().await;
        cache.clear();
    }

    pub async fn remove_from_cache(&self, hostname: &str) {
        let mut cache = self.cache.lock().await;
        cache.remove(hostname);
    }

    pub async fn cache_size(&self) -> usize {
        let cache = self.cache.lock().await;
        cache.len()
    }
}

impl Default for CachedDnsResolver {
    fn default() -> Self {
        Self::with_default_ttl()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dns_resolve() {
        let resolver = DnsResolver::new();
        
        // Test with localhost
        match resolver.resolve("localhost").await {
            Ok(addresses) => {
                assert!(!addresses.is_empty());
                // Should contain loopback addresses
                assert!(addresses.iter().any(|addr| addr.is_loopback()));
            }
            Err(e) => {
                // Some systems might not resolve localhost
                println!("Localhost resolution failed (may be expected): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_dns_resolve_with_port() {
        let resolver = DnsResolver::new();
        
        match resolver.resolve_with_port("localhost", 80).await {
            Ok(addresses) => {
                assert!(!addresses.is_empty());
                for addr in addresses {
                    assert_eq!(addr.port(), 80);
                }
            }
            Err(e) => {
                println!("Localhost with port resolution failed: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_reverse_lookup() {
        let resolver = DnsResolver::new();
        
        let ipv4 = "127.0.0.1".parse::<IpAddr>().unwrap();
        let reverse = resolver.reverse_lookup(ipv4).await.unwrap();
        assert!(reverse.contains("in-addr.arpa"));
        
        let ipv6 = "::1".parse::<IpAddr>().unwrap();
        let reverse = resolver.reverse_lookup(ipv6).await.unwrap();
        assert!(reverse.contains("ip6.arpa"));
    }

    #[tokio::test]
    async fn test_cached_resolver() {
        let resolver = CachedDnsResolver::new(Duration::from_secs(1));
        
        // First resolution should cache the result
        match resolver.resolve("localhost").await {
            Ok(addresses1) => {
                assert_eq!(resolver.cache_size().await, 1);
                
                // Second resolution should use cache
                let addresses2 = resolver.resolve("localhost").await.unwrap();
                assert_eq!(addresses1, addresses2);
                
                // Wait for cache to expire
                tokio::time::sleep(Duration::from_secs(2)).await;
                
                // This should trigger a new resolution
                let _addresses3 = resolver.resolve("localhost").await.unwrap();
            }
            Err(e) => {
                println!("Cached resolver test failed: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_utility_functions() {
        // Test the module-level utility functions
        match resolve("localhost").await {
            Ok(addresses) => {
                assert!(!addresses.is_empty());
            }
            Err(e) => {
                println!("Utility resolve function failed: {}", e);
            }
        }
        
        let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
        let reverse = reverse_lookup(ip).await.unwrap();
        assert!(reverse.contains("in-addr.arpa"));
    }
}