"""Zip yükleme: gövde belleğe alınmaz. Rust: crates/divan/src/upload.rs"""
import json
import os
import shutil
import uuid
import zipfile
from pathlib import Path

from . import allure, defterdar

NO_RESULTS_MESSAGE = ("Zip içinde hiç Allure sonuç dosyası (*-result.json) yok. Test projesinde Allure "
                      "eklentisi (allure-cucumber7-jvm) ya da cucumber.plugin ayarı eksik olabilir.")


class UploadError(Exception):
    def __init__(self, status: int, message: str):
        super().__init__(message)
        self.status = status


def validate_entry_name(name: str) -> str | None:
    """Son ad parçası; klasör ise None. Tehlikeli ad tüm yüklemeyi reddeder."""
    if name.startswith(("/", "\\")) or ":" in name or "\0" in name:
        raise UploadError(400, f"geçersiz zip girdisi: {name}")
    parts = [p for p in name.replace("\\", "/").split("/")]
    if ".." in parts:
        raise UploadError(400, f"geçersiz zip girdisi: {name}")
    return None if name.endswith(("/", "\\")) else parts[-1]


async def body_to_file(stream, tmp_dir: Path, max_bytes: int) -> Path:
    """FastAPI: `await body_to_file(request.stream(), ...)` — 64 KB parçalarla diske."""
    tmp_dir.mkdir(parents=True, exist_ok=True)
    path = tmp_dir / f"upload-{uuid.uuid4()}.zip"
    size = 0
    with open(path, "wb") as f:
        async for chunk in stream:
            size += len(chunk)
            if size > max_bytes:
                f.close()
                path.unlink(missing_ok=True)
                raise UploadError(413, "yükleme boyutu sınırı aşıldı")
            f.write(chunk)
    return path


def process_zip(db, files_dir_of, zip_path: Path, target: dict, max_entry: int, max_total: int, max_entries=50000) -> dict:
    """target = {"new": ci} ya da {"existing": run_id}. files_dir_of(run_id) -> Path."""
    try:
        zf = zipfile.ZipFile(zip_path)
    except zipfile.BadZipFile:
        raise UploadError(400, "gövde geçerli bir zip değil")
    with zf:
        infos = zf.infolist()
        if len(infos) > max_entries:
            raise UploadError(400, "zip'te çok fazla dosya var")
        entries, total = [], 0
        for i in infos:  # 1. geçiş: içerik açılmadan denetim
            name = validate_entry_name(i.filename)
            if name is None:
                continue
            if i.file_size > max_entry:
                raise UploadError(413, f"zip girdisi çok büyük: {name}")
            total += i.file_size
            if total > max_total:
                raise UploadError(413, "açılmış boyut sınırı aşıldı")
            entries.append((name, i))
        if not any(allure.file_kind(n) == "result" for n, _ in entries):
            raise UploadError(422, NO_RESULTS_MESSAGE)

        summary = {"accepted_results": 0, "duplicate_results": 0, "containers": 0, "attachments": 0}
        with db.write_lock:
            c = db.writer
            c.execute("BEGIN IMMEDIATE")
            new_dir = None
            try:
                if "new" in target:
                    ci = target["new"]
                    pid = defterdar.ensure_project(c, ci)
                    run_id = defterdar.create_run(c, pid, ci, "running")
                    row = {"run_id": run_id, "project_id": pid, "branch": ci.get("branch") or "unknown"}
                else:
                    run_id = target["existing"]
                    r = c.execute("SELECT project_id, branch FROM runs WHERE id=?", (run_id,)).fetchone()
                    if not r:
                        raise UploadError(404, "koşum bulunamadı")
                    row = {"run_id": run_id, "project_id": r["project_id"], "branch": r["branch"]}
                files = files_dir_of(run_id)
                if "new" in target:
                    new_dir = files
                files.mkdir(parents=True, exist_ok=True)
                for name, info in entries:  # 2. geçiş
                    kind = allure.file_kind(name)
                    if kind in ("result", "container"):
                        try:
                            data = json.loads(zf.read(info))
                        except ValueError:
                            continue  # bozuk json yüklemeyi düşürmez
                        if kind == "result":
                            key = "accepted_results" if defterdar.ingest_result(c, row, data) else "duplicate_results"
                            summary[key] += 1
                        elif defterdar.ingest_container(c, run_id, data):
                            summary["containers"] += 1
                    elif kind == "attachment" and allure.is_safe_file_name(name):
                        dst = files / name
                        if not dst.exists():
                            part = files / f".{name}.part"  # yarım dosya görünmez
                            with zf.open(info) as src, open(part, "wb") as out:
                                shutil.copyfileobj(src, out, 64 * 1024)
                            os.replace(part, dst)
                            summary["attachments"] += 1
                defterdar.link_fixture_attachments(c, run_id)
                defterdar.recompute_run(c, run_id)
                if "new" in target:
                    defterdar.finish_run(c, run_id)
                c.execute("COMMIT")
            except BaseException:
                c.execute("ROLLBACK")
                if new_dir:
                    shutil.rmtree(new_dir, ignore_errors=True)
                raise
        summary["run_id"] = run_id
        return summary
