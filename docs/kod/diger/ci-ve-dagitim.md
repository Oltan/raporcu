# CI şablonu ve dağıtım dosyaları

## ci/gitlab/template/.gitlab-ci.yml (SW-1.1, 1.4)
Test projesi `include: project/ref/file` + `STAJ_OS: linux|windows` ekler. Aşamalar
`test → report → fix`.

| Ortak iş | Script | Notlar |
|----------|--------|--------|
| `.staj-linux` / `.staj-windows` | `before_script`: `curl -fsS -o stajci "$STAJ_SERVER/api/tools/stajci?os=…"` | etiket `linux` / `windows`; OS'e özgü tek yer (SW-10.9) |
| `.staj-test` | `stajci preflight`, `stajci start` | artifact (her zaman, 7 gün): `target/allure-results/`, `.stajci/`; `reports: dotenv: stajci.env` (`STAJ_RUN_ID`, `STAJ_REPORT_URL`); `STAJ_FIX_ONLY=true` ise atlanır |
| `.staj-report` | `stajci upload`, `stajci finish` | `when: always` (test düşse de); artifact `.stajci/queue/` 30 gün (SW-2.6); `environment: rapor/$CI_COMMIT_REF_SLUG`, `url: $STAJ_REPORT_URL` (SW-5.4) |
| `.staj-fix` | `stajci fix` | `allow_failure: true`; `STAJ_FIX=off` ise yok |

Somut işler: `staj-{test,report,fix}-{linux,windows}`, `STAJ_OS`'a göre `rules` ile seçilir;
Windows işlerinde `STAJ_XVFB=off`. Değişkenler: `STAJ_SERVER=http://127.0.0.1:8080` (aynı makine),
`STAJ_RESULTS_DIR`, `STAJ_MAVEN_XMX=512m`, `STAJ_TEST_XMX=768m`, `STAJ_VIDEO_FPS=5`,
`STAJ_XVFB=on`, `STAJ_SCREEN=1920x1080`, isteğe bağlı `STAJ_TAGS`, `STAJ_FIX`; `CI_API_TOKEN`
masked proje/grup değişkeni.

## ci/gitlab/runner/config.toml.example
`concurrent = 1` (aynı makinede tek test işi, SW-1.7), shell executor örneği.

## ci/opencode/template, ci/claude/template
Test projesine kopyalanacak ajan dosyaları: `opencode.json` (vLLM sağlayıcısı, `fix` ajanı),
`AGENTS.md`; `CLAUDE.md` (Claude Code için aynı kurallar). Ayrıntı `docs/KOD_AJANI_VLLM.md`.

## deploy/linux/raporcu.service
systemd hizmeti: `raporcu` kullanıcısı, `/opt/raporcu` (`.env` çalışma klasöründen okunur), `MemoryMax=150M` / `MemoryHigh=110M`
(SW-10.6), `Restart=on-failure`, sertleştirme (`ProtectSystem=strict`, yazma yalnızca `/var/lib/raporcu` ve `/opt/raporcu`).

## deploy/windows/kurulum.ps1
Uyku/ekran kapanması/kilit kapalı; GitLab runner'ı **oturum açılınca** başlayan zamanlanmış görev
olarak (hizmet değil, TestFX masaüstü ister, SW-1.6); raporcu'yu açılışta SYSTEM görevi olarak kurar.

## deploy/windows/rdp-cikis.cmd
RDP oturumunu `tscon <oturum> /dest:console` ile konsola geri verir; masaüstü kilitlenmez.

## .cargo/config.toml, clippy.toml, rustfmt.toml
Windows hedefi için mingw bağlayıcı ve `runner = "wine"` (Linux'ta `cargo test --target
x86_64-pc-windows-gnu`); clippy `type-complexity-threshold = 600`; biçim ayarları.
