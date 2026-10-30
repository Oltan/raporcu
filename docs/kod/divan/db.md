# divan/src/db.rs

**Amaç:** SW-2.5 — kurulumsuz SQLite. Dosya yoksa oluşturulur, gömülü numaralı migration'lar
sırayla uygulanır, gecelik yedek alınır.

## Açılış (`Db::open` / `open_with(..., migrations)`)
1. `paths.ensure()`.
2. Yazıcı bağlantısı açılır, ayarlar: `busy_timeout=10s`, `journal_mode=WAL`, `foreign_keys=ON`,
   `synchronous=NORMAL`, `cache_size=-<KiB>` (SW-10.6 bellek sınırı), `temp_store=FILE`.
3. Migration'lar: `schema_migrations(name PK, applied_at)` tablosu; listedeki her migration
   uygulanmamışsa tek transaction içinde çalışır ve kaydedilir, log: `migration uygulandı`.
4. `readers` adet salt-okur bağlantı (önbelleğin yarısı) havuza konur.

Migration listesi `MIGRATIONS`: `0001_ilk.sql`, `0002_triage_kolonu.sql` (ikiliye gömülü,
`migrations/` klasöründen). Yeni migration = yeni dosya + listeye ekleme.

## Erişim modeli
- **Tek yazıcı bağlantısı** (mutex), **okuma havuzu** (bekleyen kuyruk + koşul değişkeni).
- `write(f)`, `read(f)`: işi bloklayan iş parçacığında (`spawn_blocking`) çalıştırır.
- `write_blocking`, `read_blocking`: senkron sürümler.

## Yedek
- `backup(paths, keep)`: sunucu durmadan `VACUUM INTO 'backup/divan-<zaman>.db'`; sonra
  `divan-*.db` adlı yedekler sıralanıp en eskiler silinir, `keep` tane kalır. Log: `yedek alındı`.
- `backup_loop(db, paths, hour, keep)`: 5 dakikada bir bakar; yerel saat `hour`'u geçtiyse ve o
  gün yedek alınmadıysa alır.
- `now()`: UTC RFC3339 milisaniyeli (`2026-10-29T20:01:02.123Z`). **Tüm zaman sütunları bu
  biçimdedir ve metin olarak karşılaştırılır.**

## Testler
`test_sw_2_5_empty_data_dir_creates_db_with_tables`, `test_sw_2_5_new_migration_applied_on_next_open`,
`test_sw_2_5_backup_readable_and_rotated`

## Python karşılığı
`sqlite3` (stdlib). Yazıcı: tek bağlantı + `threading.Lock`; okuyucular için `queue.Queue` havuzu.
`VACUUM INTO` aynen çalışır. Async sunucuda `asyncio.to_thread`.
