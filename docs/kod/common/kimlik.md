# common/src/kimlik.rs

**Amaç:** SW-2.7, SW-6.4 — senaryo tag'lerinden gereksinim kimliği (`YTET_1234`, varsayılan
`DOORS-1234`) ve ek kimlikler (`YGÖ-12`).

## Ayar
- `Onekler { gereksinim, ek[] }`. `parse(GEREKSINIM_ONEKI, EK_KIMLIK_ONEKLERI)`: boş gereksinim öneki
  → `DOORS-`; ek önekler virgülle, baştaki `@` atılır, tekrarlar ve gereksinim önekiyle aynı olan atılır.
- Süreç genelinde tek ayar: `ayarla(o)` (divan `AppState::new`), `gecerli()` (ayarlanmadıysa
  varsayılan). Rapor şablonları, CSV ve Jira bunu okur. Saf yöntemler testlerde genel ayara
  dokunmadan kullanılır; farklı önekli entegrasyon testi ayrı test ikilisidir (`tests/kimlik.rs`).

## Kurallar
| Yöntem | Davranış |
|--------|----------|
| `gereksinim_id(tag)` | `DOORS-` için eski kural: `DOORS-`/`doors-` + ASCII harf/rakam/`-`/`_` → `DOORS-…`. Diğer önekler: büyük/küçük harfe duyarlı önek + en az bir harf/rakam/`-`/`_` (Unicode harf dahil, basamak sınırı yok); kimlik yazıldığı gibi |
| `ek_kimlik(tag)` | aynı kural ek öneklerle; (önek sırası, kimlik) |
| `ek_kimlikler(tags)` | her ek önek için kimlik listesi (önek sırasıyla) |
| `kimlik_mi(tag)` | gereksinim ya da ek kimlik (düz tag listesinden ayrılır, paylaşımda gizlenmez) |
| `numara(id)` | önek atılmış (`{{no}}`, DXL) |
| `arama(girdi)` | tanınan önekle başlıyorsa olduğu gibi (`DOORS-` büyük harfe), değilse gereksinim öneki eklenir |
| `ek_onekli(değer)` | arama değeri ek önekli mi (o zaman `tags` sütununda aranır) |
| `ad(önek)` | sondaki `-`/`_` atılmış görünen ad: `YTET`, `YGÖ`, `DOORS` (CSV başlığı, Jira yer tutucusu, arayüz) |

## Testler
`test_sw_2_7_gereksinim_oneki_ytet`, `test_sw_2_7_ek_kimlik_ascii_olmayan_onek`,
`test_sw_4_7_arama_onek_normallestirme`, `test_sw_2_7_varsayilan_doors_davranisi_aynen`.

## Python karşılığı
`docs/python-iskelet/divan/allure.py` `doors_id(tag, onek)` (`GEREKSINIM_ONEKI` ortam değişkeni).
Ek kimlikler için aynı kural `str.startswith` + `isalnum` ile; genel ayar modül düzeyinde değişken.
