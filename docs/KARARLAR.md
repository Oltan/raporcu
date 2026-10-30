# Uygulama kararları ve varsayımlar

`GEREKSINIMLER_1.md` içindeki açık sorular yanıtlanmadan ilgili gereksinimler istenince aşağıdaki
varsayımlarla uygulandı. Hepsi yapılandırmayla değiştirilebilir; yanıt gelince gözden geçirilmeli.

## Açık soru 1 — GitLab token yetkisi (SW-7.4, 7.5)

- Varsayım: raporcu'ye `api` kapsamlı bir token verilebilir (`GITLAB_TOKEN`). "OpenCode ile
  düzelt" düğmesi `POST /projects/:id/pipeline` ile `STAJ_FIX_ONLY=true` ve `STAJ_RUN_ID`
  değişkenli pipeline başlatır; şablonda test ve report işleri bu durumda koşmaz.
- SW-7.5 (birleştirilen/reddedilen) GitLab API sorgusu yerine **Merge request webhook**'uyla
  çalışır; bu yüzden okuma yetkisi gerekmez.
- Token verilmezse düğme "GitLab bağlantısı ayarlı değil" mesajı verir; geri kalan her şey çalışır.

## Açık soru 2 — Ubuntu ve Windows runner'ları ayrı makinedeyse (SW-3.2, 3.6, 4.2)

- Varsayım: her makinede kendi raporcu sunucusu çalışır (gereksinimdeki "aynı makine" kararı).
- Tek merkezi sunucuya geçmek kod değişikliği gerektirmez: CI değişkeni `STAJ_SERVER` merkezi
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
  birleşir ve `-Dcucumber.filter.tags` olarak verilir. `STAJ_TAGS` CI değişkeni bunu ezer.
  Ön kontrol her tag'in en az bir senaryoda geçtiğini doğrular.
- **Toplam senaryo sayısı (SW-3.1):** JVM açan Cucumber dry-run yerine `stajci` .feature dosyalarını
  kendisi sayar (Senaryo taslağı örnek satırları dahil, tag süzgeciyle). Bellek ve süre kazancı.
- **stajci alt komutları:** `start` koşumu açar, izleyiciyi (`watch`) **ayrı süreç** olarak
  başlatır ve testleri koşar; böylece CI şablonu yalnızca `stajci` çağırır ve izleyici çökerse
  test etkilenmez.
- **CI şablonunda işletim sistemine özgü tek kısım** `before_script` içinde stajci'nin indirilmesidir
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
- Self-healing ajanı `STAJ_FIX_AGENT` ile seçilir: `opencode` (varsayılan) ya da `claude`
  (Claude Code). İkisi de aynı istemi, ekran görüntüsünü ve sahne ağacını alır; stajci'nin yol
  denetimi ve assertion uyarısı ajandan bağımsızdır.
- Kalan senaryoda ekran görüntüsüne ek olarak **TestFX sahne ağacı** (metin) alınır; ajan
  seçicileri gerçek `#fxId`'lerden seçer.
- `rust-embed` debug derlemede de statik dosyaları gömer (`debug-embed`); Wine altında Windows
  testinde bulunan bir sorun buydu.
- **HTTP yeterli (kullanıcı kararı):** raporcu iç ağda düz HTTP ile çalışır, TLS/sertifika
  kurulmaz. CI işleri `http://127.0.0.1:8080`, tarayıcılar `http://<makine>:8080` kullanır; canlı
  ekran `ws://` ile bağlanır. Tokenlar yine `.env`'de durur ve loglanmaz. stajci'nin `fix`
  komutundaki GitLab push/MR istekleri GitLab'ın kendi adresine (`CI_SERVER_URL`, https olabilir)
  gider; bu raporcu'dan bağımsızdır.
