# divan/src/web/genel.rs

**Amaç:** SW-5.1 genel rapor sayfaları: ekip dışına verilen, yalnızca ürün sonucu gösteren görünüm.
Sınıf kuralı `share.md` içinde (`genel_sinif`); burası sayıları, grafikleri ve sayfaları kurar.

## Girişler
| Adres | Kapsam | Şablon |
|-------|--------|--------|
| `/g/{token}` | proje + branch ana sayfası | `genel_ana.html` |
| `/g/{token}/ozellik?ad=` | bölüm (feature) geçmişi | `genel_ozellik.html` |
| `/g/{token}/runs/{id}` | kapsamdaki biten koşumun raporu | `paylasim.html` |
| `/g/{token}/runs/{id}/scenarios/{sid}` | senaryo detayı | `paylasim_senaryo.html` |
| `/g/{token}/runs/{id}/files/{aid}` | ekran görüntüsü | dosya |
| `/s/{token}` ve altı (`gorunum='genel'`) | tek koşum, ana sayfa yok | aynı şablonlar |

`rapor_sayfasi`, `senaryo_sayfasi` ve `ek_acik` iki giriş için ortaktır; `taban` senaryo ve ek
adreslerinin başıdır, `ana` ana sayfa adresidir (tek koşum linkinde yok).

## Feature sekmeleri
Rapor sayfasında "Tümü" ve her feature için bir sekme vardır (`?bolum=<ad>`). Sekme seçiliyken sonuç
bandı, sayılar, halka, bölüm durumu ve senaryolar yalnızca o feature'ı gösterir. Bilinmeyen sekme
tüm koşumu gösterir. Senaryo sayfasındaki "Rapora dön" linki senaryonun sekmesine döner.

## Kapsam denetimi
`portal_kosumu`: link iptal edilmemiş, koşum aynı proje ve branch'te ve bitmiş olmalı, yoksa 404.
Koşan koşum gösterilmez, çünkü sonucu henüz kesin değildir.

## Hesaplar
- `Sayim`: doğrulanan, ürün hatası, inceleniyor; `oran()` doğrulananların yüzdesi.
- `kosum_satirlari`: bir koşumun senaryoları için `genel_sinif`; flaky kümesi kapsam başına bir kez
  alınır. `portal_kosumlari`: son 50 biten koşum, eskiden yeniye.
- `son_ozet`: halka, gidişat çizgisi ve sütunlar (son 20), bölüm tablosu ve mini gidişat (son 10).
- `matris`: son 12 koşumda senaryo adı × koşum. Satır anahtarı addır, çünkü Allure kimliği feature
  dosyasındaki satıra bağlıdır ve dosya değişince yenilenir. Aynı adın birden çok örneği varsa en
  kötü sonuç görünür. Son göründüğü koşumda inceleniyor olan senaryo listelenmez.

## Senaryo detayı
Doğrulanan ve ürün hatası senaryolar açılır: durum, süre, DOORS, Gherkin adımları (süre ve durumla),
ekran görüntüleri, ürün hatasında kısa hata metni. Adım adları `sanitize_text`'ten geçer; iç adımlar
(`depth > 0`), adım hata metni, tag, dosya konumu, deneme, yığın izi, video ve metin ekleri yoktur. Inceleniyor senaryosu 404.

## Link yönetimi
`portal_olustur` (`POST /admin/runs/{id}/portal`): kapsamda açık link varsa onu verir, yoksa yenisini
üretir ve `/runs/{id}?notice=portal:<token>`'a yönlendirir. `portal_iptal`: `revoked_at`.

## Testler
`test_sw_5_1_genel_ana_sayfa_has_charts_and_stays_in_scope`,
`test_sw_5_1_genel_rapor_shows_only_product_results`.

## Python karşılığı
Aynı sorgular ve Jinja2 şablonları; sınıf hesabı `genel_sinif` ile birebir.
