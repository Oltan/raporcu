-- SW-5.1: dışarıya açık genel ana sayfa linki; proje + branch başına, koşumdan bağımsız
CREATE TABLE genel_portallar (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    token TEXT NOT NULL UNIQUE,
    project_id INTEGER NOT NULL REFERENCES projects(id),
    branch TEXT NOT NULL,
    created_at TEXT NOT NULL,
    revoked_at TEXT
);
