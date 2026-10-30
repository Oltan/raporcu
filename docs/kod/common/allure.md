# common/src/allure.rs

**Amaç:** `allure-results` klasöründeki dosyaların modeli ve ayrıştırıcısı (allure-java 2.x,
`allure-cucumber7-jvm`). divan, ulak ve prova aynı modeli kullanır. Gerçek örnekler:
`testdata/allure/ornek-giris/`.

## Dosya türleri (`file_kind(dosya_adı)`)
Yalnızca dosya adına bakılır (yol değil):

| Ad sonu | Tür | İçerik |
|---------|-----|--------|
| `-result.json` | `Result` | Bir senaryo **denemesi** (rerun'da her deneme ayrı dosya) |
| `-container.json` | `Container` | Fixture'lar: Cucumber `@Before/@After` hook'ları ve onların ekleri; `children` = sonuç uuid'leri |
| `-attachment` içeren | `Attachment` | Ek dosyası (`...-attachment`, `...-attachment.png`, `.txt` ...) |
| diğer | `Other` | yok sayılır |

## `is_safe_file_name(ad)`
Ek adı güvenli mi: boş değil, ≤255 karakter, `/ \ : NUL` içermez, `.`/`..` değil, `..` ile
başlamaz. Güvensiz adlı ek kaydı atlanır (SW-2.1 zip güvenliği).

## Veri modeli (JSON alanları camelCase; hepsi isteğe bağlı okunur)
- **Status:** `passed | failed | broken | skipped | unknown`. Tanınmayan ya da **eksik** değer
  `unknown`. Gerçek çıktıda tanımsız adımlı senaryoda `status` alanı hiç yazılmaz.
  `is_failure()` = failed veya broken.
- **StatusDetails:** `message`, `trace`, `flaky`, `known`, `muted`.
- **Label:** `{name, value}` — önemli adlar: `tag` (Cucumber tag'i, `@` olmadan), `feature`,
  `suite`, `gherkin_uri` (`classpath:features/giris.feature`).
- **Attachment:** `{name, source, type}`; `source` = ek dosyasının adı.
- **Step:** `name, status, statusDetails, start, stop, steps[] (iç içe), attachments[], parameters[]`.
- **TestResult:** `uuid, historyId, testCaseId, name, fullName (giris.feature:28), description,
  status, statusDetails, stage, start, stop (epoch ms), labels[], links[], steps[], attachments[], parameters[]`.
- **Fixture:** `name, status, statusDetails, start, stop, steps[], attachments[]`.
- **Container:** `uuid, children[], befores[], afters[], start, stop`.

## TestResult yardımcıları
| Fonksiyon | Davranış |
|-----------|----------|
| `status()` | yoksa `unknown` |
| `scenario_key()` | `historyId` → yoksa `testCaseId` → `fullName` → `name`. **Rerun'da aynı kalır**; geçmiş ve deneme gruplaması buna dayanır |
| `label(ad)` | ilk eşleşen label değeri |
| `tags()` | `tag` label'ları, baştaki `@` atılmış |
| `feature()` | `feature` label'ı, yoksa `suite` |
| `message()`, `trace()` | statusDetails'ten |
| `duration_ms()` | `stop - start` (biri yoksa yok) |
| `doors_ids()` | `DOORS-xxx` tag'leri (`doors-` küçük harf de) → `DOORS-xxx` |
| `jira_keys()` | `JIRA-ABC-12` tag'leri → `ABC-12` (proje büyük harfe çevrilir; son parça rakam olmalı) |
| `all_attachments()` | sonucun ve tüm (iç içe) adımların ekleri |

`doors_id(tag)` / `jira_key(tag)` tek tag için aynı kural (paylaşımda Jira tag'ini gizlemek için de kullanılır).

## Önemli gerçek-çıktı davranışları
- `@After` hook'unda `scenario.attach(...)` ile eklenen ekran görüntüsü/video **result dosyasında
  değil container dosyasındaki `afters[].attachments`** altındadır.
- `scenario.attach` eklerinin uzantısı yoktur (`...-attachment`); tür `type` alanındadır.
- Adım içi `Allure.addAttachment` ekleri `steps[].attachments` altındadır.

## Testler
`test_sw_2_7_real_samples_parse` (11 sonuç, tüm durumlar, rerun, container ekleri),
`test_sw_2_7_doors_and_jira_tags`, `test_sw_2_1_safe_file_names`.

## Python karşılığı
`dataclasses` + `json.loads`; tüm alanlara `.get()` ile varsayılan. `file_kind` için `str.endswith`.
