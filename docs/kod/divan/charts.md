# divan/src/charts.rs

**Amaç:** Sunucu tarafında SVG grafik üretimi (JS'siz çizim; ipucu/takip çizgisi `static/charts.js`).

## Ortak kurallar
İnce işaretler (çizgi 2px, sütun ≤24px, veri ucu 4px yuvarlak, taban düz), dokunan parçalar
arasında 2px yüzey boşluğu, işaretçilerde 2px yüzey halkası, sönük 1px ızgara, **tek eksen**.
Renkler CSS sınıfıyla (`m-gecti`, `m-kaldi`, `m-broken`, `m-diger`, `m-bos`) gelir, metin hiçbir
zaman veri renginde değildir. Her işaret `data-tip="Ad: değer (%x)"` ve `tabindex=0` taşır.
Durum paleti renk körlüğü denetiminden geçti (açık: `#119c8c #d03b3b #e8a33a #7b6fd0`; koyu temada
broken `#c47f1a`). Tüm veri metni `esc()` ile HTML kaçışlanır.

## Fonksiyonlar
| Fonksiyon | Çıktı |
|-----------|-------|
| `pct(parça, toplam)` | yuvarlanmış yüzde (toplam 0 → 0) |
| `status_slices(geçti, kaldı, broken, diğer)` | 4 dilim (sınıf, ad, değer) |
| `genel_slices(doğrulanan, ürün, inceleniyor)` | genel rapor için 3 dilim |
| `donut(dilimler, orta_değer, orta_etiket, aria)` | 200×200 halka (r=78, kalınlık 22); dilimler arası 2px açısal boşluk; tek dilimse tam halka; ortada değer ve etiket |
| `thin_bar(dilimler)` | 8px yüksek, uçları yuvarlak, flex oranlı HTML çubuk |
| `line_pct(noktalar, aria)` | 720×220, y 0–100 (%), ızgara 0/25/50/75/100, en fazla ~8 x etiketi, %10 alan dolgusu, uç değer etiketi; nokta verisi `data-noktalar` JSON'unda (takip çizgisi için) |
| `stacked_columns` / `stacked_columns_sized(.., w, h)` | yığılmış sütunlar; y üst sınırı `nice_max`; en fazla ~10 x etiketi; en üst parça 4px yuvarlak; isteğe bağlı sütun linki |
| `sparkline(değerler, aria)` | 110×28 mini çizgi, son nokta vurgulu |

`nice_max(v)`: 4 eşit aralıklı tam sayı ızgara için adım ∈ {1,2,3,4,5,6,8}×10ⁿ, üst = 4×adım ≥ v
(21 → 24, 7 → 8, 150 → 160). En az 4.

## Testler
`test_sw_4_2_donut_segments_and_center`, `test_sw_4_2_line_and_columns_escape_and_scale`.

## Python karşılığı
Aynı SVG dizeleri f-string ile; ya da Jinja2 makroları.
