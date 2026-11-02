//! Dış HTTPS bağlantıları (GitLab, Jira) için sertifika doğrulaması.
//!
//! Şirket içi GitLab/Jira sertifikaları çoğunlukla şirketin kendi CA'sıyla imzalıdır. Varsayılan
//! olarak işletim sisteminin sertifika deposu kullanılır (Windows: sertifika deposu, GPO ile dağıtılan
//! CA'lar dahil; Linux: /etc/ssl/certs). Sistem deposuna eklenemiyorsa CA'nın PEM dosyası bir ortam
//! değişkeniyle verilir; o zaman yalnızca o dosyadaki CA'lara güvenilir. Doğrulama kapatılamaz.

use std::sync::Arc;
use ureq::tls::{Certificate, PemItem, RootCerts, TlsConfig};

/// `ca_envs` içinde ilk dolu ortam değişkeninin gösterdiği PEM dosyasındaki CA'lar, hiçbiri yoksa
/// sistem deposu. Dosya okunamaz ya da içinde sertifika yoksa açıklayıcı hata döner.
pub fn config(ca_envs: &[&str]) -> Result<TlsConfig, String> {
    let roots = match ca_file(ca_envs) {
        Some((name, path)) => RootCerts::Specific(Arc::new(load_pem(&path).map_err(|e| format!("{name}={path}: {e}"))?)),
        None => RootCerts::PlatformVerifier,
    };
    Ok(TlsConfig::builder().root_certs(roots).build())
}

/// İlk dolu değişken ve gösterdiği dosya.
pub fn ca_file(ca_envs: &[&str]) -> Option<(String, String)> {
    ca_envs
        .iter()
        .find_map(|n| std::env::var(n).ok().filter(|v| !v.trim().is_empty()).map(|v| (n.to_string(), v)))
}

/// Hata olursa sistem deposuyla devam eder; hatayı çağıran loglar.
pub fn config_or_platform(ca_envs: &[&str]) -> (TlsConfig, Option<String>) {
    match config(ca_envs) {
        Ok(c) => (c, None),
        Err(e) => (TlsConfig::builder().root_certs(RootCerts::PlatformVerifier).build(), Some(e)),
    }
}

pub fn load_pem(path: &str) -> Result<Vec<Certificate<'static>>, String> {
    let data = std::fs::read(path).map_err(|e| format!("CA dosyası okunamadı: {e}"))?;
    let certs: Vec<Certificate<'static>> = ureq::tls::parse_pem(&data)
        .filter_map(|i| match i {
            Ok(PemItem::Certificate(c)) => Some(c),
            _ => None,
        })
        .collect();
    if certs.is_empty() {
        return Err("dosyada PEM sertifika yok (-----BEGIN CERTIFICATE----- ile başlamalı; .cer ise Base-64 olarak dışa aktarın)".into());
    }
    Ok(certs)
}

/// Bağlantı hatası sertifikadan kaynaklanıyorsa ne yapılacağını söyleyen ek metin.
pub fn hint(err: &str, ca_env: &str) -> String {
    let e = err.to_lowercase();
    if e.contains("certificate") || e.contains("unknownissuer") || e.contains("cert") {
        format!(
            " — Sunucunun HTTPS sertifikası tanınmadı (şirket CA'sı). Şirket kök sertifikasını işletim sisteminin \
             sertifika deposuna ekleyin ya da PEM dosyasını {ca_env} ortam değişkeniyle verin."
        )
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_9_1_ca_file_errors_are_clear() {
        let dir = std::env::temp_dir().join(format!("divan-tls-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let bos = dir.join("bos.pem");
        std::fs::write(&bos, "sertifika değil").unwrap();
        assert!(load_pem(bos.to_str().unwrap()).unwrap_err().contains("PEM sertifika yok"));
        assert!(load_pem(dir.join("yok.pem").to_str().unwrap()).unwrap_err().contains("okunamadı"));
        assert!(hint("io: invalid peer certificate: UnknownIssuer", "DIVAN_CA_DOSYASI").contains("DIVAN_CA_DOSYASI"));
        assert!(hint("Connection refused", "X").is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }
}
