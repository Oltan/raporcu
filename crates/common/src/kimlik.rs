//! Senaryo tag'lerinden kimlikler (SW-2.7, SW-6.4).
//!
//! - **Gereksinim kimliği**: tek önek (`GEREKSINIM_ONEKI`, varsayılan `DOORS-`; sahada `YTET_`).
//!   `doors_ids` sütununa yazılır; rozet, arama, CSV, link şablonu ve Jira yer tutucusu bunu kullanır.
//! - **Ek kimlikler**: `EK_KIMLIK_ONEKLERI` (örn. `YGÖ-`). Senaryonun ek olarak bağlandığı
//!   gereksinimi gösterir; saklanan tag'lerden okunur, `doors_ids`'e girmez.
//!
//! Önek süreç başında bir kez ayarlanır (`ayarla`); okuma `gecerli()` ile. Saf yöntemler
//! (`Onekler::…`) testlerde genel ayara dokunmadan kullanılır.

use std::sync::RwLock;

pub const VARSAYILAN_ONEK: &str = "DOORS-";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Onekler {
    /// Gereksinim kimliği öneki (`DOORS-`, `YTET_`).
    pub gereksinim: String,
    /// Ek kimlik önekleri (`YGÖ-`), yazıldığı sırayla.
    pub ek: Vec<String>,
    /// `@ABC-1234` biçimindeki tag'in Jira issue'su sayılacağı proje anahtarları (`JIRA_TAG_PROJELERI`,
    /// yoksa `JIRA_PROJECT`). Boşsa yalnızca `@JIRA-ABC-12` biçimi tanınır.
    pub jira_projeleri: Vec<String>,
}

impl Default for Onekler {
    fn default() -> Self {
        Onekler {
            gereksinim: VARSAYILAN_ONEK.into(),
            ek: vec![],
            jira_projeleri: vec![],
        }
    }
}

/// Önekten görünen ad: sondaki `-`/`_` atılır (`YTET_` → `YTET`, `YGÖ-` → `YGÖ`).
pub fn ad(onek: &str) -> String {
    let a = onek.trim_end_matches(['-', '_']);
    if a.is_empty() {
        onek.to_string()
    } else {
        a.to_string()
    }
}

fn kimlik_karakteri(c: char) -> bool {
    c.is_alphanumeric() || c == '-' || c == '_'
}

/// `tag` `onek` ile başlıyorsa ve devamı kimlik karakterlerinden oluşuyorsa tag'in kendisi.
/// Önek büyük/küçük harfe duyarlı; kimlik yazıldığı gibi döner.
fn onekli(tag: &str, onek: &str) -> Option<String> {
    let t = tag.trim_start_matches('@');
    let rest = t.strip_prefix(onek)?;
    (!rest.is_empty() && rest.chars().all(kimlik_karakteri)).then(|| t.to_string())
}

impl Onekler {
    /// `.env` değerlerinden. Boş gereksinim öneki varsayılana döner; ek önekler virgülle
    /// ayrılır, gereksinim önekiyle aynı olan ya da tekrarlanan atılır.
    pub fn parse(gereksinim: Option<&str>, ek: Option<&str>, jira_projeleri: Option<&str>) -> Onekler {
        let gereksinim = gereksinim
            .map(|g| g.trim().trim_start_matches('@').to_string())
            .filter(|g| !g.is_empty())
            .unwrap_or_else(|| VARSAYILAN_ONEK.into());
        let mut ekler: Vec<String> = Vec::new();
        for e in ek.unwrap_or("").split(',') {
            let e = e.trim().trim_start_matches('@').to_string();
            if !e.is_empty() && e != gereksinim && !ekler.contains(&e) {
                ekler.push(e);
            }
        }
        let mut jira: Vec<String> = Vec::new();
        for p in jira_projeleri.unwrap_or("").split(',') {
            let p = p.trim().to_ascii_uppercase();
            let gecerli = p.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                && p.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
            if gecerli && !jira.contains(&p) {
                jira.push(p);
            }
        }
        Onekler {
            gereksinim,
            ek: ekler,
            jira_projeleri: jira,
        }
    }

    /// Tag Jira issue anahtarı mı: `@JIRA-ABC-12` (her proje) ya da `@ABC-12` (ABC `jira_projeleri`'nde).
    /// Gereksinim/ek kimlik olan tag (ör. `DOORS-1003`) Jira sayılmaz. Dönen: `ABC-12`.
    pub fn jira_anahtari(&self, tag: &str) -> Option<String> {
        let t = tag.trim_start_matches('@');
        let ayir = |s: &str| -> Option<(String, String)> {
            let (proj, num) = s.rsplit_once('-')?;
            (!proj.is_empty()
                && proj.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && !num.is_empty()
                && num.chars().all(|c| c.is_ascii_digit()))
            .then(|| (proj.to_ascii_uppercase(), num.to_string()))
        };
        if let Some(rest) = t.strip_prefix("JIRA-").or_else(|| t.strip_prefix("jira-")) {
            return ayir(rest).map(|(p, n)| format!("{p}-{n}"));
        }
        if self.jira_projeleri.is_empty() || self.kimlik_mi(t) {
            return None;
        }
        let (p, n) = ayir(t)?;
        self.jira_projeleri.contains(&p).then(|| format!("{p}-{n}"))
    }

    fn eski_doors(&self) -> bool {
        self.gereksinim == VARSAYILAN_ONEK
    }

    /// Gereksinim kimliğinin görünen adı (`YTET`, varsayılanda `DOORS`).
    pub fn gereksinim_adi(&self) -> String {
        ad(&self.gereksinim)
    }

    /// Tag gereksinim kimliği mi? `DOORS-` için eski davranış korunur (küçük harfli `doors-`
    /// de tanınır, `DOORS-` olarak saklanır); diğer önekler yazıldığı gibi.
    pub fn gereksinim_id(&self, tag: &str) -> Option<String> {
        if self.eski_doors() {
            let t = tag.trim_start_matches('@');
            let rest = t.strip_prefix("DOORS-").or_else(|| t.strip_prefix("doors-"))?;
            return (!rest.is_empty() && rest.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
                .then(|| format!("DOORS-{rest}"));
        }
        onekli(tag, &self.gereksinim)
    }

    /// Kimlikten numara (link şablonundaki `{{no}}`): önek atılır.
    pub fn numara<'a>(&self, id: &'a str) -> &'a str {
        id.strip_prefix(self.gereksinim.as_str()).unwrap_or(id)
    }

    /// Tag ek kimlik mi: (önek sırası, kimlik).
    pub fn ek_kimlik(&self, tag: &str) -> Option<(usize, String)> {
        self.ek.iter().enumerate().find_map(|(i, o)| onekli(tag, o).map(|k| (i, k)))
    }

    /// Her ek önek için o önekli kimlikler (önek sırasıyla).
    pub fn ek_kimlikler<S: AsRef<str>>(&self, tags: &[S]) -> Vec<Vec<String>> {
        let mut out = vec![Vec::new(); self.ek.len()];
        for t in tags {
            if let Some((i, k)) = self.ek_kimlik(t.as_ref()) {
                out[i].push(k);
            }
        }
        out
    }

    /// Tag özel kimlik mi (gereksinim ya da ek): düz tag listesinden ayrı gösterilir.
    pub fn kimlik_mi(&self, tag: &str) -> bool {
        self.gereksinim_id(tag).is_some() || self.ek_kimlik(tag).is_some()
    }

    /// Arama kutusuna yazılan değer: tanınan önekle başlıyorsa olduğu gibi (`DOORS-` için büyük
    /// harf), değilse gereksinim öneki eklenir (`1234` → `YTET_1234`).
    pub fn arama(&self, girdi: &str) -> String {
        let g = girdi.trim().trim_start_matches('@');
        if self.eski_doors() && g.to_uppercase().starts_with("DOORS-") {
            return g.to_uppercase();
        }
        if g.starts_with(self.gereksinim.as_str()) || self.ek.iter().any(|o| g.starts_with(o.as_str())) {
            return g.to_string();
        }
        format!("{}{g}", self.gereksinim)
    }

    /// Arama değeri ek önekli mi (o zaman `doors_ids` yerine tag'lerde aranır).
    pub fn ek_onekli(&self, deger: &str) -> bool {
        self.ek.iter().any(|o| deger.starts_with(o.as_str()))
    }
}

static GENEL: RwLock<Option<Onekler>> = RwLock::new(None);

/// Süreç başında bir kez (divan `AppState::new`).
pub fn ayarla(o: Onekler) {
    *GENEL.write().unwrap_or_else(|e| e.into_inner()) = Some(o);
}

/// Geçerli önekler (ayarlanmadıysa varsayılan: yalnızca `DOORS-`).
pub fn gecerli() -> Onekler {
    GENEL.read().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saha() -> Onekler {
        Onekler::parse(Some("YTET_"), Some("YGÖ-, YTET_ ,YGÖ-"), Some("STAJ, abc,1x"))
    }

    #[test]
    fn test_sw_2_7_gereksinim_oneki_ytet() {
        let o = saha();
        assert_eq!(o.ek, vec!["YGÖ-"], "tekrar ve gereksinim öneki ek listesinden atılır");
        assert_eq!(o.gereksinim_adi(), "YTET");
        assert_eq!(o.gereksinim_id("@YTET_1234"), Some("YTET_1234".into()));
        assert_eq!(o.gereksinim_id("YTET_12a"), Some("YTET_12a".into()));
        assert_eq!(o.gereksinim_id("ytet_1234"), None, "önek büyük/küçük harfe duyarlı");
        assert_eq!(o.gereksinim_id("YTET_"), None);
        assert_eq!(o.gereksinim_id("@DOORS-1003"), None, "DOORS- artık tanınmaz");
        assert_eq!(o.numara("YTET_1234"), "1234");
    }

    #[test]
    fn test_sw_2_7_ek_kimlik_ascii_olmayan_onek() {
        let o = saha();
        assert_eq!(o.ek_kimlik("@YGÖ-12"), Some((0, "YGÖ-12".into())));
        assert_eq!(o.ek_kimlik("@YGÖ-12345678"), Some((0, "YGÖ-12345678".into())));
        assert_eq!(o.ek_kimlik("@ygö-12"), None);
        assert_eq!(o.ek_kimlik("@YTET_1"), None, "gereksinim kimliği ek sayılmaz");
        assert_eq!(ad("YGÖ-"), "YGÖ");
        let tags = ["giris", "YGÖ-12", "YTET_1", "YGÖ-40"];
        assert_eq!(o.ek_kimlikler(&tags), vec![vec!["YGÖ-12".to_string(), "YGÖ-40".to_string()]]);
        assert!(o.kimlik_mi("YTET_1") && o.kimlik_mi("YGÖ-12") && !o.kimlik_mi("giris"));
    }

    #[test]
    fn test_sw_4_7_arama_onek_normallestirme() {
        let o = saha();
        assert_eq!(o.arama("1234"), "YTET_1234");
        assert_eq!(o.arama("@YTET_1234"), "YTET_1234");
        assert_eq!(o.arama("YGÖ-12"), "YGÖ-12");
        assert!(o.ek_onekli("YGÖ-12") && !o.ek_onekli("YTET_1"));
        let d = Onekler::default();
        assert_eq!(d.arama("doors-1003"), "DOORS-1003");
        assert_eq!(d.arama("1003"), "DOORS-1003");
    }

    #[test]
    fn test_sw_6_2_jira_tag_anahtari_listeye_gore() {
        let o = saha();
        assert_eq!(o.jira_projeleri, vec!["STAJ", "ABC"], "geçersiz (1x) atılır, büyük harfe çevrilir");
        assert_eq!(o.jira_anahtari("@STAJ-42"), Some("STAJ-42".into()));
        assert_eq!(o.jira_anahtari("abc-7"), Some("ABC-7".into()));
        assert_eq!(o.jira_anahtari("@REL-2"), None, "listede olmayan proje");
        assert_eq!(o.jira_anahtari("@STAJ-4a"), None);
        assert_eq!(o.jira_anahtari("@JIRA-XYZ-9"), Some("XYZ-9".into()), "eski biçim her projede");
        let doors = Onekler::parse(None, None, Some("DOORS"));
        assert_eq!(doors.jira_anahtari("DOORS-1003"), None, "gereksinim kimliği Jira sayılmaz");
        assert_eq!(
            Onekler::default().jira_anahtari("@STAJ-42"),
            None,
            "liste boşsa yalnızca JIRA- biçimi"
        );
    }

    #[test]
    fn test_sw_2_7_varsayilan_doors_davranisi_aynen() {
        let d = Onekler::parse(None, Some(""), None);
        assert_eq!(d, Onekler::default());
        assert_eq!(d.gereksinim_adi(), "DOORS");
        assert_eq!(d.gereksinim_id("@doors-1003"), Some("DOORS-1003".into()));
        assert_eq!(d.gereksinim_id("DOORS-ç1"), None);
        assert_eq!(d.numara("DOORS-1003"), "1003");
    }
}
