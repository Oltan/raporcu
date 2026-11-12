# CI şablonu ve dağıtım dosyaları

## ci/java-sablon/DivanKanit.java
Test projesine kopyalanan yardımcı sınıf (`docs/LLM_JAVA_ENTEGRASYON.md`, adım 3). Senaryo kaldıysa
pencere bölgesinin ekran görüntüsünü ve TestFX sahne ağacını Cucumber ekine koyar; geçen senaryoda
hiçbir şey yapmaz. Kanıt alınamazsa yalnızca `System.err`'e yazar, senaryonun gerçek hatası değişmez.
İki kanıt bağımsızdır: ekran görüntüsü alınamasa da ağaç eklenir. Ağaç JavaFX iş parçacığında okunur
(10 sn içinde sıra gelmezse test iş parçacığından). İçerik taşıyan kontrollerin içine inilir:
`TabPane` (her `Tab` ayrı satır, `(seçili)` / `(seçili değil)`), `ScrollPane`, `TitledPane`, `SplitPane`,
`Accordion`, `ToolBar`, `ButtonBar`; diğer kontrollerin iç yapısı (skin) yazılmaz. Ağaç boşsa (pencere
kapatıldıktan sonra çağrıldıysa) ek konmaz, uyarı yazılır. Aynı ağaç kodu `ornek-test-projesi`
içindeki `Hooks.java`'da da durur.

## ci/gitlab/template/.gitlab-ci.yml (SW-1.1, 1.4)
Test projesi `include: project/ref/file` + `ULAK_OS: linux|windows` ekler. Aşamalar
`test → report → fix`.

| Ortak iş | Script | Notlar |
|----------|--------|--------|
| `.ulak-linux` / `.ulak-windows` | `before_script`: `curl -fsS -o ulak "$ULAK_SERVER/api/tools/ulak?os=…"` | etiket `linux` / `windows`; OS'e özgü tek yer (SW-10.9) |
| `.ulak-test` | `ulak preflight`, `ulak start` | artifact (her zaman, 7 gün): `target/allure-results/`, `.ulak/`; `reports: dotenv: ulak.env` (`ULAK_RUN_ID`, `ULAK_REPORT_URL`); `ULAK_FIX_ONLY=true` ise atlanır |
| `.ulak-report` | `ulak upload`, `ulak finish` | `when: always` (test düşse de); artifact `.ulak/queue/` 30 gün (SW-2.6); `environment: rapor/$CI_COMMIT_REF_SLUG`, `url: $ULAK_REPORT_URL` (SW-5.4) |
| `.ulak-fix` | `ulak degerlendir`, `ulak fix` | `allow_failure: true`; `ULAK_FIX=off` ise yok (`ULAK_DEGERLENDIR=on` verilmediyse) |

Somut işler: `ulak-{test,report,fix}-{linux,windows}`, `ULAK_OS`'a göre `rules` ile seçilir. Hiçbiri
`merge_request_event` pipeline'ında koşmaz; `auto-fix/…` branch'inde test/report yalnızca
`ULAK_FIX_PIPELINE=on` ile, fix asla (döngü önleme);
Windows işlerinde `ULAK_XVFB=off`. Değişkenler: `ULAK_SERVER=http://127.0.0.1:8080` (aynı makine),
`ULAK_RESULTS_DIR`, `ULAK_MAVEN_XMX=512m`, `ULAK_TEST_XMX=768m`, `ULAK_VIDEO_FPS=5`,
`ULAK_VIDEO` (şablonda yorum satırı; Windows'ta gdigrab tüm masaüstünü kaydeder, kişisel masaüstünde `false`),
`ULAK_XVFB=on`, `ULAK_SCREEN=1920x1080`, `GET_SOURCES_ATTEMPTS=5`, `ARTIFACT_DOWNLOAD_ATTEMPTS=5`
(GitLab'a giden ağ kısa süre koparsa report işi kod çekerken düşmez), isteğe bağlı `ULAK_TAGS`,
`ULAK_FIX`; `CI_API_TOKEN` masked proje/grup değişkeni.

## ci/gitlab/runner/config.toml.example
`concurrent = 1` (aynı makinede tek test işi, SW-1.7), shell executor örneği; yorum satırı olarak
`environment = ["AD=deger", ...]` (ulak ve ajan ayarlarının runner'da verilmesi).

## ci/gitlab/runner/runner.env.example
Runner tarafının bütün ortam değişkenleri, örnek değerleriyle: `ulak` (sunucu bağlantısı, test koşumu,
ekran/video, canlı aktarım, değerlendirme ve self-healing), OpenCode (`VLLM_*`), Claude Code
(`ANTHROPIC_*`), şirket CA'sı. Hiçbir süreç bu dosyayı okumaz: satırlar GitLab CI değişkenine
(`[MASKED]` işaretliler masked) ya da runner `environment` listesine yazılır; Linux'ta elle denemede
kabuğa yüklenebilir (`set -a; . ./runner.env; set +a`). Açık satırlar varsayılan değerlerdir (adres,
token, model adı hariç); davranışı değiştirenler yorum satırıdır. Sunucunun kendi örneği `.env.example`
ayrıdır. Eksiksizliği bir test denetlemez: `ulak`'a yeni değişken eklenince bu dosya elle güncellenir.

## ci/opencode/template, ci/claude/template
Test projesine kopyalanacak ajan dosyaları. OpenCode: `opencode.json` (vLLM sağlayıcısı), `AGENTS.md`,
`.opencode/agents/fix.md` ve `degerlendir.md` (salt okunur). Claude Code: `.claude/settings.json`
(düzenleme yalnızca `src/test/**`, Bash ve web kapalı), `.mcp.json` (TestFX MCP), `CLAUDE.md` (aynı
kurallar). Pakette OpenCode dosyaları `test-projesi/` altında (`projeye-ekle` kopyalar), Claude Code
dosyaları `claude-code/` altındadır (elle kopyalanır). Ayrıntı `docs/KOD_AJANI_VLLM.md`.

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

## deploy/paket-olustur.ps1, deploy/linux-derle.sh
Çevrimdışı kurulum paketini üretir (`dist/divan-kurulum.zip`; içeriği `docs/KURULUM.md`). Windows
ikilileri statik C çalışma zamanıyla (`+crt-static`) derlenir, yanında VC++ çalışma zamanı istemez.
Linux ikilileri WSL'de musl hedefiyle derlenir; `musl-gcc` yoksa sudo gerektirmeyen `cargo-zigbuild`
kullanılır. Java kaynakları `divan.destek` paketine taşınıp `ci/java-destek/pom.xml` ile jar yapılır.
Maven deposuna iki platformun bağımlılıkları iner; depo kimliği kayıtları (`_remote.repositories`)
silinir, yoksa başka `settings.xml` ile çevrimdışı çözümleme reddedilir.

## deploy/projeye-ekle.ps1, deploy/projeye-ekle.sh
Paketteki `test-projesi/` ve `ozel/` içeriğini test projesine kopyalar, `ULAK_OS`'u ayarlar. Var olan
dosyayı ezmez (`-Ustune` / `ustune`). `ozel/agents/*.md` → `.opencode/agents/`, `ozel/skills/*` →
`.claude/skills/`; `ORNEK-ajan.md` ve `ornek-skill` kopyalanmaz.
