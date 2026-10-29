//! stajproje sunucusuna HTTP istemcisi: CI_API_TOKEN ve sürüm başlığı her istekte gider.

use anyhow::{bail, Context, Result};
use std::io::Read;
use std::time::Duration;

#[derive(Clone)]
pub struct Client {
    pub server: String,
    token: String,
    agent: ureq::Agent,
    pub retries: u32,
    pub base_delay: Duration,
}

pub struct Reply {
    pub status: u16,
    pub body: String,
}

impl Reply {
    pub fn json(&self) -> Result<serde_json::Value> {
        serde_json::from_str(&self.body).with_context(|| format!("sunucu yanıtı json değil: {}", self.body))
    }
}

impl Client {
    pub fn new(server: &str, token: &str) -> Client {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(600)))
            .timeout_connect(Some(Duration::from_secs(5)))
            .http_status_as_error(false)
            // sunucu aynı makinede; sistem proxy'si kullanılmaz
            .proxy(None)
            .build()
            .into();
        Client {
            server: server.trim_end_matches('/').to_string(),
            token: token.to_string(),
            agent,
            retries: std::env::var("STAJ_RETRY_MAX").ok().and_then(|v| v.parse().ok()).unwrap_or(5),
            base_delay: Duration::from_millis(std::env::var("STAJ_RETRY_BASE_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(2000)),
        }
    }

    fn req(&self, method: &str, path: &str) -> ureq::RequestBuilder<ureq::typestate::WithBody> {
        let url = format!("{}{}", self.server, path);
        let b = match method {
            "PUT" => self.agent.put(&url),
            _ => self.agent.post(&url),
        };
        b.header("Authorization", &format!("Bearer {}", self.token)).header(common::VERSION_HEADER, common::VERSION)
    }

    fn finish(resp: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<Reply> {
        let mut resp = resp.map_err(|e| anyhow::anyhow!("sunucuya ulaşılamadı: {e}"))?;
        let status = resp.status().as_u16();
        let mut body = String::new();
        resp.body_mut().as_reader().read_to_string(&mut body)?;
        Ok(Reply { status, body })
    }

    pub fn post_json(&self, path: &str, v: &serde_json::Value) -> Result<Reply> {
        Self::finish(self.req("POST", path).header("Content-Type", "application/json").send(v.to_string()))
    }

    pub fn post_bytes(&self, path: &str, content_type: &str, data: &[u8]) -> Result<Reply> {
        Self::finish(self.req("POST", path).header("Content-Type", content_type).send(data))
    }

    pub fn post_file(&self, path: &str, file: &std::path::Path) -> Result<Reply> {
        let f = std::fs::File::open(file)?;
        Self::finish(self.req("POST", path).header("Content-Type", "application/zip").send(f))
    }

    #[allow(dead_code)]
    pub fn get(&self, path: &str) -> Result<Reply> {
        let url = format!("{}{}", self.server, path);
        Self::finish(
            self.agent
                .get(&url)
                .header("Authorization", &format!("Bearer {}", self.token))
                .header(common::VERSION_HEADER, common::VERSION)
                .call(),
        )
    }

    /// Ağ hatası ve 5xx için artan aralıklarla yeniden dener. 4xx hemen döner.
    pub fn with_retry(&self, what: &str, mut f: impl FnMut() -> Result<Reply>) -> Result<Reply> {
        let mut delay = self.base_delay;
        let mut last = None;
        for attempt in 0..=self.retries {
            match f() {
                Ok(r) if r.status < 500 => return Ok(r),
                Ok(r) => last = Some(anyhow::anyhow!("{what}: sunucu {} döndü: {}", r.status, r.body)),
                Err(e) => last = Some(e),
            }
            if attempt < self.retries {
                eprintln!("[stajci] {what} başarısız, {} sn sonra tekrar denenecek ({}/{})", delay.as_secs_f32(), attempt + 1, self.retries);
                std::thread::sleep(delay);
                delay = (delay * 2).min(Duration::from_secs(60));
            }
        }
        Err(last.unwrap_or_else(|| anyhow::anyhow!("{what} başarısız")))
    }
}

pub fn ensure_ok(r: Reply, what: &str) -> Result<Reply> {
    if (200..300).contains(&r.status) {
        Ok(r)
    } else if r.status == 409 {
        bail!("{what}: {}", r.body)
    } else {
        bail!("{what}: sunucu {} döndü: {}", r.status, r.body)
    }
}
