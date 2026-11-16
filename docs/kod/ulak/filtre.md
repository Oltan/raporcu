# ulak/src/filtre.rs

**Amaç:** SW-7.2 — tek senaryoyu seçen Cucumber filtresi. Doğrulama koşusu (`ulak fix`) ve
auto-fix branch'inin pipeline'ı (`ULAK_SENARYO` / `ULAK_TAGS`, `runner.md`) aynı kuralı kullanır.

| Öğe | Davranış |
|-----|----------|
| `Filtre::Ad(ad)` | `-Dcucumber.filter.name=^<ad>$` (`ad_ifadesi`: `fix::regex_quote`, ASCII olmayan harf `.`) |
| `Filtre::Taglar(ifade)` | `-Dcucumber.filter.tags=<ifade>` |
| `tag_ifadesi(tags)` | `@a and @b`: baştaki `@` atılıp yeniden eklenir, tekrarlar atılır; yalnızca ASCII ve boşluk/parantez/`@`/`\\` içermeyen tag'ler. Hiç yoksa `None` |
| `atilan_taglar(tags)` | ifadeye giremeyenler (uyarı için) |

Neden ASCII: runner ASCII dil ayarında (Linux'ta ulak `LC_ALL=C.UTF-8` verir, Windows `mvn.cmd` kod
sayfası) Java komut satırındaki Türkçe harfleri bozabilir; tag ifadesinde joker olmadığı için ad
filtresindeki `.` çaresi işlemez.

## Testler
`test_sw_7_2_tag_ifadesi_ascii_disi_atilir`, `test_sw_7_2_filtre_argumanlari`,
`test_sw_7_2_dogrulama_filtresi_ad_ya_da_tag` (fix.rs).

## Python karşılığı
`" and ".join(f"@{t}" for t in tags if t.isascii() and not set(t) & set("()@\\ "))`; ad için `re.escape`
yerine aynı kaçış kümesi ve ASCII dışı → `.`.
