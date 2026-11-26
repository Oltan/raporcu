//! Dışa açık raporlar (SW-5.1, 0.2.0): admin bir repo + yazılım adı + sürüm için koşumları seçip
//! yayınlar. Seçili koşumlar tek raporda birleşir: aynı test birden çok koşumda varsa en son başlayan
//! koşumdaki sonuç geçerli. Feature dosyası olmayan sonuçlar (JUnit/ortam yoklama testleri) ve admin'in
//! çıkardığı testler rapora girmez. Sınıf yalnızca iki: geçen (tekrar denemede geçen de) ve kalan.

use crate::db::now;
use crate::jira::IssueOzet;
use crate::queries::{self, RunRow, ScenarioRow};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::{HashMap, HashSet};

/// Koşumun yazılım adı: `UYGULAMA_ADI`, yoksa proje yolunun son parçası.
pub fn yazilim_adi(run: &RunRow) -> String {
    match run.app_name.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(a) => a.to_string(),
        None => proje_adi(&run.project_path),
    }
}

pub fn proje_adi(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sonuc {
    Gecen,
    Kalan,
}

impl Sonuc {
    pub fn etiket(self) -> &'static str {
        match self {
            Sonuc::Gecen => "Geçti",
            Sonuc::Kalan => "Kaldı",
        }
    }
    /// Grafik ve nokta sınıfı.
    pub fn sinif(self) -> &'static str {
        match self {
            Sonuc::Gecen => "gecti",
            Sonuc::Kalan => "kaldi",
        }
    }
}

/// Senaryo durumundan rapor sınıfı; koşulmamış (atlandı/bilinmiyor) sayılmaz.
pub fn sonuc(status: &str) -> Option<Sonuc> {
    match status {
        "passed" => Some(Sonuc::Gecen),
        "failed" | "broken" => Some(Sonuc::Kalan),
        _ => None,
    }
}

/// Toplam / geçen / kalan.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Sayim {
    pub gecen: i64,
    pub kalan: i64,
}

impl Sayim {
    pub fn toplam(&self) -> i64 {
        self.gecen + self.kalan
    }
    /// Başarı oranı (%).
    pub fn oran(&self) -> i64 {
        crate::charts::pct(self.gecen, self.toplam())
    }
    pub fn ekle(&mut self, s: Sonuc) {
        match s {
            Sonuc::Gecen => self.gecen += 1,
            Sonuc::Kalan => self.kalan += 1,
        }
    }
    pub fn dilimler(&self) -> Vec<crate::charts::Slice> {
        crate::charts::genel_slices(self.gecen, self.kalan)
    }
    pub fn cubuk(&self) -> String {
        crate::charts::thin_bar(&self.dilimler())
    }
}

pub struct Test {
    pub s: ScenarioRow,
    pub sonuc: Sonuc,
    /// test case'e bağlı Jira issue'ları (paylaşımda linksiz gösterilir)
    pub jira: Vec<IssueOzet>,
}

impl Test {
    pub fn sure(&self) -> String {
        match self.s.duration_ms {
            Some(ms) if ms >= 60_000 => format!("{} dk {} sn", ms / 60_000, (ms % 60_000) / 1000),
            Some(ms) => format!("{:.1} sn", ms as f64 / 1000.0),
            None => String::new(),
        }
    }
}

pub struct Ozellik {
    pub ad: String,
    pub anchor: String,
    pub sayim: Sayim,
    pub testler: Vec<Test>,
}

/// Birleştirme sonucu (saf; testler için ayrı).
#[derive(Debug, Default)]
pub struct Birlesim {
    pub senaryolar: Vec<ScenarioRow>,
    /// feature dosyası olmayan (JUnit/yoklama) sonuçlar
    pub cucumber_disi: usize,
    /// koşulmamış (atlandı/bilinmiyor)
    pub kosulmadi: usize,
    pub haric: usize,
}

/// `kosumlar`: (koşum başlangıcı, son denemeler). Başlangıca göre sıralanıp sonra gelen öncekini ezer.
pub fn birlestir(mut kosumlar: Vec<(String, Vec<ScenarioRow>)>, haric: &HashSet<String>) -> Birlesim {
    kosumlar.sort_by(|a, b| a.0.cmp(&b.0));
    let mut sira: Vec<String> = Vec::new();
    let mut son: HashMap<String, ScenarioRow> = HashMap::new();
    let mut out = Birlesim::default();
    let mut disi: HashSet<String> = HashSet::new();
    for (_, senaryolar) in kosumlar {
        for s in senaryolar {
            if s.feature_file.as_deref().is_none_or(|f| f.trim().is_empty()) {
                disi.insert(s.scenario_key.clone());
                continue;
            }
            if !son.contains_key(&s.scenario_key) {
                sira.push(s.scenario_key.clone());
            }
            son.insert(s.scenario_key.clone(), s);
        }
    }
    out.cucumber_disi = disi.len();
    for k in sira {
        let s = son.remove(&k).expect("sıradaki anahtar");
        if haric.contains(&k) {
            out.haric += 1;
        } else if sonuc(&s.status).is_none() {
            out.kosulmadi += 1;
        } else {
            out.senaryolar.push(s);
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct Yayin {
    pub id: i64,
    pub project_id: i64,
    pub project_path: String,
    pub app_name: String,
    pub app_version: String,
    pub created_at: String,
    pub updated_at: String,
}

fn yayin_row(r: &rusqlite::Row) -> rusqlite::Result<Yayin> {
    Ok(Yayin {
        id: r.get(0)?,
        project_id: r.get(1)?,
        project_path: r.get(2)?,
        app_name: r.get(3)?,
        app_version: r.get(4)?,
        created_at: r.get(5)?,
        updated_at: r.get(6)?,
    })
}

const YAYIN_COLS: &str = "y.id, y.project_id, p.path, y.app_name, y.app_version, y.created_at, y.updated_at";

pub fn yayin(c: &Connection, id: i64) -> Result<Option<Yayin>> {
    Ok(c.query_row(
        &format!("SELECT {YAYIN_COLS} FROM yayinlar y JOIN projects p ON p.id = y.project_id WHERE y.id = ?1"),
        [id],
        yayin_row,
    )
    .optional()?)
}

pub fn bul(c: &Connection, project_id: i64, app_name: &str, app_version: &str) -> Result<Option<i64>> {
    Ok(c.query_row(
        "SELECT id FROM yayinlar WHERE project_id = ?1 AND app_name = ?2 AND app_version = ?3",
        params![project_id, app_name, app_version],
        |r| r.get(0),
    )
    .optional()?)
}

pub fn kosum_idleri(c: &Connection, yayin_id: i64) -> Result<Vec<i64>> {
    let mut st = c.prepare("SELECT run_id FROM yayin_kosumlari WHERE yayin_id = ?1 ORDER BY run_id")?;
    let r = st.query_map([yayin_id], |r| r.get(0))?;
    Ok(r.collect::<rusqlite::Result<_>>()?)
}

pub fn haric(c: &Connection, yayin_id: i64) -> Result<HashSet<String>> {
    let mut st = c.prepare("SELECT scenario_key FROM yayin_haric WHERE yayin_id = ?1")?;
    let r = st.query_map([yayin_id], |r| r.get(0))?;
    Ok(r.collect::<rusqlite::Result<_>>()?)
}

/// Aynı yazılımın (aynı proje) en son raporunda çıkarılmış testler: yeni rapora öneri.
pub fn onceki_haric(c: &Connection, project_id: i64, app_name: &str) -> Result<HashSet<String>> {
    let id: Option<i64> = c
        .query_row(
            "SELECT id FROM yayinlar WHERE project_id = ?1 AND app_name = ?2 ORDER BY updated_at DESC LIMIT 1",
            params![project_id, app_name],
            |r| r.get(0),
        )
        .optional()?;
    match id {
        Some(id) => haric(c, id),
        None => Ok(HashSet::new()),
    }
}

/// Bir projenin (yazılım adı, sürüm) üçlülerinin biten koşumları; en yeni önce.
pub fn adaylar(c: &Connection, project_id: i64) -> Result<Vec<(String, String, Vec<RunRow>)>> {
    let mut gruplar: Vec<(String, String, Vec<RunRow>)> = Vec::new();
    for r in queries::project_runs_finished(c, project_id)? {
        let Some(v) = r.app_version.clone().filter(|v| !v.trim().is_empty()) else {
            continue;
        };
        let ad = yazilim_adi(&r);
        match gruplar.iter_mut().find(|g| g.0 == ad && g.1 == v) {
            Some(g) => g.2.push(r),
            None => gruplar.push((ad, v, vec![r])),
        }
    }
    Ok(gruplar)
}

/// Üçlünün biten koşumları (en yeni önce).
pub fn uclu_kosumlari(c: &Connection, project_id: i64, app_name: &str, app_version: &str) -> Result<Vec<RunRow>> {
    Ok(adaylar(c, project_id)?
        .into_iter()
        .find(|g| g.0 == app_name && g.1 == app_version)
        .map(|g| g.2)
        .unwrap_or_default())
}

pub struct Rapor {
    pub yazilim: String,
    pub surum: String,
    pub project_id: i64,
    /// en son koşumun başlangıcı
    pub tarih: String,
    pub sayim: Sayim,
    pub ozellikler: Vec<Ozellik>,
    pub cucumber_disi: usize,
    pub kosulmadi: usize,
    pub haric: usize,
    pub kosum_sayisi: usize,
}

/// Seçili koşumlardan rapor (yayınlanmış ya da admin önizlemesi).
pub fn olustur(c: &Connection, yazilim: &str, surum: &str, kosumlar: &[i64], haric: &HashSet<String>) -> Result<Rapor> {
    let mut girdi = Vec::new();
    let mut tarih = String::new();
    let mut project_id = 0;
    let mut jira: HashMap<i64, Vec<IssueOzet>> = HashMap::new();
    for &id in kosumlar {
        let Some(run) = queries::run(c, id)? else { continue };
        project_id = run.project_id;
        if run.started_at > tarih {
            tarih = run.started_at.clone();
        }
        for o in crate::jira::kosum_issuelari(c, id, None)? {
            for (sid, _, _) in &o.senaryolar {
                jira.entry(*sid).or_default().push(o.clone());
            }
        }
        girdi.push((run.started_at.clone(), queries::run_scenarios(c, id)?));
    }
    let b = birlestir(girdi, haric);
    let mut sayim = Sayim::default();
    let mut ozellikler: Vec<Ozellik> = Vec::new();
    for s in b.senaryolar {
        let Some(sn) = sonuc(&s.status) else { continue };
        sayim.ekle(sn);
        let ad = s.feature.clone().unwrap_or_else(|| "Diğer".into());
        let i = match ozellikler.iter().position(|o| o.ad == ad) {
            Some(i) => i,
            None => {
                ozellikler.push(Ozellik {
                    anchor: crate::web::pages::feature_anchor(&ad),
                    ad,
                    sayim: Sayim::default(),
                    testler: vec![],
                });
                ozellikler.len() - 1
            }
        };
        ozellikler[i].sayim.ekle(sn);
        let j = jira.remove(&s.id).unwrap_or_default();
        ozellikler[i].testler.push(Test { s, sonuc: sn, jira: j });
    }
    ozellikler.sort_by(|a, b| a.ad.cmp(&b.ad));
    for o in &mut ozellikler {
        // kalanlar önce, sonra dosyadaki sırayla
        o.testler
            .sort_by_key(|t| (t.sonuc == Sonuc::Gecen, t.s.full_name.clone().unwrap_or_default()));
    }
    Ok(Rapor {
        yazilim: yazilim.to_string(),
        surum: surum.to_string(),
        project_id,
        tarih,
        sayim,
        ozellikler,
        cucumber_disi: b.cucumber_disi,
        kosulmadi: b.kosulmadi,
        haric: b.haric,
        kosum_sayisi: kosumlar.len(),
    })
}

/// Yayınlanmış raporu kurar.
pub fn rapor(c: &Connection, yayin_id: i64) -> Result<Option<(Yayin, Rapor)>> {
    let Some(y) = yayin(c, yayin_id)? else { return Ok(None) };
    let r = olustur(c, &y.app_name, &y.app_version, &kosum_idleri(c, y.id)?, &haric(c, y.id)?)?;
    Ok(Some((y, r)))
}

/// Yayınlar ya da günceller (aynı proje + yazılım + sürüm tek rapor). Dönen: rapor id.
pub fn kaydet(c: &Connection, project_id: i64, app_name: &str, app_version: &str, kosumlar: &[i64], haric: &[String]) -> Result<i64> {
    anyhow::ensure!(!kosumlar.is_empty(), "en az bir koşum seçilmeli");
    let t = now();
    let id = match bul(c, project_id, app_name, app_version)? {
        Some(id) => {
            c.execute("UPDATE yayinlar SET updated_at = ?2 WHERE id = ?1", params![id, t])?;
            c.execute("DELETE FROM yayin_kosumlari WHERE yayin_id = ?1", [id])?;
            c.execute("DELETE FROM yayin_haric WHERE yayin_id = ?1", [id])?;
            id
        }
        None => {
            c.execute(
                "INSERT INTO yayinlar (project_id, app_name, app_version, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
                params![project_id, app_name, app_version, t],
            )?;
            c.last_insert_rowid()
        }
    };
    for r in kosumlar {
        c.execute(
            "INSERT OR IGNORE INTO yayin_kosumlari (yayin_id, run_id) VALUES (?1, ?2)",
            params![id, r],
        )?;
    }
    for k in haric {
        c.execute(
            "INSERT OR IGNORE INTO yayin_haric (yayin_id, scenario_key) VALUES (?1, ?2)",
            params![id, k],
        )?;
    }
    tracing::info!(yayin_id = id, yazilim = %app_name, surum = %app_version, kosum = kosumlar.len(), "rapor yayınlandı");
    Ok(id)
}

pub fn sil(c: &Connection, id: i64) -> Result<()> {
    c.execute("DELETE FROM yayin_haric WHERE yayin_id = ?1", [id])?;
    c.execute("DELETE FROM yayin_kosumlari WHERE yayin_id = ?1", [id])?;
    c.execute("DELETE FROM yayinlar WHERE id = ?1", [id])?;
    tracing::info!(yayin_id = id, "rapor yayından kaldırıldı");
    Ok(())
}

/// Ana sayfa ve admin listesi için özet.
pub struct YayinOzet {
    pub y: Yayin,
    pub tarih: String,
    pub sayim: Sayim,
}

/// Yayınlanmış raporlar; `yazilim` verilirse yalnızca onunkiler. Eskiden yeniye (tarih).
pub fn liste(c: &Connection, yazilim: Option<&str>) -> Result<Vec<YayinOzet>> {
    let mut st = c.prepare(&format!(
        "SELECT {YAYIN_COLS} FROM yayinlar y JOIN projects p ON p.id = y.project_id WHERE ?1 IS NULL OR y.app_name = ?1 ORDER BY y.id"
    ))?;
    let ys: Vec<Yayin> = st.query_map([yazilim], yayin_row)?.collect::<rusqlite::Result<_>>()?;
    let mut out = Vec::new();
    for y in ys {
        let r = olustur(c, &y.app_name, &y.app_version, &kosum_idleri(c, y.id)?, &haric(c, y.id)?)?;
        out.push(YayinOzet {
            y,
            tarih: r.tarih,
            sayim: r.sayim,
        });
    }
    out.sort_by(|a, b| a.tarih.cmp(&b.tarih));
    Ok(out)
}

/// Yayınlanmış raporu olan yazılım adları; en son yayınlanan önce.
pub fn yazilimlar(c: &Connection) -> Result<Vec<String>> {
    let mut st = c.prepare("SELECT app_name FROM yayinlar GROUP BY app_name ORDER BY MAX(updated_at) DESC")?;
    let r = st.query_map([], |r| r.get(0))?;
    Ok(r.collect::<rusqlite::Result<_>>()?)
}

/// Rapordaki bir senaryo mu (ek ve ayrıntı sayfası erişimi için).
pub fn senaryo_raporda(c: &Connection, yayin_id: i64, sid: i64) -> Result<bool> {
    let Some((_, r)) = rapor(c, yayin_id)? else { return Ok(false) };
    Ok(r.ozellikler.iter().any(|o| o.testler.iter().any(|t| t.s.id == sid)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(key: &str, status: &str, ff: Option<&str>, id: i64) -> ScenarioRow {
        ScenarioRow {
            id,
            run_id: 1,
            project_id: 1,
            branch: "main".into(),
            scenario_key: key.into(),
            name: key.into(),
            feature: Some("Giriş".into()),
            feature_file: ff.map(String::from),
            tags: vec![],
            doors_ids: vec![],
            jira_keys: vec![],
            status: status.into(),
            message: None,
            duration_ms: Some(1200),
            attempt: 2,
            is_latest: true,
            created_at: String::new(),
            full_name: None,
        }
    }

    #[test]
    fn test_sw_5_1_yayin_birlestirme_kurallari() {
        let f = Some("features/giris.feature");
        let k1 = (
            "2026-11-01T10:00".to_string(),
            vec![
                s("a", "failed", f, 1),
                s("b", "passed", f, 2),
                s("probe", "passed", None, 3),
                s("bos", "passed", Some(" "), 4),
                s("atla", "skipped", f, 5),
            ],
        );
        let k2 = (
            "2026-11-01T12:00".to_string(),
            vec![s("a", "passed", f, 11), s("c", "broken", f, 12)],
        );
        // sıra verilmeden: k2 sonra başladığı için a'nın sonucu oradan gelir
        let b = birlestir(vec![k2, k1], &HashSet::from(["c".to_string()]));
        let ids: Vec<i64> = b.senaryolar.iter().map(|s| s.id).collect();
        assert_eq!(ids, vec![11, 2], "a son koşumdan (tekrarda geçen = geçen), b; c hariç");
        assert_eq!((b.cucumber_disi, b.kosulmadi, b.haric), (2, 1, 1));
        assert_eq!(sonuc("passed"), Some(Sonuc::Gecen));
        assert_eq!(sonuc("broken"), Some(Sonuc::Kalan));
        assert_eq!(sonuc("skipped"), None);
        let mut say = Sayim::default();
        say.ekle(Sonuc::Gecen);
        say.ekle(Sonuc::Gecen);
        say.ekle(Sonuc::Kalan);
        assert_eq!((say.toplam(), say.oran()), (3, 67));
    }
}
