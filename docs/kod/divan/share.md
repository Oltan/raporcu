# divan/src/share.rs

**Amaç:** SW-5.1 — paylaşım linkinde iç bilgileri gizleme.

- `sanitize_text(metin, gizliler)`: önce gizli değerler `***`; sonra kelime kelime: `glpat-`,
  `glrt-`, `glptt-` ile başlayan, `token=`, `password=`, `private-token`, `api_key=` içeren
  kelimeler `***`; `Bearer` / `Authorization:` sonrasındaki kelime `***`; Jira anahtarı biçimindeki
  kelime (`ABC-123`, DOORS ve gereksinim/ek kimlikler hariç) `[Jira]`. Boşluklar korunur.
- `is_jira_key(w)`: en az 2 karakterli büyük harf/rakam proje + `-` + rakamlar; `DOORS` ya da `kimlik_mi` değil.
- `visible_tag(t)`: `JIRA-…` tag'leri gizlenir.

Paylaşım görünümünde ayrıca (web/pages.rs): pipeline/iş/repo linkleri, Jira rozetleri, bug
bağlantıları, OpenCode kutusu, triage gerekçeleri gizlenir; ekler `/s/<token>/files/<id>` üzerinden
ve yalnızca o koşumun ekleri sunulur; başka koşumun senaryosu 404.

## Görünümler
- Ekip linki (`share_links.gorunum = 'ekip'`): yukarıdaki gibi ayrıntılı koşum raporu (`run.html`,
  `scenario.html`); "Jira issue'ları" kartı linksiz ve atanansız.
- 0.1.x'in genel linkleri (`gorunum = 'genel'`) artık `/`'e yönlenir. Dışa açık raporlar
  `yayin.md` ve `web/genel.md` içinde; "inceleniyor" sınıfı (`genel_sinif`) 0.2.0'da kaldırıldı.

## Testler
`test_sw_5_1_text_sanitized`, `test_sw_5_1_share_link_hides_internal_info`.
