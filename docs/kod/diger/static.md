# crates/raporcu/static — statik dosyalar

İkiliye gömülür (rust-embed, `debug-embed` ile hata ayıklama derlemesinde de), `/static/<ad>`
adresinden `Cache-Control: public, max-age=3600` ile sunulur. Dış bağlantı (CDN) yok; kapalı ağda çalışır.

## app.css
Tek stil dosyası. Renkler `:root` değişkenleri; `prefers-color-scheme: dark` için ayrı değerler.
Durum renkleri renk körlüğü için sınanmış palet: geçti `#119c8c`, kaldı `#d03b3b`, broken
`#e8a33a` (koyu temada `#c47f1a`), bilinmiyor/atlandı `#7b6fd0`. Sınıf adları Türkçe:
`.kart`, `.baslik`, `.tablo`, `.durum.<gecti|kaldi|broken|kosuyor|bekliyor…>`, `.m-<durum>`
(grafik dolguları), `.ince-cubuk`, `.ozet-donut`, `.lejant-liste`, `.matris`, `.simdi` (canlı kart),
`.canli-adimlar`, `.kosuyor-nokta` (yanıp sönen nokta), `.hata-ozet`, `.kucuk-resim`.

## charts.js
Sunucunun ürettiği SVG grafiklere etkileşim ekler (grafikler JS olmadan da okunur):
- `data-tip` taşıyan her öğe: fare/odakta ipucu kutusu (`|` ile ayrılmış satırlar, **textContent**
  ile yazılır).
- `svg.cizgi-grafik[data-noktalar]`: çizgi grafikte fareyle dikey artı imleç ve en yakın noktanın
  ipucu.

## htmx.min.js
htmx kütüphanesi (yerel kopya). Admin formları ve sayfa parçaları için.
