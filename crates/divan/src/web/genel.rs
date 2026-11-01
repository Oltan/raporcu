//! Genel rapor (SW-5.1): ekibin ayrıntılı raporundan ayrı, yalnızca ürün sonucu gösteren sayfalar.
//! İki giriş var: tek koşumluk `/s/<token>` ve proje + branch başına ana sayfa `/g/<token>`.

use super::pages::{feature_anchor, filters, legend, load_run, message_page, sanitize_run, share_layout, Layout, LegendItem, RunScenario};
use crate::auth::{self, Admin};
use crate::charts::{self, Column, Point};
use crate::queries::{self, AttachmentRow, RunRow, ScenarioRow, StepRow};
use crate::share::{genel_sinif, sanitize_text, GenelSinif};
use crate::web::{render, AppError};
use crate::AppState;
use askama::Template;
use axum::extract::{Path, Query, Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};

/// Ana sayfadaki koşum listesi.
const KOSUM_SAYISI: i64 = 50;
/// Gidişat grafiklerindeki koşum sayısı.
const GIDISAT: usize = 20;
const OZELLIK_GIDISAT: usize = 10;
const MATRIS_KOSUM: usize = 12;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Sayim {
    pub dogrulanan: i64,
    pub urun: i64,
    pub inceleniyor: i64,
}

impl Sayim {
    fn ekle(&mut self, sinif: GenelSinif) {
        match sinif {
            GenelSinif::Dogrulandi => self.dogrulanan += 1,
            GenelSinif::UrunHatasi => self.urun += 1,
            GenelSinif::Inceleniyor => self.inceleniyor += 1,
        }
    }
    pub fn toplam(&self) -> i64 {
        self.dogrulanan + self.urun + self.inceleniyor
    }
    /// Doğrulanan senaryoların yüzdesi.
    pub fn oran(&self) -> i64 {
        charts::pct(self.dogrulanan, self.toplam())
    }
    fn dilimler(&self) -> Vec<charts::Slice> {
        charts::genel_slices(self.dogrulanan, self.urun, self.inceleniyor)
    }
    fn cubuk(&self) -> String {
        charts::thin_bar(&self.dilimler())
    }
}

/// Bir koşumdaki senaryonun genel sınıfı; gidişat için yalnızca gereken alanlar.
struct Satir {
    id: i64,
    ad: String,
    feature: String,
    sinif: GenelSinif,
}

fn feature_adi(f: Option<&str>) -> String {
    f.unwrap_or("Diğer").to_string()
}

fn kosum_satirlari(c: &rusqlite::Connection, run_id: i64, flaky: &HashSet<String>, conf: f64) -> anyhow::Result<Vec<Satir>> {
    let mut denemeler: HashMap<String, i64> = HashMap::new();
    {
        let mut st = c.prepare("SELECT scenario_key, COUNT(*) FROM scenarios WHERE run_id = ?1 GROUP BY scenario_key")?;
        let mut rows = st.query([run_id])?;
        while let Some(r) = rows.next()? {
            denemeler.insert(r.get(0)?, r.get(1)?);
        }
    }
    let mut out = Vec::new();
    for s in queries::run_scenarios(c, run_id)? {
        let triage = if s.is_failure() {
            crate::triage::current(c, s.id, conf)?
        } else {
            None
        };
        let sinif = genel_sinif(
            &s.status,
            *denemeler.get(&s.scenario_key).unwrap_or(&1),
            flaky.contains(&s.scenario_key),
            triage.as_ref().map(|t| t.category.as_str()),
        );
        out.push(Satir {
            id: s.id,
            feature: feature_adi(s.feature.as_deref()),
            ad: s.name,
            sinif,
        });
    }
    Ok(out)
}

struct PortalKosum {
    run: RunRow,
    satirlar: Vec<Satir>,
    sayim: Sayim,
}

fn sayim(satirlar: &[Satir]) -> Sayim {
    let mut s = Sayim::default();
    for r in satirlar {
        s.ekle(r.sinif);
    }
    s
}

/// Kapsamın biten koşumları, eskiden yeniye.
fn portal_kosumlari(c: &rusqlite::Connection, proje: i64, branch: &str, conf: f64, window: usize) -> anyhow::Result<Vec<PortalKosum>> {
    let flaky = crate::history::flaky_keys(c, proje, branch, window)?;
    let mut out = Vec::new();
    for run in queries::trend(c, proje, branch, KOSUM_SAYISI)? {
        let satirlar = kosum_satirlari(c, run.id, &flaky, conf)?;
        out.push(PortalKosum {
            sayim: sayim(&satirlar),
            run,
            satirlar,
        });
    }
    Ok(out)
}

fn ozellik_sayimlari(satirlar: &[Satir]) -> Vec<(String, Sayim)> {
    let mut m: BTreeMap<String, Sayim> = BTreeMap::new();
    for r in satirlar {
        m.entry(r.feature.clone()).or_default().ekle(r.sinif);
    }
    let mut v: Vec<_> = m.into_iter().collect();
    v.sort_by_key(|(_, s)| -s.urun);
    v
}

fn son_ogeler<T>(v: &[T], n: usize) -> &[T] {
    &v[v.len().saturating_sub(n)..]
}

fn gun(s: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|t| t.with_timezone(&chrono::Local).format("%d.%m").to_string())
        .unwrap_or_default()
}

fn grafikler(seri: &[(&RunRow, Sayim)], ana: &str) -> (String, String) {
    let noktalar: Vec<Point> = seri
        .iter()
        .map(|(r, s)| Point {
            label: format!("#{}", r.id),
            href: format!("{ana}/runs/{}", r.id),
            value: s.oran() as f64,
            detail: format!(
                "{} · {} senaryo, {} doğrulandı, {} ürün hatası",
                gun(&r.started_at),
                s.toplam(),
                s.dogrulanan,
                s.urun
            ),
        })
        .collect();
    let sutunlar: Vec<Column> = seri
        .iter()
        .map(|(r, s)| Column {
            label: format!("#{}", r.id),
            href: Some(format!("{ana}/runs/{}", r.id)),
            title: format!("Rapor #{} ({})", r.id, gun(&r.started_at)),
            parts: s.dilimler(),
        })
        .collect();
    (
        charts::line_pct(&noktalar, "Rapor başına doğrulanma oranı"),
        charts::stacked_columns(&sutunlar, "Rapor başına senaryo sonuçları"),
    )
}

fn sonuc_cumlesi(run: &RunRow, s: Sayim) -> (String, &'static str) {
    match run.status.as_str() {
        "running" => ("Testler sürüyor, sonuçlar henüz kesin değil".into(), "kosuyor"),
        "interrupted" => ("Test yarıda kesildi, sonuçlar eksik".into(), "incele"),
        _ if s.urun > 0 => (format!("{} ürün hatası bulundu", s.urun), "kaldi"),
        _ if s.inceleniyor > 0 => (
            format!("Şu ana kadar ürün hatası yok, {} senaryo inceleniyor", s.inceleniyor),
            "incele",
        ),
        _ if s.dogrulanan > 0 => ("Tüm senaryolar doğrulandı".into(), "gecti"),
        _ => ("Henüz sonuç gelmedi".into(), "kosuyor"),
    }
}

fn kisa_durum(run: &RunRow, s: Sayim) -> (String, &'static str) {
    if run.status == "interrupted" {
        return ("Yarıda kesildi".into(), "incele");
    }
    if s.urun > 0 {
        (format!("{} ürün hatası", s.urun), "kaldi")
    } else if s.inceleniyor > 0 {
        ("İnceleniyor".into(), "incele")
    } else {
        ("Doğrulandı".into(), "gecti")
    }
}

fn proje_adi(path: &str) -> String {
    path.rsplit('/').next().unwrap_or_default().to_string()
}

fn halka(s: Sayim) -> String {
    charts::donut(
        &s.dilimler(),
        &s.toplam().to_string(),
        &format!("%{} doğrulandı", s.oran()),
        "Senaryo sonuçlarının dağılımı",
    )
}

// ---------------------------------------------------------------------------------------------
// Ana sayfa

pub struct OzellikSatir {
    pub ad: String,
    pub anchor: String,
    pub sayim: Sayim,
    pub cubuk: String,
    pub gidisat: String,
    pub degisim: i64,
}

pub struct KosumSatir {
    pub run: RunRow,
    pub sayim: Sayim,
    pub cubuk: String,
    pub durum: String,
    pub durum_sinif: &'static str,
}

pub struct SonOzet {
    pub run: RunRow,
    pub sayim: Sayim,
    pub sonuc: String,
    pub sonuc_sinif: &'static str,
    pub halka: String,
    pub lejant: Vec<LegendItem>,
    pub cizgi: String,
    pub sutunlar: String,
    pub gidisat_n: usize,
    pub ozellikler: Vec<OzellikSatir>,
}

#[derive(Template)]
#[template(path = "genel_ana.html")]
struct AnaSayfa {
    l: Layout,
    ana: Option<String>,
    proje: String,
    son: Option<SonOzet>,
    kosumlar: Vec<KosumSatir>,
}

fn ozellik_gidisati(kosumlar: &[PortalKosum], ad: &str) -> Vec<f64> {
    son_ogeler(kosumlar, OZELLIK_GIDISAT)
        .iter()
        .filter_map(|k| {
            let mut s = Sayim::default();
            for r in k.satirlar.iter().filter(|r| r.feature == ad) {
                s.ekle(r.sinif);
            }
            (s.toplam() > 0).then(|| s.oran() as f64)
        })
        .collect()
}

fn son_ozet(kosumlar: &[PortalKosum], ana: &str) -> Option<SonOzet> {
    let son = kosumlar.last()?;
    let seri: Vec<(&RunRow, Sayim)> = son_ogeler(kosumlar, GIDISAT).iter().map(|k| (&k.run, k.sayim)).collect();
    let (cizgi, sutunlar) = grafikler(&seri, ana);
    let ozellikler = ozellik_sayimlari(&son.satirlar)
        .into_iter()
        .map(|(ad, sayim)| {
            let degerler = ozellik_gidisati(kosumlar, &ad);
            let degisim = match (degerler.first(), degerler.last()) {
                (Some(a), Some(b)) => (b - a).round() as i64,
                _ => 0,
            };
            OzellikSatir {
                anchor: feature_anchor(&ad),
                cubuk: sayim.cubuk(),
                gidisat: charts::sparkline(&degerler, &format!("{ad} doğrulanma oranı gidişatı")),
                degisim,
                sayim,
                ad,
            }
        })
        .collect();
    let (sonuc, sonuc_sinif) = sonuc_cumlesi(&son.run, son.sayim);
    Some(SonOzet {
        run: son.run.clone(),
        sayim: son.sayim,
        sonuc,
        sonuc_sinif,
        halka: halka(son.sayim),
        lejant: legend(&son.sayim.dilimler()),
        cizgi,
        sutunlar,
        gidisat_n: seri.len(),
        ozellikler,
    })
}

async fn portal_kapsami(st: &AppState, token: &str) -> Result<Option<(i64, String)>, AppError> {
    let t = token.to_string();
    Ok(st.db.read(move |c| queries::portal(c, &t)).await?)
}

fn gecersiz(token: &str) -> Response {
    message_page(
        share_layout(token),
        StatusCode::NOT_FOUND,
        "Link geçersiz",
        "Bu paylaşım linki iptal edilmiş ya da hiç yok.",
    )
}

pub async fn ana_sayfa(State(st): State<AppState>, Path(token): Path<String>) -> Result<Response, AppError> {
    let Some((proje, branch)) = portal_kapsami(&st, &token).await? else {
        return Ok(gecersiz(&token));
    };
    let (conf, window) = (st.cfg.triage_auto_confidence, st.cfg.flaky_window);
    let b = branch.clone();
    let (kosumlar, ad) = st
        .db
        .read(move |c| {
            let ad: String = c.query_row("SELECT path FROM projects WHERE id = ?1", [proje], |r| r.get(0))?;
            Ok((portal_kosumlari(c, proje, &b, conf, window)?, ad))
        })
        .await?;
    let ana = format!("/g/{token}");
    let son = son_ozet(&kosumlar, &ana);
    let satirlar = kosumlar
        .into_iter()
        .rev()
        .map(|k| {
            let (durum, durum_sinif) = kisa_durum(&k.run, k.sayim);
            KosumSatir {
                cubuk: k.sayim.cubuk(),
                sayim: k.sayim,
                run: k.run,
                durum,
                durum_sinif,
            }
        })
        .collect();
    Ok(render(&AnaSayfa {
        l: share_layout(&token),
        ana: Some(ana),
        proje: proje_adi(&ad),
        son,
        kosumlar: satirlar,
    }))
}

// ---------------------------------------------------------------------------------------------
// Feature geçmişi

pub struct MatrisHucre {
    pub run_id: i64,
    pub isaret: &'static str,
    pub etiket: &'static str,
    pub url: Option<String>,
}

pub struct MatrisSatir {
    pub ad: String,
    pub hucreler: Vec<MatrisHucre>,
}

#[derive(Template)]
#[template(path = "genel_ozellik.html")]
struct OzellikSayfa {
    l: Layout,
    ana: Option<String>,
    proje: String,
    ad: String,
    son: Option<Sayim>,
    cizgi: String,
    sutunlar: String,
    basliklar: Vec<i64>,
    matris: Vec<MatrisSatir>,
}

#[derive(Deserialize)]
pub struct OzellikQuery {
    ad: String,
}

/// Satırlar senaryo adına göredir: Allure kimliği feature dosyasındaki satıra bağlıdır, dosya
/// değişince aynı senaryo yeni kimlik alır. Taslak örneklerinde en kötü sonuç gösterilir. Son
/// göründüğü raporda inceleniyor olan senaryo listelenmez.
fn matris(kosumlar: &[PortalKosum], ad: &str, ana: &str) -> (Vec<i64>, Vec<MatrisSatir>) {
    let son = son_ogeler(kosumlar, MATRIS_KOSUM);
    let mut adlar: BTreeMap<&str, Vec<MatrisHucre>> = BTreeMap::new();
    for k in son {
        for r in k.satirlar.iter().filter(|r| r.feature == ad) {
            adlar.entry(r.ad.as_str()).or_default();
        }
    }
    for k in son {
        for (senaryo, hucreler) in adlar.iter_mut() {
            let bulunan = k
                .satirlar
                .iter()
                .filter(|r| r.feature == ad && r.ad == *senaryo)
                .min_by_key(|r| match r.sinif {
                    GenelSinif::UrunHatasi => 0,
                    GenelSinif::Inceleniyor => 1,
                    GenelSinif::Dogrulandi => 2,
                });
            hucreler.push(match bulunan {
                Some(r) => MatrisHucre {
                    run_id: k.run.id,
                    isaret: r.sinif.isaret(),
                    etiket: r.sinif.etiket(),
                    url: (r.sinif != GenelSinif::Inceleniyor).then(|| format!("{ana}/runs/{}/scenarios/{}", k.run.id, r.id)),
                },
                None => MatrisHucre {
                    run_id: k.run.id,
                    isaret: "yok",
                    etiket: "Koşmadı",
                    url: None,
                },
            });
        }
    }
    let satirlar = adlar
        .into_iter()
        .filter(|(_, h)| h.iter().rev().find(|c| c.isaret != "yok").is_some_and(|c| c.url.is_some()))
        .map(|(ad, hucreler)| MatrisSatir {
            ad: ad.to_string(),
            hucreler,
        })
        .collect();
    (son.iter().map(|k| k.run.id).collect(), satirlar)
}

pub async fn ozellik_sayfasi(
    State(st): State<AppState>,
    Path(token): Path<String>,
    Query(q): Query<OzellikQuery>,
) -> Result<Response, AppError> {
    let Some((proje, branch)) = portal_kapsami(&st, &token).await? else {
        return Ok(gecersiz(&token));
    };
    let (conf, window) = (st.cfg.triage_auto_confidence, st.cfg.flaky_window);
    let (kosumlar, yol) = st
        .db
        .read(move |c| {
            let yol: String = c.query_row("SELECT path FROM projects WHERE id = ?1", [proje], |r| r.get(0))?;
            Ok((portal_kosumlari(c, proje, &branch, conf, window)?, yol))
        })
        .await?;
    let sayimlar: Vec<(&RunRow, Sayim)> = son_ogeler(&kosumlar, GIDISAT)
        .iter()
        .filter_map(|k| {
            let mut s = Sayim::default();
            for r in k.satirlar.iter().filter(|r| r.feature == q.ad) {
                s.ekle(r.sinif);
            }
            (s.toplam() > 0).then_some((&k.run, s))
        })
        .collect();
    if sayimlar.is_empty() {
        return Err(AppError::not_found());
    }
    let ana = format!("/g/{token}");
    let (cizgi, sutunlar) = grafikler(&sayimlar, &ana);
    let (basliklar, matris) = matris(&kosumlar, &q.ad, &ana);
    let son = sayimlar.last().map(|(_, s)| *s);
    Ok(render(&OzellikSayfa {
        l: share_layout(&token),
        ana: Some(ana),
        proje: proje_adi(&yol),
        ad: q.ad,
        son,
        cizgi,
        sutunlar,
        basliklar,
        matris,
    }))
}

// ---------------------------------------------------------------------------------------------
// Koşum raporu

pub struct GenelGrup {
    pub ad: String,
    pub anchor: String,
    pub satirlar: Vec<GenelSatir>,
}

pub struct GenelSatir {
    pub s: ScenarioRow,
    /// Aynı adlı senaryo taslağı satırlarını ayırır.
    pub ornek: Option<usize>,
}

#[derive(Template)]
#[template(path = "paylasim.html")]
struct RaporSayfa {
    l: Layout,
    ana: Option<String>,
    /// Senaryo ve ek adreslerinin başı.
    taban: String,
    run: RunRow,
    proje: String,
    sonuc: String,
    sonuc_sinif: &'static str,
    sayim: Sayim,
    halka: String,
    lejant: Vec<LegendItem>,
    /// Seçili feature sekmesi; yoksa tüm koşum.
    bolum: Option<String>,
    tumu: i64,
    sekmeler: Vec<(String, Sayim)>,
    ozellikler: Vec<(String, Sayim, String)>,
    hatalar: Vec<RunScenario>,
    gruplar: Vec<GenelGrup>,
}

#[derive(Deserialize, Default)]
pub struct BolumQuery {
    pub bolum: Option<String>,
}

async fn genel_rows(st: &AppState, run_id: i64) -> Result<Option<(RunRow, Vec<(GenelSinif, RunScenario)>)>, AppError> {
    let (conf, window) = (st.cfg.triage_auto_confidence, st.cfg.flaky_window);
    let Some((mut run, mut rows, _)) = st.db.read(move |c| load_run(c, run_id, conf, window)).await? else {
        return Ok(None);
    };
    sanitize_run(st, &mut run, &mut rows);
    let rows = rows
        .into_iter()
        .map(|r| {
            let triage = r.triage.as_ref().map(|t| t.category.as_str());
            (genel_sinif(&r.s.status, r.attempts, r.flaky, triage), r)
        })
        .collect();
    Ok(Some((run, rows)))
}

fn dogrulanan_gruplari(rows: Vec<RunScenario>) -> Vec<GenelGrup> {
    let mut adlar: HashMap<(String, String), usize> = HashMap::new();
    for r in &rows {
        *adlar.entry((feature_adi(r.s.feature.as_deref()), r.s.name.clone())).or_default() += 1;
    }
    let mut sira: HashMap<(String, String), usize> = HashMap::new();
    let mut gruplar: BTreeMap<String, Vec<GenelSatir>> = BTreeMap::new();
    for r in rows {
        let f = feature_adi(r.s.feature.as_deref());
        let k = (f.clone(), r.s.name.clone());
        let ornek = (adlar[&k] > 1).then(|| {
            let n = sira.entry(k).or_default();
            *n += 1;
            *n
        });
        gruplar.entry(f).or_default().push(GenelSatir { s: r.s, ornek });
    }
    gruplar
        .into_iter()
        .map(|(ad, satirlar)| GenelGrup {
            anchor: feature_anchor(&ad),
            ad,
            satirlar,
        })
        .collect()
}

fn rapor_satirlari(rows: &[(GenelSinif, RunScenario)]) -> Vec<Satir> {
    rows.iter()
        .map(|(sinif, r)| Satir {
            id: r.s.id,
            ad: String::new(),
            feature: feature_adi(r.s.feature.as_deref()),
            sinif: *sinif,
        })
        .collect()
}

pub async fn rapor_sayfasi(
    st: &AppState,
    token: &str,
    taban: String,
    ana: Option<String>,
    run_id: i64,
    bolum: Option<String>,
) -> Result<Response, AppError> {
    let Some((run, mut rows)) = genel_rows(st, run_id).await? else {
        return Err(AppError::not_found());
    };
    let tumu = rows.len() as i64;
    let sekmeler = ozellik_sayimlari(&rapor_satirlari(&rows));
    // bilinmeyen sekme tüm koşumu gösterir
    let bolum = bolum.filter(|b| sekmeler.iter().any(|(ad, _)| ad == b));
    if let Some(b) = &bolum {
        rows.retain(|(_, r)| feature_adi(r.s.feature.as_deref()) == *b);
    }
    let satirlar = rapor_satirlari(&rows);
    let toplam = sayim(&satirlar);
    let ozellikler = ozellik_sayimlari(&satirlar)
        .into_iter()
        .map(|(ad, s)| {
            let cubuk = s.cubuk();
            (ad, s, cubuk)
        })
        .collect();
    let (mut hatalar, mut dogrulanan) = (Vec::new(), Vec::new());
    for (sinif, r) in rows {
        match sinif {
            GenelSinif::UrunHatasi => hatalar.push(r),
            GenelSinif::Dogrulandi => dogrulanan.push(r),
            GenelSinif::Inceleniyor => {}
        }
    }
    let (sonuc, sonuc_sinif) = sonuc_cumlesi(&run, toplam);
    Ok(render(&RaporSayfa {
        l: share_layout(token),
        ana,
        taban,
        proje: proje_adi(&run.project_path),
        run,
        sonuc,
        sonuc_sinif,
        sayim: toplam,
        halka: halka(toplam),
        lejant: legend(&toplam.dilimler()),
        bolum,
        tumu,
        sekmeler,
        ozellikler,
        hatalar,
        gruplar: dogrulanan_gruplari(dogrulanan),
    }))
}

// ---------------------------------------------------------------------------------------------
// Senaryo detayı

#[derive(Template)]
#[template(path = "paylasim_senaryo.html")]
struct SenaryoSayfa {
    l: Layout,
    ana: Option<String>,
    taban: String,
    sinif: GenelSinif,
    s: ScenarioRow,
    steps: Vec<StepRow>,
    atts: Vec<AttachmentRow>,
}

fn adim_suresi(ms: Option<i64>) -> String {
    match ms {
        Some(ms) if ms >= 1000 => format!("{:.1} sn", ms as f64 / 1000.0),
        Some(ms) => format!("{ms} ms"),
        None => String::new(),
    }
}

/// Doğrulanan ve ürün hatası olan senaryoların ayrıntısı açılır; iç bilgi gösterilmez.
pub async fn senaryo_sayfasi(
    st: &AppState,
    token: &str,
    taban: String,
    ana: Option<String>,
    run_id: i64,
    sid: i64,
) -> Result<Response, AppError> {
    let Some((_, rows)) = genel_rows(st, run_id).await? else {
        return Err(AppError::not_found());
    };
    let Some((sinif, r)) = rows
        .into_iter()
        .find(|(sinif, r)| *sinif != GenelSinif::Inceleniyor && r.s.id == sid)
    else {
        return Err(AppError::not_found());
    };
    let (mut steps, mut atts) = st
        .db
        .read(move |c| Ok((queries::steps(c, sid)?, queries::attachments(c, sid)?)))
        .await?;
    // iç adımlar test kodunun ayrıntısıdır (seçiciler, yardımcı metotlar)
    steps.retain(|s| s.depth == 0);
    let secrets = st.cfg.secrets();
    for s in steps.iter_mut() {
        s.name = sanitize_text(&s.name, &secrets);
        s.message = None;
    }
    atts.retain(|a| a.is_image() && !a.is_deleted());
    Ok(render(&SenaryoSayfa {
        l: share_layout(token),
        ana,
        taban,
        sinif,
        s: r.s,
        steps,
        atts,
    }))
}

/// Genel link yalnızca görünen senaryoların ekran görüntülerini açar (video, log, metin yok).
pub async fn ek_acik(st: &AppState, run_id: i64, att: &AttachmentRow) -> Result<bool, AppError> {
    let Some(sid) = att.scenario_id.filter(|_| att.is_image() && !att.is_deleted()) else {
        return Ok(false);
    };
    let Some((_, rows)) = genel_rows(st, run_id).await? else {
        return Ok(false);
    };
    Ok(rows.iter().any(|(sinif, r)| *sinif != GenelSinif::Inceleniyor && r.s.id == sid))
}

// ---------------------------------------------------------------------------------------------
// Ana sayfa linkinden açılan koşumlar: yalnızca kapsamdaki biten koşumlar

async fn portal_kosumu(st: &AppState, token: &str, id: i64) -> Result<bool, AppError> {
    let Some((proje, branch)) = portal_kapsami(st, token).await? else {
        return Ok(false);
    };
    let run = st.db.read(move |c| queries::run(c, id)).await?;
    Ok(run.is_some_and(|r| r.project_id == proje && r.branch == branch && r.status != "running"))
}

fn portal_adresleri(token: &str, id: i64) -> (String, Option<String>) {
    (format!("/g/{token}/runs/{id}"), Some(format!("/g/{token}")))
}

pub async fn portal_rapor(
    State(st): State<AppState>,
    Path((token, id)): Path<(String, i64)>,
    Query(q): Query<BolumQuery>,
) -> Result<Response, AppError> {
    if !portal_kosumu(&st, &token, id).await? {
        return Err(AppError::not_found());
    }
    let (taban, ana) = portal_adresleri(&token, id);
    rapor_sayfasi(&st, &token, taban, ana, id, q.bolum).await
}

pub async fn portal_senaryo(State(st): State<AppState>, Path((token, id, sid)): Path<(String, i64, i64)>) -> Result<Response, AppError> {
    if !portal_kosumu(&st, &token, id).await? {
        return Err(AppError::not_found());
    }
    let (taban, ana) = portal_adresleri(&token, id);
    senaryo_sayfasi(&st, &token, taban, ana, id, sid).await
}

pub async fn portal_dosya(
    State(st): State<AppState>,
    Path((token, id, aid)): Path<(String, i64, i64)>,
    req: Request,
) -> Result<Response, AppError> {
    if !portal_kosumu(&st, &token, id).await? {
        return Err(AppError::not_found());
    }
    let Some(att) = st.db.read(move |c| queries::attachment(c, aid)).await? else {
        return Err(AppError::not_found());
    };
    if att.run_id != id || !ek_acik(&st, id, &att).await? {
        return Err(AppError::not_found());
    }
    Ok(crate::web::files::serve_attachment_file(&st, &att, req).await)
}

// ---------------------------------------------------------------------------------------------
// Link yönetimi

/// Kapsamda açık link varsa onu verir; düğmeye tekrar basmak yeni link üretmez.
pub async fn portal_olustur(State(st): State<AppState>, _a: Admin, Path(id): Path<i64>) -> Result<Response, AppError> {
    let yeni = auth::random_token();
    let token = st
        .db
        .write(move |c| {
            let Some(run) = queries::run(c, id)? else { return Ok(None) };
            let acik: Option<String> = c
                .query_row(
                    "SELECT token FROM genel_portallar WHERE project_id = ?1 AND branch = ?2 AND revoked_at IS NULL",
                    rusqlite::params![run.project_id, run.branch],
                    |r| r.get(0),
                )
                .ok();
            if acik.is_some() {
                return Ok(acik);
            }
            c.execute(
                "INSERT INTO genel_portallar (token, project_id, branch, created_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![yeni, run.project_id, run.branch, crate::db::now()],
            )?;
            tracing::info!(run_id = id, "genel ana sayfa linki oluşturuldu");
            Ok(Some(yeni))
        })
        .await?
        .ok_or_else(AppError::not_found)?;
    Ok(Redirect::to(&format!("/runs/{id}?notice=portal:{token}")).into_response())
}

pub async fn portal_iptal(State(st): State<AppState>, _a: Admin, Path(id): Path<i64>) -> Result<Response, AppError> {
    st.db
        .write(move |c| {
            c.execute(
                "UPDATE genel_portallar SET revoked_at = ?2 WHERE id = ?1 AND revoked_at IS NULL",
                rusqlite::params![id, crate::db::now()],
            )?;
            Ok(())
        })
        .await?;
    tracing::info!(portal_id = id, "genel ana sayfa linki iptal edildi");
    Ok(Redirect::to("/admin").into_response())
}

impl SenaryoSayfa {
    fn adim_suresi(&self, ms: &Option<i64>) -> String {
        adim_suresi(*ms)
    }
}
