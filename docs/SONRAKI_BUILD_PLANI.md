# Bir sonraki build planı (divan.exe + ulak.exe)

## Context

Pilot kullanımda çıkan eksikler tek build'de toplanıyor. Flaky kuralı zaten yazıldı (`ad23659`).
Kullanıcının onayladığı kalemler: gereksinim öneki (YTET_) ve ek kimlikler (YGÖ-), Jira arama kriteri, doğrulama filtresi (tag),
Claude yolu + ağaç sınırı; kalem 5 (auto-fix'te yalnızca düzeltilen senaryo). Dal: `llm-degerlendirme`. Durum: **onay bekliyor** (kalem 1: YTET_ gereksinim no, YGÖ- ek kimlik), henüz uygulanmadı.

## Sorularının kod gerektirmeyen yanıtları

- **Hangi tag'ler iletiliyor:** Bütün tag'ler saklanır, raporda görünür ve aranır; kendi tag'lerini
  serbestçe ekleyebilirsin. Özel anlam taşıyan yalnızca ikisi: `@DOORS-…` (gereksinim numarası; kalem 1 ile `@YTET_…`) ve
  `@JIRA-ABC-12` (issue'ya doğrudan bağlama). `@YTET_1234` şu an sıradan tag sayılıyor (kalem 1).
- **Jira eşleşmesi:** Evet, yalnızca `failed` senaryolar için çalışır; `broken`, geçen ve atlanan
  senaryolara dokunmaz.
- **`ulak degerlendir` yml'da:** Depodaki şablonda fix işi zaten `ulak degerlendir` + `ulak fix`
  koşuyor. Projendeki `.gitlab-ci.yml` eski bir kopyaysa `ulak-fix-windows` işinin `script`
  bölümüne `- ulak fix` satırından önce `- ulak degerlendir` eklemen yeter; ek kod gerekmez.
- **Triage dar mı:** Kural kararı dar (`failed` → ürün, `broken` → test kodu). Bunu genişleten
  adım `ulak degerlendir`: ajan `failed` bir senaryoya `test_kodu` derse (güven ≥ 0,70) senaryo
  düzeltmeye gider. Yeni flaky kuralıyla flaky denenler de değerlendirilir (yeni `divan.exe` ile).
- **Süreç ağacı:** Depodaki kod zaman aşımında ağacı sonlandırıyor (`taskkill /T /F`); bulgu eski
  sürümden olabilir. Kod eklenmeyecek; yeni exe ile bir zaman aşımı denemesinde doğrulanacak.
- **Hangi jar:** `divan-test-destek-1.0.0.jar` (paketteki `java/` klasörü). İçinde
  `DivanKanit`, `CanliOlayEklentisi`, `VideoKaydi` var. Sahne ağacı düzeltmesi için bu jar
  yeniden üretilmeli (ya da `DivanKanit.java` projeye kaynak olarak kopyalandıysa dosya yenilenmeli).
- **`sslVerify=false` uyarısı:** Depoda böyle bir ayar yok; runner makinesinin git ayarından
  geliyor. Kaldırıp CA'yı tanıtmak doğru yol: `DIVAN_CA_DOSYASI` (ulak bunu `git push`'a
  `http.sslCAInfo` olarak verir) ya da Windows'ta `git config --system http.sslBackend schannel`.
- **Java testleri ne işe yarar (bu plana dahil değil):** Birim testi, sahne ağacı düzeltmesinin
  ileride yanlışlıkla bozulmasını yakalar. fxId lint, `fx:id`'siz düğme/alanları listeler; id yoksa
  ajan güvenilir seçici yazamaz. İkisi de exe gerektirmez; istersen ayrıca eklenir.

## Kalemler

### 1. Gereksinim kimliği öneki (YTET_) ve ek kimlikler (YGÖ-) (divan.exe)
Karar: Gereksinim numarası `@YTET_1234` diye yazılır; `DOORS-` öneki kullanılmayacak. `@YGÖ-…`
senaryonun ek olarak bağlandığı gereksinimi gösterir, eşleşmenin kaybolmaması için tutulur.
- **Gereksinim öneki** `GEREKSINIM_ONEKI` (`crates/divan/src/config.rs`; varsayılan `DOORS-`,
  geriye uyum için; sahada `YTET_`). Tek önek: ayarlanınca bugün `DOORS-`'un yaptığı her şeyi
  o yapar: `doors_ids` sütunu, rozet, arama, CSV "DOORS numarası"
  sütunu, link şablonu (`{{no}}` = önekten sonrası, `{{id}}` = `YTET_1234`), DXL; Jira'ya
  yalnızca açıklama yer tutucusu olarak gider (aşağıda).
- `crates/common/src/allure.rs` `doors_id(tag, onek)`: kimlik yazıldığı gibi saklanır
  (`YTET_1234`); önekten sonrası harf/rakam/`-`/`_`, basamak sınırı yok. `DOORS-` için mevcut
  büyük harf normalleştirmesi korunur. `defterdar.rs:141` ayarı geçirir.
- Öneki söken/ekleyen üç yer ortak yardımcıya bağlanır: `queries.rs:609` (arama: yalnızca rakam
  girilirse önek eklenir), `ayarlar.rs:109` (`{{no}}`), `export.rs:271` (DXL; kapalı ama tutarlı).
- **Ek kimlikler** `EK_KIMLIK_ONEKLERI` (varsayılan boş; sahada `YGÖ-`): `doors_ids`'e ve Jira'ya
  girmez; değer saklanan `tags` sütunundan okunur (migration yok, eski koşumlarda da görünür).
  `ek_kimlik(tag, onek)` önek büyük/küçük harfe duyarlı, kimlik yazıldığı gibi.
  - CSV: her önek için ayrı sütun, başlık önekin sondaki `-`/`_` atılmış hâli (`YGÖ`), alan
    anahtarı `ek:YGÖ-`; YTET sütunundan sonra. `parse_fields` bu dinamik alanları tanır.
  - Jira şablonunda yer tutucu (aşağıda, "Jira'da YTET/YGÖ").
  - Rapor: rozet (`queries.rs:157` "sıradan tag" süzgeci bunları da ayırır) ve arama (tam tag
    eşleşmesi, `YGÖ-12` `YGÖ-123`'ü getirmez).
- **Jira'da YTET/YGÖ** (karar: yalnızca açıklamaya yazılır; label yok, Jira'da alan yok, aranmaz):
  - Yer tutucular önek adıyla: `{{YTET}}` → `YTET_1234, YTET_1240`, `{{YGÖ}}` → `YGÖ-12`
    (`jira.rs:401` `vars`; anahtarlar `String` olur). Eski `{{doors}}` aynı değeri verir, mevcut
    şablonlar bozulmaz. Önek tanımlı ama senaryoda yoksa boş.
  - Label yok: `build_bug` gereksinim kimliklerini label olarak eklemez; `doors_label` ayarı ve
    ayarlar/bug sayfasındaki "DOORS numaralarını label olarak ekle" kutusu kaldırılır (kayıtlı
    ayarda `true` olsa da yok sayılır).
  - `find_open_bug` `labels in (<YTET…>)` koşulunu bırakır; eşleşme senaryo adı + kalem 2'deki
    issuetype/`divan` label'ı ile yapılır.
- **Arayüz ve çıktıda "DOORS" adı kalkar** (karar: kafa karıştırıyor; çıktıda YTET görünsün):
  kullanıcıya görünen "DOORS" yazıları önek adını kullanır (önekin sondaki `-`/`_` atılmış hâli,
  `YTET`). Koşum sayfası düğmesi "YTET CSV", o CSV'nin "DOORS ID" sütunu "YTET", arama CSV'sinin
  "DOORS numarası" sütunu "YTET", arama kutusu etiketi/örneği (`YTET_1001`), rozet ipucu,
  ayarlar sayfasındaki DOORS bölümü ("Gereksinim (YTET)", link şablonu açıklaması). Adres
  (`/runs/{id}/doors.csv`), alan anahtarı (`doors`), veritabanı sütunu ve `.env` adları
  değişmez (iç adlar). Kullanıcının aktarım betiği "DOORS ID" başlığına bakıyorsa güncellenmeli
  (`docs/JIRA_DOORS_AYARLARI.md`'de not).
- Eski koşumlar yeniden işlenmez: önek değişmeden önce yüklenen koşumlarda `YTET_` tag'leri
  gereksinim numarası olarak görünmez (tag olarak durur).
- Testler: `YTET_` önekiyle tanıma, `{{YTET}}`/`{{YGÖ}}`/`{{doors}}` doldurma, bug'da label
  olmaması, link şablonu, CSV; ASCII olmayan `YGÖ-` için
  tanıma, CSV başlığı/değeri (UTF-8 BOM ile), arama; varsayılan ayarla bugünkü `DOORS-` testleri
  değişmeden geçer.

### 2. Jira arama kriteri (divan.exe)
- `crates/divan/src/jira.rs` `find_open_bug`: JQL'e `issuetype = "<JIRA_ISSUE_TYPE>"` ve
  `labels = "<JIRA_MATCH_LABEL>"` eklenir (yeni ayar, varsayılan `divan`).
- `create_bug`: aynı label açılan her bug'a eklenir (label reddedilirse mevcut label'sız yeniden
  deneme yolu aynen çalışır).
- Sonuç: story/task eşleşmez, fotoğraf yalnızca divan'ın açtığı ya da bilerek etiketlenen bug'a
  gider. Önceden açılmış etiketsiz bug'lar Jira aramasıyla eşleşmez (yerel parmak izi bağı sürer).

### 3. Doğrulama koşusu filtresi (ulak.exe)
- `crates/ulak/src/fix.rs` `validate_command`: `ULAK_FIX_VALIDATE_FILTER=name|tags`
  (varsayılan `name`, bugünkü davranış). `tags` iken `-Dcucumber.filter.tags=(@a and @b …)`
  verilir (adayın tag'leri, `FixCandidate.tags`); tag yoksa uyarıyla ada düşülür.
- `ULAK_FIX_VALIDATE_CMD` içinde `{senaryo}` ve `{taglar}` yer tutucuları (mevcut `subst`
  düzeniyle).
- Not: aynı tag kümesini taşıyan başka senaryolar da koşar; senaryoya özgü tag (DOORS no) varsa tek
  senaryo koşar.
- ASCII olmayan tag (`@YGÖ-12`): runner ASCII dil ayarında Java komut satırı bozulur ve tag
  ifadesinde joker yok (`regex_quote` çaresi işlemez); bu tag'ler ifadeden çıkarılır, hiç tag
  kalmazsa ada düşülür (uyarıyla).
- Windows'ta `mvn.cmd` üzerinden parantez/boşluklu argüman gerçek runner'da denenir.

### 4. Claude yolu, ön kontrol, ağaç sınırı
- `crates/ulak/src/fix.rs` `agent_command`, `degerlendir.rs` `command_for`: `ULAK_CLAUDE_BIN`
  (varsayılan `claude`).
- `crates/ulak/src/preflight.rs` `kod_ajani_kontrol`: ajan `claude` iken `.claude/settings.json`
  ve `CLAUDE.md` yoksa uyarı; skill denetimi iki ajan için ortak.
- `crates/divan/src/fix.rs:142`: metin eki sınırı 16 KB → 64 KB (sahne ağacı artık daha uzun).
  Ek başına 64 KB, toplamda en fazla 96 KB (3 × 64 KB küçük bağlamlı vLLM modelinde istemi
  taşırabilir); `docs/KOD_AJANI_VLLM.md`'ye `max-model-len` notu.
- `ULAK_CLAUDE_BIN` boşluklu yol olabilir (`C:\Program Files\…`): şablona tırnaklı girer
  (`split_command` bölmesin); testle doğrulanır.

### 5. auto-fix branch'inde yalnızca düzeltilen senaryo (ulak.exe)
Durum: sahada fix push'undan sonra tüm testler koşuyor; projedeki `.gitlab-ci.yml` eski kopya
(şablondaki auto-fix kuralı yok). Karar: `ulak fix` push'ta GitLab'a pipeline değişkeni verir
(`git push -o ci.variable=…`); değişken yalnızca o push'un açtığı pipeline'a gider.
- `ULAK_FIX_PIPELINE_FILTRE` (ulak, varsayılan `off`): `senaryo` iken push'a
  `-o ci.variable=ULAK_FIX_PIPELINE=on` ve filtre eklenir:
  - senaryonun YTET (gereksinim önekli) tag'i varsa `ULAK_TAGS=@YTET_1234` (birden çoksa `or`);
  - yoksa `ULAK_SENARYO=<senaryo adı>` (YTET tag'i henüz her senaryoda yok).
  Gereksinim öneki ulak'a `GET /api/ci/fix-candidates` yanıtında gelir (aday zaten
  `doors_ids` taşıyor; ulak'ta ayrı ayar yok).
- `ulak start` / `runner.rs`: `ULAK_SENARYO` doluysa `-Dcucumber.filter.name=^…$`
  (`regex_quote`), `ULAK_TAGS` ve `features.txt`'nin önüne geçer; beklenen senaryo sayısı da
  `.feature` dosyalarından aynı adla sayılır.
- Şablon değişmez: auto-fix kuralı `ULAK_FIX_PIPELINE=on` ile test/report'u zaten açıyor, fix işi
  auto-fix'te asla koşmaz. Eski kopya kullananlar için `docs/KURULUM.md`'ye kural satırları.
- Sınır: MR'a sonradan elle eklenen commit'in pipeline'ı değişkeni almaz (bugünkü gibi test yok).
- Test: `test_sw_7_2_push_ci_degiskenleri` (tag/ad seçimi, argümanlar),
  `test_sw_1_5_ulak_senaryo_filtresi` (runner filtresi ve sayım); rehearse'te sahte GitLab push
  argümanları `ck.check("SW-7.2", …)`.

## Her kalemde ortak işler
- Test adı `test_sw_<bölüm>_<madde>_…`: 1 → SW-6.4 (CSV) / 2.x, 2 → SW-6.2, 3 → SW-7.2, 4 → SW-7.2/7.6, 5 → SW-7.2 (fix pipeline).
  Mevcut `test_sw_6_2_auto_match_after_upload` sahte Jira'sı yeni JQL'e göre güncellenir.
- Belgeler: `GEREKSINIMLER_1.md`, `docs/KARARLAR.md`, `docs/JIRA_DOORS_AYARLARI.md`,
  `docs/KOD_AJANI_VLLM.md`, `docs/KURULUM.md`, ilgili `docs/kod/*`, `.env.example`,
  `ci/gitlab/runner/runner.env.example`. Sunucu sözleşmesi değişmiyor (python iskeleti aynı).
- Her kalem ayrı commit; `surum/windows` bu oturumda yeniden derlenmez (araç zinciri yok), exe'leri
  sen derlersin.

## Doğrulama
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `cargo run -p rehearse`.
- Örnek proje: `cd ornek-test-projesi && xvfb-run -a mvn -B test` (tag filtresiyle doğrulama
  komutu elle denenir).
- Windows hedefi bu ortamda koşturulamıyor; gerçek runner'da yapılacaklar: zaman aşımında süreç
  ağacının kapandığı, `ULAK_FIX_VALIDATE_FILTER=tags` ile özel runner'ın tek senaryo koştuğu,
  yeni Jira sorgusunun story ile eşleşmediği.
