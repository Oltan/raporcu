# Uygulama kararları ve varsayımlar

`GEREKSINIMLER_1.md` içindeki açık sorular yanıtlanmadan ilgili gereksinimler istenince aşağıdaki
varsayımlarla uygulandı. Hepsi yapılandırmayla değiştirilebilir; yanıt gelince gözden geçirilmeli.

## Açık soru 1 — GitLab token yetkisi (SW-7.4, 7.5)

- Varsayım: divan'a `api` kapsamlı bir token verilebilir (`GITLAB_TOKEN`). "OpenCode ile
  düzelt" düğmesi `POST /projects/:id/pipeline` ile `ULAK_FIX_ONLY=true` ve `ULAK_RUN_ID`
  değişkenli pipeline başlatır; şablonda test ve report işleri bu durumda koşmaz.
- SW-7.5 (birleştirilen/reddedilen) GitLab API sorgusu yerine **Merge request webhook**'uyla
  çalışır; bu yüzden okuma yetkisi gerekmez.
- Token verilmezse düğme "GitLab bağlantısı ayarlı değil" mesajı verir; geri kalan her şey çalışır.

## Açık soru 2 — Ubuntu ve Windows runner'ları ayrı makinedeyse (SW-3.2, 3.6, 4.2)

- Varsayım: her makinede kendi divan sunucusu çalışır (gereksinimdeki "aynı makine" kararı).
- Tek merkezi sunucuya geçmek kod değişikliği gerektirmez: CI değişkeni `ULAK_SERVER` merkezi
  adrese çevrilir, sunucu `BIND_ADDR=0.0.0.0:8080` ile dinler. Geçmiş ve flaky tespiti zaten
  proje + branch bazındadır; işletim sistemi koşumda `os` alanı olarak tutulur.

## Açık soru 3 — Triage (SW-4.5)

- Kategoriler: `flaky`, `test_kodu`, `urun`, `ortam`, `bilinmiyor` (gereksinimdeki liste).
- Kural kararı koşum bitince kendiliğinden verilir (flaky geçmişi → ortam kalıbı → broken/failed).
- OpenCode kararı **güven ≥ 0,70 ise** (`TRIAGE_AUTO_CONFIDENCE`) admin onayı beklemeden geçerli
  olur; altındaysa yalnızca geçmişte öneri olarak görünür, geçerli karar kural kararı kalır.
- Admin kararı her zaman en üsttedir; hiçbir karar silinmez, senaryo sayfasında "Karar geçmişi".

## Diğer kararlar

- **features.txt (SW-1.5):** proje kökünde, her satır bir Cucumber tag ifadesi; satırlar `or` ile
  birleşir ve `-Dcucumber.filter.tags` olarak verilir. `ULAK_TAGS` CI değişkeni bunu ezer.
  Ön kontrol her tag'in en az bir senaryoda geçtiğini doğrular.
- **Toplam senaryo sayısı (SW-3.1):** JVM açan Cucumber dry-run yerine `ulak` .feature dosyalarını
  kendisi sayar (Senaryo taslağı örnek satırları dahil, tag süzgeciyle). Bellek ve süre kazancı.
- **ulak alt komutları:** `start` koşumu açar, izleyiciyi (`watch`) **ayrı süreç** olarak
  başlatır ve testleri koşar; böylece CI şablonu yalnızca `ulak` çağırır ve izleyici çökerse
  test etkilenmez.
- **CI şablonunda işletim sistemine özgü tek kısım** `before_script` içinde ulak'ın indirilmesidir
  (Linux `curl`, Windows `curl.exe`) — script satırları her iki sistemde aynıdır.
- **Video (SW-1.2, 1.6, 1.7):** test projesinin hook'unda senaryo başına ffmpeg ile kaydedilir
  (5 fps, H.264 CRF 35), yalnızca kalan senaryoda Allure eki olarak saklanır.
- **Jira otomatik bug (SW-6.3):** varsayılan gözlem modu; yalnızca `urun` (ya da triage'sız
  `failed`) ve en az iki deneme (retry) sonrasında; aynı hata parmak izi (proje + ilk satır,
  sayılar normalize) için tek bug; koşum başına `JIRA_MAX_BUGS_PER_RUN`.
- **E-posta (SW-5.3):** yalnızca `schedule` kaynaklı pipeline'ın webhook'u `success`/`failed`
  geldiğinde; SMTP düz relay (kimlik doğrulama/TLS yok). `DRY_RUN=true` ya da `SMTP_HOST` yoksa
  `DATA_DIR/outbox/` altına yazılır.
- **CSV ayırıcı (SW-4.8):** varsayılan virgül; Türkçe Excel için noktalı virgül seçeneği var.
- **Clippy:** `clippy.toml` içinde `type-complexity-threshold` yükseltildi (SQL satırı tuple'ları).

## Ad değişikliği ve ajan seçimi (2. tur)

- Sunucunun adı `stajproje` yerine **`raporcu`** oldu (crate, ikili `raporcu`/`raporcu.exe`,
  veritabanı `DATA_DIR/raporcu.db`, log dosyaları `raporcu.*.log`, systemd `raporcu.service`).
  CI aracının adı (`stajci`) ve `STAJ_*` CI değişkenleri gereksinim belgesindeki gibi kaldı.
- **Son adlar (3. tur, kullanıcı kararı):** sunucu `raporcu` → **`divan`**, CI aracı `stajci` →
  **`ulak`**, `STAJ_*` → `ULAK_*`, `.stajci/` → `.ulak/`, `x-stajci-version` → `x-ulak-version`.
  Sonuçları veritabanına işleyen `ingest` modülünün adı **`defterdar`** oldu; API'deki
  `IngestSummary` ile `ingest_result` gibi adlar kaldı. Kurulu pilot olmadığı için eski adlarla
  uyumluluk tutulmadı. Örnek Jira proje anahtarı `STAJ` ve `STAJ-42` gibi test verileri ürün adı
  değildir, kaldı. GitHub deposu ve yerel klasör şimdilik `raporcu` adındadır.
- Self-healing ajanı `ULAK_FIX_AGENT` ile seçilir: `opencode` (varsayılan) ya da `claude`
  (Claude Code). İkisi de aynı istemi, ekran görüntüsünü ve sahne ağacını alır; ulak'ın yol
  denetimi ve assertion uyarısı ajandan bağımsızdır.
- Kalan senaryoda ekran görüntüsüne ek olarak **TestFX sahne ağacı** (metin) alınır; ajan
  seçicileri gerçek `#fxId`'lerden seçer.
- `rust-embed` debug derlemede de statik dosyaları gömer (`debug-embed`); Wine altında Windows
  testinde bulunan bir sorun buydu.
- **HTTP yeterli (kullanıcı kararı):** divan iç ağda düz HTTP ile çalışır, TLS/sertifika
  kurulmaz. CI işleri `http://127.0.0.1:8080`, tarayıcılar `http://<makine>:8080` kullanır; canlı
  ekran `ws://` ile bağlanır. Tokenlar yine `.env`'de durur ve loglanmaz. ulak'ın `fix`
  komutundaki GitLab push/MR istekleri GitLab'ın kendi adresine (`CI_SERVER_URL`, https olabilir)
  gider; bu divan'dan bağımsızdır.
- **Jira bug içeriği ve DOORS eşleşmesi arayüzden:** bağlantı bilgileri `.env`'de kalır; bug
  şablonu (yer tutuculu özet/açıklama, ek alanlar JSON) ve DOORS öznitelikleri `settings`
  tablosunda, Admin > Ayarlar'dan değişir (`docs/JIRA_DOORS_AYARLARI.md`). Zorunlu alanlar Jira'nın
  createmeta ucundan okunur; şirketten şirkete değişen alanlar için kod değişikliği gerekmez.
- **DOORS bağı tag ile:** `@DOORS-<no>`, `<no>` = DOORS kimlik özniteliğinin değeri. Önek sabit
  (`DOORS-`); kimlik özniteliği ayarlanabilir.
- **Self-healing ajanı ve skill'ler:** OpenCode ajanı `.opencode/agents/fix.md`; skill'ler
  `.claude/skills/` (OpenCode ve Claude Code ikisi de okur, tek kopya). OpenCode runner'da çalışır,
  divan sunucusunda değil; divan'daki düğmeler GitLab API ile fix işini başlatır. Senaryo
  düğmesi yalnızca o senaryoyu, triage hedefinden bağımsız düzeltir.
- **Xvfb ekran numarası:** `xvfb-run -a` aramayı süreç numarasına göre farklı yerden başlatır
  (`-n 99+pid%400`); aynı anda başlayan iki işin aynı ekranı alması (provada görüldü) önlenir.
- **DOORS aktarımı CSV:** kullanıcının kendi aktarım betiği var; DXL betiği öncelik değil,
  `DOORS_DXL=false` varsayılanıyla kapalı (kod duruyor, `true` ile açılır).
- **Self-healing ajanı komutla çağrılır:** `ulak fix` runner'da `opencode run …` ya da `claude -p …`
  komutunu çalıştırır (`ULAK_FIX_CMD` ile tamamen değiştirilebilir); dış servise erişim gerekmez.
- **Türkçe argümanlar ve dil ayarı:** GitLab runner servisi POSIX (ASCII) dil ayarıyla çalışınca Java
  komut satırındaki Türkçe karakterleri bozuyor (`-Dcucumber.filter.name=^Geçerli…$` hiç eşleşmiyor;
  gerçek Maven ile görüldü). ulak Maven'a `LC_ALL=C.UTF-8` verir; fix doğrulamasında ad filtresi
  ayrıca ASCII'ye indirilir (`ş` → `.`).
- **Ürün hatası için taslak:** gözlem modunda divan Jira'ya yazmadan bug taslağı hazırlar, admin tek
  tıkla açar (kullanıcı isteği). OpenCode "ürün hatası" dediğinde de taslak hazırlanır.
- **auto-fix döngüsü:** auto-fix branch push'u varsayılan olarak tam e2e koşturmaz (fix işi tek senaryoyu
  zaten doğruladı); fix işi auto-fix branch'inde hiç koşmaz; açık MR'ı olan senaryo yeniden denenmez.
  `workflow:` kuralı yerine iş kuralları kullanıldı: test projesinin kendi işleri etkilenmesin.
