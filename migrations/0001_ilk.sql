-- SW-2.5: ilk şema. Koşum, senaryo, adım ve triage kayıtları hiçbir zaman silinmez (SW-10.8).

CREATE TABLE projects (
    id          INTEGER PRIMARY KEY,
    gitlab_id   TEXT UNIQUE,
    path        TEXT NOT NULL UNIQUE,
    url         TEXT,
    created_at  TEXT NOT NULL
);

CREATE TABLE runs (
    id               INTEGER PRIMARY KEY,
    project_id       INTEGER NOT NULL REFERENCES projects(id),
    branch           TEXT NOT NULL,
    commit_sha       TEXT,
    pipeline_id      TEXT,
    pipeline_url     TEXT,
    job_id           TEXT,
    job_url          TEXT,
    os               TEXT,
    status           TEXT NOT NULL,          -- running | finished | interrupted
    total_expected   INTEGER,
    total            INTEGER NOT NULL DEFAULT 0,
    passed           INTEGER NOT NULL DEFAULT 0,
    failed           INTEGER NOT NULL DEFAULT 0,
    broken           INTEGER NOT NULL DEFAULT 0,
    skipped          INTEGER NOT NULL DEFAULT 0,
    unknown          INTEGER NOT NULL DEFAULT 0,
    started_at       TEXT NOT NULL,
    finished_at      TEXT,
    last_activity_at TEXT NOT NULL
);
CREATE INDEX runs_project_branch ON runs(project_id, branch, id);
CREATE INDEX runs_status ON runs(status);

-- Her satır bir denemedir (Allure result dosyası). Senaryonun koşumdaki sonucu is_latest=1 olan denemedir (SW-4.9).
CREATE TABLE scenarios (
    id            INTEGER PRIMARY KEY,
    run_id        INTEGER NOT NULL REFERENCES runs(id),
    project_id    INTEGER NOT NULL REFERENCES projects(id),
    branch        TEXT NOT NULL,
    result_uuid   TEXT NOT NULL,
    scenario_key  TEXT NOT NULL,
    name          TEXT NOT NULL,
    full_name     TEXT,
    feature       TEXT,
    feature_file  TEXT,
    tags          TEXT NOT NULL DEFAULT '',  -- ' tag1 tag2 ' (boşlukla çevrili, LIKE araması için)
    doors_ids     TEXT NOT NULL DEFAULT '',
    jira_keys     TEXT NOT NULL DEFAULT '',
    status        TEXT NOT NULL,
    message       TEXT,
    trace         TEXT,
    start_ms      INTEGER,
    stop_ms       INTEGER,
    duration_ms   INTEGER,
    parameters    TEXT,
    attempt       INTEGER NOT NULL DEFAULT 1,
    is_latest     INTEGER NOT NULL DEFAULT 1,
    created_at    TEXT NOT NULL,
    UNIQUE (run_id, result_uuid)
);
CREATE INDEX scenarios_run_latest ON scenarios(run_id, is_latest);
CREATE INDEX scenarios_history ON scenarios(project_id, branch, scenario_key);

CREATE TABLE steps (
    id           INTEGER PRIMARY KEY,
    scenario_id  INTEGER NOT NULL REFERENCES scenarios(id),
    parent_id    INTEGER REFERENCES steps(id),
    position     INTEGER NOT NULL,
    depth        INTEGER NOT NULL,
    name         TEXT NOT NULL,
    status       TEXT NOT NULL,
    message      TEXT,
    start_ms     INTEGER,
    stop_ms      INTEGER
);
CREATE INDEX steps_scenario ON steps(scenario_id, position);

-- Allure container dosyaları: @Before/@After fixture'ları hangi sonuçlara ait (SW-2.2)
CREATE TABLE containers (
    id       INTEGER PRIMARY KEY,
    run_id   INTEGER NOT NULL REFERENCES runs(id),
    uuid     TEXT NOT NULL,
    UNIQUE (run_id, uuid)
);
CREATE TABLE container_children (
    container_id INTEGER NOT NULL REFERENCES containers(id),
    result_uuid  TEXT NOT NULL,
    PRIMARY KEY (container_id, result_uuid)
);

CREATE TABLE attachments (
    id              INTEGER PRIMARY KEY,
    run_id          INTEGER NOT NULL REFERENCES runs(id),
    scenario_id     INTEGER REFERENCES scenarios(id),
    step_id         INTEGER REFERENCES steps(id),
    container_id    INTEGER REFERENCES containers(id),
    origin          TEXT NOT NULL,             -- result | step | before | after
    name            TEXT,
    mime            TEXT,
    source          TEXT NOT NULL,             -- allure dosya adı; dosya DATA_DIR/runs/<run>/files/<source>
    deleted_at      TEXT,
    deleted_reason  TEXT,
    created_at      TEXT NOT NULL,
    UNIQUE (run_id, source)
);
CREATE INDEX attachments_scenario ON attachments(scenario_id);

-- ulak start ile bildirilen beklenen senaryolar (canlı ekranda "bekliyor") (SW-3.1, SW-3.4)
CREATE TABLE expected_scenarios (
    id       INTEGER PRIMARY KEY,
    run_id   INTEGER NOT NULL REFERENCES runs(id),
    position INTEGER NOT NULL,
    name     TEXT NOT NULL,
    feature  TEXT
);
CREATE INDEX expected_run ON expected_scenarios(run_id, position);

CREATE TABLE triage_decisions (
    id           INTEGER PRIMARY KEY,
    scenario_id  INTEGER NOT NULL REFERENCES scenarios(id),
    category     TEXT NOT NULL,   -- flaky | test_kodu | urun | ortam | bilinmiyor
    source       TEXT NOT NULL,   -- kural | opencode | admin
    reason       TEXT NOT NULL,
    confidence   REAL,
    created_at   TEXT NOT NULL
);
CREATE INDEX triage_scenario ON triage_decisions(scenario_id, id);

CREATE TABLE bug_links (
    id           INTEGER PRIMARY KEY,
    scenario_id  INTEGER NOT NULL REFERENCES scenarios(id),
    project_id   INTEGER NOT NULL REFERENCES projects(id),
    scenario_key TEXT NOT NULL,
    jira_key     TEXT NOT NULL,
    source       TEXT NOT NULL,   -- tag | eslesme | otomatik | admin
    fingerprint  TEXT,
    created_at   TEXT NOT NULL,
    UNIQUE (scenario_id, jira_key)
);

-- Jira otomasyonu kayıtları; gözlem modunda "açılacaktı" burada tutulur (SW-6.3)
CREATE TABLE jira_actions (
    id           INTEGER PRIMARY KEY,
    run_id       INTEGER NOT NULL REFERENCES runs(id),
    scenario_id  INTEGER REFERENCES scenarios(id),
    action       TEXT NOT NULL,   -- acilacakti | acildi | eslesti | hata
    fingerprint  TEXT,
    jira_key     TEXT,
    detail       TEXT,
    created_at   TEXT NOT NULL
);

CREATE TABLE share_links (
    id          INTEGER PRIMARY KEY,
    token       TEXT NOT NULL UNIQUE,
    run_id      INTEGER NOT NULL REFERENCES runs(id),
    created_at  TEXT NOT NULL,
    revoked_at  TEXT
);

CREATE TABLE fix_attempts (
    id                 INTEGER PRIMARY KEY,
    run_id             INTEGER NOT NULL REFERENCES runs(id),
    scenario_id        INTEGER REFERENCES scenarios(id),
    project_id         INTEGER NOT NULL REFERENCES projects(id),
    scenario_key       TEXT NOT NULL,
    outcome            TEXT NOT NULL,  -- mr_opened | draft_mr | no_change | rejected | error
    mr_iid             INTEGER,
    mr_url             TEXT,
    mr_state           TEXT,           -- opened | merged | closed
    branch             TEXT,
    validation_passed  INTEGER,
    summary            TEXT,
    warnings           TEXT,
    diff               TEXT,
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL
);
CREATE INDEX fix_attempts_run ON fix_attempts(run_id);

-- Pipeline düzeyi durum, GitLab webhook'undan (SW-3.6)
CREATE TABLE pipelines (
    id           INTEGER PRIMARY KEY,
    project_id   INTEGER NOT NULL REFERENCES projects(id),
    pipeline_id  TEXT NOT NULL,
    ref          TEXT,
    status       TEXT NOT NULL,
    source       TEXT,
    url          TEXT,
    updated_at   TEXT NOT NULL,
    UNIQUE (project_id, pipeline_id)
);

-- Pipeline başına tek özet e-posta (SW-5.3)
CREATE TABLE emails (
    id           INTEGER PRIMARY KEY,
    pipeline_key TEXT NOT NULL UNIQUE,
    recipients   TEXT NOT NULL,
    subject      TEXT NOT NULL,
    dry_run      INTEGER NOT NULL,
    sent_at      TEXT NOT NULL
);

CREATE TABLE users (
    id          INTEGER PRIMARY KEY,
    username    TEXT NOT NULL UNIQUE,
    role        TEXT NOT NULL,
    created_at  TEXT NOT NULL
);
INSERT INTO users (username, role, created_at) VALUES ('admin', 'admin', datetime('now'));
