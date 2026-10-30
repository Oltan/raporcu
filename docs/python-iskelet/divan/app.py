"""FastAPI iskeleti: CI uçları + canlı. Rust: crates/divan/src/lib.rs, web/ci.rs, auth.rs, live.rs"""
import asyncio
import hmac
import json
import logging
from pathlib import Path

from fastapi import Depends, FastAPI, HTTPException, Request, WebSocket, WebSocketDisconnect
from fastapi.responses import JSONResponse, Response

from . import allure, defterdar, upload
from .db import Db, now
from .live import Broadcast, LiveStates

SERVER_VERSION = "0.1.0"
log = logging.getLogger("divan")


def create_app(cfg: dict) -> FastAPI:
    """cfg: data_dir, public_url, ci_api_token, migrations_dir, max_upload_mb, max_entry_mb"""
    data = Path(cfg["data_dir"])
    data.mkdir(parents=True, exist_ok=True)
    db = Db(data / "divan.db", Path(cfg["migrations_dir"]))
    canli, yayin = LiveStates(), Broadcast()
    files_dir = lambda run_id: data / "runs" / str(run_id) / "files"
    report_url = lambda run_id: f"{cfg['public_url']}/runs/{run_id}"
    app = FastAPI()
    app.state.db, app.state.canli = db, canli

    @app.exception_handler(upload.UploadError)
    async def upload_error(_req, e: upload.UploadError):
        return JSONResponse({"hata": str(e)}, status_code=e.status)

    def ci_guard(request: Request):
        """SW-9.1 token, SW-10.9 sürüm (major.minor)."""
        token = cfg.get("ci_api_token") or ""
        auth = request.headers.get("authorization", "")
        given = auth.removeprefix("Bearer ") if auth.startswith("Bearer ") else request.headers.get("x-ci-token", "")
        if not token or not hmac.compare_digest(given.encode(), token.encode()):
            log.warning("CI token reddedildi", extra={"yol": request.url.path})
            raise HTTPException(401, "CI_API_TOKEN eksik ya da yanlış")
        v = request.headers.get("x-ulak-version")
        if v and v.split(".")[:2] != SERVER_VERSION.split(".")[:2]:
            raise HTTPException(409, f"ulak sürümü uyumsuz: ulak {v}, sunucu {SERVER_VERSION}. "
                                     f"Yeni ulak'ı {cfg['public_url']}/api/tools/ulak adresinden indirin.")

    ci = Depends(ci_guard)

    @app.get("/api/health")
    def health():
        db.reader().execute("SELECT 1")
        return {"durum": "ok", "surum": SERVER_VERSION}

    @app.post("/api/ci/runs", dependencies=[ci])
    async def upload_new(request: Request):
        info = dict(request.query_params)
        if not info.get("project_path"):
            raise HTTPException(400, "project_path zorunlu")
        mb = 1024 * 1024
        z = await upload.body_to_file(request.stream(), data / "tmp", cfg["max_upload_mb"] * mb)
        try:  # zip işleme bloklayıcı: olay döngüsünü tutmamak için iş parçacığında
            s = await asyncio.to_thread(upload.process_zip, db, files_dir, z, {"new": info},
                                        cfg["max_entry_mb"] * mb, 4 * cfg["max_upload_mb"] * mb)
        finally:
            z.unlink(missing_ok=True)
        s["report_url"] = report_url(s["run_id"])
        return s

    @app.post("/api/ci/runs/start", dependencies=[ci])
    def start(body: dict):
        expected = body.get("expected") or []
        with db.write_lock:
            c = db.writer
            c.execute("BEGIN IMMEDIATE")
            try:
                pid = defterdar.ensure_project(c, body)
                rid = defterdar.create_run(c, pid, body, "running", body.get("total") or len(expected))
                for i, e in enumerate(expected, 1):
                    c.execute("INSERT INTO expected_scenarios (run_id, position, name, feature) VALUES (?,?,?,?)",
                              (rid, i, e.get("name", ""), e.get("feature")))
                c.execute("COMMIT")
            except BaseException:
                c.execute("ROLLBACK")
                raise
        return {"run_id": rid, "report_url": report_url(rid)}

    @app.post("/api/ci/runs/{run_id}/results", dependencies=[ci])
    async def result_file(run_id: int, name: str, request: Request):
        """ulak watch: tek Allure dosyası, senaryo biter bitmez (SW-3.2)."""
        if not allure.is_safe_file_name(name):
            raise HTTPException(400, "geçersiz dosya adı")
        body = await request.body()  # tek dosya; ekler büyükse body_to_file kullanın
        s = {"run_id": run_id, "accepted_results": 0, "duplicate_results": 0, "containers": 0, "attachments": 0}
        with db.write_lock:
            c = db.writer
            r = c.execute("SELECT project_id, branch FROM runs WHERE id=?", (run_id,)).fetchone()
            if not r:
                raise HTTPException(404, "koşum bulunamadı")
            kind = allure.file_kind(name)
            if kind == "attachment":
                d = files_dir(run_id)
                d.mkdir(parents=True, exist_ok=True)
                if not (d / name).exists():
                    (d / f".{name}.part").write_bytes(body)
                    (d / f".{name}.part").replace(d / name)
                    s["attachments"] = 1
                c.execute("UPDATE runs SET last_activity_at=? WHERE id=?", (now(), run_id))
            elif kind in ("result", "container"):
                c.execute("BEGIN IMMEDIATE")
                try:
                    doc = json.loads(body)
                    if kind == "result":
                        ok = defterdar.ingest_result(c, {"run_id": run_id, **dict(r)}, doc)
                        s["accepted_results" if ok else "duplicate_results"] = 1
                    elif defterdar.ingest_container(c, run_id, doc):
                        s["containers"] = 1
                    defterdar.link_fixture_attachments(c, run_id)
                    defterdar.recompute_run(c, run_id)
                    c.execute("COMMIT")
                except BaseException:
                    c.execute("ROLLBACK")
                    raise
        yayin.notify(run_id)
        s["report_url"] = report_url(run_id)
        return s

    @app.post("/api/ci/runs/{run_id}/events", dependencies=[ci])
    async def live_events(run_id: int, request: Request):
        events = await request.json()
        if not isinstance(events, list) or len(events) > 5000:
            raise HTTPException(400, "olay dizisi bekleniyor (en fazla 5000)")
        r = db.reader().execute("SELECT status FROM runs WHERE id=?", (run_id,)).fetchone()
        if not r or r["status"] != "running":
            return Response(status_code=202)  # bitmiş koşumun geç olayları yok sayılır
        with db.write_lock:
            db.writer.execute("UPDATE runs SET last_activity_at=? WHERE id=?", (now(), run_id))
        canli.apply(run_id, events)
        yayin.notify(run_id)
        return Response(status_code=204)

    @app.post("/api/ci/runs/{run_id}/finish", dependencies=[ci])
    def finish(run_id: int, body: dict | None = None):
        status = (body or {}).get("status", "finished")
        if status not in ("finished", "interrupted"):
            raise HTTPException(400, "status finished ya da interrupted olmalı")
        with db.write_lock:
            c = db.writer
            defterdar.recompute_run(c, run_id)
            defterdar.finish_run(c, run_id, status)
            row = dict(c.execute("SELECT status, total_expected, total, passed, failed, broken, skipped, unknown "
                                 "FROM runs WHERE id=?", (run_id,)).fetchone())
        canli.remove(run_id)  # + Rust'taki after_finish: triage, Jira, e-posta
        yayin.notify(run_id)
        return row

    def snapshot(run_id: int) -> dict:
        """Basitleştirilmiş live::snapshot: Allure sonucu > senaryo_bitti olayı > koşan > bekliyor."""
        c = db.reader()
        run = c.execute("SELECT * FROM runs WHERE id=?", (run_id,)).fetchone()
        if not run:
            raise HTTPException(404, "koşum bulunamadı")
        results = {}
        for s in c.execute("SELECT id, name, feature, status, message FROM scenarios "
                           "WHERE run_id=? AND is_latest=1 ORDER BY id", (run_id,)):
            results.setdefault((s["name"], s["feature"]), []).append(s)
        by_event = canli.finished_status(run_id)
        cur = canli.get(run_id) if run["status"] == "running" else None
        durum = {"passed": "gecti", "failed": "kaldi", "broken": "kaldi", "skipped": "atlandi"}
        olay = {"PASSED": "gecti", "FAILED": "kaldi", "AMBIGUOUS": "kaldi"}
        rows, cur_used = [], False
        for e in c.execute("SELECT name, feature FROM expected_scenarios WHERE run_id=? ORDER BY position", (run_id,)):
            k = (e["name"], e["feature"])
            if results.get(k):
                s = results[k].pop(0)
                rows.append({"state": durum.get(s["status"], "bilinmiyor"), "name": s["name"], "feature": s["feature"],
                             "message": s["message"], "scenario_id": s["id"]})
            elif by_event.get(k):
                rows.append({"state": olay.get(by_event[k].pop(0), "bilinmiyor"), "name": k[0], "feature": k[1]})
            elif not cur_used and cur and cur["active"] and cur["name"] == k[0] and cur["feature"] == k[1]:
                cur_used = True                  # taslak satırları aynı adı taşır: yalnızca ilki koşuyor
                rows.append({"state": "kosuyor", "name": k[0], "feature": k[1]})
            else:
                rows.append({"state": "bekliyor" if run["status"] == "running" else "bilinmiyor",
                             "name": k[0], "feature": k[1]})
        n = lambda st: sum(r["state"] == st for r in rows)
        done = len(rows) - n("bekliyor") - n("kosuyor")
        return {"run_id": run_id, "status": run["status"], "progress": f"{done}/{run['total_expected'] or len(rows)}",
                "passed": n("gecti"), "failed": n("kaldi"), "running": n("kosuyor"), "waiting": n("bekliyor"),
                "current": cur, "scenarios": rows}

    @app.get("/api/runs/{run_id}/live")
    def live_api(run_id: int):
        return snapshot(run_id)

    @app.websocket("/ws/runs/{run_id}")
    async def ws(sock: WebSocket, run_id: int):
        await sock.accept()
        q = yayin.subscribe(run_id)
        try:
            await sock.send_json(snapshot(run_id))
            while True:
                await q.get()
                await asyncio.sleep(0.25)       # 250 ms içindeki sinyaller tek görüntüde birleşir
                while not q.empty():
                    q.get_nowait()
                await sock.send_json(snapshot(run_id))
        except WebSocketDisconnect:
            pass
        finally:
            yayin.unsubscribe(run_id, q)

    return app
