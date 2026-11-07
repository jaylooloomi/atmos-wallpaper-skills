//! Bounded HTTPS transport for public, unauthenticated content.
use reqwest::{header, redirect::Policy, Url};
use std::{net::IpAddr, sync::OnceLock, time::Duration};
use tokio::runtime::{Builder, Runtime};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Error {
    Url,
    Blocked,
    Fetch,
    Type,
    Size,
}

pub(crate) fn public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !matches!(
                (a, b, c),
                (0 | 10 | 127 | 224..=255, _, _)
                    | (100, 64..=127, _)
                    | (169, 254, _)
                    | (172, 16..=31, _)
                    | (192, 168, _)
                    | (192, 0, 0 | 2)
                    | (192, 88, 99)
                    | (198, 18 | 19, _)
                    | (198, 51, 100)
                    | (203, 0, 113)
            )
        }
        IpAddr::V6(ip) => {
            let [a, b, ..] = ip.segments();
            (0x2000..=0x3fff).contains(&a)
                && !(a == 0x2001 && (b <= 0x01ff || b == 0xdb8) || a == 0x2002)
                && !(a == 0x3fff && b <= 0x0fff)
        }
    }
}

pub(crate) fn validate(url: &Url) -> Result<(), Error> {
    let host = url.host_str().ok_or(Error::Url)?;
    if url.as_str().len() > 4096
        || url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.port_or_known_default() != Some(443)
    {
        return Err(Error::Url);
    }
    let host = host.trim_end_matches('.').trim_matches(['[', ']']);
    let blocked = host.eq_ignore_ascii_case("localhost")
        || host.ends_with(".local")
        || host.parse::<IpAddr>().is_ok_and(|ip| !public(ip));
    if blocked {
        Err(Error::Blocked)
    } else {
        Ok(())
    }
}

pub(crate) struct Fetch {
    pub accept: &'static str,
    pub content_types: &'static [&'static str],
    pub max_bytes: usize,
    pub redirects: usize,
    pub seconds: u64,
}

pub(crate) fn get(url: Url, policy: &Fetch) -> Result<Vec<u8>, Error> {
    static RUNTIME: OnceLock<Result<Runtime, std::io::Error>> = OnceLock::new();
    let runtime = RUNTIME
        .get_or_init(|| {
            Builder::new_multi_thread()
                .worker_threads(2)
                .max_blocking_threads(8)
                .enable_all()
                .build()
        })
        .as_ref()
        .map_err(|_| Error::Fetch)?;
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(policy.seconds), download(url, policy))
            .await
            .map_err(|_| Error::Fetch)?
    })
}

async fn download(mut url: Url, policy: &Fetch) -> Result<Vec<u8>, Error> {
    for remaining in (0..=policy.redirects).rev() {
        validate(&url)?;
        let host = url.host_str().ok_or(Error::Url)?;
        let mut builder = reqwest::Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(Policy::none())
            .pool_max_idle_per_host(0);
        if host.trim_matches(['[', ']']).parse::<IpAddr>().is_err() {
            let answers: Vec<_> = tokio::net::lookup_host((host, 443))
                .await
                .map_err(|_| Error::Fetch)?
                .take(65)
                .collect();
            if answers.is_empty() || answers.len() > 64 || answers.iter().any(|a| !public(a.ip())) {
                return Err(Error::Blocked);
            }
            // Connect only to the checked answers, retaining original TLS hostname.
            builder = builder.resolve_to_addrs(host, &answers);
        }
        let mut response = builder
            .build()
            .map_err(|_| Error::Fetch)?
            .get(url.clone())
            .header(header::ACCEPT, policy.accept)
            .header(header::ACCEPT_ENCODING, "identity")
            .header(header::USER_AGENT, "AtmosWallpaper/0.7")
            .send()
            .await
            .map_err(|_| Error::Fetch)?;
        if matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
            if remaining == 0 {
                return Err(Error::Fetch);
            }
            let next = response
                .headers()
                .get(header::LOCATION)
                .and_then(|h| h.to_str().ok())
                .ok_or(Error::Fetch)?;
            url = url.join(next).map_err(|_| Error::Url)?;
            continue;
        }
        if response.status() != reqwest::StatusCode::OK {
            return Err(Error::Fetch);
        }
        let kind = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.split(';').next())
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if !policy.content_types.is_empty() && !policy.content_types.contains(&kind.as_str()) {
            return Err(Error::Type);
        }
        if response
            .content_length()
            .is_some_and(|n| n > policy.max_bytes as u64)
        {
            return Err(Error::Size);
        }
        let mut body = Vec::new();
        while let Some(part) = response.chunk().await.map_err(|_| Error::Fetch)? {
            if part.len() > policy.max_bytes - body.len() {
                return Err(Error::Size);
            }
            body.extend_from_slice(&part);
        }
        return Ok(body);
    }
    Err(Error::Fetch)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_private_metadata_and_non_https_destinations() {
        for s in [
            "http://example.org",
            "https://example.org:8443",
            "https://user@example.org",
            "https://example.org/#a",
            "https://localhost.",
            "https://127.1",
            "https://[::ffff:127.0.0.1]",
            "https://169.254.169.254",
            "https://[2001:db8::1]",
            "https://router.local",
        ] {
            assert!(validate(&Url::parse(s).unwrap()).is_err(), "{s}");
        }
        for s in [
            "https://example.org/path?a=b",
            "https://8.8.8.8",
            "https://[2606:4700:4700::1111]",
        ] {
            assert!(validate(&Url::parse(s).unwrap()).is_ok(), "{s}");
        }
    }
}
