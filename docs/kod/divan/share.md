# divan/src/share.rs

**Amaç:** SW-5.1 — paylaşım linkinde iç bilgileri gizleme.

- `sanitize_text(metin, gizliler)`: önce gizli değerler `***`; sonra kelime kelime: `glpat-`,
  `glrt-`, `glptt-` ile başlayan, `token=`, `password=`, `private-token`, `api_key=` içeren
  kelimeler `***`; `Bearer` / `Authorization:` sonrasındaki kelime `***`; Jira anahtarı biçimindeki
  kelime (`ABC-123`, DOORS hariç) `[Jira]`. Boşluklar korunur.
- `is_jira_key(w)`: en az 2 karakterli büyük harf/rakam proje + `-` + rakamlar; `DOORS` değil.
- `visible_tag(t)`: `JIRA-…` tag'leri gizlenir.

Paylaşım görünümünde ayrıca (web/pages.rs): pipeline/iş/repo linkleri, Jira rozetleri, bug
bağlantıları, OpenCode kutusu, triage gerekçeleri gizlenir; ekler `/s/<token>/files/<id>` üzerinden
ve yalnızca o koşumun ekleri sunulur; başka koşumun senaryosu 404.

## İki görünüm (`share_links.gorunum`)
- `ekip`: yukarıdaki gibi ayrıntılı koşum raporu (`run.html`, `scenario.html`).
- `genel`: ekip dışına verilen ayrı rapor (`paylasim.html`, `paylasim_senaryo.html`).

`genel_sinif(durum, deneme sayısı, flaky, triage)`:
- flaky → `Inceleniyor`;
- `passed` ve tek deneme → `Dogrulandi`;
- `failed`/`broken` ve triage `urun` → `UrunHatasi` (iki denemede de kalması görünürlüğü etkilemez);
- geri kalan her şey (tekrarda geçen, test kodu, ortam, bilinmiyor, atlanan) → `Inceleniyor`.

Genel raporda sonuç cümlesi, üç sayı, ürün hataları ve feature'a göre doğrulanan senaryolar vardır.
`Inceleniyor` yalnızca sayıdır: ad, neden ve yol görünmez, toplam tutarlı kalır. Branch, tag, dosya
konumu, deneme, triage, yığın izi, süre yoktur. Yalnızca ürün hatası senaryosunun ayrıntısı ve
ekran görüntüleri açılır; diğer senaryo ve ekler 404 (video, log ve metin ekleri dahil). Sınıf her
istekte hesaplanır, triage değişirse açık link de değişir.

## Testler
`test_sw_5_1_text_sanitized`, `test_sw_5_1_share_link_hides_internal_info`, `test_sw_5_1_genel_sinif`,
`test_sw_5_1_genel_rapor_shows_only_product_results`.
