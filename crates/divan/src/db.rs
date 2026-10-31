//! SW-2.5: kurulumsuz SQLite.
//!
//! - Dosya `DATA_DIR/divan.db`; yoksa oluşturulur.
//! - Numaralı migration'lar ikiliye gömülüdür, açılışta sırayla uygulanır, `schema_migrations`
//!   tablosunda kayıtlıdır.
//! - WAL, busy_timeout, foreign_keys; önbellek `cache_size` ile sınırlı (SW-10.6).
//! - Tek yazıcı bağlantısı + küçük okuma havuzu. Tüm erişim `spawn_blocking` içinde.
//! - Yedek: `VACUUM INTO` ile `DATA_DIR/backup/`; son N yedek tutulur.

use crate::paths::Paths;
use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};

pub struct Migration {
    pub name: &'static str,
    pub sql: &'static str,
}

/// Gömülü migration'lar. Yeni migration eklemek için dosyayı `migrations/` altına koyup buraya ekle.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        name: "0001_ilk",
        sql: include_str!("../../../migrations/0001_ilk.sql"),
    },
    Migration {
        name: "0002_triage_kolonu",
        sql: include_str!("../../../migrations/0002_triage_kolonu.sql"),
    },
    Migration {
        name: "0003_ayarlar",
        sql: include_str!("../../../migrations/0003_ayarlar.sql"),
    },
    Migration {
        name: "0004_bug_taslaklari",
        sql: include_str!("../../../migrations/0004_bug_taslaklari.sql"),
    },
    Migration {
        name: "0005_paylasim_gorunumu",
        sql: include_str!("../../../migrations/0005_paylasim_gorunumu.sql"),
    },
];

#[derive(Clone)]
pub struct Db {
    writer: Arc<Mutex<Connection>>,
    readers: Arc<ReadPool>,
    path: PathBuf,
}

struct ReadPool {
    conns: Mutex<Vec<Connection>>,
    available: Condvar,
}

impl ReadPool {
    fn get(&self) -> Connection {
        let mut guard = self.conns.lock().unwrap();
        loop {
            if let Some(c) = guard.pop() {
                return c;
            }
            guard = self.available.wait(guard).unwrap();
        }
    }

    fn put(&self, c: Connection) {
        self.conns.lock().unwrap().push(c);
        self.available.notify_one();
    }
}

fn configure(conn: &Connection, cache_kib: i64) -> Result<()> {
    conn.busy_timeout(std::time::Duration::from_secs(10))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    // negatif değer KiB cinsindendir
    conn.pragma_update(None, "cache_size", -cache_kib.abs())?;
    conn.pragma_update(None, "temp_store", "FILE")?;
    Ok(())
}

impl Db {
    pub fn open(paths: &Paths, cache_kib: i64, readers: usize) -> Result<Db> {
        Self::open_with(paths, cache_kib, readers, MIGRATIONS)
    }

    pub fn open_with(paths: &Paths, cache_kib: i64, readers: usize, migrations: &[Migration]) -> Result<Db> {
        paths.ensure().context("DATA_DIR oluşturulamadı")?;
        let path = paths.db();
        let mut writer = Connection::open(&path).with_context(|| format!("veritabanı açılamadı: {}", path.display()))?;
        configure(&writer, cache_kib)?;
        apply_migrations(&mut writer, migrations)?;
        let mut pool = Vec::new();
        for _ in 0..readers.max(1) {
            let c = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
            configure(&c, cache_kib / 2)?;
            pool.push(c);
        }
        Ok(Db {
            writer: Arc::new(Mutex::new(writer)),
            readers: Arc::new(ReadPool {
                conns: Mutex::new(pool),
                available: Condvar::new(),
            }),
            path,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Yazma işlemi: tek yazıcı bağlantısında, bloklayan iş parçacığında.
    pub async fn write<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Connection) -> Result<R> + Send + 'static,
        R: Send + 'static,
    {
        let w = self.writer.clone();
        tokio::task::spawn_blocking(move || {
            let mut c = w.lock().unwrap_or_else(|e| e.into_inner());
            f(&mut c)
        })
        .await?
    }

    pub async fn read<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&Connection) -> Result<R> + Send + 'static,
        R: Send + 'static,
    {
        let pool = self.readers.clone();
        tokio::task::spawn_blocking(move || {
            let c = pool.get();
            let r = f(&c);
            pool.put(c);
            r
        })
        .await?
    }

    /// Senkron yazma (bloklayan bağlamlar ve testler için).
    pub fn write_blocking<R>(&self, f: impl FnOnce(&mut Connection) -> Result<R>) -> Result<R> {
        let mut c = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut c)
    }

    pub fn read_blocking<R>(&self, f: impl FnOnce(&Connection) -> Result<R>) -> Result<R> {
        let c = self.readers.get();
        let r = f(&c);
        self.readers.put(c);
        r
    }

    /// Sunucu durmadan yedek alır; en yeni `keep` yedek kalır. Yedek dosyasının yolunu döner.
    pub fn backup(&self, paths: &Paths, keep: usize) -> Result<PathBuf> {
        let dir = paths.backup_dir();
        std::fs::create_dir_all(&dir)?;
        let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S%3f");
        let target = dir.join(format!("divan-{stamp}.db"));
        let target_str = target.to_string_lossy().to_string();
        self.write_blocking(|c| {
            c.execute("VACUUM INTO ?1", [&target_str])?;
            Ok(())
        })?;
        let mut backups: Vec<PathBuf> = std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("divan-") && n.ends_with(".db"))
            })
            .collect();
        backups.sort();
        let mut removed = 0;
        while backups.len() > keep {
            let old = backups.remove(0);
            std::fs::remove_file(&old)?;
            removed += 1;
        }
        tracing::info!(dosya = %target.display(), silinen_eski = removed, "yedek alındı");
        Ok(target)
    }
}

fn apply_migrations(conn: &mut Connection, migrations: &[Migration]) -> Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS schema_migrations (name TEXT PRIMARY KEY, applied_at TEXT NOT NULL);")?;
    for m in migrations {
        let done: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE name = ?1)", [m.name], |r| {
            r.get(0)
        })?;
        if done {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(m.sql)
            .with_context(|| format!("migration {} uygulanamadı", m.name))?;
        tx.execute(
            "INSERT INTO schema_migrations (name, applied_at) VALUES (?1, ?2)",
            rusqlite::params![m.name, now()],
        )?;
        tx.commit()?;
        tracing::info!(migration = m.name, "migration uygulandı");
    }
    Ok(())
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Gecelik yedek zamanlayıcısı: her gün `hour` (yerel saat) geçince bir kez.
pub async fn backup_loop(db: Db, paths: Paths, hour: u32, keep: usize) {
    use chrono::{Local, Timelike};
    let mut last_day = None;
    loop {
        let now = Local::now();
        let today = now.date_naive();
        if now.hour() >= hour && last_day != Some(today) {
            last_day = Some(today);
            let (db, paths) = (db.clone(), paths.clone());
            match tokio::task::spawn_blocking(move || db.backup(&paths, keep)).await {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => tracing::error!(hata = %e, "yedek alınamadı"),
                Err(e) => tracing::error!(hata = %e, "yedek görevi çöktü"),
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tables(c: &Connection) -> Vec<String> {
        let mut st = c
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        st.query_map([], |r| r.get(0)).unwrap().map(|r| r.unwrap()).collect()
    }

    #[test]
    fn test_sw_2_5_empty_data_dir_creates_db_with_tables() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path().join("bos"));
        let db = Db::open(&paths, 1024, 1).unwrap();
        assert!(paths.db().is_file());
        let t = db.read_blocking(|c| Ok(tables(c))).unwrap();
        for name in [
            "runs",
            "scenarios",
            "steps",
            "attachments",
            "bug_links",
            "share_links",
            "fix_attempts",
            "users",
        ] {
            assert!(t.contains(&name.to_string()), "{name} tablosu yok");
        }
        let mode: String = db
            .read_blocking(|c| Ok(c.query_row("PRAGMA journal_mode", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
        let fk: i64 = db
            .write_blocking(|c| Ok(c.query_row("PRAGMA foreign_keys", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(fk, 1);
    }

    #[test]
    fn test_sw_2_5_new_migration_applied_on_next_open() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path());
        drop(Db::open_with(&paths, 1024, 1, &MIGRATIONS[..1]).unwrap());
        let extra = [
            Migration {
                name: MIGRATIONS[0].name,
                sql: MIGRATIONS[0].sql,
            },
            Migration {
                name: MIGRATIONS[1].name,
                sql: MIGRATIONS[1].sql,
            },
            Migration {
                name: "0099_deneme",
                sql: "CREATE TABLE deneme (x INTEGER);",
            },
        ];
        let db = Db::open_with(&paths, 1024, 1, &extra).unwrap();
        let t = db.read_blocking(|c| Ok(tables(c))).unwrap();
        assert!(t.contains(&"deneme".to_string()));
        let n: i64 = db
            .read_blocking(|c| Ok(c.query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(n, 3);
        // Üçüncü açılışta tekrar uygulanmaz (hata vermez)
        drop(db);
        Db::open_with(&paths, 1024, 1, &extra).unwrap();
    }

    #[test]
    fn test_sw_2_5_backup_readable_and_rotated() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path());
        let db = Db::open(&paths, 1024, 1).unwrap();
        let mut last = PathBuf::new();
        for _ in 0..9 {
            last = db.backup(&paths, 7).unwrap();
        }
        let count = std::fs::read_dir(paths.backup_dir()).unwrap().count();
        assert_eq!(count, 7);
        let c = Connection::open(&last).unwrap();
        let users: i64 = c.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0)).unwrap();
        assert_eq!(users, 1);
    }
}
