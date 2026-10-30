# divan/src/queries.rs

**Amaç:** Okuma sorguları; sayfalar ve JSON API ortak kullanır. Liste sorguları sayfalıdır ya da tek
koşum/senaryo ile sınırlıdır (SW-10.6).

## Satır tipleri ve türetilen alanlar
- **RunRow** (runs + projects): `failures()` = failed+broken; `short_commit()` ilk 8;
  `commit_url()` = `<project_url>/-/commit/<sha>`; `progress()` = `"toplam/beklenen"` ya da
  `"toplam"`; `pass_pct()` = round(100·passed/total).
- **ScenarioRow**: `tags`, `doors_ids`, `jira_keys` listeye açılır; `plain_tags()` DOORS/Jira
  tag'leri hariç; `location()` = fullName'in son parçası (`giris.feature:28`); `duration()`
  ("2.0 sn" / "392 ms"); `short_message()` ilk satır, 160 karakter.
- **StepRow** (`depth` ile), **AttachmentRow** (`is_image/is_video/is_text/is_deleted`,
  `preview` = küçük metin ekinin içeriği, sayfa doldurur), **BugLink**, **FixRow**
  (`outcome_label`, `badge` "OpenCode !41"), **PipelineRow** (`status_label`: kuyrukta /
  çalışıyor / bitti…), **ShareRow**, **FeatureRow**, **MatrixRow/MatrixCell**.

## Sorgular
| Fonksiyon | Ne döner |
|-----------|----------|
| `run`, `list_runs(proje?, branch?, limit, offset)` | koşumlar, yeniden eskiye |
| `projects` | projeler + her birinin branch'leri |
| `run_scenarios(run)` | son denemeler; sıra: failed, broken, unknown, skipped, passed; sonra feature, başlangıç |
| `scenario`, `attempts(run, key)` | tek senaryo; aynı koşumdaki denemeler sırayla |
| `steps`, `attachments`, `attachment`, `first_screenshot` | senaryonun adımları/ekleri; ilk silinmemiş görüntü |
| `bug_links`, `fixes_for_run`, `fixes_for_scenario`, `pipelines`, `share_links`, `share_run(token)` | iptal edilmemiş link → koşum |
| `search(filtre)` | son denemeler + toplam sayı, 50'lik sayfa |
| `trend(proje, branch, n)` | son n **biten** koşum, eskiden yeniye |
| `run_features(run)` | feature başına sayılar ve süre; sorunlu olanlar önce |
| `feature_trends(proje, branch, n)` | feature → son n koşumdaki geçme yüzdeleri |
| `feature_runs(proje, branch, feature, n)` | feature'ın geçtiği son n koşum + o koşumdaki sayıları |
| `feature_matrix(feature, run_ids)` | senaryo × koşum: hücre durumu (koşmadıysa `yok`), deneme sayısı; satır başına kalan sayısı, flaky (hem geçti hem kaldı ya da tekrar denemede geçti); en çok kalan üstte |

## Arama (SW-4.7) — `SearchFilter`
Alanlar: `project, branch, run, tag, feature, doors, jira, triage, status, q, page`.
`normalized()`: boşları atar, tag'den `@` atar, `doors=1004` → `DOORS-1004`, Jira büyük harf.
`search_where` her zaman `is_latest=1`:
- tag/doors: `" <değer> "` ile LIKE (tam kelime), feature: içerir, jira: `jira_keys` LIKE **veya**
  `bug_links`'te kayıt, triage: `triage_category =`, status `kaldi` = failed+broken, `q`: mesaj ya
  da ad içinde. LIKE özel karakterleri kaçışlanır.
`query_string()` sayfalama ve CSV linkleri için.

## Python karşılığı
Aynı SQL dizeleri; satırları `sqlite3.Row` + dataclass'a dönüştüren küçük fonksiyonlar.
