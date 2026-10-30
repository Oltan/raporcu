# raporcu/src/config.rs

**Amaç:** Sunucu yapılandırması (SW-10.5). Önce `.env` (çalışma klasöründe, varsa), sonra ortam
değişkenleri okunur; ortam değişkeni `.env`'i ezer. Gizli değerler `Secret` içinde tutulur ve
log/hata/Debug çıktısında `***` görünür (SW-9.2).

## Ortam değişkenleri → alanlar (varsayılanlar)
| Değişken | Alan | Varsayılan |
|----------|------|-----------|
| `DATA_DIR` | data_dir | `data` |
| `BIND_ADDR` | bind | `0.0.0.0:8080` |
| `PUBLIC_URL` | public_url (sondaki `/` atılır) | `http://127.0.0.1:<port>` |
| `CI_API_TOKEN` | ci_api_token (Secret) | yok → tüm CI istekleri 401 |
| `ADMIN_PASSWORD_HASH` | admin_password_hash (argon2) | yok → giriş yapılamaz |
| `GITLAB_WEBHOOK_TOKEN` | gitlab_webhook_token | yok → webhook 401 |
| `GITLAB_URL`, `GITLAB_TOKEN` | fix pipeline tetikleme | yok |
| `MAX_UPLOAD_MB` / `MAX_ENTRY_MB` / `MAX_ZIP_ENTRIES` | yükleme sınırları | 512 / 256 / 50000 |
| `MEMORY_TARGET_MB` | bellek hedefi (test eşiği) | 100 |
| `SQLITE_CACHE_KIB`, `DB_READ_POOL` | SQLite önbelleği, okuma bağlantısı sayısı | 8192, 4 |
| `RUN_STALL_TIMEOUT_SECS` | koşum zaman aşımı | 900 |
| `BACKUP_HOUR`, `BACKUP_KEEP` | gecelik yedek saati (yerel), tutulan yedek | 3, 7 |
| `LOG_MAX_DAYS`, `LOG_MAX_MB`, `LOG_STDOUT` | log dönüşümü | 14, 500, true |
| `DISK_WARN_PCT`, `DISK_CLEAN_PCT` | disk eşikleri | 80, 90 |
| `JIRA_URL`, `JIRA_USER`, `JIRA_TOKEN`, `JIRA_PROJECT`, `JIRA_ISSUE_TYPE` | Jira | issue type `Bug` |
| `JIRA_AUTO_CREATE` | `off`/`kapali`, `observe`/`gozlem` (varsayılan), `on`/`acik` | observe |
| `JIRA_MAX_BUGS_PER_RUN`, `JIRA_REQUIRE_RETRY` | otomatik bug sınırları | 3, true |
| `SMTP_HOST`, `SMTP_PORT`, `MAIL_FROM`, `MAIL_TO` (`,` ya da `;` ayrılmış), `DRY_RUN` | e-posta | port 25 |
| `TRIAGE_AUTO_CONFIDENCE`, `FLAKY_WINDOW` | triage eşiği, flaky penceresi (koşum) | 0.7, 10 |
| `DOORS_DXL` | doors_dxl: DXL betiği ucu ve düğmesi | false |
| `STAJCI_DIR` | stajci ikililerinin klasörü | `DATA_DIR/tools` |

Geçersiz sayı ya da `JIRA_AUTO_CREATE` değeri açılışta açık hata mesajıyla durdurur.

## Fonksiyonlar
- `Config::from_env()`; `Config::for_tests(data_dir)` — testler için: sabit tokenlar
  (`test-ci-token`, `test-webhook-token`), DRY_RUN açık, dış servisler kapalı.
- `secrets()` — 4 karakterden uzun tüm gizli değerler (token'lar ve şifre hash'i).
- `redact(metin, gizliler)` — metindeki her gizli değeri `***` yapar. Dış servis hata mesajları
  kullanıcıya/loga gitmeden önce bundan geçer.

## Testler
`test_sw_9_2_secret_debug_and_redact`

## Python karşılığı
`python-dotenv` + `os.environ`; `Secret` için `__repr__` → `"***"` döndüren küçük sınıf.
