# raporcu/src/logging.rs

**Amaç:** SW-10.7 — JSON satır logları, gizli alan maskeleme, dönüşüm, admin panelindeki son
WARN/ERROR listesi.

## Satır biçimi
Her olay bir satır JSON: `zaman` (UTC, ms), `seviye` (`INFO`/`WARN`/`ERROR`), `olay` (mesaj) +
olayın alanları (`run_id`, `proje`, `branch`, `pipeline_id`, `senaryo_id`, `sure_ms`, `hata`, ...).
- Alan adı `token, sifre, şifre, password, parola, cookie, authorization, secret` **içeriyorsa**
  değer `***` yazılır.
- Metin değerler en fazla 500 karakter (`MAX_FIELD_CHARS`), fazlası `…` ile kesilir.

## Bileşenler
- `JsonLayer`: her olayı JSON'a çevirip tüm hedeflere yazar (dosya, isteğe bağlı stdout); WARN ve
  üstünü `RecentLogs`'a ekler.
- `RecentLogs`: son 200 WARN/ERROR (halka tampon); `list()` en yeniden eskiye.
- `init(logs_dir, max_days, max_bytes, to_stdout)`: günlük dönen dosya `raporcu.YYYY-MM-DD.log`
  (bloklamayan yazıcı), filtre `LOG_LEVEL` (varsayılan info), açılışta dönüşüm uygular.
- `enforce_retention(dir, max_files, max_bytes)`: `raporcu*` dosyalarını ada göre sıralar, en
  eskiden silerek hem dosya sayısını hem toplam boyutu sınıra indirir. Saatte bir de çalışır.
- `truncate(s, n)`: karakter (bayt değil) bazında kısaltma.
- Testler için: `test_capture()` (global belleğe yazan logger), `test_recent()`, `captured_lines`.

## Loglanan olaylar (diğer modüllerden)
koşum açıldı/bitti/interrupted · yükleme alındı/reddedildi · webhook alındı · triage kararı ·
Jira bug'ı açıldı/eşleşti/açılacaktı · OpenCode denemesi başladı/bitti · admin girişi
başarılı/başarısız (yalnızca IP) · paylaşım linki oluşturuldu/iptal · migration uygulandı ·
yedek alındı · disk temizliği yapıldı · disk eşiği aşıldı (WARN) · stajci sürüm uyuşmazlığı
(WARN) · dış servis hataları (WARN; durum kodu ve deneme sayısıyla).

## Testler
`test_sw_10_7_json_fields_secret_and_truncation`, `test_sw_10_7_retention_by_count_and_size`,
`test_sw_10_7_recent_warn_error_kept`

## Python karşılığı
`logging` + JSON `Formatter` (gizli anahtar maskeleme formatter'da),
`TimedRotatingFileHandler(when="midnight", backupCount=14)` + boyut temizliği için ayrı fonksiyon;
WARN listesi için `collections.deque(maxlen=200)` tutan `Handler`.
