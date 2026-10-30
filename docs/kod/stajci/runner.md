# stajci/src/runner.rs

**Amaç:** `stajci start` — koşumu açar (SW-3.1), izleyiciyi ayrı süreç olarak başlatır (SW-3.2),
testleri koşar (SW-1.2, 1.6, 1.7).

## Akış
1. **Windows oturum kontrolü (SW-1.6):** Windows'ta (ya da `STAJ_FAKE_WINDOWS_SESSION` ile
   benzetimde) kullanıcı adı `$` ile bitiyorsa (SYSTEM), `SESSIONNAME` yoksa ya da `Services` ise
   açık mesajla **çıkış 4**. `STAJ_ALLOW_SERVICE_SESSION=true` atlatır.
2. `.stajci` hazırlanır; `stop`, `sent.txt` silinir; testler koşacaksa eski `allure-results` ve
   canlı olay dosyası silinir (`STAJ_KEEP_RESULTS` ile korunur).
3. Tag ifadesi: `STAJ_TAGS` (geçerli olmalı) ya da `features.txt`.
4. Beklenen senaryolar `gherkin::plan` ile; `POST /api/ci/runs/start` (CiInfo + `total` +
   `expected`), yeniden denemeli. Başarı → `run.json` + `stajci.env`, "koşum açıldı … Canlı
   durum: <rapor>/live". Sürüm uyumsuzsa çıkış 2. Sunucuya ulaşılamazsa UYARI, testler yine koşar.
5. `--no-tests` ise burada çıkar.
6. Koşum açıldıysa izleyici: kendi ikilisini `watch --run-id N` ile, çıktısı `.stajci/watch.log`
   (`STAJ_WATCH=off` ile kapalı).
7. Test komutu (`build_test_command`):
   - `STAJ_TEST_COMMAND` (varsayılan `mvn -B test`), kabuksuz parçalanır; Windows'ta `mvn` → `mvn.cmd`.
   - Maven ise (ya da `STAJ_FORCE_MAVEN_ARGS`): `-Dtest.xmx=<STAJ_TEST_XMX, 768m>`,
     `-Dallure.results.directory=<…>`, `-Dstajci.canli.dosya=<…>`, tag varsa
     `-Dcucumber.filter.tags=<ifade>`. `STAJ_EXTRA_ARGS` eklenir.
   - Ortam: `MAVEN_OPTS` içinde `-Xmx` yoksa `-Xmx<STAJ_MAVEN_XMX, 512m> -XX:+ExitOnOutOfMemoryError`;
     `stdout/stderr.encoding=UTF-8`; `STAJ_VIDEO_FPS` (5).
   - `wrap_display`: Linux'ta `STAJ_XVFB=on` ya da (`auto` ve `DISPLAY` yok) ise
     `xvfb-run -a -n <99+pid%400> -s "-screen 0 <STAJ_SCREEN, 1920x1080>x24 -nolisten tcp" <komut>` (`-n`: boş ekran aramasının başlangıcı; aynı anda başlayan iki iş çakışmaz), `GDK_BACKEND=x11`.
8. `run_streaming`: stdout+stderr satır satır basılır; `java.lang.OutOfMemoryError`,
   "Terminating due to java.lang.OutOfMemoryError" ya da "Cannot allocate memory" görülürse işaretlenir.
9. Bitince `stop` dosyası yazılır, izleyici `STAJ_WATCH_DRAIN_SECS` (30) sn beklenir, sonra öldürülür.
10. OOM görüldüyse "BELLEK SINIRI AŞILDI … STAJ_TEST_XMX / STAJ_MAVEN_XMX artırın" ve **çıkış 3**;
    değilse test komutunun çıkış kodu (kalan senaryo → 1, iş kırmızı; report/fix yine çalışır).

## Testler
`test_sw_1_7_memory_limits_in_test_command`, `test_sw_1_7_oom_detected`; prova SW-1.x, 3.1, 3.2.

## Python karşılığı
`subprocess.Popen(cmd, stdout=PIPE, stderr=STDOUT, text=True, encoding="utf-8")`, satır satır okuma; izleyici `subprocess.Popen([sys.executable, …, "watch"])` (PyInstaller ikilisinde `sys.executable` ikilinin kendisi). Windows oturum kontrolü `os.environ["SESSIONNAME"]`, `getpass.getuser()`.
