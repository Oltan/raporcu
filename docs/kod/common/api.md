# common/src/api.rs

**Amaç:** ulak ↔ divan istek/yanıt tipleri (JSON). Alan adları snake_case.

## CiInfo (GitLab CI değişkenlerinden; SW-4.3, 4.6)
| Alan | GitLab değişkeni | Not |
|------|------------------|-----|
| `project_id` | `CI_PROJECT_ID` | yoksa proje yolu kimliktir |
| `project_path` | `CI_PROJECT_PATH` | zorunlu (ör. `ekip/javafx-testleri`) |
| `project_url` | `CI_PROJECT_URL` | commit linki için |
| `branch` | `CI_COMMIT_REF_NAME` | |
| `commit` | `CI_COMMIT_SHA` | |
| `pipeline_id`, `pipeline_url` | `CI_PIPELINE_ID`, `CI_PIPELINE_URL` | |
| `job_id`, `job_url` | `CI_JOB_ID`, `CI_JOB_URL` | |
| `os` | `ULAK_OS` ya da çalışan sistem | `linux` / `windows` |

`to_query()` → `project_id=..&project_path=..` sorgu dizesi; değerler RFC 3986 ile yüzde
kodlanır (`A-Z a-z 0-9 - _ . ~` dışındaki her bayt `%XX`), boş (None) alanlar atlanır.
`POST /api/ci/runs?<sorgu>` toplu yüklemede kullanılır.

## Diğer tipler
| Tip | Yön | Alanlar |
|-----|-----|---------|
| `StartRunRequest` | ulak → `/runs/start` | CiInfo alanları (düz) + `total` |
| `RunCreated` | yanıt | `run_id`, `report_url` |
| `IngestSummary` | yükleme yanıtı | `run_id, accepted_results, duplicate_results, containers, attachments, report_url` |
| `FinishRequest` | `/finish` | `status`: `finished` (varsayılan) / `interrupted` |
| `FixResult` | ulak fix → `/fix-result` | `run_id, scenario_key, outcome (mr_opened/draft_mr/no_change/rejected/error), mr_iid, mr_url, branch, validation_passed, summary, warnings[], diff, triage` |
| `TriageSuggestion` | FixResult içinde | `category, reason, confidence (0–1)` |
| `FixCandidate` | `/fix-candidates` yanıtı | `scenario_id, scenario_key, name, feature, feature_file, status, target (opencode/jira/atla/bildirim), message, trace, steps[], screenshot_url, tags[], examples[], texts[]` |
| `FixText` | aday içinde | küçük metin ekleri: `name, content` (ör. TestFX sahne ağacı) |
| `FixExample` | aday içinde | geçmiş birleşmiş düzeltme: `scenario, message, diff` |

Not: `/runs/start` ayrıca `expected: [{name, feature}]` alır (divan tarafında `StartBody`).

## Testler
`test_sw_4_3_ci_info_query_encoding`
