//! Arayüzden düzenlenen ayarlar: Jira bug şablonu (SW-6.1) ve DOORS bağlantısı (SW-6.4).
//!
//! Değerler `settings` tablosunda JSON olarak durur; sunucu açılışta okur, admin kaydedince
//! bellekteki kopya da güncellenir. Kayıt yoksa varsayılanlar geçerlidir.

use crate::db::now;
use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

/// Jira bug'ının içeriği. Metinlerde `{{ad}}` yer tutucuları kullanılır (`yer_tutucular`).
/// Tek süslü parantez (`{noformat}`) Jira biçimidir, olduğu gibi kalır.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct JiraSablon {
    pub ozet: String,
    pub aciklama: String,
    /// Jira'ya eklenecek ek alanlar: `{"priority": {"name": "Major"}, "customfield_10200": {"value": "Yazılım"}}`.
    /// Zorunlu alanlar buraya yazılır. Metin değerlerde yer tutucu kullanılabilir.
    pub alanlar: String,
    /// DOORS numaraları label olarak eklensin mi.
    pub doors_label: bool,
}

impl Default for JiraSablon {
    fn default() -> Self {
        JiraSablon {
            ozet: "[Otomasyon] {{senaryo}}: {{hata_ilk_satir}}".into(),
            aciklama: "Otomatik test senaryosu kaldı: *{{senaryo}}*\n\nFeature: {{feature}}\nDOORS: {{doors}}\nProje: {{proje}} · Branch: {{branch}} · Commit: {{commit}}\nRapor: {{rapor}}\n\n*Hata*\n{noformat}\n{{hata}}\n{noformat}\n\n*Adımlar*\n{{adimlar}}".into(),
            alanlar: "{}".into(),
            doors_label: true,
        }
    }
}

pub const YER_TUTUCULAR: &[(&str, &str)] = &[
    ("senaryo", "senaryo adı"),
    ("feature", "feature adı"),
    ("doors", "DOORS numaraları, virgülle"),
    ("hata", "hata mesajı (en fazla 2000 karakter)"),
    ("hata_ilk_satir", "hata mesajının ilk satırı (120)"),
    ("adimlar", "numaralı adım listesi (Jira biçimi: # adım)"),
    ("rapor", "senaryonun rapor linki"),
    ("kosum_rapor", "koşum rapor linki"),
    ("proje", "GitLab proje yolu"),
    ("branch", "branch"),
    ("commit", "commit (kısa)"),
    ("kosum", "koşum numarası"),
    ("tarih", "koşum tarihi"),
];

/// DOORS bağlantısı. Test senaryoları `@DOORS-<no>` tag'i taşır; `<no>` DOORS nesnesinin
/// `id_oznitelik` değeridir (varsayılan Absolute Number).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DoorsAyar {
    /// DXL'in açacağı modül, ör. `/Proje/Gereksinimler/SRS`. Boşsa DXL o an açık modülde çalışır.
    pub modul: String,
    pub id_oznitelik: String,
    pub sonuc_oznitelik: String,
    pub tarih_oznitelik: String,
    pub jira_oznitelik: String,
    pub rapor_oznitelik: String,
    /// Sonuç özniteliğine yazılacak değerler (DOORS'taki enum değerleriyle birebir aynı olmalı).
    pub gecti: String,
    pub kaldi: String,
    pub atlandi: String,
    pub kosulmadi: String,
    /// Rapordaki DOORS rozetinin linki, ör. `doors://doors.sirket.local:36677/?version=2&prodID=0&urn=...{{no}}`.
    /// `{{no}}` numara, `{{id}}` tam kimlik (DOORS-1003). Boşsa rozet link olmaz.
    pub link_sablonu: String,
}

impl Default for DoorsAyar {
    fn default() -> Self {
        DoorsAyar {
            modul: String::new(),
            id_oznitelik: "Absolute Number".into(),
            sonuc_oznitelik: "Test Sonucu".into(),
            tarih_oznitelik: "Test Tarihi".into(),
            jira_oznitelik: "Jira".into(),
            rapor_oznitelik: "Test Raporu".into(),
            gecti: "Geçti".into(),
            kaldi: "Kaldı".into(),
            atlandi: "Atlandı".into(),
            kosulmadi: "Koşulmadı".into(),
            link_sablonu: String::new(),
        }
    }
}

impl DoorsAyar {
    pub fn status(&self, allure: &str) -> &str {
        match allure {
            "passed" => &self.gecti,
            "failed" | "broken" => &self.kaldi,
            "skipped" => &self.atlandi,
            _ => &self.kosulmadi,
        }
    }

    pub fn link(&self, doors_id: &str) -> Option<String> {
        if self.link_sablonu.trim().is_empty() {
            return None;
        }
        let no = doors_id.trim_start_matches("DOORS-");
        Some(self.link_sablonu.replace("{{no}}", no).replace("{{id}}", doors_id))
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ayarlar {
    pub jira: JiraSablon,
    pub doors: DoorsAyar,
}

/// Bellekteki kopya; okuma her istekte, yazma yalnızca admin kaydedince.
#[derive(Clone, Default)]
pub struct Store(Arc<RwLock<Ayarlar>>);

impl Store {
    pub fn get(&self) -> Ayarlar {
        self.0.read().unwrap().clone()
    }
    pub fn set(&self, a: Ayarlar) {
        *self.0.write().unwrap() = a;
    }
}

fn read_one<T: for<'de> Deserialize<'de> + Default>(c: &Connection, key: &str) -> Result<T> {
    let v: Option<String> = c
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
        .optional()?;
    Ok(match v {
        Some(s) => serde_json::from_str(&s).with_context(|| format!("ayar okunamadı: {key}"))?,
        None => T::default(),
    })
}

pub fn load(c: &Connection) -> Result<Ayarlar> {
    Ok(Ayarlar {
        jira: read_one(c, "jira")?,
        doors: read_one(c, "doors")?,
    })
}

fn write_one<T: Serialize>(c: &Connection, key: &str, v: &T) -> Result<()> {
    c.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![key, serde_json::to_string(v)?, now()],
    )?;
    Ok(())
}

pub fn save_jira(c: &Connection, j: &JiraSablon) -> Result<()> {
    extra_fields(&j.alanlar)?;
    write_one(c, "jira", j)
}

pub fn save_doors(c: &Connection, d: &DoorsAyar) -> Result<()> {
    if d.id_oznitelik.trim().is_empty() {
        bail!("DOORS kimlik özniteliği boş olamaz");
    }
    write_one(c, "doors", d)
}

/// Ek alan metnini doğrular: boş ya da JSON nesnesi olmalı.
pub fn extra_fields(text: &str) -> Result<Map<String, Value>> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(Map::new());
    }
    match serde_json::from_str::<Value>(t) {
        Ok(Value::Object(m)) => Ok(m),
        Ok(_) => bail!("Ek alanlar bir JSON nesnesi olmalı: {{\"alan_id\": değer, ...}}"),
        Err(e) => bail!("Ek alanlar geçerli JSON değil: {e}"),
    }
}

/// `{{ad}}` yer tutucularını doldurur; bilinmeyenler olduğu gibi kalır.
pub fn fill(text: &str, vars: &BTreeMap<&str, String>) -> String {
    let mut out = text.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{{{{k}}}}}"), v);
    }
    out
}

/// JSON değerinin içindeki metinlerde yer tutucuları doldurur (yapı ve kaçışlar bozulmaz).
pub fn fill_json(v: &Value, vars: &BTreeMap<&str, String>) -> Value {
    match v {
        Value::String(s) => Value::String(fill(s, vars)),
        Value::Array(a) => Value::Array(a.iter().map(|x| fill_json(x, vars)).collect()),
        Value::Object(m) => Value::Object(m.iter().map(|(k, x)| (k.clone(), fill_json(x, vars))).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_6_1_template_fill_keeps_jira_braces_and_json_structure() {
        let mut v = BTreeMap::new();
        v.insert("senaryo", "Giriş \"özel\"".to_string());
        assert_eq!(
            fill("{noformat}{{senaryo}}{noformat} {{yok}}", &v),
            "{noformat}Giriş \"özel\"{noformat} {{yok}}"
        );
        let f = extra_fields(r#"{"customfield_1": {"value": "{{senaryo}}"}, "labels": ["a"]}"#).unwrap();
        let out = fill_json(&Value::Object(f), &v);
        assert_eq!(out["customfield_1"]["value"], "Giriş \"özel\"");
        assert!(extra_fields("[1]").is_err());
        assert!(extra_fields("{bozuk").is_err());
        assert!(extra_fields("  ").unwrap().is_empty());
    }

    #[test]
    fn test_sw_6_4_doors_link_and_status_labels() {
        let mut d = DoorsAyar::default();
        assert_eq!(d.link("DOORS-1003"), None);
        d.link_sablonu = "doors://srv:36677/?urn=x-{{no}}&k={{id}}".into();
        assert_eq!(d.link("DOORS-1003").unwrap(), "doors://srv:36677/?urn=x-1003&k=DOORS-1003");
        d.kaldi = "Failed".into();
        assert_eq!(d.status("broken"), "Failed");
        assert_eq!(d.status("unknown"), "Koşulmadı");
    }
}
