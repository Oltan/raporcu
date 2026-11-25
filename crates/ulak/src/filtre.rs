//! Tek senaryoyu seçen Cucumber filtresi (SW-7.2): doğrulama koşusu (`ulak fix`) ve auto-fix
//! branch'inin pipeline'ı (`ULAK_SENARYO` / `ULAK_TAGS`) aynı kuralı kullanır.
//!
//! - Ad: `-Dcucumber.filter.name=^<ad>$` (düzenli ifade; `fix::regex_quote`).
//! - Tag: `-Dcucumber.filter.tags=@a and @b`. Yalnızca ASCII tag'ler: runner ASCII dil ayarında
//!   Java komut satırındaki Türkçe karakterleri bozar ve tag ifadesinde joker yoktur
//!   (`regex_quote`'taki `.` çaresi işlemez). Tag'de boşluk/parantez varsa o tag atılır.

/// Senaryo filtresi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Filtre {
    /// Senaryo adı (düz metin; argümanda düzenli ifadeye çevrilir).
    Ad(String),
    /// Cucumber tag ifadesi (`@YTET_1234 and @giris`).
    Taglar(String),
}

impl Filtre {
    /// Maven'a verilecek sistem özelliği.
    pub fn arguman(&self) -> String {
        match self {
            Filtre::Ad(ad) => format!("-Dcucumber.filter.name={}", ad_ifadesi(ad)),
            Filtre::Taglar(t) => format!("-Dcucumber.filter.tags={t}"),
        }
    }
}

/// `^<ad>$`, düzenli ifade karakterleri kaçırılmış, ASCII olmayan harfler `.`.
pub fn ad_ifadesi(ad: &str) -> String {
    format!("^{}$", crate::fix::regex_quote(ad))
}

fn tag_kullanilir(t: &str) -> bool {
    !t.is_empty() && t.chars().all(|c| c.is_ascii_graphic() && !matches!(c, '(' | ')' | '@' | '\\'))
}

/// Tag'lerden `and` ifadesi (`@` eklenir). Kullanılabilir tag yoksa `None`.
pub fn tag_ifadesi<S: AsRef<str>>(tags: &[S]) -> Option<String> {
    let mut v: Vec<String> = Vec::new();
    for t in tags {
        let t = t.as_ref().trim().trim_start_matches('@');
        if tag_kullanilir(t) && !v.iter().any(|x| x[1..] == *t) {
            v.push(format!("@{t}"));
        }
    }
    (!v.is_empty()).then(|| v.join(" and "))
}

/// Atılan (ASCII olmayan ya da ifadeye giremeyen) tag'ler; uyarı için.
pub fn atilan_taglar<S: AsRef<str>>(tags: &[S]) -> Vec<String> {
    tags.iter()
        .map(|t| t.as_ref().trim().trim_start_matches('@').to_string())
        .filter(|t| !t.is_empty() && !tag_kullanilir(t))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_7_2_tag_ifadesi_ascii_disi_atilir() {
        let tags = ["giris", "@YTET_1234", "YGÖ-12", "giris", "a(b)"];
        assert_eq!(tag_ifadesi(&tags).unwrap(), "@giris and @YTET_1234");
        assert_eq!(atilan_taglar(&tags), vec!["YGÖ-12", "a(b)"]);
        assert_eq!(tag_ifadesi(&["YGÖ-12"]), None);
        assert_eq!(tag_ifadesi::<&str>(&[]), None);
    }

    #[test]
    fn test_sw_7_2_filtre_argumanlari() {
        assert_eq!(
            Filtre::Ad("Geçersiz (eski) şifre".into()).arguman(),
            "-Dcucumber.filter.name=^Ge.ersiz \\(eski\\) .ifre$"
        );
        assert_eq!(
            Filtre::Taglar("@YTET_1 and @giris".into()).arguman(),
            "-Dcucumber.filter.tags=@YTET_1 and @giris"
        );
    }
}
