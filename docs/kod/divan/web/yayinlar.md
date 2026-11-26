# divan/src/web/yayinlar.rs

**Amaç:** SW-5.1 (0.2.0) Admin > Raporlar: dışa açık sürüm raporlarını yayınlama. Hepsi admin ister.

| Uç | Davranış |
|----|----------|
| `GET /admin/yayinlar` | yayınlanabilir (yazılım, sürüm) üçlüleri (repo, koşum sayısı, son koşum, yayında mı, yayına bağlı olmayan yeni koşum sayısı) ve yayınlanmış raporlar (sayılar, Düzenle, Kaldır) |
| `GET /admin/yayinlar/yeni?run=N` ya da `?proje=&yazilim=&surum=` | form: üçlünün biten koşumları (varsayılan hepsi seçili), birleşik test listesi (feature'a göre), her testte "rapordan çıkar" (varsayılan: mevcut yayının ya da aynı yazılımın son raporunun hariçleri), sayılar, JUnit/yoklama ve atlanan sayısı. Kutu değişince form `onizle=1` ile kendini yeniler (`kosum`, `haric` tekrar eden alanlar) |
| `POST /admin/yayinlar` | `proje`, `yazilim`, `surum`, `kosum`*, `haric`*; yalnızca o üçlünün koşumları kabul edilir; hiç koşum yoksa form hatayla; başarı → `/rapor/{id}` |
| `POST /admin/yayinlar/{id}/sil` | raporu kaldırır → liste |

Koşum sayfasında admin düğmesi "Sürüm raporunu yayınla" (`?run=N`; koşumun sürümü yoksa görünmez).

## Testler
`tests/rapor.rs`: `test_sw_5_1_yayin_iki_kosum_tek_raporda_ve_ana_sayfa`.
