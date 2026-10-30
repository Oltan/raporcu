# crates/raporcu/templates — HTML şablonları

**Motor:** askama 0.16 (derleme zamanında Rust koduna çevrilir; sözdizimi Jinja2 ile neredeyse
aynı). Python'da **Jinja2** ile birebir kullanılabilir; farklar aşağıda.
Veriyi hazırlayan kod: `web/pages.md` (her şablonun struct'ı orada). Grafik SVG'leri sunucuda
`charts.rs` üretir ve şablona `|safe` ile gömülür; etkileşim (ipucu, artı imleç) `static/charts.js`.

| Askama | Jinja2 karşılığı |
|--------|------------------|
| `{% if let Some(x) = alan %}` | `{% if alan is not none %}{% set x = alan %}` |
| `alan.as_deref()`, `.is_some()`, `.is_none()`, `.len()` | `alan`, `alan is not none`, `alan is none`, `alan\|length` |
| `{% match %}` | `if/elif` |
| metot çağrısı (`run.failures()`) | Python'da özellik/metot olarak aynı |
| otomatik HTML kaçışı | Jinja2'de `autoescape=True` açılmalı |

Ortak alan **`l` (Layout)**: `l.admin` (oturum var mı), `l.share` (paylaşım görünümü mü),
`l.banner` (disk/uyarı bandı), `l.run_url(id)`, `l.scenario_url(id)`, `l.file_url(id)` — paylaşım
görünümünde linkler `/s/<token>/…` biçimine döner, iç linkler (admin, arama) gizlenir (SW-5.1).

## Şablonlar

| Dosya | Sayfa (URL) | İçerik |
|-------|-------------|--------|
| `base.html` | hepsi | `<head>` (app.css, htmx), üst menü (Dashboard, Arama, Self-healing, Admin, giriş/çıkış), paylaşımda "Salt okunur paylaşım", `banner`, `content` bloğu |
| `_bar.html` | include | ince yığılmış çubuk: geçti / kaldı / broken / bilinmiyor+atlandı; `flex-grow` = sayı, `data-tip` ipucu, `aria-label` |
| `_lejant.html` | include | donut lejantı: renk, ad, sayı, % |
| `message.html` | hata/bilgi | başlık + metin + Dashboard linki |
| `login.html` | `/login` | admin şifre formu (`next` ile geri dönüş), hata mesajı |
| `dashboard.html` | `/` | proje/branch/zaman penceresi süzgeci; **son koşum donut**u + lejant; **geçme oranı çizgi grafiği**; koşum başına yığılmış sütunlar; **feature bazında** tablo (durum, oran, çubuk, değişim, sparkline, feature sayfası linki); pipeline'lar (webhook durumu, SW-3.6); koşumlar (sayfalı, ilerleme, çubuk); flaky senaryolar (SW-4.2) |
| `run.html` | `/runs/{id}` | başlık + CI bilgisi (proje, branch, commit, pipeline/iş linkleri, OS, SW-4.3); admin düğmeleri: paylaşım linki, "OpenCode ile düzelt" (yalnızca fix işini tetikler, SW-7.4), CSV; durum dağılımı donut; **feature tablosu** (her feature için çubuk, oran, süre, kalan sayısı, `#anchor` linki); OpenCode düzeltme kutusu (SW-7.3); **feature gruplu senaryo listesi** (`<details>`, kalanlar açık; durum, süre, kısa hata, triage etiketi, deneme sayısı, Jira/düzeltme rozetleri) |
| `scenario.html` | `/scenarios/{id}` | ad, feature, konum, durum, süre, tag'ler (DOORS/Jira linkleri); **hata** (mesaj + iz); **ekler** (görüntü küçük resim, video oynatıcı, metin ekleri — sahne ağacı); **adımlar** (iç içe, `st.depth` girinti, durum rengi); **triage** (kaynak, güven, gerekçe; admin formu); **Jira** (bağlı bug'lar, "bug aç"); **OpenCode** (düzeltme denemeleri, MR linki, uyarılar); **denemeler** (rerun, SW-4.9) |
| `history.html` | `/history?…` | bir senaryonun koşumlar boyunca geçmişi (sayfalı) |
| `feature.html` | `/feature?project_id&name&branch` | feature özeti (ortalama süre, flaky sayısı); oran çizgisi; koşum başına durum sütunları; **senaryo × koşum matrisi** (hücre = durum, deneme sayısı, koşuma link) |
| `search.html` | `/search` | serbest metin, tag, feature, DOORS, Jira, triage, durum, proje, branch süzgeçleri; CSV dışa aktarma formu (alan seçimi ve sırası, SW-4.8); DOORS CSV/DXL linkleri; sayfalı sonuç tablosu |
| `live.html` | `/runs/{id}/live` | sayaçlar (ilerleme x/N, geçti, kaldı, koşuyor, bekliyor), **"şu an koşuyor" kartı** (`#simdi`: senaryo, feature:satır, geçen süre sayacı, adımlar + koşan adım; adım yoksa "Hazırlık: @Before"), senaryo tablosu (durum, ad→rapor linki, ilk hata satırı, feature, ekran görüntüsü) |
| `ayarlar.html` | `/admin/ayarlar` | Jira bug şablonu (özet, açıklama, ek alanlar JSON, DOORS label), yer tutucu ve alan biçimi tabloları, "Jira alanlarını getir" tablosu, DOORS ayarları |
| `jira_bug.html` | `/admin/scenarios/{id}/jira` | bug formu: özet, açıklama, Jira alanları (seçenekli → açılır liste, çoklu → çoklu liste), diğer alanlar JSON |
| `_taslaklar.html` | include | Jira bug taslağı kartları (`drafts`, `taslak_geri`): durum, özet, kaynak ve gerekçe, içerik (açılır), admin için Jira'da aç / Düzenleyip aç / Yok say |
| `admin.html` | `/admin` | disk doluluğu, Jira modu, sürüm; paylaşım linkleri (iptal et); Jira gözlem kayıtları (SW-6.3); son uyarı/hata log satırları |
| `healing.html` | `/healing` | son 30 gece: deneme / MR / birleşen / reddedilen grafiği ve tablosu (SW-7.5) |

## live.html betiği (sunucu tarafı yok, saf JS)
1. `ws://<host>/ws/runs/{id}` WebSocket'i açılır (HTTP için `ws://`; sayfa https ise `wss://`).
2. Her mesaj bir **anlık görüntü** JSON'u (`live.md` → `snapshot`): `status`, `progress`,
   `passed/failed/running/waiting`, `current` (şu an koşan), `scenarios[]`
   (`state, name, feature, message, scenario_id, screenshot_id`).
3. `ciz(s)` DOM'u **textContent** ile yeniden kurar (hata mesajları HTML olarak yorumlanmaz).
4. Bağlantı koparsa 1 sn'den 15 sn'ye kadar artan beklemeyle yeniden bağlanır.
5. Geçen süre sayacı tarayıcıda her saniye artar (`elapsed_s` + yerel saat farkı).
Python'da yalnızca WebSocket ucu aynı JSON'u göndermelidir; şablon ve betik değişmeden kalır.
