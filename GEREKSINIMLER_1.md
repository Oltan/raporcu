# stajproje Gereksinimleri

## Bağlam

stajproje sıfırdan yazılacak bir "test sonuç merkezi". Testleri stajproje koşmaz, GitLab runner koşar. stajproje sonuçları alır; canlı durum, rapor, paylaşım, triage, Jira/DOORS ve self-healing sağlar.

- **stajproje**: bu raporlama projesi. Henüz hiçbir kodu yok.
- **test projesi**: JavaFX uygulamasının Cucumber + TestFX test reposu (GitLab). Testler Allure raporu üretir.
- **Teknoloji**: Rust. Sunucu ve CI araçları Rust ile yazılır; bellek kullanımı en düşük seçenek olduğu ve dış çalışma ortamı gerektirmediği için seçildi.
  - Sunucu: `axum` + `tokio`; sayfalar `askama` şablonları + htmx; canlı ekran için axum WebSocket. Şablonlar derlemede ikiliye girer, statik dosyalar `rust-embed` ile gömülür.
  - Veritabanı: SQLite, `rusqlite` (`bundled` özelliğiyle; SQLite ikilinin içine derlenir). Ayrı veritabanı sunucusu kurulmaz (SW-2.5).
  - Dosyalar (ekran görüntüsü, video, Allure dosyaları) diskte `DATA_DIR` altında; veritabanında yalnızca yolları tutulur.
  - CI araçları tek bir yardımcı ikili: `stajci` (alt komutlar: `preflight`, `start`, `watch`, `upload`, `finish`, `fix`).
  - Derleme: Linux için `x86_64-unknown-linux-gnu`, Windows için `x86_64-pc-windows-gnu` (Linux'ta mingw ile) ya da Windows'ta `x86_64-pc-windows-msvc`.
- **İşletim sistemi**: Runner'lar hem Ubuntu hem Windows sanal makinelerde çalışır. stajproje sunucusu runner ile aynı makinede çalışır; her iki işletim sisteminde de çalışabilmelidir.

Kısaltmalar:

- `SYS-n`: sistem gereksinimi (ne yapılacak)
- `SW-n.m`: yazılım gereksinimi (nasıl yapılacak)
- Durum: `YAPILACAK`, `KISMEN` (başlandı, eksik kısmı var), `MEVCUT` (bitti, testi geçiyor)
- Test: B birim, E entegrasyon, P prova (yerelde CI benzetimi), U uçtan uca, M manuel

Toplam 58 yazılım gereksinimi var; hepsi YAPILACAK durumunda.

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

Testler stajproje'de değil, test projesinin GitLab pipeline'ında, Ubuntu veya Windows sanal makinedeki runner'da koşar. Test projesi hangi işletim sisteminde koşacağını runner etiketiyle (`linux` / `windows`) seçer.

### SW-1.1 [YAPILACAK] CI şablonu test → report → fix aşamalarını içerir
- Dosya: `ci/gitlab/template/.gitlab-ci.yml`
- Şablon iki işletim sistemini destekler; test projesi bir değişkenle (`linux` veya `windows`) seçer, işler ilgili runner etiketine gider.
- Kabul: Pipeline 3 iş çalıştırır; test işi düşse de report ve fix çalışır. Bu, hem Ubuntu hem Windows runner'da geçerlidir.
- Test: P, U

### SW-1.2 [YAPILACAK] Testler Ubuntu sanal makinede Xvfb sanal ekranında koşar
- Headless çalışma ve Monocle kullanılmaz; uygulama Xvfb'nin sağladığı X11 ekranında gerçekten açılır.
- Test işi Xvfb'yi sabit çözünürlükle başlatır (örneğin `xvfb-run -a -s "-screen 0 1920x1080x24"`), testler bitince kapatır. Her iş kendi ekranını alır.
- GitLab runner sistem hizmeti olarak çalışabilir; oturum açma, ekran kilidi veya uyku ayarı gerekmez.
- Wayland kullanılmaz; JavaFX ve TestFX robotu X11 üzerinde çalışır.
- Video kaydı aynı ekrandan alınır (örneğin `ffmpeg -f x11grab`).
- Kabul: Kimse bağlı değilken gece koşumu geçer; ekran görüntüleri ve video uygulamanın gerçek görüntüsünü gösterir; aynı makinede iki iş aynı anda koşarsa ekranları karışmaz.
- Test: P, U

### SW-1.3 [YAPILACAK] Gece zamanlaması GitLab schedule ile çalışır
- Kabul: Pipeline 02:00'de kendiliğinden başlar, rapor sabah stajproje'de hazır olur.
- Test: U, M

### SW-1.4 [YAPILACAK] Merkezi şablon
- Test projeleri yalnızca bir `include` satırı ekler; betikler stajproje reposundan çekilir.
- Kabul: Merkezdeki bir betik değişince ertesi koşumda tüm projeler yeni sürümü kullanır.
- Test: U

### SW-1.5 [YAPILACAK] Ön kontrol CI içinde testlerden önce çalışır
- Kontrol edilenler: pom, Allure eklentisi, features.txt tag'leri.
- Kabul: Adaptör eksikse iş açık bir mesajla düşer, testler hiç başlamaz.
- Test: B, P

### SW-1.6 [YAPILACAK] Testler Windows sanal makinede, açık bir masaüstü oturumunda koşar
- Headless çalışma ve Monocle kullanılmaz; uygulama sanal makinenin sanal ekranında gerçekten açılır.
- GitLab runner Windows hizmeti olarak değil, oturum açmış kullanıcının masaüstünde çalışır (TestFX fare ve klavye olaylarını bu oturuma gönderir).
- Sanal makine yeniden başlayınca otomatik oturum açar ve runner'ı başlatır; ekran kilidi, ekran koruyucu ve uyku kapalıdır; ekran çözünürlüğü sabittir.
- Uzak Masaüstü (RDP) bağlantısı kesilince oturum kilitlenir; bağlantıdan çıkarken oturum `tscon` ile konsola geri verilir.
- Video kaydı masaüstünden alınır (örneğin `ffmpeg -f gdigrab`).
- Kabul: Kimse bağlı değilken gece koşumu geçer; ekran görüntüleri ve video uygulamanın gerçek görüntüsünü gösterir.
- Test: P, U, M

### SW-1.7 [YAPILACAK] Test işinin bellek sınırları
- CI şablonu Maven ve test JVM'leri için bellek üst sınırı koyar: `MAVEN_OPTS` ve surefire `argLine` içinde `-Xmx`; surefire tek fork ile çalışır.
- Runner aynı anda tek iş koşar (`concurrent = 1`).
- Video kaydı düşük kare hızı ve sıkıştırılmış biçimle alınır; yalnızca başarısız senaryoların videosu saklanır.
- Sınır değerleri CI değişkeniyle değiştirilebilir.
- Kabul: Gece koşumu boyunca makinede bellek yetersizliği (OOM) olmaz; ayarlanan sınır aşılınca test işi açık bir mesajla düşer.
- Test: P, U

---

## SYS-2: Sonuçlar alınır ve saklanır

CI'dan gelen Allure sonuçları alınır, işlenir ve saklanır.

### SW-2.1 [YAPILACAK] POST /api/ci/runs
- Allure sonuç zip'ini alır, ayrıştırır ve bir koşum olarak kaydeder.
- Kabul: Koşum listede görünür; rapor, triage ve CSV dışa aktarma bu koşum için çalışır; zip güvenliği sağlanır (boyut sınırı, `..` içeren yollar reddedilir).
- Test: B

### SW-2.2 [YAPILACAK] @After hook ekleri (container dosyası) senaryoya bağlanır
- Ekran görüntüsü yalnızca başarısız senaryoda alınır; başarılı senaryolarda görüntü yoktur.
- Kabul: Başarısız senaryonun ekran görüntüsü raporda görünür; başarılı senaryoda ek yoktur.
- Test: B, U

### SW-2.3 [YAPILACAK] Sonuç dosyası olmayan yükleme reddedilir
- Kabul: 422 döner, "Allure eklentisi" eksikliğini anlatan açıklamayla.
- Test: B

### SW-2.4 [YAPILACAK] Tüm çalışma verisi DATA_DIR altında tutulur
- Yollar tek bir modülde (`paths.rs`) tanımlanır.
- Kabul: DATA_DIR değiştirilince tüm veri yeni yere yazılır; testler geçici bir DATA_DIR kullanır, gerçek veriye yazmaz.
- Test: B

### SW-2.5 [YAPILACAK] Veritabanı: kurulumsuz SQLite
- Veritabanı `DATA_DIR/stajproje.db` dosyasıdır. Sunucu ilk açılışta dosyayı kendisi oluşturur; ayrı kurulum, kullanıcı veya şifre yoktur.
- Şema değişiklikleri numaralı migration dosyalarıyla yapılır (`migrations/0001_ilk.sql`, ...). Migration'lar ikiliye gömülür ve sunucu açılırken sırayla uygulanır; uygulananlar veritabanında kayıtlıdır.
- Açılışta ayarlar: `journal_mode=WAL` (okuma ve yazma birbirini beklemez), `busy_timeout`, `foreign_keys=ON`.
- Yazmalar tek bir yazıcı bağlantısından, okumalar küçük bir bağlantı havuzundan yapılır.
- Yedek: Her gece çalışan sunucu durdurulmadan `VACUUM INTO` ile `DATA_DIR/backup/` altına kopya alınır; son 7 yedek tutulur.
- Ana tablolar: `runs`, `scenarios`, `steps`, `attachments`, `bug_links`, `share_links`, `fix_attempts`, `users`.
- Kabul: Boş bir DATA_DIR ile açılan sunucu veritabanını oluşturur; yeni bir migration eklenince bir sonraki açılışta uygulanır; yedek dosyası açılıp okunabilir; testler geçici bir veritabanı kullanır.
- Test: B

### SW-2.6 [YAPILACAK] Sunucuya ulaşılamazsa sonuçlar kaybolmaz
- `stajci` gönderemediği sonuçları ve ekleri iş dizinindeki bir kuyruk klasörüne yazar ve artan aralıklarla yeniden dener.
- İş bitiminde hâlâ gönderilemeyen sonuçlar GitLab artifact'i olarak saklanır; sunucu geri gelince `stajci upload --resume` ile gönderilir.
- Kabul: Sunucu koşum sırasında kapatılıp açılınca hiçbir sonuç eksik ya da çift sayılmaz (SW-3.3 ile).
- Test: B, P

### SW-2.7 [YAPILACAK] Gerçek Allure örnekleri test verisi olarak kullanılır
- Test projesi Allure ile ayarlandıktan sonra, ondan alınan `allure-results` örnekleri `testdata/` altına konur: geçen, kalan, broken, rerun edilmiş ve ekli senaryolar.
- Ayrıştırıcı testleri uydurma değil bu örneklerle yazılır.
- Kabul: SW-2.1, 2.2 ve 4.9 testleri gerçek örneklerle geçer.
- Test: B

---

## SYS-3: Canlı test durumu

Testler koşarken hangi senaryonun geçtiği, hangisinin kaldığı anlık görünür.

### SW-3.1 [YAPILACAK] POST /api/ci/runs/start
- Koşum "çalışıyor" durumunda açılır; toplam senaryo sayısı (dry-run ile) kaydedilir.
- Kabul: Koşum panelde, testler bitmeden "47/120" gibi bir ilerlemeyle görünür.
- Test: B, P

### SW-3.2 [YAPILACAK] CI izleyicisi (`stajci watch`)
- Her yeni sonuç dosyasını ve eklerini anında gönderir; test işiyle paralel çalışır.
- stajproje runner ile aynı makinede çalıştığı için izleyici sonuçları yerel adrese (localhost) gönderir.
- Kabul: Senaryo bittikten en geç 5 sn sonra panelde görünür; izleyici çökerse test işi etkilenmez.
- Test: P

### SW-3.3 [YAPILACAK] POST /api/ci/runs/{id}/results
- Sonuçlar artımlı alınır; aynı dosya iki kez sayılmaz.
- Kabul: Aynı sonucu iki kez göndermek sayıyı değiştirmez; sonuçlar son toplu zip (SW-2.1) ile tutarlıdır.
- Test: B

### SW-3.4 [YAPILACAK] Canlı koşum ekranı
- Senaryo listesi (geçti, kaldı, koşuyor, bekliyor), sayaçlar, hata ve ekran görüntüsü; güncellemeler WebSocket ile gelir.
- Kabul: Sayfa yenilenmeden güncellenir; hata, koşum bitmeden görünür.
- Test: B, P, M

### SW-3.5 [YAPILACAK] POST /api/ci/runs/{id}/finish ve zaman aşımı
- Yarıda kesilen koşum "interrupted" olur.
- Kabul: Runner durdurulunca koşum RUN_STALL_TIMEOUT içinde interrupted durumuna geçer.
- Test: B

### SW-3.6 [YAPILACAK] Pipeline düzeyi durum (kuyrukta, çalışıyor, bitti)
- GitLab webhook ile alınır. GitLab sunucusu makineye erişebildiği için API sorgusuyla yedek yol gerekmez.
- Kabul: Pipeline başlayınca panelde "kuyrukta / çalışıyor" görünür.
- Test: B, U

---

## SYS-4: Raporlama

Koşum sonuçları ayrıntılı olarak ve geçmişiyle birlikte görüntülenir.

### SW-4.1 [YAPILACAK] Koşum raporu, senaryo detayı, adımlar, ekran görüntüsü ve video gösterimi
- Kabul: Bir koşumdan senaryoya, oradan adımlara ve eklere gidilebilir; ekran görüntüsü ve video sayfada açılır.
- Test: B, U

### SW-4.2 [YAPILACAK] Dashboard, senaryo geçmişi, flaky tespiti
- Kabul: Dashboard son koşumların özetini gösterir; bir senaryonun geçmiş koşumlardaki sonuçları listelenir; geçip kalmayı değişken olan senaryo flaky olarak işaretlenir.
- Test: B

### SW-4.3 [YAPILACAK] CI bilgisi raporda: repo, branch, commit ve pipeline linki
- Kabul: Rapor başlığında bu bilgiler görünür; pipeline linki tıklanabilir.
- Test: B

### SW-4.4 [YAPILACAK] OpenCode düzeltme kutusu ve senaryo rozeti
- SW-7.3'e bağlıdır.
- Kabul: fix-result geldikten sonra raporda düzeltme kutusu ve senaryoda "OpenCode !MR" rozeti görünür.
- Test: B, P

### SW-4.5 [YAPILACAK] Triage: başarısız senaryonun nedenine karar verilmesi (taslak)
- Kategoriler: `flaky`, `test kodu hatası` (bizden), `ürün hatası`, `ortam sorunu`, `bilinmiyor`.
- Karar üç kaynaktan gelir ve her kararın kaynağı kayıtlıdır:
  1. Kural: flaky geçmişi (SW-4.2), Allure durumu (`broken` → test kodu, `failed` → ürün), bilinen ortam hata kalıpları (zaman aşımı, bağlantı hatası, ekran bulunamadı).
  2. OpenCode: hata mesajı, adımlar ve ekran görüntüsü vLLM'e verilir; kategori, kısa gerekçe ve güven döner. Fix işinde (SW-7.2) çalışır, sonucu stajproje'ye bildirir.
  3. Admin: kararı onaylar ya da değiştirir. Admin kararı diğerlerinin üstündedir.
- Kategori sonraki adımı belirler: test kodu hatası → OpenCode düzeltmesi, ürün hatası → Jira, flaky → atlanır, ortam sorunu → bildirim.
- Ayrıntılar netleşmedi (açık soru 3); bu gereksinime başlamadan önce sor.
- Kabul: Her başarısız senaryonun raporda kategorisi, kaynağı ve gerekçesi görünür; admin değiştirince yeni karar geçerli olur ve eski karar geçmişte kalır.
- Test: B, P

### SW-4.6 [YAPILACAK] Proje ve branch ayrımı
- Her koşum bir test projesine ve branch'e bağlıdır. Dashboard, geçmiş ve flaky tespiti proje ve branch bazında yapılır.
- Yeni proje, ilk yüklemede GitLab proje bilgisinden kendiliğinden oluşur.
- Kabul: İki projenin aynı adlı senaryoları birbirinin geçmişine karışmaz; dashboard proje ve branch seçilerek süzülür.
- Test: B

### SW-4.7 [YAPILACAK] Arama ve filtre
- Senaryolar tag, feature, DOORS numarası, Jira issue numarası, triage kategorisi ve hata mesajı ile aranır.
- Kabul: Her alanla arama doğru sonuç verir; sonuçlar sayfalanır.
- Test: B

### SW-4.8 [YAPILACAK] CSV dışa aktarma (alan seçmeli)
- Jira'daki dışa aktarma gibi: kullanıcı o anki filtreyle listelenen senaryoları, istediği alanları seçerek düz CSV olarak indirir.
- Seçilebilir alanlar: proje, branch, koşum, senaryo, feature, tag'ler, durum, triage kategorisi, hata mesajı, süre, DOORS numarası, Jira issue numarası, tarih.
- Kabul: Yalnızca seçilen alanlar seçilen sırayla gelir; Excel'de Türkçe karakterler doğru açılır (UTF-8 BOM); büyük listeler akış halinde yazılır (SW-10.6).
- Test: B

### SW-4.9 [YAPILACAK] Aynı koşumda tekrar koşan senaryolar
- Cucumber rerun ya da retry nedeniyle aynı senaryonun birden fazla sonucu olabilir. Senaryonun sonucu son denemedir; önceki denemeler senaryo detayında görünür.
- Önce kalıp sonra geçen senaryo flaky tespitine girdi olarak sayılır.
- Kabul: Sayaçlarda her senaryo bir kez sayılır; denemeler detayda sırayla listelenir.
- Test: B

---

## SYS-5: Rapor paylaşımı

Raporlar ekip ve paydaşlarla güvenli biçimde paylaşılır.

### SW-5.1 [YAPILACAK] İç bilgileri gizlenmiş salt okunur paylaşım linki
- Link, şirket ağındaki herkesin giriş yapmadan açabileceği bir linktir.
- Kabul: Token, Jira anahtarı ve ham ek linki görünmez.
- Test: B

### SW-5.2 [YAPILACAK] Paylaşım linkinin iptali
- Bağlam: Raporlar yalnızca şirket içindeki kapalı yerel ağda paylaşılır, dışarıya açılmaz. Bu yüzden önceliği düşük.
- Süre sınırı yok: Link, rapor saklandığı sürece (diskte yer olduğu sürece) açık kalır.
- Kabul: İptal edilen link açılmaz; iptal admin panelinden yapılır. Süre sınırı eklenmez.
- Test: B

### SW-5.3 [YAPILACAK] Gece pipeline'ı bitince özet e-posta
- İçerik: geçen, kalan, rapor linki, açılan MR'lar.
- Kabul: DRY_RUN'da gönderim simüle edilir; gerçekte pipeline başına tek e-posta gider.
- Test: B, M

### SW-5.4 [YAPILACAK] Rapor linki pipeline'da ve MR açıklamasında
- Kabul: Rapor linki yükleme işinin çıktısında, pipeline "environment" linkinde ve OpenCode MR açıklamasında yer alır.
- Test: P, U

---

## SYS-6: Hata yönetimi: Jira ve DOORS

Başarısız senaryolar Jira bug'larıyla eşleşir, gerektiğinde bug açılır.

### SW-6.1 [YAPILACAK] Jira istemcisi: bug'da DOORS numarası label'da ve açıklamada
- Jira label'ı reddederse bug label'sız açılır.
- Bug açılırken ya da senaryo var olan bir bug'a bağlanırken ekran görüntüsü Jira issue'suna eklenir.
- Kabul: DOORS numarası her iki yerde de görünür; label hatasında bug yine de açılır; ekran görüntüsü Jira issue'sunda görünür.
- Test: B

### SW-6.2 [YAPILACAK] CI koşumundan sonra FAILED senaryolar için otomatik eşleşme kendiliğinden çalışır
- Kabul: Yükleme sonrasında açık bug'ı olan senaryolar bağlanmış görünür.
- Test: B, E

### SW-6.3 [YAPILACAK] Sınırlı otomatik bug açma
- Koşum başına üst sınır vardır; aynı hata için tek bug açılır; bug retry sonrasında açılır; önce gözlem modunda çalışır.
- Kabul: Gözlem modunda Jira'ya yazılmaz, kayıt tutulur; üst sınır aşılmaz.
- Test: B

### SW-6.4 [YAPILACAK] DOORS dışa aktarma: CSV ve DXL
- Kabul: Koşum sonuçları DOORS'a aktarılabilecek CSV ve DXL dosyası olarak indirilir.
- Test: B

---

## SYS-7: Self-healing (OpenCode)

Test kodu yüzünden düşen senaryolar OpenCode ile düzeltilir ve insan onayıyla birleştirilir.

### SW-7.1 [YAPILACAK] fix-candidates sınıflandırması
- Hedef triage kategorisine göre seçilir (SW-4.5): test kodu hatası → OpenCode, ürün hatası → Jira, @sample-fail ve flaky → atlanır. Triage kararı yoksa Allure durumu kullanılır (broken → OpenCode, failed → Jira).
- Kabul: Her durum için doğru hedef seçilir.
- Test: B

### SW-7.2 [YAPILACAK] OpenCode düzeltmesi (`stajci fix`)
- Yalnızca `src/test/` altında değişiklik yapar, doğrulama koşusu çalıştırır, auto-fix branch'i ve MR açar (doğrulama geçmezse Draft MR).
- vLLM modeli görsel destekli: başarısız senaryonun ekran görüntüsü OpenCode isteğine eklenir.
- Kabul: Önce provada sahte OpenCode ile, sonra gerçek OpenCode, vLLM, TestFX MCP ve GitLab MR ile geçer.
- Test: P, U

### SW-7.3 [YAPILACAK] Sonuç stajproje'ye bildirilir (fix-result) ve raporda görünür
- Test: B, P

### SW-7.4 [YAPILACAK] Rapordaki "OpenCode ile düzelt" düğmesi GitLab API ile yalnızca fix işini tetikler
- Kabul: Düğme → pipeline → sonuç aynı rapora yazılır.
- Test: B, U

### SW-7.5 [YAPILACAK] Self-healing ölçüm paneli: deneme, MR, birleştirilen, reddedilen
- Kabul: Panel son 30 gecenin sayılarını gösterir.
- Test: B

### SW-7.6 [YAPILACAK] Test projesi için AGENTS.md, fix ajanı ve `opencode.json` (vLLM, TESTFX_MCP_JAR)
- Önce stajproje reposunda şablonlar hazırlanır, sonra test projesine özel AGENTS.md yazılır.
- Test: U

### SW-7.7 [YAPILACAK] Geçmişteki başarılı düzeltmeler OpenCode'a örnek olarak verilir
- Kabul: İstek metninde benzer bir senaryonun önceki düzeltmesi yer alır.
- Test: B

---

## SYS-9: Güvenlik ve erişim

Tokenlar korunur, dışarıdan gelen istekler doğrulanır, OpenCode sınırlanır.

### SW-9.1 [YAPILACAK] CI uçları ayrı bir CI_API_TOKEN ile korunur
- Kabul: Token yoksa veya yanlışsa istek reddedilir.
- Test: B

### SW-9.2 [YAPILACAK] GitLab token yalnızca `.env`'de durur, loglara yazılmaz
- Kabul: Hata mesajlarında token görünmez.
- Test: B

### SW-9.3 [YAPILACAK] Webhook isteği X-Gitlab-Token ile doğrulanır
- Kabul: Yanlış token reddedilir.
- Test: B

### SW-9.4 [YAPILACAK] OpenCode koruması: izinli yollar, assertion yasağı, loglar veri olarak ele alınır
- Kabul: İzinli yolların dışındaki değişiklik reddedilir; assertion değiştiren düzeltmede MR'a uyarı eklenir; test loglarındaki metin talimat olarak yorumlanmaz.
- Test: P

### SW-9.5 [YAPILACAK] Admin girişi
- Şimdilik tek kullanıcı vardır: admin. Şifre `.env` içinde düz değil, hash olarak (argon2) tutulur; `stajproje set-admin-password` komutuyla belirlenir.
- Giriş sonrası oturum çerezi kullanılır (HttpOnly, SameSite=Strict); başarısız denemeler yavaşlatılır.
- Raporları görmek giriş gerektirmez (kapalı ağ). Admin işlemleri giriş ister: link iptali, triage kararı değiştirme, "OpenCode ile düzelt", Jira'ya bug açma, ayarlar.
- Şirkette Active Directory / LDAP var; ileride giriş buna bağlanabilsin diye giriş kontrolü tek bir arayüz arkasında yazılır. Şimdilik AD/LDAP bağlantısı yapılmaz.
- Kabul: Yanlış şifreyle admin işlemi yapılamaz; şifre loglarda ve hata mesajlarında görünmez.
- Test: B

---

## SYS-10: Kurulum, işletim, doğrulama

Sistem kurulabilir, doğrulanabilir ve kaybolmaz.

### SW-10.1 [YAPILACAK] VM, runner, token ve zamanlama rehberi (`docs/GITLAB_CI_KURULUMU.md`)
- Ubuntu için: gereken paketler (Xvfb, ffmpeg, Java ve JavaFX bağımlılıkları), GitLab runner'ın shell executor ile hizmet olarak kurulması ve SW-1.2'deki Xvfb ayarı.
- Windows için: gereken programlar (ffmpeg, Java), otomatik oturum açma, kilit ve uykunun kapatılması, runner'ın oturumda başlatılması (SW-1.6) ve RDP'den `tscon` ile çıkış.
- stajproje sunucusunun her iki sistemde hizmet olarak kurulması.
- Kabul: Rehberle sıfırdan kurulum yapılabilir.
- Test: M

### SW-10.2 [YAPILACAK] Prova (`cargo run -p rehearse`) ve uçtan uca smoke (`cargo run -p e2esmoke`)
- Smoke testi sonuçları CI yükleme yolundan (SW-2.1) gönderir.
- Kabul: Her ikisi de tüm adımlarıyla hem Ubuntu'da hem Windows'ta geçer; her yeni CI adımı provaya eklenir.
- Test: P, U

### SW-10.3 [YAPILACAK] stajproje reposu şirket GitLab'ında açılır
- Push kapalıysa `git bundle` ile aktarılır.
- Kabul: Repo GitLab'da; ilk commit'ten itibaren kod orada.
- Test: M

### SW-10.4 [YAPILACAK] Yeni test projesi ekleme rehberi (yarım sayfa)
- Kabul: Bir ekip rehberi kullanarak kendi projesini bağlayabilir.
- Test: M

### SW-10.5 [YAPILACAK] Proje iskeleti
- Cargo workspace; `stajproje` (sunucu) ve `stajci` (CI aracı) crate'leri, ortak kod için `common` crate'i; yapılandırma ortam değişkenleri ve `.env` dosyasından; temel sağlık ucu (`GET /api/health`).
- Kabul: Sunucu açılır, sağlık ucu 200 döner, `cargo test` geçer, Linux ve Windows ikilileri derlenir.
- Test: B

### SW-10.6 [YAPILACAK] Sunucunun bellek bütçesi
- Allure zip'i belleğe alınmaz, dosya dosya akış halinde işlenir; videolar ve ekler HTTP range desteğiyle parça parça sunulur.
- Yükleme boyutuna üst sınır vardır; listeler ve senaryo geçmişi sayfalanır; hiçbir istek bir koşumun tüm verisini belleğe yüklemez.
- SQLite önbelleği sınırlıdır (`PRAGMA cache_size`); Ubuntu'da systemd `MemoryMax=` ile sert sınır konur.
- Hedef: Normal gece yükünde (birkaç yüz senaryo, ekleriyle) sunucunun bellek kullanımı 100 MB'ın altında kalır. Hedef değer yapılandırmadan değiştirilebilir.
- Kabul: Büyük bir zip (yükleme sınırına yakın) yüklenirken bellek kullanımı hedefin altında kalır; bu durum bir testle ölçülür.
- Test: B, P

### SW-10.7 [YAPILACAK] Log içeriği
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

### SW-10.8 [YAPILACAK] Disk doluluğu ve temizlik
- Disk %80 dolunca admin panelinde ve logda uyarı çıkar.
- Disk %90 dolunca en eski koşumlardan başlayarak önce videolar, sonra ekran görüntüleri silinir; doluluk %80'in altına inince durur.
- Jira issue'su olan senaryoların ekran görüntüsü Jira'da durduğu için silinebilir; senaryoda Jira issue numarası kalır ve silinen ekin yerinde "Silindi, bkz. JIRA-123" yazar.
- Koşum, senaryo, adım ve triage kayıtları ile geçmiş hiçbir zaman silinmez.
- Eşik değerleri yapılandırmadan değiştirilebilir.
- Kabul: Eşik aşılınca doğru sırayla silinir; rapor sayfaları silinen ekler için hata vermez.
- Test: B

### SW-10.9 [YAPILACAK] `stajci` dağıtımı ve sürüm uyumu
- stajproje sunucusu Linux ve Windows için `stajci` ikilisini `GET /api/tools/stajci?os=linux|windows` ucundan sunar; CI şablonu aracı buradan indirir.
- `stajci` her istekte kendi sürümünü gönderir; sunucu uyumsuz sürümü açık bir mesajla reddeder.
- Kabul: Sunucu güncellenince ertesi koşumda runner'lar yeni `stajci`'yi kullanır; eski sürüm "stajci sürümü uyumsuz" mesajıyla düşer.
- Test: B, P

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

1. GitLab sunucusu stajproje makinesine erişebiliyor. Pipeline durumu webhook ile alınır (SW-3.6).
2. GitLab runner ve stajproje sunucusu aynı makinede çalışır. CI işleri stajproje'ye yerel adresten ulaşır (SW-3.2 ve diğer CI yüklemeleri).
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

1. stajproje'ye pipeline tetikleme ve okuma yetkisi olan bir GitLab token verilebilir mi? (SW-7.4, 7.5)
2. Ubuntu ve Windows runner'ları ayrı makinelerdeyse her makinede ayrı bir stajproje sunucusu mu çalışacak, yoksa tek bir merkezi sunucu mu olacak? Ayrı sunucular raporları ve geçmişi ikiye böler. (SW-3.2, SW-3.6, SW-4.2)
3. Triage nasıl çalışacak? Kategoriler doğru mu, OpenCode'un kararı kendiliğinden mi geçerli olsun yoksa admin onayı mı beklesin, düşük güvenli kararlar ne olsun? (SW-4.5)
