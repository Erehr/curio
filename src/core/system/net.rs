//! Outgoing web requests. Every request resolves through `public_only`, so neither a link nor a redirect can reach this computer or the local network.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::time::Duration;

pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36";

fn public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    !(ip.is_unspecified() || ip.is_loopback() || ip.is_private() || ip.is_link_local() || ip.is_broadcast() || ip.is_multicast() || ip.is_documentation() || a == 0 || a >= 240 || (a == 100 && (64..128).contains(&b)) || (a == 198 && (b == 18 || b == 19)))
}

fn public_v6(ip: Ipv6Addr) -> bool {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return public_v4(v4);
    }
    let first = ip.segments()[0];
    // unique local fc00::/7, link-local fe80::/10, site-local fec0::/10, NAT64 64:ff9b::/96, 6to4 2002::/16, Teredo 2001::/32
    !(ip.is_unspecified() || ip.is_loopback() || ip.is_multicast() || (first & 0xfe00) == 0xfc00 || (first & 0xffc0) == 0xfe80 || (first & 0xffc0) == 0xfec0 || first == 0x64 || first == 0x2002 || (first == 0x2001 && ip.segments()[1] == 0) || (first == 0x2001 && ip.segments()[1] == 0xdb8))
}

pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => public_v4(v4),
        IpAddr::V6(v6) => public_v6(v6),
    }
}

/// Resolves `host:port` and keeps only public addresses.
pub fn public_only(netloc: &str) -> std::io::Result<Vec<SocketAddr>> {
    let addrs: Vec<SocketAddr> = netloc.to_socket_addrs()?.collect();
    let public: Vec<SocketAddr> = addrs.iter().copied().filter(|a| is_public(a.ip())).collect();
    if public.is_empty() {
        return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, format!("{netloc} is not a public internet address")));
    }
    Ok(public)
}

pub fn agent(timeout: Duration) -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(timeout).redirects(5).user_agent(USER_AGENT).resolver(public_only).build()
}

/// For a file downloaded in full: no limit on the whole transfer, only on connecting and on each wait for more of it.
/// Redirects are followed by `downloads`, which decides per hop what goes with them.
pub fn download_agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(15)).timeout_read(Duration::from_secs(30)).redirects(0).user_agent(USER_AGENT).resolver(download_resolver).build()
}

/// `public_only`, and in a debug build the one port on this computer `CURIO_DEBUG_LOOPBACK_PORT` names, for the fixture site of the browser extension's end-to-end check (its repository's `tools/ext.mjs`).
fn download_resolver(netloc: &str) -> std::io::Result<Vec<SocketAddr>> {
    #[cfg(debug_assertions)]
    if let Some(port) = std::env::var("CURIO_DEBUG_LOOPBACK_PORT").ok().and_then(|p| p.trim().parse::<u16>().ok()).filter(|p| *p != 0) {
        if netloc == format!("127.0.0.1:{port}") || netloc == format!("localhost:{port}") {
            return Ok(vec![SocketAddr::from(([127, 0, 0, 1], port))]);
        }
    }
    public_only(netloc)
}

/// For requests made by other programs (the page-capture browser), which can't use `agent`.
pub fn check_public_url(url: &str) -> Result<(), String> {
    let u = url::Url::parse(url).map_err(|e| e.to_string())?;
    if !matches!(u.scheme(), "http" | "https") {
        return Err(format!("{url} isn't a web address"));
    }
    let host = u.host_str().ok_or("the address has no host")?;
    let port = u.port_or_known_default().unwrap_or(443);
    public_only(&format!("{}:{port}", host.trim_start_matches('[').trim_end_matches(']'))).map(|_| ()).map_err(|e| e.to_string())
}

/// A proxy on this computer for programs Curio starts that fetch by themselves (the page-capture browser): every connection, redirect and subresource goes out only to public addresses, resolved here, so a page can't reach the local network or rebind its name to it.
pub struct PublicProxy {
    pub port: u16,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl PublicProxy {
    pub fn start() -> std::io::Result<PublicProxy> {
        use std::sync::atomic::Ordering;
        let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let port = listener.local_addr()?.port();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stopped = stop.clone();
        std::thread::Builder::new().name("capture proxy".into()).spawn(move || {
            for conn in listener.incoming() {
                if stopped.load(Ordering::Relaxed) {
                    break;
                }
                if let Ok(c) = conn {
                    let _ = std::thread::Builder::new().name("capture proxy conn".into()).spawn(move || proxy_one(c));
                }
            }
        })?;
        Ok(PublicProxy { port, stop })
    }
}

impl Drop for PublicProxy {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        // wakes the accepting thread so it sees the flag
        let _ = std::net::TcpStream::connect_timeout(&SocketAddr::from((Ipv4Addr::LOCALHOST, self.port)), Duration::from_millis(200));
    }
}

const PROXY_HEAD_LIMIT: usize = 32 << 10;
const PROXY_TIMEOUT: Duration = Duration::from_secs(20);
const PROXY_IDLE: Duration = Duration::from_secs(60);

fn proxy_one(mut client: std::net::TcpStream) {
    use std::io::{Read, Write};
    let _ = client.set_read_timeout(Some(PROXY_TIMEOUT));
    let mut head = Vec::new();
    let mut buf = [0u8; 4096];
    let end = loop {
        match client.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => head.extend_from_slice(&buf[..n]),
        }
        if let Some(i) = head.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        if head.len() > PROXY_HEAD_LIMIT {
            return;
        }
    };
    let refuse = |mut c: std::net::TcpStream| {
        let _ = c.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    };
    let text = String::from_utf8_lossy(&head[..end]).into_owned();
    let mut lines = text.split("\r\n");
    let first: Vec<&str> = lines.next().unwrap_or_default().split(' ').collect();
    let [method, target, version] = first[..] else { return refuse(client) };
    let (netloc, request) = if method.eq_ignore_ascii_case("CONNECT") {
        (target.to_string(), None)
    } else {
        let Ok(u) = url::Url::parse(target) else { return refuse(client) };
        let Some(host) = u.host_str().filter(|_| u.scheme() == "http") else { return refuse(client) };
        let path = &u[url::Position::BeforePath..url::Position::AfterQuery];
        // one request per connection, so the next one is checked again
        let mut req = format!("{method} {path} {version}\r\n");
        for l in lines.filter(|l| !l.is_empty()) {
            let name = l.split(':').next().unwrap_or_default().trim().to_ascii_lowercase();
            if name != "connection" && name != "proxy-connection" && name != "keep-alive" {
                req += l;
                req += "\r\n";
            }
        }
        req += "Connection: close\r\n\r\n";
        (format!("{host}:{}", u.port_or_known_default().unwrap_or(80)), Some(req))
    };
    let Ok(addrs) = public_only(&netloc) else { return refuse(client) };
    let Some(mut server) = addrs.iter().find_map(|a| std::net::TcpStream::connect_timeout(a, PROXY_TIMEOUT).ok()) else { return refuse(client) };
    let sent = match &request {
        None => client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n"),
        Some(req) => server.write_all(req.as_bytes()).and_then(|_| server.write_all(&head[end..])),
    };
    if sent.is_err() {
        return;
    }
    if request.is_none() && head.len() > end {
        let _ = server.write_all(&head[end..]);
    }
    // a connection quiet this long is let go, so no thread waits on it for ever
    let _ = client.set_read_timeout(Some(PROXY_IDLE));
    let _ = server.set_read_timeout(Some(PROXY_IDLE));
    let (Ok(mut c2), Ok(mut s2)) = (client.try_clone(), server.try_clone()) else { return };
    let up = std::thread::spawn(move || {
        let _ = std::io::copy(&mut c2, &mut s2);
        let _ = s2.shutdown(std::net::Shutdown::Write);
    });
    let _ = std::io::copy(&mut server, &mut client);
    let _ = client.shutdown(std::net::Shutdown::Both);
    let _ = up.join();
}
