-- Ürün hatası için hazırlanan, admin onayıyla tek tıkla açılan Jira bug taslakları (SW-6.3).
-- Gözlem modunda koşum sonrası kurallarla ya da OpenCode "urun" triage'ıyla oluşur.
CREATE TABLE bug_taslaklari (
    id           INTEGER PRIMARY KEY,
    run_id       INTEGER NOT NULL REFERENCES runs(id),
    scenario_id  INTEGER NOT NULL REFERENCES scenarios(id),
    project_id   INTEGER NOT NULL REFERENCES projects(id),
    scenario_key TEXT NOT NULL,
    fingerprint  TEXT,
    source       TEXT NOT NULL,              -- kural | opencode
    reason       TEXT,
    summary      TEXT NOT NULL,
    description  TEXT NOT NULL,
    labels       TEXT NOT NULL DEFAULT '[]', -- JSON dizi
    fields       TEXT NOT NULL DEFAULT '{}', -- JSON nesne (ek Jira alanları)
    status       TEXT NOT NULL,              -- bekliyor | acildi | yok_sayildi
    jira_key     TEXT,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);
CREATE INDEX bug_taslaklari_durum ON bug_taslaklari(project_id, status);
CREATE INDEX bug_taslaklari_kosum ON bug_taslaklari(run_id);
