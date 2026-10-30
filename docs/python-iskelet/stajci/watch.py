"""stajci watch: canlı olaylar + Allure dosyaları. Rust: crates/stajci/src/watch.rs"""
import json
import os
import time
from pathlib import Path

import requests

from raporcu import allure  # stajci ayrı pakette ise allure.py'nin kopyası


class EventTail:
    """Olay dosyasının yalnızca TAM satırlarını gönderir; konum yalnızca başarıda ilerler."""
    def __init__(self, path: Path):
        self.path, self.offset = path, 0

    def poll(self, send) -> bool:
        try:
            size = self.path.stat().st_size
        except FileNotFoundError:
            return True
        if size < self.offset:              # dosya yeniden oluşturulmuş
            self.offset = 0
        with open(self.path, "rb") as f:
            f.seek(self.offset)
            data = f.read(1024 * 1024)
        end = data.rfind(b"\n")
        if end < 0:
            return True                     # yarım satır: sonraki tura
        events = []
        for line in data[:end].splitlines():
            try:
                events.append(json.loads(line))
            except ValueError:
                pass                        # bozuk satır atlanır
        if events and not send(events):
            return False
        self.offset += end + 1
        return True


def watch(server: str, token: str, run_id: int, results: Path, events_file: Path, state: Path,
          interval=1.0, max_secs=6 * 3600):
    s = requests.Session()
    s.trust_env = False                     # sistem proxy'si kullanılmaz (aynı makine)
    s.headers.update({"Authorization": f"Bearer {token}", "x-stajci-version": "0.1.0"})
    base = f"{server}/api/ci/runs/{run_id}"
    sent_file = state / "sent.txt"
    sent = set(sent_file.read_text(encoding="utf-8").split()) if sent_file.exists() else set()
    tail, sizes, backoff, t0 = EventTail(events_file), {}, interval, time.monotonic()

    def send_events(ev):
        try:
            return s.post(f"{base}/events", json=ev, timeout=(5, 60)).status_code < 500
        except requests.RequestException:
            return False

    while True:
        stopping = (state / "stop").exists() or time.monotonic() - t0 > max_secs
        ok = tail.poll(send_events)
        if ok and results.is_dir():
            names = [n for n in sorted(os.listdir(results)) if n not in sent and allure.file_kind(n) != "other"]
            names.sort(key=lambda n: allure.file_kind(n) != "attachment")   # önce ekler, sonra json
            for n in names:
                p = results / n
                st = p.stat()
                if not stopping and time.time() - st.st_mtime < 0.3 and sizes.get(n) != st.st_size:
                    sizes[n] = st.st_size   # hâlâ yazılıyor olabilir
                    continue
                try:
                    r = s.post(f"{base}/results", params={"name": n}, data=p.read_bytes(), timeout=(5, 600))
                except requests.RequestException:
                    ok = False
                    break
                if r.status_code >= 500:
                    ok = False
                    break
                if r.status_code >= 400:
                    print(f"[stajci run_id={run_id}] UYARI: {n} reddedildi: {r.text[:200]}")
                sent.add(n)
                with open(sent_file, "a", encoding="utf-8") as f:
                    f.write(n + "\n")
        if stopping:
            return 0
        backoff = interval if ok else min(backoff * 2, 30)
        time.sleep(backoff)
