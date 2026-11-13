# Bir sonraki build planı (divan.exe + ulak.exe)

## Context

Pilot kullanımda çıkan eksikler tek build'de toplanıyor. Flaky kuralı zaten yazıldı (`ad23659`).
Kullanıcının onayladığı dört kalem: ek kimlik önekleri (CSV sütunu), Jira arama kriteri, doğrulama filtresi (tag),
Claude yolu + ağaç sınırı. Dal: `llm-degerlendirme`. Durum: **onay bekliyor** (kalem 1 CSV sütunu kararıyla güncellendi), henüz uygulanmadı.

## Sorularının kod gerektirmeyen yanıtları

- **Hangi tag'ler iletiliyor:** Bütün tag'ler saklanır, raporda görünür ve aranır; kendi tag'lerini
  serbestçe ekleyebilirsin. Özel anlam taşıyan yalnızca ikisi: `@DOORS-…` (DOORS numarası) ve
  `@JIRA-ABC-12` (issue'ya doğrudan bağlama). `@YTET_1234` şu an sıradan tag sayılıyor; kalem 1 ile CSV'de kendi sütununa çıkar.
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

### 1. Ek kimlik önekleri: YTET, YGÖ (divan.exe)
Karar: `DOORS-` kimlikleri bugünkü gibi kalır (`doors_ids` sütunu, Jira label'ı ve araması, CSV'nin
"DOORS numarası" sütunu, DXL). Ek önekler `doors_ids`'e **girmez**; değerleri saklanan `tags`
sütunundan okunur. Böylece Jira'ya ve DOORS sütununa sızmazlar, migration gerekmez ve eski
koşumlar da yeni sütunlarda görünür.
- `crates/divan/src/config.rs`: `EK_KIMLIK_ONEKLERI` (varsayılan boş; örn. `YTET_,YGÖ-`).
- `crates/common/src/allure.rs`: `ek_kimlik(tag, onek)`: önek büyük/küçük harfe duyarlı, önekten
  sonraki kısım harf/rakam (basamak sınırı yok), kimlik yazıldığı gibi (`YTET_1234`, `YGÖ-12`).
  `doors_id` değişmez.
- CSV (`crates/divan/src/export.rs`): her önek için ayrı sütun; başlık önekin sondaki `-`/`_`
  atılmış hâli (`YTET`, `YGÖ`), alan anahtarı `ek:YTET_` gibi; değer o önekli kimlikler, boşlukla
  ayrılmış. `FIELDS` sabit listesine ek olarak `parse_fields` bu dinamik alanları da tanır;
  alan seçilmezse tüm sütunlar (ek sütunlar DOORS'tan sonra) gelir. Hücreler `csv_cell`'den geçer.
- Rapor: senaryo satırında DOORS rozetleri gibi rozet (`queries.rs:157`'deki "sıradan tag" süzgeci
  ek önekleri de ayırır); arama kutusu: tanınan önekle başlayan girdi tag üzerinden aranır
  (tam tag eşleşmesi, `YTET_12` `YTET_123`'ü getirmez), yalnızca rakam girilirse bugünkü gibi
  `DOORS-` eklenir.
- ASCII olmayan önek (`YGÖ-`) için test: tanıma, CSV başlığı/değeri (UTF-8 BOM ile), arama.
- `ayarlar.rs:109` ve `export.rs:271` (DXL) değişmez.

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

## Her kalemde ortak işler
- Test adı `test_sw_<bölüm>_<madde>_…`: 1 → SW-6.4 (CSV) / 2.x, 2 → SW-6.2, 3 → SW-7.2, 4 → SW-7.2/7.6.
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
