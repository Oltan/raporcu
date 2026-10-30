# divan/src/jira.rs

**Amaç:** SW-6.1–6.3 — Jira Server/Data Center REST v2 istemcisi ve koşum sonrası eşleşme / bug açma.

## İstemci (`Jira::from_config`; URL, token, proje yoksa yok)
- Kimlik: `JIRA_USER` varsa **Basic** (`kullanıcı:token` base64), yoksa **Bearer** (PAT).
- `create_bug(NewBug)`: `POST /rest/api/2/issue` (`project`, `issuetype`, `summary` ≤250,
  `description`, `labels`). **400 ve hata `labels` alanındaysa** label'sız tekrar dener (WARN).
  Başarısızsa hata metni gizli değerlerden temizlenir.
- `attach(key, ad, bayt)`: `POST /rest/api/2/issue/<key>/attachments`, multipart, başlık
  `X-Atlassian-Token: no-check`.
- `find_open_bug(doors[], senaryo)`: JQL `project = "<P>" AND statusCategory != Done AND (labels in
  ("DOORS-…") OR summary ~ "\"<senaryo>\"") ORDER BY created DESC`, ilk sonuç.

## Bug içeriği — şablon (Admin > Ayarlar, `ayarlar.md`)
- `vars(c, public_url, BugInput)`: yer tutucu değerleri; proje/branch/commit/tarih koşumdan okunur.
- `build_bug(şablon, değerler, doors)`: özet (doldurulup ilk satır), açıklama, DOORS label'ları
  (`doors_label` ise), ek alanlar (`fill_json`). Ek alan JSON'u bozuksa hata.
- `draft_for_scenario`: senaryo için taslak (form ve admin).

## Bug oluşturma — `create_bug(NewBug)`
`fields` = project, issuetype, summary (≤250), description + **ek alanlar** (project hariç her şeyi
ezer; `labels` DOORS label'larına eklenir). 400 ve hata `labels` alanındaysa label'sız tekrar.
Başarısızsa `jira_error`: Jira'nın `errors` (alan: mesaj) ve `errorMessages`'ı tek satırda, alan
hatası varsa "Admin > Ayarlar > Jira'da Ek alanlar…" ipucu; gizli değerler temizlenir.

## Alan bilgisi — `field_meta()`
`GET /rest/api/2/issue/createmeta/{proje}/issuetypes` → iş türü kimliği (ad `JIRA_ISSUE_TYPE`,
büyük/küçük harf duyarsız; yoksa var olanları listeleyen hata) → `…/issuetypes/{id}?maxResults=500`
→ `values[]`. Uç yoksa (eski Jira) `createmeta?projectKeys&issuetypeNames&expand=projects.issuetypes.fields`.
`FieldMeta {id, name, required, kind (schema.type), items, has_default, allowed [(id, ad)]}`.

`field_value(meta, metin)`: türüne göre JSON — seçenekli → `{"id"}` (dizi ise `[{"id"}]`),
`array/string` → metin dizisi (label), `number` → sayı, `user` → `{"name"}`, `option` → `{"value"}`,
priority/version/component → `{"name"}`, diğer → metin. Boş → yok.

## Parmak izi — `fingerprint(proje, mesaj)`
Mesajın ilk satırı; rakamlar `#`, `@` sonrası (nesne adresi) atılır, küçük harf; `proje|metin`'in
SHA-256'sının ilk 12 baytı (hex). "Aynı hata" bununla tanınır.

## Koşum sonrası — `process_run(c, paths, cfg, şablon, jira?, koşum)` (hooks.after_finish, arka planda)
Her `failed` son deneme için:
1. `@JIRA-X-1` tag'leri → `bug_links` (kaynak `tag`). Tag'i ya da bağlantısı olan senaryo geçilir.
2. Aynı projede aynı parmak izli bağlantı varsa → onu bağla (`eslesme`), `jira_actions: eslesti`.
3. Jira ayarlıysa açık bug ara (Jira bağlı değilse bu adım atlanır, taslak yine hazırlanabilir); bulunursa bağla, ekran görüntüsünü ekle.
4. Otomatik bug (SW-6.3), hepsi sağlanmalı: mod `off` değil; triage yoksa ya da `urun`;
   `JIRA_REQUIRE_RETRY` ise deneme ≥ 2; bu koşumda aynı parmak izi için açılmamış; koşum başına
   `JIRA_MAX_BUGS_PER_RUN` aşılmamış (aşılırsa sayılır, atlanır).
   - **observe** (varsayılan): Jira'ya yazmaz; **taslak** hazırlar (`create_draft`, kaynak `kural`) + `jira_actions: acilacakti`.
   - **on**: bug açar, bağlar (`otomatik`), ekran görüntüsünü ekler; hata olursa `jira_actions: hata`.

`create_for_scenario(c, paths, jira, sid, bug)`: formdan gelen bug'ı moddan bağımsız açar, bağlar (`admin`), ekran görüntüsünü ekler.

## Testler
`test_sw_6_1_*` (şablon, alan türleri, hata metni); `tests/entegrasyon.rs` (sahte Jira sunucusuyla 6.1, 6.2, 6.3).

## Bug taslakları
- `create_draft(c, koşum, senaryo, parmak izi, kaynak, gerekçe, bug)`: senaryonun bağlı bug'ı varsa ya da
  aynı projede aynı `scenario_key` veya aynı parmak izi için **bekleyen** taslak varsa oluşturmaz (None).
- `draft_from_opencode(c, cfg, şablon, senaryo, gerekçe)`: fix-result'ta OpenCode `urun` (güven ≥ eşik)
  dediğinde `web/ci.rs` çağırır; mod `off` ise yok.
- `drafts_for_run`, `drafts_for_scenario` (aynı proje + scenario_key), `pending_drafts`, `draft`.
- `open_draft(c, paths, jira, id, bug?)`: yalnızca `bekliyor`; `create_for_scenario` ile açar, `mark_draft(acildi, key)`.
- `mark_draft(id, durum, key?)`: `acildi` / `yok_sayildi`.
