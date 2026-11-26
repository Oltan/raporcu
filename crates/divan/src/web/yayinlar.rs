//! Admin > Raporlar (SW-5.1, 0.2.0): dışa açık sürüm raporlarını yayınlama.
//! Rapor = repo + yazılım adı + sürüm. Admin o üçlünün koşumlarından istediklerini seçer, istemediği
//! testleri (ör. ortam yoklama testleri) çıkarır, yayınlar. Aynı üçlü tekrar yayınlanırsa güncellenir.

use super::pages::{filters, layout, Layout};
use crate::auth::Admin;
use crate::queries::{self, RunRow};
use crate::web::{render, AppError};
use crate::yayin::{self, Rapor, YayinOzet};
use crate::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Redirect, Response};
use axum::Form;
use std::collections::HashSet;

/// Bir projede yayınlanabilir (yazılım, sürüm) ve durumu.
pub struct Aday {
    pub project_id: i64,
    pub project_path: String,
    pub yazilim: String,
    pub surum: String,
    pub kosum: usize,
    pub son: String,
    /// yayınlanmışsa rapor id'si ve o yayına bağlı olmayan yeni koşum sayısı
    pub yayin: Option<(i64, usize)>,
}

#[derive(Template)]
#[template(path = "yayinlar.html")]
struct ListeSayfa {
    l: Layout,
    yayinlar: Vec<YayinOzet>,
    adaylar: Vec<Aday>,
}

pub async fn liste(State(st): State<AppState>, _a: Admin, headers: HeaderMap) -> Result<Response, AppError> {
    let l = layout(&st, &headers);
    let (yayinlar, adaylar) = st
        .db
        .read(|c| {
            let mut yayinlar = yayin::liste(c, None)?;
            yayinlar.reverse();
            let mut adaylar = Vec::new();
            for p in queries::projects(c)? {
                for (ad, surum, runs) in yayin::adaylar(c, p.id)? {
                    let yayin = match yayin::bul(c, p.id, &ad, &surum)? {
                        Some(id) => {
                            let bagli: HashSet<i64> = yayin::kosum_idleri(c, id)?.into_iter().collect();
                            Some((id, runs.iter().filter(|r| !bagli.contains(&r.id)).count()))
                        }
                        None => None,
                    };
                    adaylar.push(Aday {
                        project_id: p.id,
                        project_path: p.path.clone(),
                        son: runs.first().map(|r| r.started_at.clone()).unwrap_or_default(),
                        kosum: runs.len(),
                        yazilim: ad,
                        surum,
                        yayin,
                    });
                }
            }
            adaylar.sort_by(|a, b| b.son.cmp(&a.son));
            Ok((yayinlar, adaylar))
        })
        .await?;
    Ok(render(&ListeSayfa { l, yayinlar, adaylar }))
}

// ---------------------------------------------------------------------------------------------
// Yayın formu

pub struct KosumSecim {
    pub run: RunRow,
    pub secili: bool,
    pub featurelar: Vec<String>,
}

pub struct TestSecim {
    pub key: String,
    pub ad: String,
    pub durum: &'static str,
    pub sinif: &'static str,
    pub haric: bool,
}

pub struct FeatureSecim {
    pub ad: String,
    pub testler: Vec<TestSecim>,
}

#[derive(Template)]
#[template(path = "yayin_form.html")]
struct FormSayfa {
    l: Layout,
    project_id: i64,
    project_path: String,
    yazilim: String,
    surum: String,
    mevcut: Option<i64>,
    kosumlar: Vec<KosumSecim>,
    ozellikler: Vec<FeatureSecim>,
    toplam: i64,
    gecen: i64,
    kalan: i64,
    cucumber_disi: usize,
    kosulmadi: usize,
    hata: Option<String>,
}

/// Sorgu: `run=N` (koşumun üçlüsü) ya da `proje`, `yazilim`, `surum`; önizlemede `kosum` ve `haric`
/// tekrar eden alanlar, `onizle=1`.
fn alan<'a>(q: &'a [(String, String)], k: &str) -> Option<&'a str> {
    q.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str())
}

fn alanlar(q: &[(String, String)], k: &str) -> Vec<String> {
    q.iter().filter(|(a, _)| a == k).map(|(_, v)| v.clone()).collect()
}

pub async fn form(
    State(st): State<AppState>,
    _a: Admin,
    headers: HeaderMap,
    Query(q): Query<Vec<(String, String)>>,
) -> Result<Response, AppError> {
    let l = layout(&st, &headers);
    let sayfa = st.db.read(move |c| form_kur(c, l, &q, None)).await?;
    match sayfa {
        Some(p) => Ok(render(&p)),
        None => Err(AppError::not_found()),
    }
}

fn form_kur(c: &rusqlite::Connection, l: Layout, q: &[(String, String)], hata: Option<String>) -> anyhow::Result<Option<FormSayfa>> {
    // üçlü
    let (project_id, yazilim, surum) = if let Some(run) = alan(q, "run").and_then(|r| r.parse().ok()) {
        let Some(r) = queries::run(c, run)? else { return Ok(None) };
        let Some(v) = r.app_version.clone() else { return Ok(None) };
        (r.project_id, yayin::yazilim_adi(&r), v)
    } else {
        let (Some(p), Some(y), Some(s)) = (
            alan(q, "proje").and_then(|p| p.parse::<i64>().ok()),
            alan(q, "yazilim"),
            alan(q, "surum"),
        ) else {
            return Ok(None);
        };
        (p, y.to_string(), s.to_string())
    };
    let runs = yayin::uclu_kosumlari(c, project_id, &yazilim, &surum)?;
    if runs.is_empty() {
        return Ok(None);
    }
    let project_path = runs[0].project_path.clone();
    let mevcut = yayin::bul(c, project_id, &yazilim, &surum)?;
    let onizle = alan(q, "onizle").is_some();
    // seçili koşumlar: önizlemede formdan; yoksa hepsi (yayınlanmışsa bağlılar + yeni gelenler de seçili)
    let secili: HashSet<i64> = if onizle {
        alanlar(q, "kosum").iter().filter_map(|k| k.parse().ok()).collect()
    } else {
        runs.iter().map(|r| r.id).collect()
    };
    let haric: HashSet<String> = if onizle {
        alanlar(q, "haric").into_iter().collect()
    } else {
        match mevcut {
            Some(id) => yayin::haric(c, id)?,
            None => yayin::onceki_haric(c, project_id, &yazilim)?,
        }
    };
    let mut kosumlar = Vec::new();
    for r in runs {
        let mut featurelar: Vec<String> = queries::run_scenarios(c, r.id)?
            .into_iter()
            .filter(|s| s.feature_file.as_deref().is_some_and(|f| !f.trim().is_empty()))
            .filter_map(|s| s.feature)
            .collect();
        featurelar.sort();
        featurelar.dedup();
        kosumlar.push(KosumSecim {
            secili: secili.contains(&r.id),
            run: r,
            featurelar,
        });
    }
    let secili_idler: Vec<i64> = kosumlar.iter().filter(|k| k.secili).map(|k| k.run.id).collect();
    // test listesi: hariç tutulanlar da görünsün diye boş hariç kümesiyle kurulur
    let r: Rapor = yayin::olustur(c, &yazilim, &surum, &secili_idler, &HashSet::new())?;
    let (mut gecen, mut kalan) = (0, 0);
    let ozellikler = r
        .ozellikler
        .into_iter()
        .map(|o| FeatureSecim {
            ad: o.ad,
            testler: o
                .testler
                .into_iter()
                .map(|t| {
                    let h = haric.contains(&t.s.scenario_key);
                    if !h {
                        match t.sonuc {
                            yayin::Sonuc::Gecen => gecen += 1,
                            yayin::Sonuc::Kalan => kalan += 1,
                        }
                    }
                    TestSecim {
                        key: t.s.scenario_key,
                        ad: t.s.name,
                        durum: t.sonuc.etiket(),
                        sinif: t.sonuc.sinif(),
                        haric: h,
                    }
                })
                .collect(),
        })
        .collect();
    Ok(Some(FormSayfa {
        l,
        project_id,
        project_path,
        yazilim,
        surum,
        mevcut,
        kosumlar,
        ozellikler,
        toplam: gecen + kalan,
        gecen,
        kalan,
        cucumber_disi: r.cucumber_disi,
        kosulmadi: r.kosulmadi,
        hata,
    }))
}

pub async fn kaydet(
    State(st): State<AppState>,
    _a: Admin,
    headers: HeaderMap,
    Form(f): Form<Vec<(String, String)>>,
) -> Result<Response, AppError> {
    let l = layout(&st, &headers);
    let sonuc = st
        .db
        .write(move |c| {
            let (Some(p), Some(y), Some(s)) = (
                alan(&f, "proje").and_then(|p| p.parse::<i64>().ok()),
                alan(&f, "yazilim"),
                alan(&f, "surum"),
            ) else {
                anyhow::bail!("eksik alan");
            };
            // yalnızca o üçlünün koşumları kabul edilir
            let gecerli: HashSet<i64> = yayin::uclu_kosumlari(c, p, y, s)?.into_iter().map(|r| r.id).collect();
            let kosumlar: Vec<i64> = alanlar(&f, "kosum")
                .iter()
                .filter_map(|k| k.parse().ok())
                .filter(|k| gecerli.contains(k))
                .collect();
            if kosumlar.is_empty() {
                let mut q = f.clone();
                q.push(("onizle".into(), "1".into()));
                return Ok(Err(form_kur(c, l, &q, Some("En az bir koşum seçin.".into()))?));
            }
            Ok(Ok(yayin::kaydet(c, p, y, s, &kosumlar, &alanlar(&f, "haric"))?))
        })
        .await?;
    match sonuc {
        Ok(id) => Ok(Redirect::to(&format!("/rapor/{id}")).into_response()),
        Err(Some(p)) => Ok(render(&p)),
        Err(None) => Err(AppError::not_found()),
    }
}

pub async fn sil(State(st): State<AppState>, _a: Admin, Path(id): Path<i64>) -> Result<Response, AppError> {
    st.db.write(move |c| yayin::sil(c, id)).await?;
    Ok(Redirect::to("/admin/yayinlar").into_response())
}
