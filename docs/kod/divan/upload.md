# divan/src/upload.rs

**Amaç:** Allure zip yükleme (SW-2.1, 2.3, 10.6). Gövde **belleğe alınmaz**; önce diske, sonra zip
dosya dosya işlenir. Ölçüm: 150 MB zip yüklenirken bellek artışı ~1 MB.

## Adımlar
1. `body_to_file(body, tmp, max)`: akışı `tmp/upload-<uuid>.zip`'e yazar, bayt sayar; `max`'ı
   (MAX_UPLOAD_MB) aşınca dosyayı siler, **413** "yükleme boyutu sınırı aşıldı".
2. `process_zip(conn, paths, limits, zip, target)`:
   - Geçerli zip değilse **400**. Giriş sayısı > MAX_ZIP_ENTRIES → 400.
   - **1. geçiş (içerik açılmadan):** her giriş adı `validate_entry_name` ile denetlenir —
     `/` ya da `\` ile başlayan, `:` ya da NUL içeren, herhangi bir parçası `..` olan ad **tüm
     yüklemeyi 400 ile reddeder**. Klasör girişleri atlanır; yalnızca son ad parçası kullanılır
     (`allure-results/x-result.json` → `x-result.json`). Tek giriş > MAX_ENTRY_MB → 413; açılmış
     toplam > 4×MAX_UPLOAD → 413 (zip bombası). Hiç `-result.json` yoksa **422** +
     `NO_RESULTS_MESSAGE` ("Allure eklentisi ... eksik olabilir", SW-2.3).
   - Transaction açılır. `Target::New(ci)`: proje + yeni koşum (`running`). `Target::Existing(id)`:
     var olan koşum (yoksa 404) — canlı koşumun son toplu zip'i.
   - **2. geçiş:** result/container json'ları tek tek okunur (en fazla 32 MB) ve `ingest_json` ile
     işlenir; ekler `runs/<id>/files/` altına önce `.<ad>.part` olarak yazılıp sonra yeniden
     adlandırılır (yarım dosya görünmez); zaten varsa atlanır.
   - `link_fixture_attachments`, `recompute_run`; yeni koşumsa `finish_run(finished)`.
   - Hata olursa transaction geri alınır, yeni koşumun klasörü silinir.
3. `ingest_json(c, run, ad, bayt, özet)`: türüne göre `ingest_result`/`ingest_container`;
   ayrıştırılamayan json atlanır (WARN), yüklemeyi düşürmez. Özet: kabul edilen, tekrar, container, ek sayıları.

## Testler
`test_sw_2_1_zip_entry_names`; `tests/sonuc_alma.rs`; `tests/bellek.rs` (SW-10.6).

## Python karşılığı
`zipfile.ZipFile` dosyadan okur (belleğe almaz); `zf.infolist()` ile 1. geçiş, `zf.open(info)` ile
akış halinde kopyalama (`shutil.copyfileobj`).
