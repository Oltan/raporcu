# divan/src/export.rs

**Amaç:** SW-4.8 alan seçmeli CSV, SW-6.4 gereksinim (YTET/DOORS) CSV'si ve DXL.

## CSV (`GET /export.csv?<arama filtresi>&fields=a,b,c` ya da `alan=a&alan=b`, `sep=;`)
- Alanlar (sırayla seçilebilir): `proje, branch, kosum, senaryo, feature, taglar (@ ile), durum,
  triage (etiket), hata (ilk satır), sure (ms), doors, jira (tag + bug_links, tekilleştirilmiş),
  tarih`; `doors`'tan hemen sonra her ek önek için `ek:<önek>` (`ek:YGÖ-`, değer tag'lerden).
  Başlıklar `alanlar(önekler)`'den: `doors` sütununun başlığı önek adıdır (`YTET`), ek sütunlar
  `YGÖ`. Bilinmeyen alan atlanır; hiç alan yoksa hepsi.
- İlk karakter UTF-8 BOM (Excel Türkçe karakterleri doğru açar); satır sonu `\r\n`.
- Hücre: NUL silinir; `= + - @` ile başlıyorsa başına `'` (formül enjeksiyonu); ayırıcı, `"` ya da
  satır sonu içeriyorsa çift tırnak içinde, `"` → `""`.
- **Akış:** satırlar okuma bağlantısında okunur, ~64 KB parçalar halinde kanala ve HTTP gövdesine
  yazılır (Content-Length yok); büyük listeler belleğe alınmaz (SW-10.6). İstemci koparsa durur.

## DOORS (ayarlar: `ayarlar.md` → DoorsAyar)
Aktarım CSV ile. DXL varsayılan kapalı: `DOORS_DXL=true` değilse `/runs/{id}/doors.dxl` 404 ve düğme gizli.

- `doors_rows(koşum, ayar)`: gereksinim numarası olan her son deneme, **her numara için bir satır**;
  sonuç ayardaki değerlerle (`ayar.status`); `ekler` her ek önek için o önekli kimlikler (boşlukla).
- `doors_csv` / `doors_csv_with(önekler, …)`: BOM + `<önek adı>,Senaryo,Feature,Sonuç,Jira,Koşum,Tarih`
  ve her ek önek için bir sütun (`YTET,…,Tarih,YGÖ`).
- `doors_dxl(satırlar, koşum, rapor, ayar)`: başta ayarlar DXL değişkeni olarak (`modulYolu`,
  `idAttr`, `resultAttr`, `dateAttr`, `jiraAttr`, `linkAttr`), dizilerde numara (gereksinim öneki
  atılmış), sonuç, tarih, Jira. Modül yolu boşsa `current`, doluysa `edit(yol, false)` (açılamazsa
  açık mesaj). Kimlik özniteliği eşleşen nesnelerde boş olmayan ve `canWrite` olan öznitelikleri yazar;
  modülü betik açtıysa `save`. Sonunda güncellenen nesne ve bulunamayan numara sayısı.

## Testler
`test_sw_4_8_*`, `test_sw_6_4_doors_csv_and_dxl`, `test_sw_6_4_ytet_ve_ygo_rapor_arama_csv_jira`.
