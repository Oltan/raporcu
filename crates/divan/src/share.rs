//! SW-5.1: iç bilgileri gizlenmiş salt okunur paylaşım. Token, Jira anahtarı ve ham ek linki
//! görünmez; ekler paylaşım tokeni üzerinden sunulur.

/// Genel rapordaki sınıf. Link açan yalnızca doğrulanan ve ürün hatası olan senaryoları görür;
/// test kodu, ortam, flaky ve tekrar denemesi gibi iç konular adsız "inceleniyor" sayısına girer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenelSinif {
    Dogrulandi,
    UrunHatasi,
    Inceleniyor,
}

pub fn genel_sinif(status: &str, attempts: i64, flaky: bool, triage: Option<&str>) -> GenelSinif {
    if flaky {
        return GenelSinif::Inceleniyor;
    }
    match status {
        "passed" if attempts == 1 => GenelSinif::Dogrulandi,
        "failed" | "broken" if triage == Some("urun") => GenelSinif::UrunHatasi,
        _ => GenelSinif::Inceleniyor,
    }
}

/// Metinden gizli değerleri ve Jira anahtarı biçimindeki kelimeleri çıkarır.
pub fn sanitize_text(s: &str, secrets: &[String]) -> String {
    let s = crate::config::redact(s, secrets);
    let mut out = String::with_capacity(s.len());
    let mut hide_next = false;
    for (i, word) in s.split_inclusive(char::is_whitespace).enumerate() {
        let core = word.trim_end();
        let tail = &word[core.len()..];
        let lower = core.to_lowercase();
        let secretish = lower.starts_with("glpat-")
            || lower.starts_with("glrt-")
            || lower.starts_with("glptt-")
            || lower.contains("token=")
            || lower.contains("password=")
            || lower.contains("private-token")
            || lower.contains("api_key=");
        if hide_next {
            out.push_str("***");
            out.push_str(tail);
            hide_next = false;
            continue;
        }
        if lower == "bearer" || lower == "authorization:" {
            hide_next = true;
            out.push_str(word);
            continue;
        }
        if secretish {
            out.push_str("***");
        } else if is_jira_key(core.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-')) {
            out.push_str("[Jira]");
        } else {
            out.push_str(core);
        }
        out.push_str(tail);
        let _ = i;
    }
    out
}

/// `ABC-123` biçimi (DOORS-123 hariç; DOORS numaraları paylaşımda görünür).
pub fn is_jira_key(w: &str) -> bool {
    let Some((p, n)) = w.split_once('-') else { return false };
    p.len() >= 2
        && p != "DOORS"
        && p.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && p.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        && !n.is_empty()
        && n.chars().all(|c| c.is_ascii_digit())
}

pub fn visible_tag(t: &str) -> bool {
    common::allure::jira_key(t).is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_5_1_genel_sinif() {
        assert_eq!(genel_sinif("passed", 1, false, None), GenelSinif::Dogrulandi);
        // tekrar denemede geçen ve flaky olan doğrulanmış sayılmaz
        assert_eq!(genel_sinif("passed", 2, false, None), GenelSinif::Inceleniyor);
        assert_eq!(genel_sinif("passed", 1, true, None), GenelSinif::Inceleniyor);
        // iki denemede de kalan ürün hatası görünür kalır
        assert_eq!(genel_sinif("failed", 2, false, Some("urun")), GenelSinif::UrunHatasi);
        assert_eq!(genel_sinif("failed", 2, true, Some("urun")), GenelSinif::Inceleniyor);
        for t in [Some("test_kodu"), Some("ortam"), Some("bilinmiyor"), None] {
            assert_eq!(genel_sinif("broken", 1, false, t), GenelSinif::Inceleniyor);
        }
        assert_eq!(genel_sinif("skipped", 1, false, None), GenelSinif::Inceleniyor);
    }

    #[test]
    fn test_sw_5_1_text_sanitized() {
        let s = sanitize_text(
            "Bkz STAJ-42, token=abc123 Bearer eyJhbGci glpat-XYZ DOORS-1003 ok",
            &["gizli-deger".into()],
        );
        assert!(!s.contains("STAJ-42"));
        assert!(!s.contains("abc123"));
        assert!(!s.contains("eyJhbGci"));
        assert!(!s.contains("glpat-XYZ"));
        assert!(s.contains("DOORS-1003"));
        assert!(s.contains("ok"));
        assert_eq!(sanitize_text("x gizli-deger y", &["gizli-deger".into()]), "x *** y");
    }
}
