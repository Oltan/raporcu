"""Allure → veritabanı. Rust: crates/raporcu/src/ingest.rs. Çağıran transaction açar."""
import json
import logging

from . import allure
from .db import now

log = logging.getLogger("raporcu")
MAX_TRACE = 64 * 1024


def space_list(items) -> str:
    return f" {' '.join(items)} " if items else ""  # LIKE '% tag %' araması için


def ensure_project(c, ci: dict) -> int:
    path = ci.get("project_path") or ""
    if not path:
        raise ValueError("project_path zorunlu")
    gid = ci.get("project_id")
    if gid and (row := c.execute("SELECT id, path FROM projects WHERE gitlab_id=?", (gid,)).fetchone()):
        if row["path"] != path:
            c.execute("UPDATE projects SET path=? WHERE id=?", (path, row["id"]))
        return row["id"]
    if row := c.execute("SELECT id FROM projects WHERE path=?", (path,)).fetchone():
        c.execute("UPDATE projects SET gitlab_id=COALESCE(gitlab_id, ?), url=COALESCE(url, ?) WHERE id=?",
                  (gid, ci.get("project_url"), row["id"]))
        return row["id"]
    cur = c.execute("INSERT INTO projects (gitlab_id, path, url, created_at) VALUES (?,?,?,?)",
                    (gid, path, ci.get("project_url"), now()))
    log.info("proje oluşturuldu", extra={"proje": path})
    return cur.lastrowid


def create_run(c, project_id: int, ci: dict, status: str, total_expected=None) -> int:
    t = now()
    cur = c.execute(
        """INSERT INTO runs (project_id, branch, commit_sha, pipeline_id, pipeline_url, job_id, job_url, os,
                             status, total_expected, started_at, last_activity_at)
           VALUES (?,?,?,?,?,?,?,?,?,?,?,?)""",
        (project_id, ci.get("branch") or "unknown", ci.get("commit"), ci.get("pipeline_id"), ci.get("pipeline_url"),
         ci.get("job_id"), ci.get("job_url"), ci.get("os"), status, total_expected, t, t))
    log.info("koşum açıldı", extra={"run_id": cur.lastrowid})
    return cur.lastrowid


def _attachment(c, run_id, sid, step_id, container_id, origin, a):
    if not allure.is_safe_file_name(a.get("source", "")):
        log.warning("güvensiz ek adı atlandı", extra={"run_id": run_id})
        return
    c.execute("""INSERT OR IGNORE INTO attachments (run_id, scenario_id, step_id, container_id, origin, name, mime, source, created_at)
                 VALUES (?,?,?,?,?,?,?,?,?)""",
              (run_id, sid, step_id, container_id, origin, a.get("name"), a.get("type"), a["source"], now()))


def _steps(c, run_id, sid, parent, depth, steps, pos):
    for s in steps:  # derinlik öncelikli sıra: position 1,2,3…
        pos[0] += 1
        cur = c.execute("""INSERT INTO steps (scenario_id, parent_id, position, depth, name, status, message, start_ms, stop_ms)
                           VALUES (?,?,?,?,?,?,?,?,?)""",
                        (sid, parent, pos[0], depth, s.get("name", ""), allure.status(s),
                         (s.get("statusDetails") or {}).get("message"), s.get("start"), s.get("stop")))
        for a in s.get("attachments", []):
            _attachment(c, run_id, sid, cur.lastrowid, None, "step", a)
        _steps(c, run_id, sid, cur.lastrowid, depth + 1, s.get("steps", []), pos)


def ingest_result(c, run: dict, r: dict):
    """run = {run_id, project_id, branch}. Aynı uuid ikinci kez gelirse None (SW-3.3)."""
    if not r.get("uuid"):
        raise ValueError("result uuid boş")
    if c.execute("SELECT 1 FROM scenarios WHERE run_id=? AND result_uuid=?", (run["run_id"], r["uuid"])).fetchone():
        return None
    tags = allure.tags(r)
    det = r.get("statusDetails") or {}
    uri = allure.label(r, "gherkin_uri")
    cur = c.execute(
        """INSERT INTO scenarios (run_id, project_id, branch, result_uuid, scenario_key, name, full_name, feature, feature_file,
                                  tags, doors_ids, jira_keys, status, message, trace, start_ms, stop_ms, duration_ms, parameters, created_at)
           VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)""",
        (run["run_id"], run["project_id"], run["branch"], r["uuid"], allure.scenario_key(r), r.get("name", ""),
         r.get("fullName"), allure.feature(r), uri.removeprefix("classpath:") if uri else None,
         space_list(tags), space_list([d for t in tags if (d := allure.doors_id(t))]),
         space_list([j for t in tags if (j := allure.jira_key(t))]),
         allure.status(r), det.get("message"), (det.get("trace") or "")[:MAX_TRACE] or None,
         r.get("start"), r.get("stop"), allure.duration_ms(r), json.dumps(r.get("parameters", []), ensure_ascii=False), now()))
    sid = cur.lastrowid
    for a in r.get("attachments", []):
        _attachment(c, run["run_id"], sid, None, None, "result", a)
    _steps(c, run["run_id"], sid, None, 0, r.get("steps", []), [0])
    return sid


def ingest_container(c, run_id: int, ct: dict) -> bool:
    if not ct.get("uuid"):
        raise ValueError("container uuid boş")
    cur = c.execute("INSERT OR IGNORE INTO containers (run_id, uuid) VALUES (?,?)", (run_id, ct["uuid"]))
    if cur.rowcount == 0:
        return False
    cid = c.execute("SELECT id FROM containers WHERE run_id=? AND uuid=?", (run_id, ct["uuid"])).fetchone()["id"]
    for child in ct.get("children", []):
        c.execute("INSERT OR IGNORE INTO container_children VALUES (?,?)", (cid, child))

    def walk(origin, items):
        for it in items:
            for a in it.get("attachments", []):
                _attachment(c, run_id, None, None, cid, origin, a)
            walk(origin, it.get("steps", []))

    walk("before", ct.get("befores", []))
    walk("after", ct.get("afters", []))  # @After ekran görüntüsü/video burada (SW-2.2)
    return True


def link_fixture_attachments(c, run_id: int) -> int:
    """Container ekleri senaryoya bağlanır; dosya geliş sırası önemsiz."""
    return c.execute(
        """UPDATE attachments SET scenario_id = (
               SELECT s.id FROM container_children cc
               JOIN scenarios s ON s.run_id = attachments.run_id AND s.result_uuid = cc.result_uuid
               WHERE cc.container_id = attachments.container_id ORDER BY s.id LIMIT 1)
           WHERE run_id = ? AND scenario_id IS NULL AND container_id IS NOT NULL""", (run_id,)).rowcount


def recompute_run(c, run_id: int):
    """SW-4.9: deneme numarası, is_latest ve koşum sayaçları (yalnızca son denemeler)."""
    c.execute(
        """WITH ranked AS (
               SELECT id, ROW_NUMBER() OVER (PARTITION BY scenario_key ORDER BY COALESCE(start_ms, 0), id) AS rn,
                          COUNT(*) OVER (PARTITION BY scenario_key) AS cnt
               FROM scenarios WHERE run_id = :r)
           UPDATE scenarios
              SET attempt = (SELECT rn FROM ranked WHERE ranked.id = scenarios.id),
                  is_latest = (SELECT rn = cnt FROM ranked WHERE ranked.id = scenarios.id)
            WHERE run_id = :r""", {"r": run_id})
    q = "SELECT COUNT(*) FROM scenarios WHERE run_id = :r AND is_latest = 1"
    c.execute(f"""UPDATE runs SET total = ({q}),
                     passed = ({q} AND status='passed'), failed = ({q} AND status='failed'),
                     broken = ({q} AND status='broken'), skipped = ({q} AND status='skipped'),
                     unknown = ({q} AND status='unknown'), last_activity_at = :t
                  WHERE id = :r""", {"r": run_id, "t": now()})


def finish_run(c, run_id: int, status: str = "finished"):
    c.execute("UPDATE runs SET status=?, finished_at=COALESCE(finished_at, ?) WHERE id=?", (status, now(), run_id))
