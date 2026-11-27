//! Dışa açık sayfalar (SW-5.1, 0.2.0). Giriş istemez; yalnızca admin'in yayınladığı raporlar görünür.
//! - `/` : yazılım adı sekmeleri; seçili yazılımın sürüm raporları, son rapor, sürümlere göre başarı.
//! - `/rapor/{id}` : bir yazılım sürümünün test raporu (feature ve test case'ler açılır/kapanır).
//! - `/rapor/{id}/test/{sid}` : test ayrıntısı (üst düzey adımlar, ekran görüntüleri).
//! - `/rapor/{id}/dosya/{aid}` : rapordaki testlerin ekran görüntüleri.
//!
//! İç bilgi gösterilmez: koşum numarası, commit, branch, OS, CI linkleri, Jira linki, atanan kişi,
//! test kodu ayrıntısı (iç adımlar, adım mesajları), video ve metin ekleri.

use super::pages::{filters, legend, share_layout, Layout, LegendItem};
use crate::charts::{self, Column, Point};
use crate::queries::{self, AttachmentRow, ScenarioRow, StepRow};
use crate::share::sanitize_text;
use crate::web::{render, AppError};
use crate::yayin::{self, Rapor, Sayim, Sonuc, YayinOzet};
use crate::AppState;
use askama::Template;
use axum::extract::{Path, Query, Request, State};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

/// Gidişat grafiklerindeki rapor (sürüm) sayısı.
const GIDISAT: usize = 20;

fn genel_layout() -> Layout {
    // share = Some: üst çubukta panel menüsü yerine sayfanın kendi başlığı
    share_layout("")
}

fn gun(s: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|t| t.with_timezone(&chrono::Local).format("%d.%m.%Y").to_string())
        .unwrap_or_default()
}

fn halka(s: Sayim) -> String {
    charts::donut(
        &s.dilimler(),
        &s.toplam().to_string(),
        &format!("%{} başarı", s.oran()),
        "Test sonuçlarının dağılımı",
    )
}

/// Sürüm başına başarı çizgisi ve sonuç sütunları.
fn grafikler(seri: &[&YayinOzet]) -> (String, String) {
    let noktalar: Vec<Point> = seri
        .iter()
        .map(|o| Point {
            label: o.y.app_version.clone(),
            href: format!("/rapor/{}", o.y.id),
            value: o.sayim.oran() as f64,
            detail: format!(
                "Sürüm {} · {} · {} test, {} geçti, {} kaldı",
                o.y.app_version,
                gun(&o.tarih),
                o.sayim.toplam(),
                o.sayim.gecen,
                o.sayim.kalan
            ),
        })
        .collect();
    let sutunlar: Vec<Column> = seri
        .iter()
        .map(|o| Column {
            label: o.y.app_version.clone(),
            href: Some(format!("/rapor/{}", o.y.id)),
            title: format!("Sürüm {} ({})", o.y.app_version, gun(&o.tarih)),
            parts: o.sayim.dilimler(),
        })
        .collect();
    (
        charts::line_pct(&noktalar, "Sürümlere göre başarı oranı"),
        charts::stacked_columns(&sutunlar, "Sürüm başına test sonuçları"),
    )
}

// ---------------------------------------------------------------------------------------------
// Ana sayfa `/`

pub struct SonRapor {
    pub o: RaporSatir,
    pub halka: String,
    pub lejant: Vec<LegendItem>,
    pub cizgi: String,
    pub sutunlar: String,
    pub gidisat_n: usize,
}

pub struct RaporSatir {
    pub id: i64,
    pub surum: String,
    pub tarih: String,
    pub sayim: Sayim,
    pub cubuk: String,
}

impl From<&YayinOzet> for RaporSatir {
    fn from(o: &YayinOzet) -> Self {
        RaporSatir {
            id: o.y.id,
            surum: o.y.app_version.clone(),
            tarih: o.tarih.clone(),
            sayim: o.sayim,
            cubuk: o.sayim.cubuk(),
        }
    }
}

#[derive(Template)]
#[template(path = "genel_ana.html")]
struct AnaSayfa {
    l: Layout,
    yazilimlar: Vec<String>,
    secili: Option<String>,
    son: Option<SonRapor>,
    raporlar: Vec<RaporSatir>,
}

#[derive(Deserialize, Default)]
pub struct AnaQuery {
    pub yazilim: Option<String>,
}

pub async fn ana_sayfa(State(st): State<AppState>, Query(q): Query<AnaQuery>) -> Result<Response, AppError> {
    let (yazilimlar, secili, liste) = st
        .db
        .read(move |c| {
            let yazilimlar = yayin::yazilimlar(c)?;
            let secili = q.yazilim.filter(|y| yazilimlar.contains(y)).or_else(|| yazilimlar.first().cloned());
            let liste = match &secili {
                Some(y) => yayin::liste(c, Some(y))?,
                None => vec![],
            };
            Ok((yazilimlar, secili, liste))
        })
        .await?;
    let son = liste.last().map(|s| {
        let seri: Vec<&YayinOzet> = liste[liste.len().saturating_sub(GIDISAT)..].iter().collect();
        let (cizgi, sutunlar) = grafikler(&seri);
        SonRapor {
            o: RaporSatir::from(s),
            halka: halka(s.sayim),
            lejant: legend(&s.sayim.dilimler()),
            cizgi,
            sutunlar,
            gidisat_n: seri.len(),
        }
    });
    let raporlar = liste.iter().rev().map(RaporSatir::from).collect();
    Ok(render(&AnaSayfa {
        l: genel_layout(),
        yazilimlar,
        secili,
        son,
        raporlar,
    }))
}

// ---------------------------------------------------------------------------------------------
// Test raporu `/rapor/{id}`

/// Açılır test satırında gösterilenler (iç bilgiden arındırılmış).
pub struct TestSatir {
    pub id: i64,
    pub ad: String,
    pub sonuc: Sonuc,
    pub sure: String,
    pub gereksinimler: Vec<String>,
    pub ek_kimlikler: Vec<String>,
    pub hata: Option<String>,
    pub resim: Option<i64>,
    /// (anahtar, başlık, durum, kapalı mı)
    pub jira: Vec<(String, String, String, bool)>,
}

pub struct OzellikBolum {
    pub ad: String,
    pub anchor: String,
    pub sayim: Sayim,
    pub cubuk: String,
    pub testler: Vec<TestSatir>,
}

#[derive(Template)]
#[template(path = "paylasim.html")]
struct RaporSayfa {
    l: Layout,
    id: i64,
    yazilim: String,
    surum: String,
    tarih: String,
    sayim: Sayim,
    halka: String,
    lejant: Vec<LegendItem>,
    kosulmadi: usize,
    ozellikler: Vec<OzellikBolum>,
    /// Jira adresi (`JIRA_URL`); varsa issue anahtarları Jira'da yeni sekmede açılır
    jira_link: Option<String>,
}

fn test_satiri(c: &rusqlite::Connection, t: yayin::Test, secrets: &[String]) -> anyhow::Result<TestSatir> {
    let resim = if t.sonuc == Sonuc::Kalan {
        queries::first_screenshot(c, t.s.id)?.map(|a| a.id)
    } else {
        None
    };
    Ok(TestSatir {
        id: t.s.id,
        sure: t.sure(),
        sonuc: t.sonuc,
        gereksinimler: t.s.doors_ids.clone(),
        ek_kimlikler: t.s.ek_kimlikler(),
        hata: (t.sonuc == Sonuc::Kalan).then(|| sanitize_text(&t.s.short_message(), secrets)),
        resim,
        jira: t
            .jira
            .iter()
            .map(|o| (o.key.clone(), o.baslik(), o.durum(), o.sinif() == "kapali"))
            .collect(),
        ad: t.s.name,
    })
}

pub async fn rapor_sayfasi(State(st): State<AppState>, Path(id): Path<i64>) -> Result<Response, AppError> {
    let secrets = st.cfg.secrets();
    let Some((y, r)) = st
        .db
        .read(move |c| {
            let Some((y, r)) = yayin::rapor(c, id)? else { return Ok(None) };
            let Rapor {
                tarih,
                sayim,
                ozellikler,
                kosulmadi,
                ..
            } = r;
            let mut bolumler = Vec::new();
            for o in ozellikler {
                let mut testler = Vec::new();
                for t in o.testler {
                    testler.push(test_satiri(c, t, &secrets)?);
                }
                bolumler.push(OzellikBolum {
                    cubuk: o.sayim.cubuk(),
                    ad: o.ad,
                    anchor: o.anchor,
                    sayim: o.sayim,
                    testler,
                });
            }
            Ok(Some((y, (tarih, sayim, kosulmadi, bolumler))))
        })
        .await?
    else {
        return Err(AppError::not_found());
    };
    let (tarih, sayim, kosulmadi, ozellikler) = r;
    Ok(render(&RaporSayfa {
        l: genel_layout(),
        id,
        yazilim: y.app_name,
        surum: y.app_version,
        tarih,
        halka: halka(sayim),
        lejant: legend(&sayim.dilimler()),
        sayim,
        kosulmadi,
        ozellikler,
        jira_link: st.cfg.jira_url.clone(),
    }))
}

// ---------------------------------------------------------------------------------------------
// Test ayrıntısı `/rapor/{id}/test/{sid}`

#[derive(Template)]
#[template(path = "paylasim_senaryo.html")]
struct TestSayfa {
    l: Layout,
    id: i64,
    yazilim: String,
    surum: String,
    sonuc: Sonuc,
    s: ScenarioRow,
    hata: Option<String>,
    steps: Vec<StepRow>,
    atts: Vec<AttachmentRow>,
}

impl TestSayfa {
    fn adim_suresi(&self, ms: &Option<i64>) -> String {
        match *ms {
            Some(ms) if ms >= 1000 => format!("{:.1} sn", ms as f64 / 1000.0),
            Some(ms) => format!("{ms} ms"),
            None => String::new(),
        }
    }
}

pub async fn test_sayfasi(State(st): State<AppState>, Path((id, sid)): Path<(i64, i64)>) -> Result<Response, AppError> {
    let secrets = st.cfg.secrets();
    let Some(p) = st
        .db
        .read(move |c| {
            let Some((y, r)) = yayin::rapor(c, id)? else { return Ok(None) };
            let Some(t) = r.ozellikler.into_iter().flat_map(|o| o.testler).find(|t| t.s.id == sid) else {
                return Ok(None);
            };
            let mut steps = queries::steps(c, sid)?;
            // iç adımlar test kodunun ayrıntısıdır (seçiciler, yardımcı metotlar)
            steps.retain(|s| s.depth == 0);
            for s in steps.iter_mut() {
                s.name = sanitize_text(&s.name, &secrets);
                s.message = None;
            }
            let mut atts = queries::attachments(c, sid)?;
            atts.retain(|a| a.is_image() && !a.is_deleted());
            Ok(Some(TestSayfa {
                l: genel_layout(),
                id,
                yazilim: y.app_name,
                surum: y.app_version,
                sonuc: t.sonuc,
                hata: (t.sonuc == Sonuc::Kalan).then(|| sanitize_text(&t.s.short_message(), &secrets)),
                s: t.s,
                steps,
                atts,
            }))
        })
        .await?
    else {
        return Err(AppError::not_found());
    };
    Ok(render(&p))
}

/// Yalnızca rapordaki testlerin ekran görüntüleri (video, log, metin yok).
pub async fn rapor_dosya(State(st): State<AppState>, Path((id, aid)): Path<(i64, i64)>, req: Request) -> Result<Response, AppError> {
    let att = st
        .db
        .read(move |c| {
            let Some(att) = queries::attachment(c, aid)? else { return Ok(None) };
            let Some(sid) = att.scenario_id.filter(|_| att.is_image() && !att.is_deleted()) else {
                return Ok(None);
            };
            Ok(yayin::senaryo_raporda(c, id, sid)?.then_some(att))
        })
        .await?;
    match att {
        Some(att) => Ok(crate::web::files::serve_attachment_file(&st, &att, req).await),
        None => Err(AppError::not_found()),
    }
}

/// 0.1.x'in genel ana sayfa (`/g/<token>/…`) ve genel rapor linkleri artık `/`'e gider.
pub async fn eski_link() -> Response {
    Redirect::to("/").into_response()
}
