# divan/src/web/genel.rs

**Amaç:** SW-5.1 (0.2.0) dışa açık sayfalar. Giriş istemez; yalnızca admin'in yayınladığı raporlar
(`yayin.md`) görünür. İç bilgi yok: koşum numarası, commit, branch, OS, CI linkleri, Jira linki,
atanan kişi, iç adımlar, adım mesajları, video ve metin ekleri. "İnceleniyor" sınıfı yoktur.

| Adres | Ne | Şablon |
|-------|----|--------|
| `/?yazilim=<ad>` | ana sayfa: yazılım sekmeleri (varsayılan en son yayınlanan) | `genel_ana.html` |
| `/rapor/{id}` | sürüm raporu | `paylasim.html` |
| `/rapor/{id}/test/{sid}` | test ayrıntısı | `paylasim_senaryo.html` |
| `/rapor/{id}/dosya/{aid}` | rapordaki testin ekran görüntüsü | dosya |
| `/g/...` | 0.1.x genel ana sayfa linkleri → `/` | yönlendirme |

## Ana sayfa
Seçili yazılımın raporları (`yayin::liste`). Son rapor: dört sayı (`_genel_sayilar.html`: Toplam
test, Geçen test, Kalan test, Başarı oranı), halka ve lejant, "Raporu aç". "Sürümlere göre başarı"
çizgisi ve "Sürüm başına test sonuçları" sütunları son 20 rapor için; etiket sürüm, nokta/sütun raporu
açar. Raporlar tablosu: sürüm, tarih, toplam, geçti, kaldı, başarı, çubuk, "Raporu açmak için
tıklayın →"; satırın tamamı tıklanınca rapor açılır. Bölüm tablosu, feature
geçmişi, sonuç cümlesi ve commit yok.

## Sürüm raporu
Başlık "Yazılım · sürüm X", tarih. Dört sayı, koşulmayan sayısı (varsa). Dağılım kartı: halka,
yanında lejant ve "N testten X tanesi geçti, Y tanesi kaldı." cümlesi. Yanında "Feature bazında
başarı" tablosu: feature, sürüm, son koşum çubuğu, geçti, kaldı, başarı oranı; satıra tıklayınca o
feature'ın test case'leri açılır. "Test case'ler":
feature'lar `<details>` (kalanı olan açık), içinde her test `<details class="test-case">`: kapalıyken
durum, ad, gereksinim/ek kimlik rozetleri, Jira anahtarları (`JIRA_URL` varsa Jira'da yeni sekmede
açılan link, ipucunda başlık), süre; açınca kalan testte hata özeti (`sanitize_text`), bağlı Jira
issue'ları (anahtar, başlık, durum), ilk ekran
görüntüsü ve "Adımlar ve ekran görüntüleri →". "Hepsini aç / kapa". Ayrı ürün hataları ya da Jira
bölümü yoktur.

## Test ayrıntısı
Rapordaki testler açılır (başkası 404): durum, süre, gereksinim ve Jira rozetleri, "Jira issue'ları"
kartı (anahtar, başlık, durum; `JIRA_URL` varsa Jira'da yeni sekmede açılan link), kalan testte hata özeti,
üst düzey adımlar (`depth = 0`, ad `sanitize_text`'ten, mesaj yok), ekran görüntüleri; bölümler
açılır/kapanır.

## Testler
`tests/rapor.rs`: `test_sw_5_1_yayin_iki_kosum_tek_raporda_ve_ana_sayfa`; `tests/kimlik.rs`:
`test_sw_6_2_bagli_jira_issuelari_raporda`.

## Python karşılığı
Aynı sorgular ve Jinja2 şablonları.
