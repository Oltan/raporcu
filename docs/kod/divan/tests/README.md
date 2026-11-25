# crates/divan/tests — entegrasyon testleri

Tüm testler sunucuyu **süreç açmadan**, `divan::app(config)` router'ına `tower::ServiceExt::oneshot`
ile istek göndererek sınar (geçici `DATA_DIR`). Ad kuralı: `test_sw_<bölüm>_<madde>_<ne>` —
gereksinim numarası adın içinde (`cargo test sw_4_1` o gereksinimin hepsini koşar).

Python'a geçişte bu dosyalar **davranış tanımı** olarak kullanılmalı: her testin girdisi
(`testdata/allure/...` zip'i, istek) ve beklenen çıktısı (durum kodu, sayfada geçen metin, JSON
alanı) dil bağımsızdır; `pytest` + `httpx.AsyncClient(app=...)` ile birebir yazılabilir.

## util/mod.rs — ortak yardımcılar
| Öğe | Ne yapar |
|-----|----------|
| `CI_TOKEN` | testlerde kullanılan CI tokenı |
| `app()`, `app_with(f)` | geçici klasörle `Config` kurar (`f` ayarları değiştirir), `TestApp {router, state, dir}` |
| `TestApp::send`, `get`, `get_admin`, `admin_cookie`, `ci_post`, `ci_json` | oneshot istek yardımcıları (CI için Bearer token, admin için oturum çerezi) |
| `body_string` | yanıt gövdesi |
| `sample_dir`, `zip_dir`, `zip_dir_filtered`, `zip_entries` | örnek sonuçlardan bellek içi zip |
| `upload_sample(app, sorgu)` | örneği `POST /api/ci/runs?…` ile yükler, run_id döner |
| `run_json`, `scenario_id` | `GET /api/runs/{id}` ve adla senaryo kimliği |
| `log_lines(aranan)` | JSON log satırlarını süzer (SW-10.7, 9.2) |
| `spawn_fake(router)` | sahte Jira/GitLab/SMTP için yerel sunucu, URL döner |
| `post_form`, `settle()` | form gönderimi; arka plan görevleri için kısa bekleme |

## temel.rs — Faz 1 (SW-10.5, 9.1, 9.5, 10.9, 3.5, 10.7)
sağlık ucu; gömülü statik dosyalar; CI uçları token ister (yoksa/yanlışsa 401, token tanımsızsa
hepsi reddedilir); uyumsuz `x-ulak-version` 409; yanlış şifreyle admin işlemi yapılamaz, admin
sayfası login'e yönlendirir; ulak OS'e göre servis edilir; durgun koşum `interrupted` olur,
`finish?status=interrupted`; kalan senaryo olayları loglanır.

## sonuc_alma.rs — Faz 2 (SW-2.1, 2.2, 2.3, 4.6, 3.3, 4.9; gerçek örneklerle SW-2.7)
gerçek örnek zip'i koşum oluşturur (7 senaryo, sayılar); `@After` ekran görüntüsü yalnızca kalan
senaryoya bağlanır; container `befores` sonucu da bağlar; sonuç dosyası yoksa 422 + açıklama;
`../` yollu zip, boyut sınırı, zip olmayan gövde reddedilir; GitLab bilgisinden proje oluşur,
projelerin geçmişi ayrı; aynı sonuç iki kez gelirse bir kez sayılır; rerun'da son deneme geçerli,
denemeler listelenir.

## rapor.rs — Faz 4–6 (raporlama, arama, CSV, paylaşım, canlı, disk)
koşum → senaryo → adımlar/ekler; rapor başlığında CI bilgisi; dashboard geçmişi ve flaky tespiti;
proje/branch süzgeci; her alanla arama ve sayfalama; CSV (seçili alanlar sırasıyla, BOM, büyük
dosya akış halinde); DOORS CSV/DXL; paylaşım linki iç bilgileri gizler, iptal edilince açılmaz;
canlı anlık görüntü (bekliyor/koşuyor/hata), WebSocket sayfa yenilemeden günceller; disk temizliği
önce videolar sonra görüntüler en eskiden; koşum sayfası feature gruplu ve grafikli; feature geçmiş
matrisi; **Cucumber olaylarından şu an koşan senaryo ve adım** (`testdata/canli/` gerçek kaydı).

## entegrasyon.rs — Faz 6–8 (sahte Jira / GitLab / SMTP)
webhook yanlış tokenı reddeder; pipeline durumu dashboard'da; gece özeti pipeline başına bir
e-posta (DRY_RUN ve gerçek SMTP); Jira bug'ı DOORS no'yu etikette ve açıklamada taşır, ekran
görüntüsü ekler, etiket reddedilse de bug oluşur; yüklemeden sonra otomatik eşleşme; gözlem
modunda Jira'ya yazılmaz; canlı modda üst sınır, tekrar önleme, yeniden deneme; triage önceliği
(admin > OpenCode > kural) ve geçmişi; düzeltme adayı hedefleri; düzeltme sonucu kutusu ve rozet;
self-healing paneli sayıları (MR webhook ile); önceki başarılı düzeltme örnek olarak verilir;
"düzelt" düğmesi GitLab API ile yalnızca fix işini tetikler; GitLab tokenı hata ve loglarda yok.

## bellek.rs — SW-10.6
büyük zip yüklemesinde tepe RSS `/proc` ile ölçülür, hedefin altında kalmalı (yalnızca Linux).
