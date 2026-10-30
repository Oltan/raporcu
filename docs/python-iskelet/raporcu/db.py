"""SQLite: tek yazıcı + okuyucular, gömülü migration'lar. Rust: crates/raporcu/src/db.rs"""
import sqlite3
import threading
from datetime import datetime, timezone
from pathlib import Path

MIGRATIONS = ["0001_ilk.sql", "0002_triage_kolonu.sql", "0003_ayarlar.sql"]  # yeni dosya buraya eklenir


def now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="milliseconds")


def connect(path: Path) -> sqlite3.Connection:
    c = sqlite3.connect(path, check_same_thread=False, isolation_level=None)  # transaction'lar elle
    c.row_factory = sqlite3.Row
    c.execute("PRAGMA journal_mode=WAL")
    c.execute("PRAGMA foreign_keys=ON")
    c.execute("PRAGMA busy_timeout=5000")
    return c


class Db:
    def __init__(self, path: Path, migrations_dir: Path):
        self.writer = connect(path)
        self.write_lock = threading.Lock()  # SQLite'ta aynı anda tek yazıcı
        self._local = threading.local()
        self.path = path
        self._migrate(migrations_dir)

    def reader(self) -> sqlite3.Connection:  # iş parçacığı başına okuma bağlantısı
        if not hasattr(self._local, "c"):
            self._local.c = connect(self.path)
        return self._local.c

    def _migrate(self, d: Path):
        c = self.writer
        c.execute("CREATE TABLE IF NOT EXISTS schema_migrations (name TEXT PRIMARY KEY, applied_at TEXT NOT NULL)")
        for name in MIGRATIONS:
            if c.execute("SELECT 1 FROM schema_migrations WHERE name=?", (name,)).fetchone():
                continue
            sql = (d / name).read_text(encoding="utf-8")
            c.executescript(f"BEGIN;\n{sql}\nINSERT INTO schema_migrations VALUES ('{name}', '{now()}');\nCOMMIT;")
