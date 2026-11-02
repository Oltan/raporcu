//! Dış HTTPS bağlantıları (GitLab, Jira) için sertifika doğrulaması.
//!
//! Şirket içi GitLab/Jira sertifikaları çoğunlukla şirketin kendi CA'sıyla imzalıdır. Varsayılan
//! olarak işletim sisteminin sertifika deposu kullanılır (Windows: sertifika deposu, GPO ile dağıtılan
//! CA'lar dahil; Linux: /etc/ssl/certs). Sistem deposuna eklenemiyorsa CA dosyası ya da klasörü bir
//! ortam değişkeniyle verilir; o zaman yalnızca oradaki CA'lara güvenilir. Doğrulama kapatılamaz.

use std::sync::Arc;
use ureq::tls::{Certificate, PemItem, RootCerts, TlsConfig};

/// `ca_envs` içinde ilk dolu ortam değişkeninin gösterdiği dosya ya da klasördeki CA'lar, hiçbiri yoksa
/// sistem deposu. Okunamaz ya da içinde sertifika yoksa açıklayıcı hata döner.
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

/// CA dosyasını okur. Uzantı önemsiz (.pem, .crt, .cer, .cert): içerik PEM metni
/// (`-----BEGIN CERTIFICATE-----`, birden çok olabilir) ya da tek bir ikili DER sertifika olabilir.
pub fn load_pem(path: &str) -> Result<Vec<Certificate<'static>>, String> {
    let p = std::path::Path::new(path);
    if p.is_dir() {
        return load_dir(p);
    }
    let data = std::fs::read(path).map_err(|e| format!("CA dosyası okunamadı: {e}"))?;
    parse_certs(&data)
}

/// Klasördeki bütün sertifika dosyaları (alt klasörlere inilmez). Sertifika olmayan dosyalar atlanır;
/// hiç sertifika yoksa hata. Kök ve ara sertifikalar ayrı dosyalarda durabilir.
fn load_dir(dir: &std::path::Path) -> Result<Vec<Certificate<'static>>, String> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("CA klasörü okunamadı: {e}"))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    files.sort();
    let mut certs = Vec::new();
    for f in files {
        if let Ok(data) = std::fs::read(&f) {
            if let Ok(mut c) = parse_certs(&data) {
                certs.append(&mut c);
            }
        }
    }
    if certs.is_empty() {
        return Err("klasörde sertifika dosyası bulunamadı (.pem/.crt/.cer/.cert, PEM ya da DER)".into());
    }
    Ok(certs)
}

fn parse_certs(data: &[u8]) -> Result<Vec<Certificate<'static>>, String> {
    let certs: Vec<Certificate<'static>> = ureq::tls::parse_pem(data)
        .filter_map(|i| match i {
            Ok(PemItem::Certificate(c)) => Some(c),
            _ => None,
        })
        .collect();
    if !certs.is_empty() {
        return Ok(certs);
    }
    // ikili DER: X.509 sertifikası bir ASN.1 SEQUENCE ile (0x30) başlar
    if is_der(data) {
        return Ok(vec![Certificate::from_der(data).to_owned()]);
    }
    Err("dosyada sertifika bulunamadı: ne PEM metni (-----BEGIN CERTIFICATE-----) ne de ikili DER sertifika".into())
}

fn is_der(data: &[u8]) -> bool {
    // 0x30 0x82 <2 bayt uzunluk>: 256 bayttan büyük SEQUENCE (gerçek sertifikalar böyledir)
    data.len() > 100 && data[0] == 0x30 && data[1] == 0x82 && {
        let n = ((data[2] as usize) << 8) | data[3] as usize;
        n + 4 == data.len()
    }
}

/// Git (`http.sslCAInfo`) ve Node/Bun tabanlı araçlar (OpenCode, Claude Code: `NODE_EXTRA_CA_CERTS`)
/// yalnızca tek bir PEM dosyası okur. Dosya, DER ya da klasör ne verilmişse bütün sertifikalar tek PEM
/// olarak `dir/sirket-ca.pem`'e yazılır ve yolu döner.
pub fn combined_pem(path: &str, dir: &std::path::Path) -> Result<String, String> {
    let certs = load_pem(path)?;
    let mut pem = String::new();
    for c in &certs {
        pem.push_str("-----BEGIN CERTIFICATE-----\n");
        for chunk in base64(c.der()).as_bytes().chunks(64) {
            pem.push_str(std::str::from_utf8(chunk).unwrap_or_default());
            pem.push('\n');
        }
        pem.push_str("-----END CERTIFICATE-----\n");
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let out = dir.join("sirket-ca.pem");
    std::fs::write(&out, pem).map_err(|e| e.to_string())?;
    Ok(out.display().to_string())
}

fn base64(b: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(b.len().div_ceil(3) * 4);
    for ch in b.chunks(3) {
        let n = (ch[0] as u32) << 16 | (*ch.get(1).unwrap_or(&0) as u32) << 8 | *ch.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= ch.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Bağlantı hatası sertifikadan kaynaklanıyorsa ne yapılacağını söyleyen ek metin.
pub fn hint(err: &str, ca_env: &str) -> String {
    let e = err.to_lowercase();
    if e.contains("certificate") || e.contains("unknownissuer") || e.contains("cert") {
        format!(
            " — Sunucunun HTTPS sertifikası tanınmadı (şirket CA'sı). Şirket kök sertifikasını işletim sisteminin \
             sertifika deposuna ekleyin ya da sertifika dosyasını / klasörünü {ca_env} ortam değişkeniyle verin."
        )
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_9_1_ca_file_pem_and_der() {
        let dir = std::env::temp_dir().join(format!("divan-tls-der-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // gerçek bir DER: 0x30 0x82 uzunluk + gövde
        let mut der = vec![0x30u8, 0x82, 0x01, 0x00];
        der.extend(std::iter::repeat_n(0x01u8, 0x100));
        let f = dir.join("sirket.cert");
        std::fs::write(&f, &der).unwrap();
        let certs = load_pem(f.to_str().unwrap()).unwrap();
        assert_eq!(certs.len(), 1);
        assert_eq!(certs[0].der(), &der[..]);
        let git = combined_pem(f.to_str().unwrap(), &dir).unwrap();
        let text = std::fs::read_to_string(&git).unwrap();
        assert!(text.starts_with("-----BEGIN CERTIFICATE-----\nMIIBAAEB"), "{text}");
        assert!(text.lines().all(|l| l.len() <= 64));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Testler için üretilmiş kendinden imzalı CA ("Sirket Ic CA"); gizli bir şey değil.
    const TEST_CA: &str = "-----BEGIN CERTIFICATE-----\nMIIDDzCCAfegAwIBAgIUAuVx5KO+HR90a3zB8xwhb3jNR4MwDQYJKoZIhvcNAQEL\nBQAwFzEVMBMGA1UEAwwMU2lya2V0IEljIENBMB4XDTI2MTEwMjExNDgxM1oXDTI2\nMTIwMjExNDgxM1owFzEVMBMGA1UEAwwMU2lya2V0IEljIENBMIIBIjANBgkqhkiG\n9w0BAQEFAAOCAQ8AMIIBCgKCAQEApkeu5VMc0cSCcSnyozrWpxcv3nqv2yxyUELI\nmOeusuJc6IfR4yiSv6oeB0P6YImhHSo+w7kG867BZbCm2PY/lEUjf+6+SMBfwpvT\nfMiLxqSp67isizy4lMv2K7kRDRj/m/2ESTF2i86mbKT0mRLhbr3YwSVGaA7Ntv4K\nnR0ZRMS98aHm4cbG2HwHUQq0C0VNKVmaNVqjxbs64EIz17a1HD/YXT4Jii7ZvhxX\nXYh9vOVuJQTrTWr2pbTJI8JKGMICNDXr89YehCykjGCK8fQO09hSiFOTM2hX+4Lt\nR9Ik8EDw8pMuxgkcOI6a5qMmCe/k/rjw/ojimuDhDOwTzU2hiQIDAQABo1MwUTAd\nBgNVHQ4EFgQU7wLjS7AOPLlBqNKWIX87oCWfX1owHwYDVR0jBBgwFoAU7wLjS7AO\nPLlBqNKWIX87oCWfX1owDwYDVR0TAQH/BAUwAwEB/zANBgkqhkiG9w0BAQsFAAOC\nAQEAGkRQzHbvqeaRbcgticDNXipPX9HRDqGvhS64tKEOv6gzIPArr3eYnwGi1Kv8\n2dZuGX5NShGlz6xIRNaeS2iw6Lc83QYwCQP+nWnh3Dj9Cm1v5eVR+qYlKUuVWHqt\nR351gyVyS6L7TfEcJoEjyjQPxrRRj2+Uo4n1EPjZJJtiSIdHNdOicz18TGdEU2lb\njWPeUkIvI4L/SfLlZwQkFqO3+7iCw+fx/Ve1DTqyl0Et5nQVG1b8RgRiXBMXRWmy\nHwSRGtykmNdT0dKDD35N8ay8XvZanB4umw4IbJ1doqMRkVaIecHZGrCwqrTRi3ek\njkl7C3sJpkxrktWSoUWpuLYiaw==\n-----END CERTIFICATE-----\n";

    #[test]
    fn test_sw_9_1_der_cert_converted_back_to_same_pem_for_git() {
        let dir = std::env::temp_dir().join(format!("divan-tls-git-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pem_file = dir.join("ca.pem");
        std::fs::write(&pem_file, TEST_CA).unwrap();
        let der = load_pem(pem_file.to_str().unwrap()).unwrap()[0].der().to_vec();
        let der_file = dir.join("ca.cert");
        std::fs::write(&der_file, &der).unwrap();
        assert_eq!(load_pem(der_file.to_str().unwrap()).unwrap()[0].der(), &der[..], "DER .cert okunur");
        // DER .cert tek PEM'e çevrilir ve OpenSSL çıktısıyla aynıdır
        let out = dir.join("cikti");
        let git = combined_pem(der_file.to_str().unwrap(), &out).unwrap();
        assert_eq!(std::fs::read_to_string(&git).unwrap(), TEST_CA);
        // klasör: içindeki bütün sertifikalar (PEM + DER), sertifika olmayan dosya atlanır
        std::fs::write(dir.join("not.txt"), "sertifika değil").unwrap();
        assert_eq!(load_pem(dir.to_str().unwrap()).unwrap().len(), 2);
        let both = std::fs::read_to_string(combined_pem(dir.to_str().unwrap(), &out).unwrap()).unwrap();
        assert_eq!(both, format!("{TEST_CA}{TEST_CA}"));
        let bos = dir.join("bos");
        std::fs::create_dir_all(&bos).unwrap();
        assert!(load_pem(bos.to_str().unwrap()).unwrap_err().contains("klasörde"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_sw_9_1_ca_file_errors_are_clear() {
        let dir = std::env::temp_dir().join(format!("divan-tls-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let bos = dir.join("bos.pem");
        std::fs::write(&bos, "sertifika değil").unwrap();
        assert!(load_pem(bos.to_str().unwrap()).unwrap_err().contains("sertifika bulunamadı"));
        assert!(load_pem(dir.join("yok.pem").to_str().unwrap()).unwrap_err().contains("okunamadı"));
        assert!(hint("io: invalid peer certificate: UnknownIssuer", "DIVAN_CA_DOSYASI").contains("DIVAN_CA_DOSYASI"));
        assert!(hint("Connection refused", "X").is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }
}
