# raporcu/src/share.rs

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

## Testler
`test_sw_5_1_text_sanitized`, `test_sw_5_1_share_link_hides_internal_info`.
