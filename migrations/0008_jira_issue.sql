-- Senaryolarda tag'le (@ABC-12) ya da bağla geçen Jira issue'larının Jira'dan çekilen bilgisi.
-- Koşum bitince yenilenir (30 dk'dan eskiyse). Rapor yalnızca buradan okur; sayfa açılışında Jira'ya gidilmez.
CREATE TABLE jira_issues (
    key              TEXT PRIMARY KEY,
    summary          TEXT,
    status           TEXT,
    status_category  TEXT,             -- Jira statusCategory.key: new | indeterminate | done
    issue_type       TEXT,
    assignee         TEXT,
    bulunamadi       INTEGER NOT NULL DEFAULT 0,
    hata             TEXT,
    fetched_at       TEXT NOT NULL
);
