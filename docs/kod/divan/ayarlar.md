# divan/src/ayarlar.rs

**Amaç:** Arayüzden düzenlenen ayarlar: Jira bug şablonu (SW-6.1) ve DOORS bağlantısı (SW-6.4).
Kullanıcı rehberi: `docs/JIRA_DOORS_AYARLARI.md`.

## Saklama
- `settings(key, value JSON, updated_at)` tablosu (migration 0003). Anahtarlar: `jira`, `doors`.
- `load(c)`: iki anahtarı okur; kayıt yoksa varsayılan. Bozuk JSON → hata (AppState açılışta
  WARN basıp varsayılanla devam eder).
- `Store`: `Arc<RwLock<Ayarlar>>`, AppState'te. Her istek `get()` ile okur; admin kaydedince
  önce veritabanına, sonra belleğe yazılır (`set`). Yeniden başlatma gerekmez.
- `save_jira`: ek alanlar JSON nesnesi değilse reddeder. `save_doors`: kimlik özniteliği boşsa reddeder.

## JiraSablon (varsayılanlar)
| Alan | Varsayılan |
|------|-----------|
| `ozet` | `[Otomasyon] {{senaryo}}: {{hata_ilk_satir}}` |
| `aciklama` | senaryo, feature, DOORS, proje/branch/commit, rapor linki, `{noformat}` içinde hata, `*Adımlar*` + `{{adimlar}}` |
| `alanlar` | `{}` — Jira REST v2 `fields` biçiminde ek alanlar |
| `doors_label` | true |

`YER_TUTUCULAR`: senaryo, feature, doors, hata, hata_ilk_satir, adimlar, rapor, kosum_rapor, proje,
branch, commit, kosum, tarih (değerleri `jira::vars` üretir).

## Yardımcılar
- `fill(metin, değerler)`: `{{ad}}` → değer; bilinmeyen yer tutucu ve tek süslü parantez (`{noformat}`) aynen kalır.
- `fill_json(v, değerler)`: JSON yapısını bozmadan yalnızca metin değerlerde doldurur (kaçış sorunu olmaz).
- `extra_fields(metin)`: boş → boş nesne; JSON nesnesi değilse açıklayıcı hata.

## DoorsAyar
CSV için yalnızca sonuç değerleri ve link şablonu kullanılır; modül ve öznitelik alanları DXL içindir (`DOORS_DXL`), kapalıyken formda gizli alan olarak taşınır.
`modul` (boş = açık modül), `id_oznitelik` (Absolute Number), `sonuc_/tarih_/jira_/rapor_oznitelik`
(boş = yazılmaz), sonuç değerleri `gecti/kaldi/atlandi/kosulmadi`, `link_sablonu` (`{{no}}`, `{{id}}`).
- `status(allure)`: passed → gecti, failed/broken → kaldi, skipped → atlandi, diğer → kosulmadi.
- `link(doors_id)`: şablon boşsa yok.

## Testler
`test_sw_6_1_template_fill_keeps_jira_braces_and_json_structure`, `test_sw_6_4_doors_link_and_status_labels`.

## Python karşılığı
`dataclasses` + `json`; `Store` için `threading.RLock`. `fill` için `str.replace` döngüsü yeterli (Jinja kullanmayın: `{noformat}` ile karışır).
