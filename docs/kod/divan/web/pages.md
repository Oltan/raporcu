# divan/src/web/pages.rs

**Amaç:** HTML sayfaları (askama şablonları + htmx + küçük JS). Raporları görmek giriş
gerektirmez; admin işlemleri `Admin` çıkarıcısıyla korunur.

## Ortak
- `Layout { admin, banner, share }`: admin oturumu var mı; admin'e disk uyarı bandı; paylaşım
  görünümündeyse token. `file_url/scenario_url/run_url` paylaşımda `/s/<token>/…` döner (ham ek
  linki görünmez).
- Şablon filtreleri: `zaman` (RFC3339 → yerel `gg.aa.yyyy ss:dd`), `enc` (URL kodlama).
- `feature_anchor(ad)` → `f-giris-ekrani` (Türkçe harfler sadeleşir); rapor ve dashboard bağlantıları.

## Sayfalar
| Sayfa | Veri |
|-------|------|
| **Dashboard** `/` | Filtre: proje, branch. Kapsam = seçili proje+branch, yoksa en son **biten** koşumunki. Son koşum halka grafiği + lejant (sayı, %) + flaky sayısı; geçme oranı gidişatı (son 20 biten koşum); koşum başına yığılmış sütunlar; feature tablosu (durum çubuğu, sayılar, %, son 10 koşum mini çizgisi, ilk koşuma göre değişim ▲▼, geçmiş linki); pipeline'lar (son 10); koşumlar (50'lik sayfa, ince dağılım çubuğu); flaky senaryolar (en fazla 50) |
| **Koşum** `/runs/{id}` | Başlık: proje, branch, commit linki, repo, pipeline, iş, OS, tarih, durum. Halka + lejant; feature özet tablosu; OpenCode düzeltmeleri kutusu; **feature grupları** (`<details>`, kalanı olan açık): senaryo, `dosya:satır`, deneme rozeti, flaky, OpenCode !MR, DOORS/Jira/tag rozetleri, hata ilk satırı, triage + kaynak, süre, ekran görüntüsü küçük resmi. Admin: paylaşım linki, OpenCode ile düzelt |
| **Senaryo** `/scenarios/{id}` | Hata + yığın izi, ekler (görüntü, video, ≤8 KB metin önizleme, silinmişse neden), adımlar (derinliğe göre girintili, durum, süre, hata), triage (geçerli + admin formu + geçmiş), Jira (bağlantılar + admin "bug aç"), OpenCode denemeleri, aynı koşumdaki denemeler |
| **Canlı** `/runs/{id}/live` | Sayaçlar, şu an koşan senaryo kartı, senaryo listesi; WebSocket ile güncellenir (`live.md`) |
| **Canlı giriş** `/canli` | Sabit bekleme sayfası: saniyede bir `/api/canli` sorar, etkin koşum çıkınca `/runs/{id}/live` adresine geçer. Etkin = `running` ve son 120 sn içinde etkinlik; yarım kalıp zaman aşımını bekleyen koşum sayılmaz |
| **Geçmiş** `/history` | Senaryonun koşumlardaki son denemeleri, 50'lik |
| **Feature** `/feature` | Son 30 biten koşum: özet kutuları, geçme oranı çizgisi, durum sütunları, senaryo × koşum matrisi (hücre linki senaryoya) |
| **Arama** `/search` | Filtreler + 50'lik sonuç + CSV alan seçimi formu |
| **Self-healing** `/healing` | Son 30 gece: deneme, MR, birleştirilen, reddedilen; yığılmış sütunlar + tablo |
| **Admin** `/admin` | Paylaşım linkleri (iptal), Jira gözlem kayıtları (son 100), son WARN/ERROR (son 200), disk %, Jira modu, sürüm |
| **Paylaşım** `/s/{token}` | Koşum sayfası, temizlenmiş (`share.md`); iptal edilmiş/yok → 404 |

`parse_filter(sorgu)`: arama/CSV sorgusunu çözer (`+` boşluk; `run=#12` kabul).
`healing_days(c, bugün)`: son 30 günün (UTC tarih) deneme/MR/birleşen/reddedilen sayıları.

## Admin işlemleri
- Paylaşım oluştur: 64 hex token, `/runs/{id}?notice=share:<token>`'a yönlendirir; iptal: `revoked_at`.
- Triage: kategori geçerli değilse 400; admin kararı eklenir, geçerli karar tazelenir.
- Jira'ya bug aç: Jira ayarlı değilse 400 sayfa; hata 502 sayfa.
- OpenCode ile düzelt: projenin GitLab id'si (yoksa yolu) ve branch ile `gitlab::trigger_fix_pipeline`;
  başarı → `?notice=fix`, hata → 502 sayfa (token içermez).

## Ek (Jira/DOORS ayarları, senaryo düğmesi)
- `Layout.doors_link` + `doors_url(id)`: DOORS rozeti linki (Ayarlar > DOORS); paylaşım görünümünde yok.
- Jira bug formu ve ayarlar sayfası `web/ayarlar.rs`'e taşındı (`ayarlar.md`); `message_page` dışarı açık.
- `POST /admin/runs/{id}/fix` (koşumun tüm adayları) ve `POST /admin/scenarios/{id}/fix` (yalnızca o
  senaryo) → `start_fix` → `gitlab::trigger_fix_pipeline`; senaryo düğmesi `?notice=fix` ile senaryo sayfasına döner.
- DOORS CSV/DXL ayarlarla üretilir.
