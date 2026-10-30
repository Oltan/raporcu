# ulak/src/runner.rs

**Amaç:** `ulak start` — koşumu açar (SW-3.1), izleyiciyi ayrı süreç olarak başlatır (SW-3.2),
testleri koşar (SW-1.2, 1.6, 1.7).

## Akış
1. **Windows oturum kontrolü (SW-1.6):** Windows'ta (ya da `ULAK_FAKE_WINDOWS_SESSION` ile
   benzetimde) kullanıcı adı `$` ile bitiyorsa (SYSTEM), `SESSIONNAME` yoksa ve Windows'un verdiği oturum numarası (`ProcessIdToSessionId`) okunamıyorsa ya da oturum `Services` (numara 0) ise
   açık mesajla **çıkış 4**. `ULAK_ALLOW_SERVICE_SESSION=true` atlatır.
2. `.ulak` hazırlanır; `stop`, `sent.txt` silinir; testler koşacaksa eski `allure-results` ve
   canlı olay dosyası silinir (`ULAK_KEEP_RESULTS` ile korunur).
3. Tag ifadesi: `ULAK_TAGS` (geçerli olmalı) ya da `features.txt`.
4. Beklenen senaryolar `gherkin::plan` ile; `POST /api/ci/runs/start` (CiInfo + `total` +
   `expected`), yeniden denemeli. Başarı → `run.json` + `ulak.env`, "koşum açıldı … Canlı
   durum: <rapor>/live". Sürüm uyumsuzsa çıkış 2. Sunucuya ulaşılamazsa UYARI, testler yine koşar.
5. `--no-tests` ise burada çıkar.
6. Koşum açıldıysa izleyici: kendi ikilisini `watch --run-id N` ile, çıktısı `.ulak/watch.log`
   (`ULAK_WATCH=off` ile kapalı).
7. Test komutu (`build_test_command`):
   - `ULAK_TEST_COMMAND` (varsayılan `mvn -B test`), kabuksuz parçalanır; Windows'ta `mvn` → `mvn.cmd`.
   - Maven ise (ya da `ULAK_FORCE_MAVEN_ARGS`): `-Dtest.xmx=<ULAK_TEST_XMX, 768m>`,
     `-Dallure.results.directory=<…>`, `-Dulak.canli.dosya=<…>`, tag varsa
     `-Dcucumber.filter.tags=<ifade>`. `ULAK_EXTRA_ARGS` eklenir.
   - Ortam: Linux'ta dil ayarı (LC_ALL > LC_CTYPE > LANG) UTF-8 değilse `LC_ALL=C.UTF-8` (runner servisi
     POSIX ile çalışınca Java komut satırındaki Türkçe tag/ad filtrelerini bozar);
     `MAVEN_OPTS` içinde `-Xmx` yoksa `-Xmx<ULAK_MAVEN_XMX, 512m> -XX:+ExitOnOutOfMemoryError`;
     `stdout/stderr.encoding=UTF-8`; `ULAK_VIDEO_FPS` (5).
   - `wrap_display`: Linux'ta `ULAK_XVFB=on` ya da (`auto` ve `DISPLAY` yok) ise
     `xvfb-run -a -n <99+pid%400> -s "-screen 0 <ULAK_SCREEN, 1920x1080>x24 -nolisten tcp" <komut>` (`-n`: boş ekran aramasının başlangıcı; aynı anda başlayan iki iş çakışmaz), `GDK_BACKEND=x11`.
8. `run_streaming`: stdout+stderr satır satır basılır; `java.lang.OutOfMemoryError`,
   "Terminating due to java.lang.OutOfMemoryError" ya da "Cannot allocate memory" görülürse işaretlenir.
9. Bitince `stop` dosyası yazılır, izleyici `ULAK_WATCH_DRAIN_SECS` (30) sn beklenir, sonra öldürülür.
10. OOM görüldüyse "BELLEK SINIRI AŞILDI … ULAK_TEST_XMX / ULAK_MAVEN_XMX artırın" ve **çıkış 3**;
    değilse test komutunun çıkış kodu (kalan senaryo → 1, iş kırmızı; report/fix yine çalışır).

## Testler
`test_sw_1_7_memory_limits_in_test_command`, `test_sw_1_7_oom_detected`; prova SW-1.x, 3.1, 3.2.

## Python karşılığı
`subprocess.Popen(cmd, stdout=PIPE, stderr=STDOUT, text=True, encoding="utf-8")`, satır satır okuma; izleyici `subprocess.Popen([sys.executable, …, "watch"])` (PyInstaller ikilisinde `sys.executable` ikilinin kendisi). Windows oturum kontrolü `os.environ.get("SESSIONNAME")`, yoksa `ctypes.windll.kernel32.ProcessIdToSessionId`; `getpass.getuser()`.
