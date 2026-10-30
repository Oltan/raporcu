# divan/src/fix.rs

**Amaç:** Self-healing'in sunucu tarafı: SW-7.1 aday sınıflandırma, SW-7.3 sonuç kaydı, SW-7.7
geçmiş düzeltme örnekleri.

## `target(durum, tag'ler, triage) -> hedef`
- `@sample-fail` tag'i → `atla`.
- Triage: `test_kodu` → `opencode`, `urun` → `jira`, `flaky` → `atla`, `ortam` → `bildirim`.
- Triage yok / `bilinmiyor`: `broken` → `opencode`, `failed` → `jira`, diğer → `atla`.

## `candidates(c, paths, koşum, public_url)` → `GET /api/ci/runs/{id}/fix-candidates`
Koşumun kalan (failed/broken) son denemeleri; her biri için: hedef, mesaj, iz (6000 karakter),
adımlar (`"metin [durum]"`), ilk ekran görüntüsünün tam URL'si (`<public_url>/files/<id>`),
tag'ler, ilk 3 küçük **metin eki** (her biri 16 KB, ör. TestFX sahne ağacı) ve hedef opencode
ise en benzer 2 örnek.

## `examples(c, proje, ad, mesaj, n)` (SW-7.7)
Aynı projede `mr_state='merged'` ve diff'i olan son 200 düzeltme arasından benzerlik:
ad+mesaj kelimeleri (≥3 harf, salt rakam değil, küçük harf) üzerinde **Jaccard**; 0.1'in üstündeki
en yüksek n tanesi. Diff 8000, mesaj 500 karakterle kesilir.

## `record(c, koşum, FixResult, eşik)` → `POST /api/ci/runs/{id}/fix-result`
`fix_attempts` satırı (MR varsa `mr_state=opened`, diff 200 KB). Log: `OpenCode denemesi bitti`.
`triage` önerisi geçerli bir kategoriyse `opencode` kaynaklı karar olarak eklenir (güven 0–1'e
kırpılır) ve geçerli karar tazelenir.

## Testler
`test_sw_7_1_target_for_each_case`; `tests/entegrasyon.rs` (7.1, 7.3/4.4, 7.5, 7.7).

## Açık MR kontrolü
`open_fix_mr(c, proje, scenario_key)`: son 14 günde `mr_opened`/`draft_mr` sonuçlu ve `mr_state`
`opened` (ya da webhook gelmediği için boş) deneme varsa URL/branch. `candidates` bu durumda hedefi
`opencode` yerine `acik-mr` yapar; ulak atlar (senaryo düğmesi `ULAK_FIX_SCENARIO_ID` yine çalışır).
