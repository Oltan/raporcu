# raporcu/src/ingest.rs

**Amaç:** Allure sonuçlarını veritabanına işler (SW-2.1, 2.2, 3.3, 4.6, 4.9). Tüm fonksiyonlar
çağıranın açtığı transaction içinde çalışır; zip ve artımlı yükleme aynı fonksiyonları kullanır.

## Proje ve koşum
- `ensure_project(c, ci)` (SW-4.6): `project_path` boşsa hata. Kimlik önceliği:
  1. `gitlab_id` (= CI_PROJECT_ID) eşleşirse o proje; yol değiştiyse yeni yol yazılır.
  2. `path` eşleşirse o proje; eksik `gitlab_id`/`url` doldurulur.
  3. Yoksa yeni proje (log: `proje oluşturuldu`).
- `create_run(c, project_id, ci, status, total_expected)`: branch boşsa `unknown`; `started_at` ve
  `last_activity_at` = şimdi. Log: `koşum açıldı` (run_id, proje, branch, pipeline_id, beklenen).
- `run_ctx(c, run_id)` → `(run_id, project_id, branch)` ya da yok.

## Sonuç işleme — `ingest_result(c, run, result)`
- Aynı koşumda aynı `uuid` varsa **hiçbir şey yapmaz**, `None` döner (SW-3.3: tekrar sayılmaz).
- `scenarios` satırı: `scenario_key` (historyId…), ad, fullName, feature, `feature_file`
  (`gherkin_uri` label'ından, `classpath:` atılmış), `tags`/`doors_ids`/`jira_keys`
  **boşlukla çevrili** metin (`" giris smoke "`, LIKE araması için), durum, mesaj, iz (en fazla
  64 KB), start/stop/süre, parametreler (JSON).
- Sonucun ekleri (`origin=result`) ve adımlar: adımlar **derinlik öncelikli sırayla** `position`
  1,2,3… ve `depth` (0 = Gherkin adımı, 1+ = `Allure.step` alt adımları) ile; adım ekleri
  `origin=step`.
- Ek kaydı: güvensiz adlı `source` atlanır (WARN); `(run_id, source)` tekil, tekrar eklenmez.

## Container — `ingest_container(c, run_id, container)` (SW-2.2)
- `(run_id, uuid)` tekil; tekrar gelirse false.
- `children` → `container_children`.
- `befores`/`afters` fixture eklerinin ve iç içe adım eklerinin hepsi `origin=before|after`,
  `container_id` ile, `scenario_id` boş kaydedilir.
- `link_fixture_attachments(c, run_id)`: senaryosu boş container eklerini, container'ın
  `children` uuid'si ile eşleşen senaryoya bağlar. **Dosya geliş sırası önemsizdir**: container
  sonuçtan önce gelirse sonuç gelince bağlanır.

## Sayaçlar — `recompute_run(c, run_id)` (SW-4.9)
- Her `scenario_key` grubunda denemeler `start_ms` (yoksa 0), sonra `id` sırasıyla numaralanır:
  `attempt` = 1..n, `is_latest` = en son deneme. **Senaryonun koşumdaki sonucu son denemedir.**
- Koşum sayaçları yalnızca `is_latest=1` satırlardan: `total, passed, failed, broken, skipped,
  unknown`; `last_activity_at` = şimdi.

## Bitirme — `finish_run(c, run_id, status)`
`status` (finished/interrupted), `finished_at` (ilk kez), sayaçları döner. Log: `koşum bitti` ya da
WARN `koşum interrupted oldu` (toplam, geçen, kalan).

## Testler
`tests/sonuc_alma.rs`: zip, @After ekleri, container önce gelirse, 422, `..` yolu, boyut sınırı,
proje kimliği ve geçmiş ayrımı, tekrar sayılmama, rerun.

## Python karşılığı
Aynı SQL; pencere fonksiyonlu `WITH ranked AS (... ROW_NUMBER() OVER (PARTITION BY scenario_key ...))
UPDATE ...` SQLite 3.25+ ile Python'da da aynen çalışır.
