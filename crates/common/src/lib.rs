//! stajproje ve stajci'nin ortak kodu: Allure modeli, API tipleri, sürüm bilgisi.

pub mod allure;
pub mod api;

/// stajci ile sunucunun uyumlu olması gereken protokol sürümü (SW-10.9).
/// stajci her istekte `X-Stajci-Version` başlığında kendi paket sürümünü gönderir; sunucu
/// major.minor eşleşmezse isteği reddeder.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Sürüm uyumu: major ve minor aynı olmalı.
pub fn versions_compatible(a: &str, b: &str) -> bool {
    fn mm(v: &str) -> Option<(&str, &str)> {
        let mut it = v.trim().split('.');
        Some((it.next()?, it.next()?))
    }
    matches!((mm(a), mm(b)), (Some(x), Some(y)) if x == y)
}

pub const VERSION_HEADER: &str = "x-stajci-version";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_10_9_version_compat_rule() {
        assert!(versions_compatible("0.1.0", "0.1.7"));
        assert!(!versions_compatible("0.1.0", "0.2.0"));
        assert!(!versions_compatible("1.1.0", "0.1.0"));
        assert!(!versions_compatible("", "0.1.0"));
    }
}
