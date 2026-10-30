# raporcu/src/triage.rs

**Amaç:** SW-4.5 — kalan senaryonun nedenine karar. Karar kaynakları ve önceliği:
**admin > OpenCode (güven eşiği üstü) > kural**. Hiçbir karar silinmez (`triage_decisions` geçmişi).

## Kategoriler
| Kod | Etiket | Sonraki adım (fix.rs) |
|-----|--------|-----------------------|
| `flaky` | Flaky | atlanır |
| `test_kodu` | Test kodu hatası | OpenCode/Claude düzeltmesi |
| `urun` | Ürün hatası | Jira |
| `ortam` | Ortam sorunu | bildirim |
| `bilinmiyor` | Bilinmiyor | Allure durumuna göre |

## Kural kararı — `rule_decision(durum, mesaj, iz, flaky) -> (kategori, gerekçe)`
Sırayla:
1. Senaryo flaky ise → `flaky` ("Son koşumlarda hem geçti hem kaldı").
2. Mesaj+izde bilinen ortam kalıbı varsa → `ortam`. Kalıplar (`ENV_PATTERNS`): TimeoutException,
   "timed out"/"Timed out", "zaman aşımı", ConnectException, "Connection refused",
   UnknownHostException, NoRouteToHostException, SocketTimeoutException, "Can't connect to X11",
   "cannot open display", "Unable to open DISPLAY", HeadlessException, "No X11 DISPLAY",
   "Failed to initialize the display".
3. `broken` → `test_kodu`; `failed` → `urun`; diğer → `bilinmiyor`.

## Fonksiyonlar
- `add(c, senaryo, kategori, kaynak, gerekçe, güven)`: kaynak `kural|opencode|admin`; gerekçe 2000
  karakter. Log: `triage kararı` (kategori, kaynak).
- `history(c, senaryo)`: tüm kararlar, en yeni önce.
- `current(c, senaryo, eşik)`: sırayla en yeni admin kararı; yoksa güveni ≥ eşik (0.7) olan en yeni
  OpenCode kararı; yoksa en yeni kural kararı.
- `refresh(c, senaryo, eşik)`: geçerli kararı `scenarios.triage_category/triage_source`'a yazar
  (aramada süzmek için).
- `apply_rules(c, koşum, pencere, eşik)`: koşum bitince, kural kararı olmayan her kalan son
  deneme için kural kararı ekler ve tazeler.

## Varsayımlar (açık soru 3)
`docs/KARARLAR.md`.

## Testler
`test_sw_4_5_rule_categories`; `test_sw_4_5_triage_rule_opencode_admin_precedence_and_history`.
