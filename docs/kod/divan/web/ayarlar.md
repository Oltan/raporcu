# divan/src/web/ayarlar.rs

**Amaç:** Admin > Ayarlar sayfası ve senaryo için Jira bug formu. Hepsi admin oturumu ister.

| Uç | Davranış |
|----|----------|
| `GET /admin/ayarlar[?alanlar=1][&notice=jira\|doors]` | Jira şablonu formu (yer tutucu listesi `ayarlar::yer_tutucular()`: `{{YTET}}`, `{{YGÖ}}` dahil), gereksinim formu (başlık önek adıyla); `alanlar=1` ise Jira'dan alan listesi (`Jira::field_meta`, ayrı iş parçacığında) tablo olarak: kimlik, ad, zorunlu (varsayılanı var mı), tür, izinli değerler (ilk 25) |
| `POST /admin/ayarlar/jira` | form: `ozet`, `aciklama` (CRLF → LF), `alanlar`. Geçersiz JSON → 400 ve form girilen değerlerle yeniden gösterilir. Başarı → kaydet, belleği güncelle, `?notice=jira` |
| `POST /admin/ayarlar/doors` | `DoorsAyar` alanları (boşluklar kırpılır); kimlik özniteliği boşsa hata |
| `GET /admin/scenarios/{id}/jira` | bug formu: şablondan taslak (`jira::draft_for_scenario`); Jira alan listesi alınabildiyse formda alan kutuları |
| `POST /admin/scenarios/{id}/jira` | formdan bug, `jira::create_for_scenario`; başarı → senaryo sayfası; hata → 502 ve Jira'nın alan bazlı mesajı |

| `POST /admin/scenarios/{id}/jira/bagla` | form `anahtar`. `jira::issue_anahtari` geçmezse `?notice=jira-gecersiz`. Jira ayarlıysa `get_issue` (ayrı iş parçacığında): 404 → `jira-bulunamadi`, bağlanmaz; bilgi gelirse `save_issue`; Jira'ya ulaşılamazsa uyarı logu, denetimsiz bağlanır. Sonra `jira::link_manual` → senaryo sayfası `?notice=jira-baglandi` / `jira-zaten` |
| `POST /admin/scenarios/{id}/jira/kaldir` | form `anahtar`; `jira::unlink` → `?notice=jira-kaldirildi` |
| `GET /admin/scenarios/{id}/jira?taslak=N` | form, bekleyen taslağın içeriğiyle; gizli `taslak` alanı → bug açılınca taslak `acildi` |
| `POST /admin/taslaklar/{id}/ac` | taslağı olduğu gibi açar (`jira::open_draft`); `geri` (yerel yol) varsa oraya döner; hata → 502 ve düzenleme linki |
| `POST /admin/taslaklar/{id}/yoksay` | bekleyen taslak `yok_sayildi` |

## Form alanları (`form_fields`)
Jira'nın alanlarından: `SYSTEM_FIELDS` (project, issuetype, summary, description, reporter,
attachment, issuelinks) ve `labels` hariç; **zorunlu olup varsayılanı olmayanlar** ya da şablonun ek
alanlarında geçenler. Değer şablondan (`form_value`: `{"id":"10"}` → `10`, dizi → virgüllü).
Her alan için gizli `tur.<id>` = `tür|öğe|seçenekli(0/1)`; POST'ta Jira'ya yeniden sormadan JSON'a
çevirmek için.

## Formdan bug (`bug_from_form`)
- `ozet` yoksa (eski boş POST) taslak aynen kullanılır.
- `ek` (JSON) ile başlar; `alan.<id>` değerleri (çoklu seçimde aynı ad tekrar eder, virgülle
  birleştirilir) `jira::field_value` ile türüne göre çevrilip üzerine yazılır; boş değer o alanı siler.
- Gereksinim/ek kimlikler label olarak gönderilmez (eski `doors_label` kutusu kaldırıldı).

## Testler
`test_sw_6_1_form_fields_and_values`; `tests/entegrasyon.rs`: `test_sw_6_1_required_fields_listed_and_bug_form_sends_them`,
`test_sw_6_3_auto_bug_uses_template_fields`, `test_sw_6_4_doors_settings_used_in_dxl_csv_and_links`,
`test_sw_6_2_elle_baglama_sonraki_kosumda_ayni_hataya_tasinir`.
