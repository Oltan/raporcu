# CI şablonu ve dağıtım dosyaları

## ci/gitlab/template/.gitlab-ci.yml (SW-1.1, 1.4)
Test projesi `include: project/ref/file` + `ULAK_OS: linux|windows` ekler. Aşamalar
`test → report → fix`.

| Ortak iş | Script | Notlar |
|----------|--------|--------|
| `.ulak-linux` / `.ulak-windows` | `before_script`: `curl -fsS -o ulak "$ULAK_SERVER/api/tools/ulak?os=…"` | etiket `linux` / `windows`; OS'e özgü tek yer (SW-10.9) |
| `.ulak-test` | `ulak preflight`, `ulak start` | artifact (her zaman, 7 gün): `target/allure-results/`, `.ulak/`; `reports: dotenv: ulak.env` (`ULAK_RUN_ID`, `ULAK_REPORT_URL`); `ULAK_FIX_ONLY=true` ise atlanır |
| `.ulak-report` | `ulak upload`, `ulak finish` | `when: always` (test düşse de); artifact `.ulak/queue/` 30 gün (SW-2.6); `environment: rapor/$CI_COMMIT_REF_SLUG`, `url: $ULAK_REPORT_URL` (SW-5.4) |
| `.ulak-fix` | `ulak fix` | `allow_failure: true`; `ULAK_FIX=off` ise yok |

Somut işler: `ulak-{test,report,fix}-{linux,windows}`, `ULAK_OS`'a göre `rules` ile seçilir. Hiçbiri
`merge_request_event` pipeline'ında koşmaz; `auto-fix/…` branch'inde test/report yalnızca
`ULAK_FIX_PIPELINE=on` ile, fix asla (döngü önleme);
Windows işlerinde `ULAK_XVFB=off`. Değişkenler: `ULAK_SERVER=http://127.0.0.1:8080` (aynı makine),
`ULAK_RESULTS_DIR`, `ULAK_MAVEN_XMX=512m`, `ULAK_TEST_XMX=768m`, `ULAK_VIDEO_FPS=5`,
`ULAK_XVFB=on`, `ULAK_SCREEN=1920x1080`, `GET_SOURCES_ATTEMPTS=5`, `ARTIFACT_DOWNLOAD_ATTEMPTS=5`
(GitLab'a giden ağ kısa süre koparsa report işi kod çekerken düşmez), isteğe bağlı `ULAK_TAGS`,
`ULAK_FIX`; `CI_API_TOKEN` masked proje/grup değişkeni.

## ci/gitlab/runner/config.toml.example
`concurrent = 1` (aynı makinede tek test işi, SW-1.7), shell executor örneği.

## ci/opencode/template, ci/claude/template
Test projesine kopyalanacak ajan dosyaları: `opencode.json` (vLLM sağlayıcısı, `fix` ajanı),
`AGENTS.md`; `CLAUDE.md` (Claude Code için aynı kurallar). Ayrıntı `docs/KOD_AJANI_VLLM.md`.

## deploy/linux/divan.service
systemd hizmeti: `divan` kullanıcısı, `/opt/divan` (`.env` çalışma klasöründen okunur), `MemoryMax=150M` / `MemoryHigh=110M`
(SW-10.6), `Restart=on-failure`, sertleştirme (`ProtectSystem=strict`, yazma yalnızca `/var/lib/divan` ve `/opt/divan`).

## deploy/windows/kurulum.ps1
Uyku/ekran kapanması/kilit kapalı; GitLab runner'ı **oturum açılınca** başlayan zamanlanmış görev
olarak (hizmet değil, TestFX masaüstü ister, SW-1.6); divan'ı açılışta SYSTEM görevi olarak kurar.

## deploy/windows/rdp-cikis.cmd
RDP oturumunu `tscon <oturum> /dest:console` ile konsola geri verir; masaüstü kilitlenmez.

## .cargo/config.toml, clippy.toml, rustfmt.toml
Windows hedefi için mingw bağlayıcı ve `runner = "wine"` (Linux'ta `cargo test --target
x86_64-pc-windows-gnu`); clippy `type-complexity-threshold = 600`; biçim ayarları.
