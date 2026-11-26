-- 0.2.0: yazılım adı (ulak: ULAK_UYGULAMA_ADI / UYGULAMA_ADI) ve admin'in yayınladığı dışa açık raporlar.
-- Aynı repodan aynı yazılım adı + sürümle koşulan koşumlar tek raporda birleşir (SW-5.1).
ALTER TABLE runs ADD COLUMN app_name TEXT;

CREATE TABLE yayinlar (
    id          INTEGER PRIMARY KEY,
    project_id  INTEGER NOT NULL REFERENCES projects(id),
    app_name    TEXT NOT NULL,
    app_version TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    UNIQUE (project_id, app_name, app_version)
);

CREATE TABLE yayin_kosumlari (
    yayin_id INTEGER NOT NULL REFERENCES yayinlar(id) ON DELETE CASCADE,
    run_id   INTEGER NOT NULL REFERENCES runs(id),
    PRIMARY KEY (yayin_id, run_id)
);

-- admin'in rapordan çıkardığı testler
CREATE TABLE yayin_haric (
    yayin_id     INTEGER NOT NULL REFERENCES yayinlar(id) ON DELETE CASCADE,
    scenario_key TEXT NOT NULL,
    PRIMARY KEY (yayin_id, scenario_key)
);
