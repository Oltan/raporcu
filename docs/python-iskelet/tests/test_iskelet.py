"""Rust testlerinin Python karşılıkları (crates/raporcu/tests/*). Aynı testdata kullanılır."""
import io
import json
import threading
import time
import zipfile
from pathlib import Path

import pytest
import uvicorn
from fastapi.testclient import TestClient

from raporcu.app import create_app
from stajci.watch import EventTail, watch

REPO = Path(__file__).resolve().parents[3]          # docs/python-iskelet/tests → depo kökü
SAMPLE = REPO / "testdata/allure/ornek-giris"
EVENTS = REPO / "testdata/canli/ornek-giris-olaylar.jsonl"
MIGR = REPO / "migrations"
TOKEN = "test-ci-token"
H = {"Authorization": f"Bearer {TOKEN}"}
Q = "project_path=ekip/javafx&branch=main&pipeline_id=9001"


@pytest.fixture
def client(tmp_path):
    app = create_app({"data_dir": tmp_path, "public_url": "http://raporcu.test", "ci_api_token": TOKEN,
                      "migrations_dir": MIGR, "max_upload_mb": 512, "max_entry_mb": 256})
    with TestClient(app) as c:
        c.app = app
        yield c


def zip_of(entries) -> bytes:
    b = io.BytesIO()
    with zipfile.ZipFile(b, "w") as z:
        for name, data in entries:
            z.writestr(name, data)
    return b.getvalue()


def sample_zip(keep=lambda n: True) -> bytes:
    return zip_of([(f"allure-results/{p.name}", p.read_bytes()) for p in sorted(SAMPLE.iterdir()) if keep(p.name)])


def test_sw_10_5_health(client):
    assert client.get("/api/health").status_code == 200


def test_sw_9_1_ci_requires_token(client):
    assert client.post(f"/api/ci/runs?{Q}", content=sample_zip()).status_code == 401
    assert client.post(f"/api/ci/runs?{Q}", content=b"x", headers={"Authorization": "Bearer yanlis"}).status_code == 401


def test_sw_10_9_version_mismatch(client):
    r = client.post("/api/ci/runs/start", json={"project_path": "a/b"}, headers={**H, "x-stajci-version": "9.9.0"})
    assert r.status_code == 409


def test_sw_2_1_upload_real_sample(client):
    r = client.post(f"/api/ci/runs?{Q}", content=sample_zip(), headers=H)
    assert r.status_code == 200, r.text
    s = r.json()
    assert s["accepted_results"] == 11 and s["report_url"].endswith(f"/runs/{s['run_id']}")
    c = client.app.state.db.reader()
    run = c.execute("SELECT * FROM runs WHERE id=?", (s["run_id"],)).fetchone()
    assert (run["status"], run["total"], run["passed"], run["failed"], run["broken"], run["unknown"]) == \
        ("finished", 7, 4, 1, 1, 1)
    sc = c.execute("SELECT * FROM scenarios WHERE name='Yanlış şifrede özel mesaj' AND is_latest=1").fetchone()
    assert "DOORS-1003" in sc["doors_ids"] and "STAJ-42" in sc["jira_keys"]
    assert sc["feature_file"] == "features/giris.feature"


def test_sw_2_2_after_screenshot_only_on_failed(client):
    rid = client.post(f"/api/ci/runs?{Q}", content=sample_zip(), headers=H).json()["run_id"]
    c = client.app.state.db.reader()
    for s in c.execute("SELECT id, status FROM scenarios WHERE run_id=? AND is_latest=1", (rid,)):
        shots = c.execute("SELECT origin FROM attachments WHERE scenario_id=? AND mime='image/png'", (s["id"],)).fetchall()
        if s["status"] == "passed":
            assert not shots
        if s["status"] in ("failed", "broken"):
            assert [x["origin"] for x in shots] == ["after"]


def test_sw_2_1_zip_slip_and_no_results(client):
    assert client.post(f"/api/ci/runs?{Q}", content=zip_of([("../x-result.json", "{}")]), headers=H).status_code == 400
    r = client.post(f"/api/ci/runs?{Q}", content=zip_of([("a.txt", "x")]), headers=H)
    assert r.status_code == 422 and "Allure" in r.json()["hata"]
    assert client.post(f"/api/ci/runs?{Q}", content=b"zip degil", headers=H).status_code == 400


def test_sw_3_3_and_2_2_file_by_file_containers_first(client):
    rid = client.post("/api/ci/runs/start", json={"project_path": "ekip/canli", "branch": "main"}, headers=H).json()["run_id"]
    files = sorted(SAMPLE.iterdir(), key=lambda p: (not p.name.endswith("-container.json"), p.name))
    for p in files + files:   # her şey iki kez: tekrarlar sayılmaz
        assert client.post(f"/api/ci/runs/{rid}/results", params={"name": p.name}, content=p.read_bytes(), headers=H).status_code == 200
    c = client.app.state.db.reader()
    run = c.execute("SELECT total, passed FROM runs WHERE id=?", (rid,)).fetchone()
    assert (run["total"], run["passed"]) == (7, 4)
    assert c.execute("SELECT COUNT(*) FROM attachments WHERE run_id=? AND origin='after' AND scenario_id IS NULL", (rid,)).fetchone()[0] == 0


def real_events():
    return [json.loads(l) for l in EVENTS.read_text(encoding="utf-8").splitlines() if l.strip()]


def test_sw_3_4_live_current_from_cucumber_events(client):
    exp = [{"name": "Geçerli bilgilerle giriş", "feature": "Giriş ekranı"},
           {"name": "Hatalı girişlerde mesaj", "feature": "Giriş ekranı"},
           {"name": "Hatalı girişlerde mesaj", "feature": "Giriş ekranı"}]
    rid = client.post("/api/ci/runs/start", json={"project_path": "ekip/canli-adim", "branch": "main", "expected": exp},
                      headers=H).json()["run_id"]
    ev = real_events()
    second = [i for i, e in enumerate(ev) if e["tur"] == "senaryo_basladi"][1]
    assert client.post(f"/api/ci/runs/{rid}/events", json=ev[:second + 4], headers=H).status_code == 204
    live = client.get(f"/api/runs/{rid}/live").json()
    cur = live["current"]
    assert cur["active"] and cur["name"] == "Hatalı girişlerde mesaj" and cur["step_no"] == 2
    assert cur["step"].startswith("Eğer ki kullanıcı adı") and cur["steps"][0]["status"] == "PASSED"
    assert [s["state"] for s in live["scenarios"]] == ["gecti", "kosuyor", "bekliyor"]
    with client.websocket_connect(f"/ws/runs/{rid}") as ws:
        assert ws.receive_json()["current"]["name"] == "Hatalı girişlerde mesaj"
        client.post(f"/api/ci/runs/{rid}/finish", json={}, headers=H)
        assert ws.receive_json()["status"] == "finished"
    assert client.get(f"/api/runs/{rid}/live").json()["current"] is None
    assert client.post(f"/api/ci/runs/{rid}/events", json=ev[:2], headers=H).status_code == 202


def test_sw_3_2_event_tail_sends_only_complete_lines(tmp_path):
    f = tmp_path / "olaylar.jsonl"
    f.write_text('{"tur":"a"}\n{"tur":"b"', encoding="utf-8")
    got, t = [], EventTail(f)
    assert t.poll(lambda ev: got.extend(ev) or True) and got == [{"tur": "a"}]
    assert t.poll(lambda ev: False) and got == [{"tur": "a"}]  # yarım satır: gönderilecek bir şey yok
    with open(f, "a", encoding="utf-8") as h:
        h.write('}\n')
    assert not t.poll(lambda ev: False)  # gönderim başarısız: konum ilerlemez
    assert t.poll(lambda ev: got.extend(ev) or True) and got[-1] == {"tur": "b"}


def test_sw_3_2_watch_against_real_server(tmp_path):
    app = create_app({"data_dir": tmp_path / "data", "public_url": "http://127.0.0.1", "ci_api_token": TOKEN,
                      "migrations_dir": MIGR, "max_upload_mb": 512, "max_entry_mb": 256})
    srv = uvicorn.Server(uvicorn.Config(app, host="127.0.0.1", port=18787, log_level="error"))
    th = threading.Thread(target=srv.run, daemon=True)
    th.start()
    while not srv.started:
        time.sleep(0.05)
    import requests
    rid = requests.post("http://127.0.0.1:18787/api/ci/runs/start", json={"project_path": "a/b"}, headers=H).json()["run_id"]
    res, state = tmp_path / "allure-results", tmp_path / ".stajci"
    res.mkdir(); state.mkdir()
    for p in SAMPLE.iterdir():
        (res / p.name).write_bytes(p.read_bytes())
    ev = tmp_path / "olaylar.jsonl"
    ev.write_text(EVENTS.read_text(encoding="utf-8")[:2000].rsplit("\n", 1)[0] + "\n", encoding="utf-8")
    (state / "stop").write_text("bitti")
    assert watch("http://127.0.0.1:18787", TOKEN, rid, res, ev, state, interval=0.1) == 0
    live = requests.get(f"http://127.0.0.1:18787/api/runs/{rid}/live").json()
    assert live["status"] == "running" and live["current"]["name"]
    c = app.state.db.reader()
    assert c.execute("SELECT total FROM runs WHERE id=?", (rid,)).fetchone()[0] == 7
    srv.should_exit = True
    th.join(5)
