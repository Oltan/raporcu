# divan/src/yayin.rs

**Amaç:** SW-5.1 (0.2.0) — dışa açık sürüm raporları. Admin bir **repo + yazılım adı + sürüm**
üçlüsü için koşumları seçip yayınlar; seçili koşumlar tek raporda birleşir.

## Kavramlar
- **Yazılım adı** (`yazilim_adi(run)`): koşumun `app_name`'i (ulak: `UYGULAMA_ADI`), yoksa proje
  yolunun son parçası. Aynı repo farklı tag'lerle farklı yazılımlar (konfigürasyonlar) olarak
  koşulabilir; panelde tek proje, dışarıda ayrı raporlar.
- **Sürüm**: `app_version` (`UYGULAMA_SURUMU` ya da pom). Sürümü olmayan koşum yayınlanamaz.
- **Sınıf** (`sonuc(status)`): `passed` → Geçen (deneme sayısına ve flaky'ye bakılmaz; tekrarda geçen
  de geçen), `failed`/`broken` → Kalan, diğerleri (atlandı/bilinmiyor) toplam dışı.
- `Sayim { gecen, kalan }`: `toplam()`, `oran()` (başarı %), `dilimler()` (Geçti/Kaldı), `cubuk()`.

## Birleştirme — `birlestir(koşumlar, hariç)` (saf)
Girdi: (koşum başlangıcı, son denemeler). Başlangıca göre sıralanır; aynı `scenario_key` sonra
başlayan koşumda varsa onun sonucu geçerli. Çıkarılanlar ve sayıları:
- feature dosyası (`feature_file`, Allure `gherkin_uri`) boş olanlar → `cucumber_disi` (JUnit /
  ortam yoklama testleri, ör. `core.HeadlessEnvironmentProbe.AwtDisplayProbeTest`);
- `hariç` kümesindekiler (admin'in çıkardığı) → `haric`;
- koşulmamışlar → `kosulmadi`.
İlk görünme sırası korunur.

## Rapor — `olustur(c, yazılım, sürüm, koşumlar, hariç)`
Birleşimi feature'a göre gruplar (`Ozellik { ad, anchor, sayim, testler }`, feature adına göre
sıralı; içinde kalanlar önce). Her `Test { s, sonuc, jira }`: `jira`, o senaryonun koşumundaki bağlı
issue'lar (`jira::kosum_issuelari`). Tarih: seçili koşumların en son başlangıcı.

## Kayıt
Tablolar (migration `0009_yayin`): `yayinlar (project_id, app_name, app_version)` tekil,
`yayin_kosumlari`, `yayin_haric`.
- `kaydet(...)`: üçlü varsa koşum ve hariç listelerini değiştirir (`updated_at`), yoksa ekler. En az bir
  koşum şart. Log: `rapor yayınlandı`.
- `sil(id)`: rapor ana sayfadan kalkar, koşumlar silinmez. Log: `rapor yayından kaldırıldı`.
- `adaylar(proje)`: biten, sürümü olan koşumlar (yazılım, sürüm) gruplu, en yeni önce;
  `uclu_kosumlari`, `bul`, `kosum_idleri`, `haric`, `onceki_haric` (aynı yazılımın son raporundaki
  hariçler: yeni rapora öneri).
- `liste(yazılım?)`: yayınlanmışların özetleri (`YayinOzet { y, tarih, sayim }`), tarihe göre eskiden yeniye.
- `yazilimlar()`: yayını olan yazılım adları, en son güncellenen önce (ana sayfa sekmeleri).
- `senaryo_raporda(yayın, sid)`: test ayrıntısı ve ek erişimi için.

## Testler
`test_sw_5_1_yayin_birlestirme_kurallari` (birim); `tests/rapor.rs`:
`test_sw_5_1_yayin_iki_kosum_tek_raporda_ve_ana_sayfa`.

## Python karşılığı
Aynı tablolar ve kurallar; `birlestir` saf fonksiyon olarak birebir taşınır.
