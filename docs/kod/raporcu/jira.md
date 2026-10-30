# raporcu/src/jira.rs

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

## Bug metni — `bug_text(...)`
Özet: `[Otomasyon] <senaryo>: <hata ilk satırı (120)>`. Açıklama: senaryo, feature, `DOORS: …`,
rapor linki, `{noformat}` içinde hata (2000), numaralı adımlar. Label'lar = DOORS numaraları.

## Parmak izi — `fingerprint(proje, mesaj)`
Mesajın ilk satırı; rakamlar `#`, `@` sonrası (nesne adresi) atılır, küçük harf; `proje|metin`'in
SHA-256'sının ilk 12 baytı (hex). "Aynı hata" bununla tanınır.

## Koşum sonrası — `process_run(c, paths, cfg, jira?, koşum)` (hooks.after_finish, arka planda)
Her `failed` son deneme için:
1. `@JIRA-X-1` tag'leri → `bug_links` (kaynak `tag`). Tag'i ya da bağlantısı olan senaryo geçilir.
2. Aynı projede aynı parmak izli bağlantı varsa → onu bağla (`eslesme`), `jira_actions: eslesti`.
3. Jira ayarlıysa açık bug ara; bulunursa bağla, ekran görüntüsünü ekle.
4. Otomatik bug (SW-6.3), hepsi sağlanmalı: mod `off` değil; triage yoksa ya da `urun`;
   `JIRA_REQUIRE_RETRY` ise deneme ≥ 2; bu koşumda aynı parmak izi için açılmamış; koşum başına
   `JIRA_MAX_BUGS_PER_RUN` aşılmamış (aşılırsa sayılır, atlanır).
   - **observe** (varsayılan): Jira'ya yazmaz; `jira_actions: acilacakti` + log "açılacaktı".
   - **on**: bug açar, bağlar (`otomatik`), ekran görüntüsünü ekler; hata olursa `jira_actions: hata`.

`create_for_scenario`: admin düğmesi; moddan bağımsız açar ve bağlar (`admin`).

## Testler
`test_sw_6_1_*`; `tests/entegrasyon.rs` (sahte Jira sunucusuyla 6.1, 6.2, 6.3).
