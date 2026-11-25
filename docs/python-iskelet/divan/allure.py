"""Allure dosya modeli (allure-cucumber7-jvm 2.x). Rust: crates/common/src/allure.rs"""
import os
import re

STATUSES = {"passed", "failed", "broken", "skipped", "unknown"}


def file_kind(name: str) -> str:
    if name.endswith("-result.json"):
        return "result"
    if name.endswith("-container.json"):
        return "container"
    if "-attachment" in name:
        return "attachment"
    return "other"


def is_safe_file_name(name: str) -> bool:
    return (
        0 < len(name) <= 255
        and not any(ch in name for ch in "/\\:\0")
        and name not in (".", "..")
        and not name.startswith("..")
    )


def status(obj: dict) -> str:
    s = obj.get("status")
    return s if s in STATUSES else "unknown"  # tanımsız adımlı senaryoda alan hiç yok


def scenario_key(r: dict) -> str:
    # rerun'da değişmez; geçmiş ve deneme gruplaması buna dayanır
    return r.get("historyId") or r.get("testCaseId") or r.get("fullName") or r.get("name", "")


def label(r: dict, name: str):
    return next((l.get("value") for l in r.get("labels", []) if l.get("name") == name), None)


def tags(r: dict) -> list[str]:
    return [l["value"].lstrip("@") for l in r.get("labels", []) if l.get("name") == "tag" and l.get("value")]


def feature(r: dict):
    return label(r, "feature") or label(r, "suite")


# Gereksinim kimliği öneki (Rust: common::kimlik, .env GEREKSINIM_ONEKI; varsayılan DOORS-).
GEREKSINIM_ONEKI = os.environ.get("GEREKSINIM_ONEKI", "").strip() or "DOORS-"


def doors_id(tag: str, onek: str | None = None):
    """Gereksinim kimliği. DOORS- için eski davranış (doors- de tanınır, DOORS- olarak);
    diğer önekler büyük/küçük harfe duyarlı, kimlik yazıldığı gibi (YTET_1234)."""
    onek = onek or GEREKSINIM_ONEKI
    t = tag.lstrip("@")
    if onek == "DOORS-":
        m = re.fullmatch(r"(?:DOORS|doors)-([A-Za-z0-9_-]+)", t)
        return f"DOORS-{m.group(1)}" if m else None
    rest = t[len(onek):] if t.startswith(onek) else ""
    return t if rest and all(ch.isalnum() or ch in "-_" for ch in rest) else None


# @ABC-12 tag'inin Jira issue'su sayılacağı projeler (Rust: kimlik::Onekler.jira_projeleri)
JIRA_TAG_PROJELERI = [p.strip().upper() for p in
                      (os.environ.get("JIRA_TAG_PROJELERI") or os.environ.get("JIRA_PROJECT") or "").split(",") if p.strip()]


def jira_key(tag: str, projeler=None):
    """@JIRA-ABC-12 (her proje) ya da @ABC-12 (ABC listede; gereksinim kimliği değilse) → ABC-12."""
    t = tag.lstrip("@")
    m = re.fullmatch(r"(?:JIRA|jira)-([A-Za-z0-9_]+)-([0-9]+)", t)
    if m:
        return f"{m.group(1).upper()}-{m.group(2)}"
    projeler = JIRA_TAG_PROJELERI if projeler is None else projeler
    m = re.fullmatch(r"([A-Za-z0-9_]+)-([0-9]+)", t)
    if not m or m.group(1).upper() not in projeler or doors_id(t):
        return None
    return f"{m.group(1).upper()}-{m.group(2)}"


def duration_ms(o: dict):
    return o["stop"] - o["start"] if o.get("start") is not None and o.get("stop") is not None else None
