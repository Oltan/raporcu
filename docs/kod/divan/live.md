# divan/src/live.rs

**Amaç:** SW-3.4 canlı koşum ekranı. Ayrıntılı akış: `docs/ANLIK_AKTARIM.md`.

## Bellekteki "şu an" durumu — `LiveStates` (koşum → `Current`)
`apply(koşum, olaylar[])` Cucumber eklentisinin olaylarını sırayla uygular:
- `senaryo_basladi`: ad, feature, uri, satır; `active=true`; adımlar sıfırlanır; aynı `uri|satır`
  kaçıncı kez başladıysa `attempt` (rerun); başlangıç anı (sunucu saati).
- `adim_basladi`: `step` (300 karakter), `step_no`.
- `adim_bitti`: biten adımlar listesine `{metin, durum, ms, hata(500)}`; en fazla 100 tutulur.
- `senaryo_bitti`: `finished` listesine `(ad, feature, durum, uri|satır)`; `active=false`.
- Tanınmayan olay yok sayılır. `get` geçen süreyi (sn) hesaplar. `remove` koşum bitince.

## Anlık görüntü — `snapshot(c, koşum, current?)`
Sayaçlar + beklenen senaryo listesi (`expected_scenarios`, sırayla). Her beklenen senaryo için:
1. Kullanılmamış bir Allure sonucu adı ve feature'ı eşleşirse → onun durumu (`gecti/kaldi/
   atlandi/bilinmiyor`), mesaj (500), kalan ise ilk ekran görüntüsü, deneme.
2. Yoksa olaylara göre bitmişse → olay durumu (`PASSED` geçti, `FAILED/AMBIGUOUS` kaldı, diğer
   bilinmiyor). Rerun'da aynı `uri|satır`'ın **son** durumu; taslak satırları ayrı.
3. Yoksa: olay varsa ve koşan senaryo bu ise `kosuyor` (kesin); olay hiç yoksa sonucu olmayan ilk
   beklenen `kosuyor` (tahmin). Diğerleri `bekliyor` (koşum bitmişse `bilinmiyor`).
Beklenmeyen sonuçlar listenin sonuna eklenir. `current` yalnızca koşum `running` iken döner.

## WebSocket — `GET /ws/runs/{id}`
Bağlanınca bir görüntü; sonra o koşum için her yayında 250 ms bekleyip biriken sinyalleri
birleştirerek yeni görüntü; geride kalınırsa (lagged) yine gönderir; istemci kapatınca biter.

## Testler
`test_sw_3_4_rerun_last_status_wins_outline_rows_separate`, `test_sw_3_4_apply_real_events_counts_scenarios`;
`tests/rapor.rs`: canlı görüntü, WebSocket, gerçek Cucumber olaylarıyla şu an koşan senaryo/adım.

## Python karşılığı
`dict[int, Current]` + `threading.Lock`; WebSocket için Starlette `WebSocket` ve `asyncio.Queue`
tabanlı yayın.
