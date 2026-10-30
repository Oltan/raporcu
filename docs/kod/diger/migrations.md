# migrations/ — SQLite şeması

`0001_ilk.sql`, `0002_triage_kolonu.sql`, `0003_ayarlar.sql`. Dosyalar ikiliye gömülüdür; sunucu açılışta
`schema_migrations` tablosunda adı olmayanları sırayla uygular (`db.md`); veritabanı `DATA_DIR/raporcu.db`, WAL kipinde. Koşum,
senaryo, adım ve triage kayıtları **hiç silinmez** (SW-10.8); yalnızca ek dosyaları silinip
`deleted_at` işaretlenir. Python'a geçişte aynı SQL dosyaları `sqlite3` ile aynen kullanılır
(mevcut veritabanı taşınmadan çalışır).

Zamanlar ISO-8601 metin (`TEXT`), süreler milisaniye tamsayı.

| Tablo | Satır neyi temsil eder | Önemli alanlar |
|-------|------------------------|----------------|
| `projects` | GitLab projesi | `gitlab_id` (CI_PROJECT_ID, tekil), `path` (tekil), `url` |
| `runs` | bir test işi koşumu | `project_id`, `branch`, commit/pipeline/job bilgisi, `os`, `status` (`running`/`finished`/`interrupted`), `total_expected` (stajci start'tan), `total/passed/failed/broken/skipped/unknown` (yalnızca son denemelerden), `last_activity_at` (durgunluk zaman aşımı) |
| `scenarios` | **bir deneme** (bir Allure result dosyası) | `result_uuid` (koşumda tekil → tekrar gelen dosya sayılmaz), `scenario_key` (Allure `historyId`; rerun ve geçmiş eşleşmesi), `tags` (`' a b '` boşlukla çevrili, `LIKE '% a %'` araması), `doors_ids`, `jira_keys`, `status`, `message`, `trace`, `attempt`, **`is_latest`** (koşumdaki geçerli sonuç), `triage_category/source` (0002) |
| `steps` | adım (iç içe) | `parent_id`, `position`, `depth`, `status`, `message` |
| `containers`, `container_children` | Allure container dosyası ve hangi result'lara ait olduğu | `@Before/@After` eklerini senaryoya bağlamak için (SW-2.2) |
| `attachments` | ek dosyası | `origin` (`result`/`step`/`before`/`after`), `source` (Allure dosya adı; disk yolu `DATA_DIR/runs/<run>/files/<source>`), `scenario_id` (container üzerinden sonradan da bağlanabilir), `deleted_at/reason`, `size` (0002) |
| `expected_scenarios` | stajci start'ın bildirdiği beklenen senaryo | canlı ekrandaki "bekliyor" satırları |
| `triage_decisions` | triage kararı (geçmişiyle) | `category` (`flaky`/`test_kodu`/`urun`/`ortam`/`bilinmiyor`), `source` (`kural`/`opencode`/`admin`), `confidence` |
| `bug_links` | senaryo ↔ Jira bug | `source` (`tag`/`eslesme`/`otomatik`/`admin`), `fingerprint` (hata parmak izi) |
| `jira_actions` | Jira otomasyonu kaydı | `action` (`acilacakti` gözlem modu, `acildi`, `eslesti`, `hata`) |
| `share_links` | salt okunur paylaşım | `token` (tekil, tahmin edilemez), `revoked_at` |
| `fix_attempts` | self-healing denemesi | `outcome` (`mr_opened`/`draft_mr`/`no_change`/`rejected`/`error`), MR bilgisi, `mr_state` (webhook ile `merged/closed`), `validation_passed`, `diff`, `warnings` |
| `pipelines` | pipeline durumu (webhook) | `(project_id, pipeline_id)` tekil |
| `emails` | gönderilen özet e-posta | `pipeline_key` tekil → pipeline başına bir e-posta |
| `settings` | arayüz ayarı (0003) | `key` (`jira`, `doors`), `value` JSON (`ayarlar.md`) |
| `users` | kullanıcı | şimdilik tek `admin` satırı; şifre hash'i `.env`'de |

## Kritik kural: is_latest
Aynı `scenario_key` aynı koşumda yeniden gelirse (surefire rerun) yeni satır `attempt+1`,
`is_latest=1` olur, öncekinin `is_latest`'i 0'a çekilir; koşum sayıları her yüklemeden sonra
`is_latest=1` satırlarından yeniden hesaplanır (`ingest.md`). Raporlar, arama, geçmiş ve
dashboard **daima** `is_latest=1` ile süzer.
