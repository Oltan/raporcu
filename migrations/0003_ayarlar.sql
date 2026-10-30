-- Arayüzden değiştirilen ayarlar (Jira bug şablonu, DOORS bağlantısı). Değer JSON metnidir.
CREATE TABLE settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
