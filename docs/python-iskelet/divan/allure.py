"""Allure dosya modeli (allure-cucumber7-jvm 2.x). Rust: crates/common/src/allure.rs"""
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


def doors_id(tag: str):
    m = re.fullmatch(r"(?:DOORS|doors)-([A-Za-z0-9_-]+)", tag.lstrip("@"))
    return f"DOORS-{m.group(1)}" if m else None


def jira_key(tag: str):
    m = re.fullmatch(r"(?:JIRA|jira)-([A-Za-z0-9_]+)-([0-9]+)", tag.lstrip("@"))
    return f"{m.group(1).upper()}-{m.group(2)}" if m else None


def duration_ms(o: dict):
    return o["stop"] - o["start"] if o.get("start") is not None and o.get("stop") is not None else None
