"""Canlı durum: Cucumber olayları bellekte. Rust: crates/divan/src/live.rs"""
import asyncio
import threading
import time
from collections import defaultdict
from dataclasses import dataclass, field


@dataclass
class Current:
    name: str = ""
    feature: str = ""
    uri: str = ""
    line: int = 0
    active: bool = False
    step: str | None = None
    step_no: int = 0
    steps: list = field(default_factory=list)       # [{text, status, ms, error}]
    attempt: int = 0
    finished_scenarios: int = 0
    finished: list = field(default_factory=list)    # [(ad, feature, durum, "uri|satir")]
    started: float = 0.0
    seen: dict = field(default_factory=lambda: defaultdict(int))

    def public(self) -> dict:  # API/WebSocket'e giden kısım
        return {"name": self.name, "feature": self.feature, "uri": self.uri, "line": self.line,
                "active": self.active, "step": self.step, "step_no": self.step_no, "steps": self.steps,
                "attempt": self.attempt, "finished_scenarios": self.finished_scenarios,
                "elapsed_s": int(time.monotonic() - self.started) if self.started else 0}


class LiveStates:
    def __init__(self):
        self._m: dict[int, Current] = {}
        self._lock = threading.Lock()

    def apply(self, run_id: int, events: list[dict]):
        with self._lock:
            cur = self._m.setdefault(run_id, Current())
            for e in events:
                t = e.get("tur")
                if t == "senaryo_basladi":
                    key = f"{e.get('uri', '')}|{e.get('satir', 0)}"
                    cur.seen[key] += 1
                    cur.name, cur.feature = e.get("ad", ""), e.get("feature", "")
                    cur.uri, cur.line = e.get("uri", ""), e.get("satir", 0)
                    cur.active, cur.step, cur.step_no, cur.steps = True, None, 0, []
                    cur.attempt = cur.seen[key]          # rerun: aynı uri|satır kaçıncı kez
                    cur.started = time.monotonic()
                elif t == "adim_basladi":
                    cur.step = (e.get("metin") or "")[:300]
                    cur.step_no = e.get("sira", cur.step_no + 1)
                elif t == "adim_bitti":
                    cur.steps.append({"text": (e.get("metin") or "")[:300], "status": e.get("durum", ""),
                                      "ms": e.get("sure_ms", 0), "error": (e.get("hata") or None) and e["hata"][:500]})
                    cur.steps = cur.steps[-100:]
                    cur.step = None
                elif t == "senaryo_bitti":
                    cur.finished.append((cur.name, cur.feature, e.get("durum", ""), f"{cur.uri}|{cur.line}"))
                    cur.active, cur.step = False, None
                    cur.finished_scenarios += 1
                # tanınmayan olay yok sayılır

    def get(self, run_id: int):
        with self._lock:
            c = self._m.get(run_id)
            return c.public() if c else None

    def finished_status(self, run_id: int) -> dict:
        """(ad, feature) -> son Cucumber durumu; rerun'da aynı uri|satır'ın SON durumu geçerli."""
        with self._lock:
            c = self._m.get(run_id)
            last = {}
            for name, feat, st, key in (c.finished if c else []):
                last[key] = (name, feat, st)
            out = defaultdict(list)
            for name, feat, st in last.values():
                out[(name, feat)].append(st)   # taslak satırları aynı adı taşır: sırayla tüketilir
            return out

    def remove(self, run_id: int):
        with self._lock:
            self._m.pop(run_id, None)


class Broadcast:
    """Koşum başına yayın: her WebSocket kendi kuyruğunu dinler."""
    def __init__(self):
        self._subs: dict[int, set[asyncio.Queue]] = defaultdict(set)

    def subscribe(self, run_id):
        q = asyncio.Queue(maxsize=16)
        self._subs[run_id].add(q)
        return q

    def unsubscribe(self, run_id, q):
        self._subs[run_id].discard(q)

    def notify(self, run_id):
        for q in list(self._subs[run_id]):
            if not q.full():
                q.put_nowait(1)
