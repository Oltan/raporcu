//! Admin > Ayarlar (Jira bug şablonu, DOORS) ve senaryo için Jira bug formu (SW-6.1, SW-6.4).

use crate::auth::Admin;
use crate::ayarlar::{self, DoorsAyar, JiraSablon};
use crate::jira::{self, FieldMeta, Jira, NewBug};
use crate::web::pages::{layout, Layout};
use crate::web::{render, AppError};
use crate::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Form;
use serde::Deserialize;
use serde_json::{Map, Value};

// ---------------------------------------------------------------------------------------------
// Ayarlar sayfası

#[derive(Template)]
#[template(path = "ayarlar.html")]
struct AyarlarPage {
    l: Layout,
    jira: JiraSablon,
    doors: DoorsAyar,
    yer_tutucular: &'static [(&'static str, &'static str)],
    jira_ayarli: bool,
    jira_bilgi: String,
    doors_dxl: bool,
    /// "Jira alanlarını getir" sonucu
    alanlar: Option<Result<Vec<FieldMeta>, String>>,
    notice: Option<String>,
    hata: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct AyarlarQuery {
    alanlar: Option<String>,
    notice: Option<String>,
}

fn jira_bilgi(st: &AppState) -> String {
    match (&st.cfg.jira_url, &st.cfg.jira_project) {
        (Some(u), Some(p)) => format!("{u} · proje {p} · iş türü {}", st.cfg.jira_issue_type),
        _ => "Jira bağlantısı .env'de ayarlı değil (JIRA_URL, JIRA_TOKEN, JIRA_PROJECT)".into(),
    }
}

async fn fetch_meta(st: &AppState) -> Option<Result<Vec<FieldMeta>, String>> {
    let j = Jira::from_config(&st.cfg)?;
    Some(
        tokio::task::spawn_blocking(move || j.field_meta().map_err(|e| e.to_string()))
            .await
            .unwrap_or_else(|e| Err(e.to_string())),
    )
}

pub async fn page(State(st): State<AppState>, _a: Admin, headers: HeaderMap, Query(q): Query<AyarlarQuery>) -> Result<Response, AppError> {
    let a = st.ayarlar.get();
    let alanlar = if q.alanlar.is_some() { fetch_meta(&st).await } else { None };
    Ok(render(&AyarlarPage {
        l: layout(&st, &headers),
        jira: a.jira,
        doors: a.doors,
        yer_tutucular: ayarlar::YER_TUTUCULAR,
        jira_ayarli: Jira::from_config(&st.cfg).is_some(),
        doors_dxl: st.cfg.doors_dxl,
        jira_bilgi: jira_bilgi(&st),
        alanlar,
        notice: q.notice,
        hata: None,
    }))
}

#[derive(Deserialize)]
pub struct JiraForm {
    ozet: String,
    aciklama: String,
    alanlar: String,
    doors_label: Option<String>,
}

pub async fn save_jira(State(st): State<AppState>, _a: Admin, headers: HeaderMap, Form(f): Form<JiraForm>) -> Result<Response, AppError> {
    let t = JiraSablon {
        ozet: f.ozet.trim().to_string(),
        aciklama: f.aciklama.replace("\r\n", "\n"),
        alanlar: f.alanlar.trim().to_string(),
        doors_label: f.doors_label.is_some(),
    };
    let t2 = t.clone();
    match st.db.write(move |c| ayarlar::save_jira(c, &t2)).await {
        Ok(()) => {
            let mut a = st.ayarlar.get();
            a.jira = t;
            st.ayarlar.set(a);
            tracing::info!("Jira bug şablonu güncellendi");
            Ok(Redirect::to("/admin/ayarlar?notice=jira").into_response())
        }
        Err(e) => {
            // girilen değerler kaybolmasın: formu hatayla yeniden göster
            let a = st.ayarlar.get();
            Ok((
                StatusCode::BAD_REQUEST,
                render(&AyarlarPage {
                    l: layout(&st, &headers),
                    jira: t,
                    doors: a.doors,
                    yer_tutucular: ayarlar::YER_TUTUCULAR,
                    jira_ayarli: Jira::from_config(&st.cfg).is_some(),
                    doors_dxl: st.cfg.doors_dxl,
                    jira_bilgi: jira_bilgi(&st),
                    alanlar: None,
                    notice: None,
                    hata: Some(e.to_string()),
                }),
            )
                .into_response())
        }
    }
}

pub async fn save_doors(State(st): State<AppState>, _a: Admin, Form(d): Form<DoorsAyar>) -> Result<Response, AppError> {
    let d = DoorsAyar {
        modul: d.modul.trim().into(),
        id_oznitelik: d.id_oznitelik.trim().into(),
        sonuc_oznitelik: d.sonuc_oznitelik.trim().into(),
        tarih_oznitelik: d.tarih_oznitelik.trim().into(),
        jira_oznitelik: d.jira_oznitelik.trim().into(),
        rapor_oznitelik: d.rapor_oznitelik.trim().into(),
        gecti: d.gecti.trim().into(),
        kaldi: d.kaldi.trim().into(),
        atlandi: d.atlandi.trim().into(),
        kosulmadi: d.kosulmadi.trim().into(),
        link_sablonu: d.link_sablonu.trim().into(),
    };
    let d2 = d.clone();
    st.db.write(move |c| ayarlar::save_doors(c, &d2)).await?;
    let mut a = st.ayarlar.get();
    a.doors = d;
    st.ayarlar.set(a);
    tracing::info!("DOORS ayarları güncellendi");
    Ok(Redirect::to("/admin/ayarlar?notice=doors").into_response())
}

// ---------------------------------------------------------------------------------------------
// Senaryo için Jira bug formu

pub struct FormField {
    pub meta: FieldMeta,
    pub value: String,
    /// POST'ta türü bilmek için: "tür|öğe|seçenekli"
    pub hint: String,
}

impl FormField {
    pub fn selected(&self, id: &str) -> bool {
        self.value.split(',').any(|v| v.trim() == id)
    }
    pub fn multi(&self) -> bool {
        self.meta.kind == "array"
    }
}

#[derive(Template)]
#[template(path = "jira_bug.html")]
struct BugPage {
    l: Layout,
    sid: i64,
    scenario: String,
    bug: NewBug,
    ek: String,
    fields: Vec<FormField>,
    meta_hata: Option<String>,
    hata: Option<String>,
}

fn not_configured(l: Layout) -> Response {
    crate::web::pages::message_page(
        l,
        StatusCode::BAD_REQUEST,
        "Jira ayarlı değil",
        "JIRA_URL, JIRA_TOKEN ve JIRA_PROJECT .env içinde ayarlanmalı.",
    )
}

/// Formda gösterilecek alanlar: Jira'nın zorunlu alanları (raporcu'nun doldurdukları hariç) ve
/// şablonun ek alanlarında geçenler. Değerler şablondan gelir.
fn form_fields(meta: &[FieldMeta], extra: &Map<String, Value>) -> Vec<FormField> {
    meta.iter()
        .filter(|m| !jira::SYSTEM_FIELDS.contains(&m.id.as_str()) && m.id != "labels")
        .filter(|m| (m.required && !m.has_default) || extra.contains_key(&m.id))
        .map(|m| FormField {
            value: to_allowed_ids(m, &extra.get(&m.id).map(form_value).unwrap_or_default()),
            hint: format!("{}|{}|{}", m.kind, m.items, if m.allowed.is_empty() { 0 } else { 1 }),
            meta: m.clone(),
        })
        .collect()
}

/// Seçenekli alanda şablon değeri ad olabilir ({"value": "Yazılım"}); listede seçili gelmesi için
/// izinli değerin kimliğine çevrilir. Eşleşmeyen değer olduğu gibi kalır.
fn to_allowed_ids(m: &FieldMeta, value: &str) -> String {
    if m.allowed.is_empty() {
        return value.to_string();
    }
    value
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| {
            m.allowed
                .iter()
                .find(|(id, name)| id == v || name.eq_ignore_ascii_case(v) || name.to_lowercase() == v.to_lowercase())
                .map(|(id, _)| id.clone())
                .unwrap_or_else(|| v.to_string())
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Şablondaki JSON değerini forma yazılacak metne çevirir ({"id":"10"} → 10, [{"name":"A"}] → A).
fn form_value(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Array(a) => a.iter().map(form_value).collect::<Vec<_>>().join(", "),
        Value::Object(o) => ["id", "value", "name", "key"]
            .iter()
            .find_map(|k| o.get(*k).map(form_value))
            .unwrap_or_default(),
        _ => String::new(),
    }
}

pub async fn bug_form(State(st): State<AppState>, _a: Admin, headers: HeaderMap, Path(sid): Path<i64>) -> Result<Response, AppError> {
    let l = layout(&st, &headers);
    if Jira::from_config(&st.cfg).is_none() {
        return Ok(not_configured(l));
    }
    let (cfg, sablon) = (st.cfg.clone(), st.ayarlar.get().jira);
    let draft = st
        .db
        .read(move |c| {
            let name = crate::queries::scenario(c, sid)?.map(|s| s.name);
            Ok(name.map(|n| (n, jira::draft_for_scenario(c, &cfg, &sablon, sid))))
        })
        .await?;
    let Some((scenario, bug)) = draft else {
        return Err(AppError::not_found());
    };
    let (bug, hata) = match bug {
        Ok(b) => (b, None),
        Err(e) => (NewBug::default(), Some(e.to_string())),
    };
    Ok(form_page(&st, l, sid, scenario, bug, hata).await.into_response())
}

/// Formu çizer: alanlar Jira'dan, değerler `bug`'dan (şablon taslağı ya da gönderilen form).
async fn form_page(st: &AppState, l: Layout, sid: i64, scenario: String, bug: NewBug, hata: Option<String>) -> Response {
    let (fields, meta_hata) = match fetch_meta(st).await {
        Some(Ok(m)) => (form_fields(&m, &bug.extra), None),
        Some(Err(e)) => (vec![], Some(e)),
        None => (vec![], None),
    };
    // formda alanı olanlar JSON kutusunda tekrar edilmez
    let rest: Map<String, Value> = bug
        .extra
        .iter()
        .filter(|(k, _)| !fields.iter().any(|f| &f.meta.id == *k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let ek = if rest.is_empty() {
        String::new()
    } else {
        serde_json::to_string_pretty(&rest).unwrap_or_default()
    };
    let status = if hata.is_some() { StatusCode::BAD_GATEWAY } else { StatusCode::OK };
    (
        status,
        render(&BugPage {
            l,
            sid,
            scenario,
            bug,
            ek,
            fields,
            meta_hata,
            hata,
        }),
    )
        .into_response()
}

fn meta_from_hint(id: &str, hint: &str) -> FieldMeta {
    let mut p = hint.split('|');
    let kind = p.next().unwrap_or("string").to_string();
    let items = p.next().unwrap_or("").to_string();
    let allowed = if p.next() == Some("1") {
        vec![(String::new(), String::new())]
    } else {
        vec![]
    };
    FieldMeta {
        id: id.into(),
        name: id.into(),
        required: false,
        kind,
        items,
        has_default: false,
        allowed,
    }
}

/// Formdan bug: `ozet`, `aciklama`, `ek` (JSON), `alan.<id>` (+ `tur.<id>`), çoklu seçimde aynı ad tekrarlanır.
/// Form boş gönderilirse (eski düğme) şablon olduğu gibi kullanılır.
fn bug_from_form(form: &[(String, String)], draft: NewBug) -> anyhow::Result<NewBug> {
    let get = |k: &str| form.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
    let Some(ozet) = get("ozet") else { return Ok(draft) };
    let mut extra = ayarlar::extra_fields(&get("ek").unwrap_or_default())?;
    let mut values: Vec<(String, Vec<String>)> = Vec::new();
    for (k, v) in form {
        if let Some(id) = k.strip_prefix("alan.") {
            match values.iter_mut().find(|(a, _)| a == id) {
                Some((_, vs)) => vs.push(v.clone()),
                None => values.push((id.to_string(), vec![v.clone()])),
            }
        }
    }
    for (id, vs) in values {
        let m = meta_from_hint(&id, &get(&format!("tur.{id}")).unwrap_or_default());
        match jira::field_value(&m, &vs.join(",")) {
            Some(v) => {
                extra.insert(id, v);
            }
            None => {
                extra.remove(&id);
            }
        }
    }
    let doors_labels = form.iter().any(|(k, _)| k == "doors_label");
    Ok(NewBug {
        summary: ozet.trim().to_string(),
        description: get("aciklama").unwrap_or_default().replace("\r\n", "\n"),
        labels: if doors_labels { draft.labels } else { vec![] },
        extra,
    })
}

pub async fn bug_create(
    State(st): State<AppState>,
    _a: Admin,
    headers: HeaderMap,
    Path(sid): Path<i64>,
    Form(form): Form<Vec<(String, String)>>,
) -> Result<Response, AppError> {
    let l = layout(&st, &headers);
    let Some(j) = Jira::from_config(&st.cfg) else {
        return Ok(not_configured(l));
    };
    let (cfg, sablon, paths) = (st.cfg.clone(), st.ayarlar.get().jira, st.paths.clone());
    // dış hata: senaryo yok / JSON bozuk; iç hata: Jira reddetti (form girilen değerlerle yeniden gösterilir)
    let res = st
        .db
        .write(move |c| {
            let name = crate::queries::scenario(c, sid)?
                .map(|s| s.name)
                .ok_or_else(|| anyhow::anyhow!("senaryo yok"))?;
            let draft = jira::draft_for_scenario(c, &cfg, &sablon, sid)?;
            let bug = bug_from_form(&form, draft)?;
            let created = jira::create_for_scenario(c, &paths, &j, sid, &bug).map_err(|e| e.to_string());
            Ok((name, bug, created))
        })
        .await;
    match res {
        Ok((_, _, Ok(_))) => Ok(Redirect::to(&format!("/scenarios/{sid}")).into_response()),
        Ok((name, bug, Err(e))) => Ok(form_page(&st, l, sid, name, bug, Some(e)).await),
        Err(e) => Ok(crate::web::pages::message_page(
            l,
            StatusCode::BAD_REQUEST,
            "Jira bug açılamadı",
            &e.to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_6_1_form_fields_and_values() {
        let meta = vec![
            FieldMeta {
                id: "summary".into(),
                name: "Özet".into(),
                required: true,
                kind: "string".into(),
                items: "".into(),
                has_default: false,
                allowed: vec![],
            },
            FieldMeta {
                id: "customfield_10200".into(),
                name: "Faz".into(),
                required: true,
                kind: "option".into(),
                items: "".into(),
                has_default: false,
                allowed: vec![("10".into(), "Yazılım".into()), ("11".into(), "Sistem".into())],
            },
            FieldMeta {
                id: "priority".into(),
                name: "Öncelik".into(),
                required: false,
                kind: "priority".into(),
                items: "".into(),
                has_default: true,
                allowed: vec![("3".into(), "Major".into())],
            },
        ];
        let extra = ayarlar::extra_fields(r#"{"priority": {"id": "3"}}"#).unwrap();
        let f = form_fields(&meta, &extra);
        let ids: Vec<&str> = f.iter().map(|x| x.meta.id.as_str()).collect();
        assert_eq!(ids, vec!["customfield_10200", "priority"], "sistem alanları formda yok");
        assert_eq!(f[1].value, "3");
        // şablonda ad ile verilen seçenek listede kimliğiyle seçili gelir
        let extra = ayarlar::extra_fields(r#"{"customfield_10200": {"value": "sistem"}}"#).unwrap();
        assert_eq!(form_fields(&meta, &extra)[0].value, "11");
        let form: Vec<(String, String)> = vec![
            ("ozet".into(), "Özet".into()),
            ("aciklama".into(), "a\r\nb".into()),
            ("ek".into(), r#"{"components": [{"name": "UI"}]}"#.into()),
            ("alan.customfield_10200".into(), "11".into()),
            ("tur.customfield_10200".into(), "option||1".into()),
            ("alan.priority".into(), "".into()),
            ("tur.priority".into(), "priority||1".into()),
        ];
        let b = bug_from_form(
            &form,
            NewBug {
                labels: vec!["DOORS-1".into()],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(b.extra["customfield_10200"], serde_json::json!({"id": "11"}));
        assert!(b.extra.get("priority").is_none(), "boş bırakılan alan gönderilmez");
        assert_eq!(b.extra["components"][0]["name"], "UI");
        assert_eq!(b.description, "a\nb");
        assert!(b.labels.is_empty(), "DOORS label kutusu işaretsiz");
        // boş form: şablon olduğu gibi
        let d = bug_from_form(
            &[],
            NewBug {
                summary: "x".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(d.summary, "x");
    }
}
