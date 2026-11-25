# ulak/src/gherkin.rs

**Amaç:** JVM açmadan `.feature` dosyalarından senaryo listesi ve Cucumber tag ifadeleri (SW-3.1
toplam sayı, SW-1.5 features.txt).

## Ayrıştırma — `parse_feature(metin, dosya)`
Satır satır; `"""`/```` ``` ```` arası docstring atlanır; `#` yorum. Anahtar kelimeler (TR + EN):
`Feature/Özellik/Ability/Business Need`, `Rule/Kural`, `Background/Geçmiş`,
`Scenario Outline/Scenario Template/Senaryo taslağı`, `Scenario/Example/Senaryo/Örnek`,
`Examples/Scenarios/Örnekler`. `@` ile başlayan satır sonraki öğenin tag'leri.
- Senaryo → 1 kayıt; tag'ler = feature + rule + kendi.
- Senaryo taslağı → her `Örnekler` tablosunun **başlık dışı her satırı** 1 kayıt; tag'ler =
  feature + rule + taslak + örnekler bloğunun tag'leri.
Kayıt: `{ad, feature, dosya, tag'ler}`.

`plan(proje, ifade?)`: `src/test` altındaki tüm `.feature` dosyaları (`target` ve `.` ile başlayan
klasörler hariç), isteğe bağlı tag süzgeci.

## Tag ifadesi — `TagExpr::parse`
Dilbilgisi: `or` < `and` < `not` < atom (`@tag` ya da parantez). `@` ile başlamayan tag, eksik
parantez, fazla parça → açık hata. `eval(tag kümesi)`, `tags()` (ifadedeki tag adları).

## `read_features_txt(yol)`
Her satır bir ifade, `#` sonrası yorum, boşlar atlanır; satırlar `(a) or (b)` olarak birleşir.
Dosya yoksa ya da boşsa yok.

## Testler
`test_sw_3_1_counts_scenarios_and_outline_rows` (örnek proje: 7), `test_sw_3_1_tag_filter`,
`test_sw_1_5_tag_expression_errors`, `test_sw_3_1_examples_tags_and_rules`.

## Python karşılığı
Aynı satır tabanlı ayrıştırıcı ~150 satır; tag ifadesi için küçük özyinelemeli ayrıştırıcı. Resmî `gherkin-official` paketi de kullanılabilir (Türkçe anahtar kelimeleri destekler).
