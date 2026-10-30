# raporcu Gereksinimleri

## Bağlam

raporcu sıfırdan yazılacak bir "test sonuç merkezi". Testleri raporcu koşmaz, GitLab runner koşar. raporcu sonuçları alır; canlı durum, rapor, paylaşım, triage, Jira/DOORS ve self-healing sağlar.

- **raporcu**: bu raporlama projesi. Henüz hiçbir kodu yok.
- **test projesi**: JavaFX uygulamasının Cucumber + TestFX test reposu (GitLab). Testler Allure raporu üretir.
- **Teknoloji**: Rust. Sunucu ve CI araçları Rust ile yazılır; bellek kullanımı en düşük seçenek olduğu ve dış çalışma ortamı gerektirmediği için seçildi.
  - Sunucu: `axum` + `tokio`; sayfalar `askama` şablonları + htmx; canlı ekran için axum WebSocket. Şablonlar derlemede ikiliye girer, statik dosyalar `rust-embed` ile gömülür.
  - Veritabanı: SQLite, `rusqlite` (`bundled` özelliğiyle; SQLite ikilinin içine derlenir). Ayrı veritabanı sunucusu kurulmaz (SW-2.5).
  - Dosyalar (ekran görüntüsü, video, Allure dosyaları) diskte `DATA_DIR` altında; veritabanında yalnızca yolları tutulur.
  - CI araçları tek bir yardımcı ikili: `stajci` (alt komutlar: `preflight`, `start`, `watch`, `upload`, `finish`, `fix`).
  - Derleme: Linux için `x86_64-unknown-linux-gnu`, Windows için `x86_64-pc-windows-gnu` (Linux'ta mingw ile) ya da Windows'ta `x86_64-pc-windows-msvc`.
- **İşletim sistemi**: Runner'lar hem Ubuntu hem Windows sanal makinelerde çalışır. raporcu sunucusu runner ile aynı makinede çalışır; her iki işletim sisteminde de çalışabilmelidir.

Kısaltmalar:

- `SYS-n`: sistem gereksinimi (ne yapılacak)
- `SW-n.m`: yazılım gereksinimi (nasıl yapılacak)
- Durum: `YAPILACAK`, `KISMEN` (başlandı, eksik kısmı var), `MEVCUT` (bitti, testi geçiyor)
- Test: B birim, E entegrasyon, P prova (yerelde CI benzetimi), U uçtan uca, M manuel

Toplam 58 yazılım gereksinimi var: 34 MEVCUT, 23 KISMEN, 1 YAPILACAK. KISMEN olanların eksik kısmı (GitLab, Windows VM, gerçek Jira/OpenCode/SMTP) `docs/PILOT_KONTROL_LISTESI.md` içindedir; açık sorulardaki varsayımlar `docs/KARARLAR.md` içindedir.

## Çalışma kuralları

- Bir seferde yalnızca bir SW gereksinimi üzerinde çalış. "Uygulama sırası" bölümündeki sırayı izle.
- Her gereksinim kendi testiyle kapanır. Test adında gereksinim ID'si geçer, örneğin `test_sw_3_3_duplicate_result_is_counted_once`.
- Bir gereksinim bitince bu dosyadaki durumunu güncelle (`YAPILACAK` → `MEVCUT`).
- Tüm kod Rust ile yazılır ve Ubuntu ile Windows'ta değişiklik yapılmadan çalışır: yollar `std::path::PathBuf` ile kurulur, kabuğa özgü komutlar (bash, PowerShell) kullanılmaz; CI şablonu yalnızca `stajci` alt komutlarını çağırır. İşletim sistemine özgü tek kısım ekran ve video ayarıdır (SW-1.2, SW-1.6).
- CI ile ilgili her gereksinim iki işletim sisteminde de doğrulanmadan MEVCUT sayılmaz.

## Doğrulama

- Birim: `cargo clippy --all-targets -- -D warnings` ve `cargo test` sıfır hatayla geçmeli.
- Derleme: `cargo build --release` Linux ve Windows hedefleri için geçmeli.
- Prova: `cargo run -p rehearse` (SW-10.2). Bu betik CI akışını yerelde benzetir; her yeni CI adımı buna eklenir.
- Uçtan uca: `cargo run -p e2esmoke` (SW-10.2). GitLab tarafı pilot projede manuel kontrol listesiyle doğrulanır.

---

## SYS-1: Test koşumu GitLab CI'da yapılır

Testler raporcu'de değil, test projesinin GitLab pipeline'ında, Ubuntu veya Windows sanal makinedeki runner'da koşar. Test projesi hangi işletim sisteminde koşacağını runner etiketiyle (`linux` / `windows`) seçer.

### SW-1.1 [KISMEN] CI şablonu test → report → fix aşamalarını içerir
- Dosya: `ci/gitlab/template/.gitlab-ci.yml`
- Şablon iki işletim sistemini destekler; test projesi bir değişkenle (`linux` veya `windows`) seçer, işler ilgili runner etiketine gider.
- Kabul: Pipeline 3 iş çalıştırır; test işi düşse de report ve fix çalışır. Bu, hem Ubuntu hem Windows runner'da geçerlidir.
- Test: P, U
- Doğrulama: rehearse: SW-1.1 adımları; `test_sw_1_1_split_command_without_shell`
- Not: Şablon hazır; `rehearse` şablonu ve 3 iş akışını Linux'ta doğrular. GitLab + Windows runner'da pilot doğrulaması bekliyor.

### SW-1.2 [KISMEN] Testler Ubuntu sanal makinede Xvfb sanal ekranında koşar
- Headless çalışma ve Monocle kullanılmaz; uygulama Xvfb'nin sağladığı X11 ekranında gerçekten açılır.
- Test işi Xvfb'yi sabit çözünürlükle başlatır (örneğin `xvfb-run -a -s "-screen 0 1920x1080x24"`), testler bitince kapatır. Her iş kendi ekranını alır.
- GitLab runner sistem hizmeti olarak çalışabilir; oturum açma, ekran kilidi veya uyku ayarı gerekmez.
- Wayland kullanılmaz; JavaFX ve TestFX robotu X11 üzerinde çalışır.
- Video kaydı aynı ekrandan alınır (örneğin `ffmpeg -f x11grab`).
- Kabul: Kimse bağlı değilken gece koşumu geçer; ekran görüntüleri ve video uygulamanın gerçek görüntüsünü gösterir; aynı makinede iki iş aynı anda koşarsa ekranları karışmaz.
- Test: P, U
- Doğrulama: rehearse: SW-1.2 adımı
- Not: `stajci start` testleri `xvfb-run -a` ile koşar; gerçek JavaFX/TestFX koşusu Xvfb'de doğrulandı, `rehearse` iki eşzamanlı işin farklı ekran aldığını doğrular. GitLab gece koşumu pilotta.

### SW-1.3 [KISMEN] Gece zamanlaması GitLab schedule ile çalışır
- Kabul: Pipeline 02:00'de kendiliğinden başlar, rapor sabah raporcu'de hazır olur.
- Test: U, M
- Not: Şablon schedule pipeline'ını destekler, e-posta schedule kaynağında gider. Schedule GitLab'da kurulmalı (docs/GITLAB_CI_KURULUMU.md §5).

### SW-1.4 [KISMEN] Merkezi şablon
- Test projeleri yalnızca bir `include` satırı ekler; betikler raporcu reposundan çekilir.
- Kabul: Merkezdeki bir betik değişince ertesi koşumda tüm projeler yeni sürümü kullanır.
- Test: U
- Not: `include` ile tek satır; stajci her koşumda sunucudan indirilir. GitLab'da U testi bekliyor.

### SW-1.5 [KISMEN] Ön kontrol CI içinde testlerden önce çalışır
- Kontrol edilenler: pom, Allure eklentisi, features.txt tag'leri.
- Kabul: Adaptör eksikse iş açık bir mesajla düşer, testler hiç başlamaz.
- Test: B, P
- Doğrulama: `test_sw_1_5_*` (stajci), rehearse SW-1.5
- Not: `stajci preflight`: pom, Allure adaptörü/eklentisi, Monocle, features.txt. Birim + prova Linux'ta geçer; Windows doğrulaması bekliyor.

### SW-1.6 [KISMEN] Testler Windows sanal makinede, açık bir masaüstü oturumunda koşar
- Headless çalışma ve Monocle kullanılmaz; uygulama sanal makinenin sanal ekranında gerçekten açılır.
- GitLab runner Windows hizmeti olarak değil, oturum açmış kullanıcının masaüstünde çalışır (TestFX fare ve klavye olaylarını bu oturuma gönderir).
- Sanal makine yeniden başlayınca otomatik oturum açar ve runner'ı başlatır; ekran kilidi, ekran koruyucu ve uyku kapalıdır; ekran çözünürlüğü sabittir.
- Uzak Masaüstü (RDP) bağlantısı kesilince oturum kilitlenir; bağlantıdan çıkarken oturum `tscon` ile konsola geri verilir.
- Video kaydı masaüstünden alınır (örneğin `ffmpeg -f gdigrab`).
- Kabul: Kimse bağlı değilken gece koşumu geçer; ekran görüntüleri ve video uygulamanın gerçek görüntüsünü gösterir.
- Test: P, U, M
- Doğrulama: rehearse SW-1.6 (benzetim)
- Not: Oturum kontrolü (`stajci start`), gdigrab videosu, kurulum betiği ve `tscon` betiği hazır; Windows VM'de doğrulanmadı.

### SW-1.7 [KISMEN] Test işinin bellek sınırları
- CI şablonu Maven ve test JVM'leri için bellek üst sınırı koyar: `MAVEN_OPTS` ve surefire `argLine` içinde `-Xmx`; surefire tek fork ile çalışır.
- Runner aynı anda tek iş koşar (`concurrent = 1`).
- Video kaydı düşük kare hızı ve sıkıştırılmış biçimle alınır; yalnızca başarısız senaryoların videosu saklanır.
- Sınır değerleri CI değişkeniyle değiştirilebilir.
- Kabul: Gece koşumu boyunca makinede bellek yetersizliği (OOM) olmaz; ayarlanan sınır aşılınca test işi açık bir mesajla düşer.
- Test: P, U
- Doğrulama: `test_sw_1_7_*`, rehearse SW-1.7
- Not: MAVEN_OPTS/-Dtest.xmx, tek fork, OOM tespiti ve açık mesaj; düşük fps video yalnızca kalanlarda. Linux prova geçer; Windows ve gerçek gece koşumu bekliyor.

---

## SYS-2: Sonuçlar alınır ve saklanır

CI'dan gelen Allure sonuçları alınır, işlenir ve saklanır.

### SW-2.1 [MEVCUT] POST /api/ci/runs
- Allure sonuç zip'ini alır, ayrıştırır ve bir koşum olarak kaydeder.
- Kabul: Koşum listede görünür; rapor, triage ve CSV dışa aktarma bu koşum için çalışır; zip güvenliği sağlanır (boyut sınırı, `..` içeren yollar reddedilir).
- Test: B
- Doğrulama: `test_sw_2_1_*` (common, raporcu, stajci), e2esmoke

### SW-2.2 [MEVCUT] @After hook ekleri (container dosyası) senaryoya bağlanır
- Ekran görüntüsü yalnızca başarısız senaryoda alınır; başarılı senaryolarda görüntü yoktur.
- Kabul: Başarısız senaryonun ekran görüntüsü raporda görünür; başarılı senaryoda ek yoktur.
- Test: B, U
- Doğrulama: `test_sw_2_2_after_hook_screenshot_linked_to_failed_scenario_only`, `test_sw_2_2_container_before_result_still_links`

### SW-2.3 [MEVCUT] Sonuç dosyası olmayan yükleme reddedilir
- Kabul: 422 döner, "Allure eklentisi" eksikliğini anlatan açıklamayla.
- Test: B
- Doğrulama: `test_sw_2_3_upload_without_results_returns_422`

### SW-2.4 [MEVCUT] Tüm çalışma verisi DATA_DIR altında tutulur
- Yollar tek bir modülde (`paths.rs`) tanımlanır.
- Kabul: DATA_DIR değiştirilince tüm veri yeni yere yazılır; testler geçici bir DATA_DIR kullanır, gerçek veriye yazmaz.
- Test: B
- Doğrulama: `test_sw_2_4_all_paths_under_data_dir`

### SW-2.5 [MEVCUT] Veritabanı: kurulumsuz SQLite
- Veritabanı `DATA_DIR/raporcu.db` dosyasıdır. Sunucu ilk açılışta dosyayı kendisi oluşturur; ayrı kurulum, kullanıcı veya şifre yoktur.
- Şema değişiklikleri numaralı migration dosyalarıyla yapılır (`migrations/0001_ilk.sql`, ...). Migration'lar ikiliye gömülür ve sunucu açılırken sırayla uygulanır; uygulananlar veritabanında kayıtlıdır.
- Açılışta ayarlar: `journal_mode=WAL` (okuma ve yazma birbirini beklemez), `busy_timeout`, `foreign_keys=ON`.
- Yazmalar tek bir yazıcı bağlantısından, okumalar küçük bir bağlantı havuzundan yapılır.
- Yedek: Her gece çalışan sunucu durdurulmadan `VACUUM INTO` ile `DATA_DIR/backup/` altına kopya alınır; son 7 yedek tutulur.
- Ana tablolar: `runs`, `scenarios`, `steps`, `attachments`, `bug_links`, `share_links`, `fix_attempts`, `users`.
- Kabul: Boş bir DATA_DIR ile açılan sunucu veritabanını oluşturur; yeni bir migration eklenince bir sonraki açılışta uygulanır; yedek dosyası açılıp okunabilir; testler geçici bir veritabanı kullanır.
- Test: B
- Doğrulama: `test_sw_2_5_*`, e2esmoke yedek adımı

### SW-2.6 [KISMEN] Sunucuya ulaşılamazsa sonuçlar kaybolmaz
- `stajci` gönderemediği sonuçları ve ekleri iş dizinindeki bir kuyruk klasörüne yazar ve artan aralıklarla yeniden dener.
- İş bitiminde hâlâ gönderilemeyen sonuçlar GitLab artifact'i olarak saklanır; sunucu geri gelince `stajci upload --resume` ile gönderilir.
- Kabul: Sunucu koşum sırasında kapatılıp açılınca hiçbir sonuç eksik ya da çift sayılmaz (SW-3.3 ile).
- Test: B, P
- Doğrulama: rehearse SW-2.6 adımları
- Not: Kuyruk, artan aralıklı deneme, artifact ve `--resume`; sunucu kapatılıp açılınca eksik/çift yok (prova, Linux). Windows doğrulaması bekliyor.

### SW-2.7 [KISMEN] Gerçek Allure örnekleri test verisi olarak kullanılır
- Test projesi Allure ile ayarlandıktan sonra, ondan alınan `allure-results` örnekleri `testdata/` altına konur: geçen, kalan, broken, rerun edilmiş ve ekli senaryolar.
- Ayrıştırıcı testleri uydurma değil bu örneklerle yazılır.
- Kabul: SW-2.1, 2.2 ve 4.9 testleri gerçek örneklerle geçer.
- Test: B
- Doğrulama: `test_sw_2_7_real_samples_parse`, `test_sw_2_7_doors_and_jira_tags`
- Not: Örnekler `ornek-test-projesi`nin gerçek koşusundan (testdata/allure/ornek-giris). Gerçek test projesinin örnekleri eklenince MEVCUT.

---

## SYS-3: Canlı test durumu

Testler koşarken hangi senaryonun geçtiği, hangisinin kaldığı anlık görünür.

### SW-3.1 [MEVCUT] POST /api/ci/runs/start
- Koşum "çalışıyor" durumunda açılır; toplam senaryo sayısı (dry-run ile) kaydedilir.
- Kabul: Koşum panelde, testler bitmeden "47/120" gibi bir ilerlemeyle görünür.
- Test: B, P
- Doğrulama: `test_sw_3_1_*` (stajci), rehearse SW-3.1

### SW-3.2 [KISMEN] CI izleyicisi (`stajci watch`)
- Her yeni sonuç dosyasını ve eklerini anında gönderir; test işiyle paralel çalışır.
- raporcu runner ile aynı makinede çalıştığı için izleyici sonuçları yerel adrese (localhost) gönderir.
- Kabul: Senaryo bittikten en geç 5 sn sonra panelde görünür; izleyici çökerse test işi etkilenmez.
- Test: P
- Doğrulama: rehearse SW-3.2
- Not: Ayrı süreç izleyici, ≤5 sn gecikme provada ölçülür (Linux). Windows doğrulaması bekliyor.

### SW-3.3 [MEVCUT] POST /api/ci/runs/{id}/results
- Sonuçlar artımlı alınır; aynı dosya iki kez sayılmaz.
- Kabul: Aynı sonucu iki kez göndermek sayıyı değiştirmez; sonuçlar son toplu zip (SW-2.1) ile tutarlıdır.
- Test: B
- Doğrulama: `test_sw_3_3_duplicate_result_is_counted_once`, rehearse SW-3.3

### SW-3.4 [MEVCUT] Canlı koşum ekranı
- Senaryo listesi (geçti, kaldı, koşuyor, bekliyor), sayaçlar, hata ve ekran görüntüsü; güncellemeler WebSocket ile gelir.
- Kabul: Sayfa yenilenmeden güncellenir; hata, koşum bitmeden görünür.
- Test: B, P, M
- Doğrulama: `test_sw_3_4_live_snapshot_waiting_running_and_errors`, `test_sw_3_4_websocket_pushes_updates_without_reload`
- Not: WebSocket testi ve ekran görüntüsüyle kontrol edildi.

### SW-3.5 [MEVCUT] POST /api/ci/runs/{id}/finish ve zaman aşımı
- Yarıda kesilen koşum "interrupted" olur.
- Kabul: Runner durdurulunca koşum RUN_STALL_TIMEOUT içinde interrupted durumuna geçer.
- Test: B
- Doğrulama: `test_sw_3_5_*`

### SW-3.6 [KISMEN] Pipeline düzeyi durum (kuyrukta, çalışıyor, bitti)
- GitLab webhook ile alınır. GitLab sunucusu makineye erişebildiği için API sorgusuyla yedek yol gerekmez.
- Kabul: Pipeline başlayınca panelde "kuyrukta / çalışıyor" görünür.
- Test: B, U
- Doğrulama: `test_sw_3_6_pipeline_status_on_dashboard`, e2esmoke
- Not: Webhook işleme ve panel testleri geçer; gerçek GitLab webhook'u ile U testi bekliyor.

---

## SYS-4: Raporlama

Koşum sonuçları ayrıntılı olarak ve geçmişiyle birlikte görüntülenir.

### SW-4.1 [MEVCUT] Koşum raporu, senaryo detayı, adımlar, ekran görüntüsü ve video gösterimi
- Kabul: Bir koşumdan senaryoya, oradan adımlara ve eklere gidilebilir; ekran görüntüsü ve video sayfada açılır.
- Test: B, U
- Doğrulama: `test_sw_4_1_run_to_scenario_to_steps_and_attachments`, e2esmoke

### SW-4.2 [MEVCUT] Dashboard, senaryo geçmişi, flaky tespiti
- Kabul: Dashboard son koşumların özetini gösterir; bir senaryonun geçmiş koşumlardaki sonuçları listelenir; geçip kalmayı değişken olan senaryo flaky olarak işaretlenir.
- Test: B
- Doğrulama: `test_sw_4_2_dashboard_history_and_flaky_detection`

### SW-4.3 [MEVCUT] CI bilgisi raporda: repo, branch, commit ve pipeline linki
- Kabul: Rapor başlığında bu bilgiler görünür; pipeline linki tıklanabilir.
- Test: B
- Doğrulama: `test_sw_4_3_*`

### SW-4.4 [MEVCUT] OpenCode düzeltme kutusu ve senaryo rozeti
- SW-7.3'e bağlıdır.
- Kabul: fix-result geldikten sonra raporda düzeltme kutusu ve senaryoda "OpenCode !MR" rozeti görünür.
- Test: B, P
- Doğrulama: `test_sw_7_3_and_4_4_fix_result_box_and_badge_in_report`, rehearse SW-4.4

### SW-4.5 [KISMEN] Triage: başarısız senaryonun nedenine karar verilmesi (taslak)
- Kategoriler: `flaky`, `test kodu hatası` (bizden), `ürün hatası`, `ortam sorunu`, `bilinmiyor`.
- Karar üç kaynaktan gelir ve her kararın kaynağı kayıtlıdır:
  1. Kural: flaky geçmişi (SW-4.2), Allure durumu (`broken` → test kodu, `failed` → ürün), bilinen ortam hata kalıpları (zaman aşımı, bağlantı hatası, ekran bulunamadı).
  2. OpenCode: hata mesajı, adımlar ve ekran görüntüsü vLLM'e verilir; kategori, kısa gerekçe ve güven döner. Fix işinde (SW-7.2) çalışır, sonucu raporcu'ye bildirir.
  3. Admin: kararı onaylar ya da değiştirir. Admin kararı diğerlerinin üstündedir.
- Kategori sonraki adımı belirler: test kodu hatası → OpenCode düzeltmesi, ürün hatası → Jira, flaky → atlanır, ortam sorunu → bildirim.
- Ayrıntılar netleşmedi (açık soru 3); bu gereksinime başlamadan önce sor.
- Kabul: Her başarısız senaryonun raporda kategorisi, kaynağı ve gerekçesi görünür; admin değiştirince yeni karar geçerli olur ve eski karar geçmişte kalır.
- Test: B, P
- Doğrulama: `test_sw_4_5_*`
- Not: Açık soru 3 yanıtlanmadan varsayımlarla uygulandı (docs/KARARLAR.md). Testler geçer.

### SW-4.6 [MEVCUT] Proje ve branch ayrımı
- Her koşum bir test projesine ve branch'e bağlıdır. Dashboard, geçmiş ve flaky tespiti proje ve branch bazında yapılır.
- Yeni proje, ilk yüklemede GitLab proje bilgisinden kendiliğinden oluşur.
- Kabul: İki projenin aynı adlı senaryoları birbirinin geçmişine karışmaz; dashboard proje ve branch seçilerek süzülür.
- Test: B
- Doğrulama: `test_sw_4_6_*`

### SW-4.7 [MEVCUT] Arama ve filtre
- Senaryolar tag, feature, DOORS numarası, Jira issue numarası, triage kategorisi ve hata mesajı ile aranır.
- Kabul: Her alanla arama doğru sonuç verir; sonuçlar sayfalanır.
- Test: B
- Doğrulama: `test_sw_4_7_search_by_each_field`, `test_sw_4_7_search_paginated`

### SW-4.8 [MEVCUT] CSV dışa aktarma (alan seçmeli)
- Jira'daki dışa aktarma gibi: kullanıcı o anki filtreyle listelenen senaryoları, istediği alanları seçerek düz CSV olarak indirir.
- Seçilebilir alanlar: proje, branch, koşum, senaryo, feature, tag'ler, durum, triage kategorisi, hata mesajı, süre, DOORS numarası, Jira issue numarası, tarih.
- Kabul: Yalnızca seçilen alanlar seçilen sırayla gelir; Excel'de Türkçe karakterler doğru açılır (UTF-8 BOM); büyük listeler akış halinde yazılır (SW-10.6).
- Test: B
- Doğrulama: `test_sw_4_8_*`

### SW-4.9 [MEVCUT] Aynı koşumda tekrar koşan senaryolar
- Cucumber rerun ya da retry nedeniyle aynı senaryonun birden fazla sonucu olabilir. Senaryonun sonucu son denemedir; önceki denemeler senaryo detayında görünür.
- Önce kalıp sonra geçen senaryo flaky tespitine girdi olarak sayılır.
- Kabul: Sayaçlarda her senaryo bir kez sayılır; denemeler detayda sırayla listelenir.
- Test: B
- Doğrulama: `test_sw_4_9_rerun_last_attempt_wins_and_attempts_listed`

---

## SYS-5: Rapor paylaşımı

Raporlar ekip ve paydaşlarla güvenli biçimde paylaşılır.

### SW-5.1 [MEVCUT] İç bilgileri gizlenmiş salt okunur paylaşım linki
- Link, şirket ağındaki herkesin giriş yapmadan açabileceği bir linktir.
- Kabul: Token, Jira anahtarı ve ham ek linki görünmez.
- Test: B
- Doğrulama: `test_sw_5_1_*`, e2esmoke

### SW-5.2 [MEVCUT] Paylaşım linkinin iptali
- Bağlam: Raporlar yalnızca şirket içindeki kapalı yerel ağda paylaşılır, dışarıya açılmaz. Bu yüzden önceliği düşük.
- Süre sınırı yok: Link, rapor saklandığı sürece (diskte yer olduğu sürece) açık kalır.
- Kabul: İptal edilen link açılmaz; iptal admin panelinden yapılır. Süre sınırı eklenmez.
- Test: B
- Doğrulama: `test_sw_5_2_revoked_share_link_does_not_open`, e2esmoke

### SW-5.3 [KISMEN] Gece pipeline'ı bitince özet e-posta
- İçerik: geçen, kalan, rapor linki, açılan MR'lar.
- Kabul: DRY_RUN'da gönderim simüle edilir; gerçekte pipeline başına tek e-posta gider.
- Test: B, M
- Doğrulama: `test_sw_5_3_*`
- Not: DRY_RUN ve sahte SMTP ile testler geçer; gerçek relay ile M kontrolü bekliyor.

### SW-5.4 [KISMEN] Rapor linki pipeline'da ve MR açıklamasında
- Kabul: Rapor linki yükleme işinin çıktısında, pipeline "environment" linkinde ve OpenCode MR açıklamasında yer alır.
- Test: P, U
- Doğrulama: rehearse SW-5.4 adımları
- Not: Rapor linki stajci çıktısında, stajci.env (environment url) ve MR açıklamasında; GitLab'da U testi bekliyor.

---

## SYS-6: Hata yönetimi: Jira ve DOORS

Başarısız senaryolar Jira bug'larıyla eşleşir, gerektiğinde bug açılır.

### SW-6.1 [MEVCUT] Jira istemcisi: bug'da DOORS numarası label'da ve açıklamada
- Jira label'ı reddederse bug label'sız açılır.
- Bug açılırken ya da senaryo var olan bir bug'a bağlanırken ekran görüntüsü Jira issue'suna eklenir.
- Kabul: DOORS numarası her iki yerde de görünür; label hatasında bug yine de açılır; ekran görüntüsü Jira issue'sunda görünür.
- Test: B
- Doğrulama: `test_sw_6_1_*`
- Not: Sahte Jira sunucusuyla test edildi.
- Ek: bug içeriği arayüzden ayarlanır (Admin > Ayarlar: özet/açıklama şablonu, zorunlu ve özel alanlar JSON); "Jira alanlarını getir" zorunlu alanları ve izinli değerleri Jira createmeta ile listeler; elle bug açma formu zorunlu alanları sorar; Jira alan hataları alan adıyla gösterilir. Doğrulama: `test_sw_6_1_required_fields_listed_and_bug_form_sends_them`, `test_sw_6_3_auto_bug_uses_template_fields`, `docs/JIRA_DOORS_AYARLARI.md`.

### SW-6.2 [KISMEN] CI koşumundan sonra FAILED senaryolar için otomatik eşleşme kendiliğinden çalışır
- Kabul: Yükleme sonrasında açık bug'ı olan senaryolar bağlanmış görünür.
- Test: B, E
- Doğrulama: `test_sw_6_2_auto_match_after_upload`
- Not: Sahte Jira ile B/E testleri geçer; gerçek Jira ile E testi bekliyor.

### SW-6.3 [MEVCUT] Sınırlı otomatik bug açma
- Koşum başına üst sınır vardır; aynı hata için tek bug açılır; bug retry sonrasında açılır; önce gözlem modunda çalışır.
- Kabul: Gözlem modunda Jira'ya yazılmaz, kayıt tutulur; üst sınır aşılmaz.
- Test: B
- Doğrulama: `test_sw_6_3_*`
- Ek: gözlem modunda Jira bug **taslağı** hazırlanır (şablonla), admin koşum/senaryo/admin sayfasından tek tıkla açar, düzenleyip açar ya da yok sayar; OpenCode "ürün hatası" derse de taslak; aynı senaryo/hata için taslak birikmez. Doğrulama: `test_sw_6_3_draft_*`.

### SW-6.4 [MEVCUT] DOORS dışa aktarma: CSV ve DXL
- Kabul: Koşum sonuçları DOORS'a aktarılabilecek CSV ve DXL dosyası olarak indirilir.
- Test: B
- Doğrulama: `test_sw_6_4_doors_csv_and_dxl`, e2esmoke
- Not: DXL betiği gerçek DOORS modülünde denenmedi (pilot listesi).
- Ek: aktarım CSV ile (kullanıcının kendi aktarım betiği); sonuç değerleri ve DOORS linki şablonu Admin > Ayarlar > DOORS'tan. DXL isteğe bağlı, varsayılan kapalı (`DOORS_DXL=true`). Doğrulama: `test_sw_6_4_doors_settings_used_in_dxl_csv_and_links`.

---

## SYS-7: Self-healing (OpenCode)

Test kodu yüzünden düşen senaryolar OpenCode ile düzeltilir ve insan onayıyla birleştirilir.

### SW-7.1 [MEVCUT] fix-candidates sınıflandırması
- Hedef triage kategorisine göre seçilir (SW-4.5): test kodu hatası → OpenCode, ürün hatası → Jira, @sample-fail ve flaky → atlanır. Triage kararı yoksa Allure durumu kullanılır (broken → OpenCode, failed → Jira).
- Kabul: Her durum için doğru hedef seçilir.
- Test: B
- Doğrulama: `test_sw_7_1_*`

### SW-7.2 [KISMEN] OpenCode düzeltmesi (`stajci fix`)
- Yalnızca `src/test/` altında değişiklik yapar, doğrulama koşusu çalıştırır, auto-fix branch'i ve MR açar (doğrulama geçmezse Draft MR).
- vLLM modeli görsel destekli: başarısız senaryonun ekran görüntüsü OpenCode isteğine eklenir.
- Kabul: Önce provada sahte OpenCode ile, sonra gerçek OpenCode, vLLM, TestFX MCP ve GitLab MR ile geçer.
- Test: P, U
- Doğrulama: rehearse SW-7.2 adımları, `test_sw_7_2_command_template`
- Not: Sahte OpenCode + sahte GitLab + yerel git ile prova geçer; gerçek OpenCode, vLLM, TestFX MCP, GitLab MR ile U bekliyor.
- Ek: açık auto-fix MR'ı olan senaryo yeniden aday olmaz (`acik-mr`); CI şablonunda auto-fix branch'inde fix işi koşmaz, test/report yalnızca `STAJ_FIX_PIPELINE=on` ile; MR pipeline'ında şablon işleri koşmaz. Doğrulama: `test_sw_7_2_open_fix_mr_blocks_new_candidate_until_closed`, rehearse SW-7.2.

### SW-7.3 [MEVCUT] Sonuç raporcu'ye bildirilir (fix-result) ve raporda görünür
- Test: B, P
- Doğrulama: `test_sw_7_3_and_4_4_*`, rehearse SW-7.3

### SW-7.4 [KISMEN] Rapordaki "OpenCode ile düzelt" düğmesi GitLab API ile yalnızca fix işini tetikler
- Kabul: Düğme → pipeline → sonuç aynı rapora yazılır.
- Test: B, U
- Doğrulama: `test_sw_7_4_fix_button_triggers_only_fix_job_via_gitlab_api`
- Not: Sahte GitLab API ile test geçer; açık soru 1 (token) ve gerçek GitLab U testi bekliyor.
- Ek: senaryo sayfasında tek senaryo için düğme (`STAJ_FIX_SCENARIO_ID`, triage hedefinden bağımsız). Doğrulama: `test_sw_7_4_*`, rehearse SW-7.4.

### SW-7.5 [MEVCUT] Self-healing ölçüm paneli: deneme, MR, birleştirilen, reddedilen
- Kabul: Panel son 30 gecenin sayılarını gösterir.
- Test: B
- Doğrulama: `test_sw_7_5_healing_panel_counts_with_mr_webhook`

### SW-7.6 [KISMEN] Test projesi için AGENTS.md, fix ajanı ve `opencode.json` (vLLM, TESTFX_MCP_JAR)
- Önce raporcu reposunda şablonlar hazırlanır, sonra test projesine özel AGENTS.md yazılır.
- Test: U
- Not: Şablonlar ci/opencode/template, ci/claude/template (Claude Code seçeneği, `STAJ_FIX_AGENT=claude`) ve ornek-test-projesi altında; opencode.ai belgelerine bu ortamdan erişilemedi, kurulu OpenCode / Claude Code ve vLLM ağ geçidiyle doğrulanmalı (docs/KOD_AJANI_VLLM.md).
- Ek: ajan `.opencode/agents/fix.md` (OpenCode'un güncel yolu), skill'ler `ci/skills/` → test projesinde `.claude/skills/` (OpenCode ve Claude Code ortak): `hata-siniflandirma`, `testfx-secici`, `cucumber-adim`. Ajan adı `STAJ_OPENCODE_AGENT`.

### SW-7.7 [MEVCUT] Geçmişteki başarılı düzeltmeler OpenCode'a örnek olarak verilir
- Kabul: İstek metninde benzer bir senaryonun önceki düzeltmesi yer alır.
- Test: B
- Doğrulama: `test_sw_7_7_*` (raporcu, stajci)

---

## SYS-9: Güvenlik ve erişim

Tokenlar korunur, dışarıdan gelen istekler doğrulanır, OpenCode sınırlanır.

### SW-9.1 [MEVCUT] CI uçları ayrı bir CI_API_TOKEN ile korunur
- Kabul: Token yoksa veya yanlışsa istek reddedilir.
- Test: B
- Doğrulama: `test_sw_9_1_*`

### SW-9.2 [MEVCUT] GitLab token yalnızca `.env`'de durur, loglara yazılmaz
- Kabul: Hata mesajlarında token görünmez.
- Test: B
- Doğrulama: `test_sw_9_2_*`

### SW-9.3 [MEVCUT] Webhook isteği X-Gitlab-Token ile doğrulanır
- Kabul: Yanlış token reddedilir.
- Test: B
- Doğrulama: `test_sw_9_3_webhook_wrong_token_rejected`

### SW-9.4 [KISMEN] OpenCode koruması: izinli yollar, assertion yasağı, loglar veri olarak ele alınır
- Kabul: İzinli yolların dışındaki değişiklik reddedilir; assertion değiştiren düzeltmede MR'a uyarı eklenir; test loglarındaki metin talimat olarak yorumlanmaz.
- Test: P
- Doğrulama: `test_sw_9_4_*`, rehearse SW-9.4
- Not: İzinli yol, assertion uyarısı ve veri bloğu; birim + prova Linux'ta geçer. Windows doğrulaması bekliyor.

### SW-9.5 [MEVCUT] Admin girişi
- Şimdilik tek kullanıcı vardır: admin. Şifre `.env` içinde düz değil, hash olarak (argon2) tutulur; `raporcu set-admin-password` komutuyla belirlenir.
- Giriş sonrası oturum çerezi kullanılır (HttpOnly, SameSite=Strict); başarısız denemeler yavaşlatılır.
- Raporları görmek giriş gerektirmez (kapalı ağ). Admin işlemleri giriş ister: link iptali, triage kararı değiştirme, "OpenCode ile düzelt", Jira'ya bug açma, ayarlar.
- Şirkette Active Directory / LDAP var; ileride giriş buna bağlanabilsin diye giriş kontrolü tek bir arayüz arkasında yazılır. Şimdilik AD/LDAP bağlantısı yapılmaz.
- Kabul: Yanlış şifreyle admin işlemi yapılamaz; şifre loglarda ve hata mesajlarında görünmez.
- Test: B
- Doğrulama: `test_sw_9_5_*`, e2esmoke

---

## SYS-10: Kurulum, işletim, doğrulama

Sistem kurulabilir, doğrulanabilir ve kaybolmaz.

### SW-10.1 [KISMEN] VM, runner, token ve zamanlama rehberi (`docs/GITLAB_CI_KURULUMU.md`)
- Ubuntu için: gereken paketler (Xvfb, ffmpeg, Java ve JavaFX bağımlılıkları), GitLab runner'ın shell executor ile hizmet olarak kurulması ve SW-1.2'deki Xvfb ayarı.
- Windows için: gereken programlar (ffmpeg, Java), otomatik oturum açma, kilit ve uykunun kapatılması, runner'ın oturumda başlatılması (SW-1.6) ve RDP'den `tscon` ile çıkış.
- raporcu sunucusunun her iki sistemde hizmet olarak kurulması.
- Kabul: Rehberle sıfırdan kurulum yapılabilir.
- Test: M
- Not: Rehber yazıldı (docs/GITLAB_CI_KURULUMU.md); sıfırdan kurulumla M kontrolü bekliyor.

### SW-10.2 [KISMEN] Prova (`cargo run -p rehearse`) ve uçtan uca smoke (`cargo run -p e2esmoke`)
- Smoke testi sonuçları CI yükleme yolundan (SW-2.1) gönderir.
- Kabul: Her ikisi de tüm adımlarıyla hem Ubuntu'da hem Windows'ta geçer; her yeni CI adımı provaya eklenir.
- Test: P, U
- Doğrulama: `cargo run -p rehearse`, `cargo run -p e2esmoke`
- Not: `rehearse` (43 adım) ve `e2esmoke` (18 adım) Linux'ta geçer. Windows ikilileri Wine altında: `cargo test --target x86_64-pc-windows-gnu` 96/96 (yalnızca Linux /proc kullanan bellek ölçüm testi hariç), rehearse 31/31 (Xvfb ve git adımları atlandı), e2esmoke 18/18. Gerçek Windows VM'de koşturulmadı.

### SW-10.3 [YAPILACAK] raporcu reposu şirket GitLab'ında açılır
- Push kapalıysa `git bundle` ile aktarılır.
- Kabul: Repo GitLab'da; ilk commit'ten itibaren kod orada.
- Test: M
- Not: Şirket GitLab'ına erişim gerekiyor (M).

### SW-10.4 [KISMEN] Yeni test projesi ekleme rehberi (yarım sayfa)
- Kabul: Bir ekip rehberi kullanarak kendi projesini bağlayabilir.
- Test: M
- Not: Rehber yazıldı (docs/YENI_PROJE_EKLEME.md); bir ekiple M kontrolü bekliyor.

### SW-10.5 [MEVCUT] Proje iskeleti
- Cargo workspace; `raporcu` (sunucu) ve `stajci` (CI aracı) crate'leri, ortak kod için `common` crate'i; yapılandırma ortam değişkenleri ve `.env` dosyasından; temel sağlık ucu (`GET /api/health`).
- Kabul: Sunucu açılır, sağlık ucu 200 döner, `cargo test` geçer, Linux ve Windows ikilileri derlenir.
- Test: B
- Doğrulama: `test_sw_10_5_*`
- Not: Windows ikilisi x86_64-pc-windows-gnu ile derlendi.

### SW-10.6 [MEVCUT] Sunucunun bellek bütçesi
- Allure zip'i belleğe alınmaz, dosya dosya akış halinde işlenir; videolar ve ekler HTTP range desteğiyle parça parça sunulur.
- Yükleme boyutuna üst sınır vardır; listeler ve senaryo geçmişi sayfalanır; hiçbir istek bir koşumun tüm verisini belleğe yüklemez.
- SQLite önbelleği sınırlıdır (`PRAGMA cache_size`); Ubuntu'da systemd `MemoryMax=` ile sert sınır konur.
- Hedef: Normal gece yükünde (birkaç yüz senaryo, ekleriyle) sunucunun bellek kullanımı 100 MB'ın altında kalır. Hedef değer yapılandırmadan değiştirilebilir.
- Kabul: Büyük bir zip (yükleme sınırına yakın) yüklenirken bellek kullanımı hedefin altında kalır; bu durum bir testle ölçülür.
- Test: B, P
- Doğrulama: `test_sw_10_6_large_zip_upload_stays_under_memory_target`, e2esmoke range adımı
- Not: Release sunucu: 200 MB zip + 40 eşzamanlı sayfa isteğinde tepe RSS ~28 MB.

### SW-10.7 [MEVCUT] Log içeriği
- Biçim: Her satır bir olay, JSON. Ortak alanlar: `zaman`, `seviye`, `olay`, ve varsa `proje`, `branch`, `pipeline_id`, `run_id`, `senaryo_id`, `sure_ms`, `hata`.
- INFO olarak yazılan olaylar:
  - Koşum açıldı, bitti, interrupted oldu (senaryo sayıları ile).
  - Yükleme alındı (dosya sayısı, boyut, süre) ya da reddedildi (neden).
  - Webhook alındı (pipeline durumu).
  - Triage kararı (kategori, kaynak: kural / OpenCode / admin).
  - Jira bug'ı açıldı ya da eşleşti (issue numarası); gözlem modunda "açılacaktı" kaydı.
  - OpenCode denemesi başladı ve bitti (sonuç, MR numarası).
  - Admin girişi başarılı ya da başarısız (yalnızca IP).
  - Paylaşım linki oluşturuldu ya da iptal edildi.
  - Migration uygulandı, yedek alındı, disk temizliği yapıldı (silinen dosya sayısı, boşalan alan).
- WARN ve ERROR: dış servis hataları (Jira, GitLab, SMTP; HTTP durum kodu ve deneme sayısı ile), bellek ya da disk eşiği aşıldı, `stajci` sürüm uyuşmazlığı.
- Asla yazılmayanlar: token, şifre, çerez, `Authorization` başlığı, istek gövdeleri. Test loglarından ve hata mesajlarından yalnızca ilk 500 karakter yazılır.
- `stajci` CI iş çıktısına kısa, okunur satırlar basar; her satırda aynı `run_id` vardır, böylece CI çıktısı ile sunucu logu eşleştirilebilir.
- Dönüşüm: Günlük dosya, en fazla 14 gün ve toplam 500 MB; hangisi önce dolarsa en eski silinir.
- Admin panelinde son WARN ve ERROR kayıtları görünür.
- Kabul: Yukarıdaki her olay testte tetiklenince ilgili log satırı oluşur; bir test, log dosyalarında token ve şifre geçmediğini doğrular.
- Test: B
- Doğrulama: `test_sw_10_7_*`, e2esmoke log adımı

### SW-10.8 [MEVCUT] Disk doluluğu ve temizlik
- Disk %80 dolunca admin panelinde ve logda uyarı çıkar.
- Disk %90 dolunca en eski koşumlardan başlayarak önce videolar, sonra ekran görüntüleri silinir; doluluk %80'in altına inince durur.
- Jira issue'su olan senaryoların ekran görüntüsü Jira'da durduğu için silinebilir; senaryoda Jira issue numarası kalır ve silinen ekin yerinde "Silindi, bkz. JIRA-123" yazar.
- Koşum, senaryo, adım ve triage kayıtları ile geçmiş hiçbir zaman silinmez.
- Eşik değerleri yapılandırmadan değiştirilebilir.
- Kabul: Eşik aşılınca doğru sırayla silinir; rapor sayfaları silinen ekler için hata vermez.
- Test: B
- Doğrulama: `test_sw_10_8_*`

### SW-10.9 [KISMEN] `stajci` dağıtımı ve sürüm uyumu
- raporcu sunucusu Linux ve Windows için `stajci` ikilisini `GET /api/tools/stajci?os=linux|windows` ucundan sunar; CI şablonu aracı buradan indirir.
- `stajci` her istekte kendi sürümünü gönderir; sunucu uyumsuz sürümü açık bir mesajla reddeder.
- Kabul: Sunucu güncellenince ertesi koşumda runner'lar yeni `stajci`'yi kullanır; eski sürüm "stajci sürümü uyumsuz" mesajıyla düşer.
- Test: B, P
- Doğrulama: `test_sw_10_9_*`, rehearse SW-10.9
- Not: Sunucudan dağıtım ve sürüm reddi testleri geçer; Windows runner'da doğrulama bekliyor.

---

## Uygulama sırası

Bu sıra, her adımın bir öncekinin üzerine kurulacağı şekilde dizildi.

1. **Temel:** SW-10.3, 10.5, 2.4, 2.5, 10.7, 10.6, 9.1, 9.5.
2. **Sonuç alma:** SW-2.7, 4.6, 2.1, 2.3, 2.2.
3. **CI hattı:** SW-1.1, 1.2, 1.6, 1.7, 1.5, 10.9, 2.6, 10.2, ardından 1.3.
4. **Raporlama:** SW-4.1, 4.3, 4.9, 4.2, 4.7, 4.8, ardından 10.8.
5. **Paylaşım:** SW-5.1, 5.4, 5.2.
6. **Canlı durum:** SW-3.1 → 3.3 → 3.2 → 3.4 → 3.5, ardından 9.3 ve 3.6.
7. **Jira ve DOORS:** SW-6.1, 6.4, 6.2, 6.3 (gözlem modu), ardından 5.3.
8. **Triage ve self-healing:** SW-4.5, 7.1, 7.6, 7.2, 9.4, 7.3, 4.4, ardından 9.2, 7.4, 7.5, 7.7.
9. **Yaygınlaştırma:** SW-1.4, 10.1, 10.4.

## Kararlar (ortam)

1. GitLab sunucusu raporcu makinesine erişebiliyor. Pipeline durumu webhook ile alınır (SW-3.6).
2. GitLab runner ve raporcu sunucusu aynı makinede çalışır. CI işleri raporcu'ye yerel adresten ulaşır (SW-3.2 ve diğer CI yüklemeleri).
3. Raporlar yalnızca şirket içinde, kapalı yerel ağda paylaşılır. Paylaşım linklerinde süre sınırı yoktur; rapor diskte yer olduğu sürece saklanır (SW-5.1, 5.2).
4. vLLM modeli görsel destekli. Ekran görüntüleri OpenCode'a verilir (SW-7.2).
5. Runner'lar hem Ubuntu hem Windows sanal makinelerde çalışır. Ubuntu'da sanal ekran Xvfb'dir (SW-1.2); Windows'ta açık masaüstü oturumu kullanılır (SW-1.6). Monocle ve headless çalışma kullanılmaz.
6. Makinelerde bellek kısıtlı. Bu yüzden sunucu ve CI araçları Rust ile yazılır, veritabanı SQLite'tır ve bellek sınırları gereksinim olarak tanımlıdır (SW-1.7, SW-10.6).
7. Şimdilik tek bir admin kullanıcısı ve basit şifreli giriş yeterli. Şirkette Active Directory / LDAP var; ileride bağlanabilir (SW-9.5).
8. Ekran görüntüsü yalnızca hata olduğunda alınır. Disk dolarsa ekran görüntüleri silinebilir; Jira issue numarası kalır (SW-2.2, SW-10.8).
9. CSV dışa aktarma Jira'daki gibi alan seçmeli, düz CSV'dir (SW-4.8).
10. Test projesi Allure ile ayarlanacak; gerçek sonuç örnekleri oradan alınacak (SW-2.7).

## Açık sorular

Bu sorular yanıtlanmadan ilgili gereksinime başlama; bunun yerine soruyu sor.

1. raporcu'ye pipeline tetikleme ve okuma yetkisi olan bir GitLab token verilebilir mi? (SW-7.4, 7.5)
2. Ubuntu ve Windows runner'ları ayrı makinelerdeyse her makinede ayrı bir raporcu sunucusu mu çalışacak, yoksa tek bir merkezi sunucu mu olacak? Ayrı sunucular raporları ve geçmişi ikiye böler. (SW-3.2, SW-3.6, SW-4.2)
3. Triage nasıl çalışacak? Kategoriler doğru mu, OpenCode'un kararı kendiliğinden mi geçerli olsun yoksa admin onayı mı beklesin, düşük güvenli kararlar ne olsun? (SW-4.5)
