# Jira ve gereksinim (YTET/DOORS) ayarları

Bağlantı bilgileri (adres, token, proje) `.env`'de durur. **Bug'ın içeriği ve gereksinim eşleşmesi**
arayüzden ayarlanır: **Admin → Ayarlar** (`/admin/ayarlar`). Ayarlar veritabanında saklanır,
sunucu yeniden başlasa da kalır, kaydedince hemen geçerli olur.

## 1. Jira bağlantısı (`.env`)

```
JIRA_URL=https://jira.sirket.local
JIRA_TOKEN=<kişisel erişim tokenı (PAT)>     # JIRA_USER verilirse Basic kimlik doğrulama
JIRA_PROJECT=STAJ
JIRA_ISSUE_TYPE=Bug
JIRA_MATCH_LABEL=divan                         # açılan bug'a eklenir, aramada aranır; off = kapalı
JIRA_AUTO_CREATE=observe                      # off | observe (yalnızca kayıt) | on
JIRA_MAX_BUGS_PER_RUN=3
JIRA_REQUIRE_RETRY=true
```

## 2. Zorunlu alanları bulma

Şirket Jira'larında bug ekranında genellikle zorunlu özel alanlar olur (faz, bulunduğu ortam,
sürüm, bileşen...). Bunlar eksikse Jira bug'ı reddeder.

1. **Admin → Ayarlar → "Jira alanlarını getir"**. divan, projenizin `JIRA_ISSUE_TYPE` ekranındaki
   alanları Jira'dan okur ve bir tabloda gösterir:

   | Kimlik | Ad | Zorunlu | Tür | İzinli değerler |
   |--------|----|---------|-----|-----------------|
   | `customfield_10200` | Faz | zorunlu | option | `10300`: Yazılım · `10301`: Sistem |
   | `priority` | Priority | zorunlu (varsayılanı var) | priority | `2`: Critical · `3`: Major |
   | `components` | Component/s | | array / component | `500`: UI · `501`: Giriş |

   Jira 8.4+, 9.x, Data Center ve Cloud'daki yeni uç (`createmeta/{proje}/issuetypes`), yoksa
   eski uç kullanılır. Token'ın o projede "Create Issues" yetkisi olmalı.

2. Her bug'da **aynı** olan değerleri şablonun **Ek alanlar** kutusuna yazın (aşağıda).
   Bug'dan bug'a değişen zorunlu alanlar ise elle açılan bug formunda ayrıca sorulur.

## 3. Bug şablonu

Admin → Ayarlar → **Jira bug şablonu**. Hem otomatik açılan bug'larda hem "Jira'ya bug aç"
formunun ilk halinde kullanılır.

| Alan | Açıklama |
|------|----------|
| Özet | tek satır, en fazla 250 karakter |
| Açıklama | Jira wiki biçimi (`*kalın*`, `{noformat}…{noformat}`, `# madde`) |
| Ek alanlar | JSON nesnesi; Jira REST v2 `fields` biçiminde |

**Yer tutucular** (iki süslü parantez; tek süslü parantez Jira biçimidir, dokunulmaz):

| | |
|---|---|
| `{{senaryo}}`, `{{feature}}` | senaryo ve feature adı |
| `{{YTET}}` (önek adı) ya da eski `{{doors}}` | gereksinim numaraları (virgülle); label olarak eklenmez |
| `{{YGÖ}}` (her ek önek için) | ek kimlikler (virgülle), ör. `YGÖ-12, YGÖ-40`; label olarak eklenmez |
| `{{hata}}`, `{{hata_ilk_satir}}` | hata mesajı (2000) ve ilk satırı (120) |
| `{{adimlar}}` | `# adım` biçiminde numaralı liste |
| `{{rapor}}`, `{{kosum_rapor}}` | senaryonun ve koşumun rapor linki |
| `{{proje}}`, `{{branch}}`, `{{commit}}`, `{{kosum}}`, `{{tarih}}` | CI bilgisi |

**Ek alanlar örneği:**

```json
{
  "priority": { "name": "Major" },
  "components": [{ "name": "Test Otomasyonu" }],
  "customfield_10200": { "value": "Yazılım" },
  "customfield_10301": "{{branch}}",
  "fixVersions": [{ "name": "2.4" }],
  "labels": ["otomasyon"]
}
```

| Alan türü | Değer biçimi |
|-----------|--------------|
| Seçim listesi (select) | `{"value": "Yazılım"}` ya da `{"id": "10300"}` |
| Çoklu seçim, bileşen, sürüm | `[{"name": "UI"}]` ya da `[{"id": "500"}]` |
| Öncelik | `{"name": "Major"}` |
| Kullanıcı (Server/DC) | `{"name": "kullanici.adi"}` |
| Metin, sayı, tarih | `"metin"`, `3`, `"2026-12-31"` |
| Label | `["a", "b"]` |

Geçersiz JSON kaydedilmez; hata mesajı gösterilir, yazdıklarınız kaybolmaz.

## 4. Elle bug açma formu

Senaryo sayfası → **Jira'ya bug aç…** (admin). Form:

- Özet ve açıklama şablondan doldurulmuş gelir; düzenlenebilir.
- **Jira alanları:** zorunlu olup varsayılanı olmayan alanlar ve şablonda geçen alanlar. İzinli
  değeri olan alanlar açılır liste (çoklu seçimde çoklu liste), diğerleri metin kutusu.
  Boş bırakılan alan gönderilmez.
- **Diğer alanlar (JSON):** formda kutusu olmayan şablon alanları; buraya başka alan da eklenebilir.
- Gönderince bug açılır, ekran görüntüsü issue'ya eklenir, senaryo bug'a bağlanır.

Jira bug'ı reddederse hata **alan adıyla** gösterilir, ör.
`Jira 400: customfield_10200: Faz is required. — Eksik/yanlış alanları Admin > Ayarlar > Jira'da …`.
Otomatik açmada aynı hata Admin panelindeki "Jira gözlem kayıtları"nda `hata` satırı olarak görünür.

**Token süresi dolduysa:** Jira Cloud geçersiz token'ı reddetmez, isteği anonim sayar ve
`project: The target project doesn't exist or you don't have permission…` der. divan bu durumda
Jira'ya kimliğini sorar; kimlik reddedilirse hata `Jira kimliği reddetti (401): JIRA_TOKEN geçersiz
ya da süresi dolmuş…` olur. Yeni API token oluşturup `.env` içindeki `JIRA_TOKEN`'ı güncelleyin ve
divan'ı yeniden başlatın. Token geçerliyken aynı proje hatası gelirse `JIRA_PROJECT` anahtarı yanlıştır
ya da kullanıcının o projede issue açma izni yoktur.

## 5. Ürün hatası: taslak → tek tıkla aç

Varsayılan (`JIRA_AUTO_CREATE=observe`): divan Jira'da **bug açmaz**, bug'ı **taslak** olarak
hazırlar; siz göz atıp açarsınız. Bu ayar yalnızca bug açmayı yönetir; aşağıdaki otomatik eşleşme
ondan bağımsız çalışır.

**Taslak ne zaman hazırlanır:**
- Koşum bitince, kalan (`failed`) ve ürün hatası sayılan senaryo için: triage kararı yoksa ya da
  `urun` ise, rerun'da da kaldıysa (`JIRA_REQUIRE_RETRY`), koşum başına en fazla
  `JIRA_MAX_BUGS_PER_RUN`.
- Fix işinde OpenCode bir senaryoya **"ürün hatası"** derse (güven ≥ `TRIAGE_AUTO_CONFIDENCE`, 0,70):
  OpenCode kodu değiştirmez, divan o senaryo için taslak hazırlar (gerekçesiyle).
- Aynı senaryo ya da aynı hata (parmak izi) için bekleyen taslak varsa, ya da senaryonun bağlı bir
  bug'ı varsa yenisi hazırlanmaz: her gece taslak birikmez. Jira'da aynı hatanın açık bug'ı bulunursa
  senaryo ona bağlanır, taslak gerekmez.

Taslak, şablonla doldurulmuş özet, açıklama, label ve ek alanları taşır.

**Nerede görünür:** koşum sayfası ("Jira bug taslakları"), senaryo sayfasının Jira kutusu,
Admin paneli ("Onay bekleyen Jira bug taslakları").

**Admin için düğmeler:**

| Düğme | Ne yapar |
|-------|----------|
| **Jira'da aç** | taslağı olduğu gibi açar, ekran görüntüsünü ekler, senaryoyu bağlar; taslak "açıldı" (issue anahtarıyla) |
| **Düzenleyip aç…** | bug formunu taslağın içeriğiyle açar (zorunlu alanlar sorulur); gönderince taslak "açıldı" |
| **Yok say** | taslak kapanır, Jira'ya bir şey gitmez |

Jira bir alanı reddederse hata alan adıyla gösterilir; "Düzenleyip aç…" ile eksik alanı girip açın.
`JIRA_AUTO_CREATE=on` taslak beklemeden doğrudan açar; `off` hiçbir şey hazırlamaz.

### Senaryoda Jira issue tag'i (`@ABC-123`)

Feature dosyasında senaryoya (ya da feature'a) Jira issue anahtarı tag olarak yazılabilir:

```gherkin
@YTET_1234 @STAJ-42
Senaryo: Yanlış şifrede özel mesaj
```

- Hangi projelerin anahtarı tanınır: `.env`'de `JIRA_TAG_PROJELERI=STAJ,ABC` (virgülle); verilmezse
  `JIRA_PROJECT`. Listede olmayan `@REL-2` gibi tag'ler sıradan tag kalır. Eski `@JIRA-ABC-123` biçimi her
  projede çalışmaya devam eder. Gereksinim kimliği (`@YTET_…`, `@DOORS-…`) Jira sayılmaz.
- Koşum bitince divan bu anahtarların (ve senaryolara bağlı bug'ların) **başlık, durum, tür ve atanan**
  bilgisini Jira'dan çeker (30 dk'dan eskiyse; koşum başına en fazla 200). Rapor sayfaları Jira'ya
  gitmez, saklanan bilgiyi gösterir.
- Raporda: koşum, senaryo ve arama sayfalarında rozet `STAJ-42 · İşlemde`, üzerine gelince başlık; kapalı
  issue soluk ve üstü çizili, Jira'da olmayan anahtar kesikli çerçeveli "Jira'da bulunamadı". Senaryo
  sayfasının Jira bölümünde başlık, durum, tür, atanan ve kaynak (`tag` ya da bağ türü).
- Kalan senaryo tag'deki issue'ya bağlanır (yeni bug açılmaz). Paylaşım linklerinde Jira gösterilmez.
- Eski koşumlar yeniden işlenmez: ayardan önce yüklenen koşumlarda `@STAJ-42` sıradan tag olarak durur.

### Otomatik eşleşme (koşum bitince, `JIRA_AUTO_CREATE`'ten bağımsız)

Jira bağlıysa (`JIRA_URL`, `JIRA_TOKEN`, `JIRA_PROJECT` üçü de dolu) her koşumun sonunda, bağlı bug'ı
olmayan her kalan (`failed`) senaryo için sırayla:

1. Senaryoda Jira issue tag'i varsa (`@ABC-123` ya da `@JIRA-ABC-123`, aşağıda) senaryo o issue'ya bağlanır.
2. Aynı hata (parmak izi) daha önce bir issue'ya bağlandıysa senaryo ona bağlanır (Jira'ya gidilmez).
3. Jira'da aranır: `JIRA_PROJECT` içinde, türü `JIRA_ISSUE_TYPE` (varsayılan `Bug`), label'ı
   `JIRA_MATCH_LABEL` (varsayılan `divan`), kapanmamış ve **özetinde senaryo adı geçen** en yeni issue.
   Bulunursa senaryo ona bağlanır ve **ekran görüntüsü o issue'ya eklenir** (`ekran-<senaryo no>.png`).
   Story/task eşleşmez; fotoğraf yalnızca divan'ın açtığı ya da elle `divan` label'ı verilen bug'a gider.
   divan'ın açtığı her bug'a bu label eklenir (Jira label'ı reddederse bug label'sız açılır ve sonraki
   aramalarda Jira'dan bulunmaz; 2. adımdaki yerel bağ yine çalışır). Bu sürümden önce açılmış etiketsiz
   bug'lar da Jira aramasıyla bulunmaz; eşleşmesini istediğiniz bug'a Jira'da `divan` label'ını ekleyin.
   Gereksinim numaraları label olmadığı için aranmaz.

`JIRA_AUTO_CREATE=off` ya da `observe` bunu **durdurmaz**. Yanlış bir eşleşme 2. adım yüzünden sonraki
koşumlarda da tekrarlanır.

**Kapatmak / daraltmak (yalnızca `.env`, sonra divan yeniden başlatılır):**

| Ne istiyorsunuz | Ayar | Kaybedilen |
|-----------------|------|------------|
| Jira'ya hiç gidilmesin | `JIRA_URL` satırını yorum yapın (sistem ortam değişkeni olarak da tanımlı olmamalı) | "Jira'ya bug aç…" ve "Jira'da aç" düğmeleri çalışmaz; taslak da istemiyorsanız `JIRA_AUTO_CREATE=off` |
| Yanlış eşleşme olmasın, düğmeler çalışsın | `JIRA_MATCH_LABEL` açık kalsın (varsayılan); daha da daraltmak için `JIRA_PROJECT` yalnızca otomasyon bug'larının durduğu ayrı bir Jira projesi olsun | etiketsiz bug'lar Jira aramasıyla eşleşmez |
| Label'a bakmadan arasın (eski davranışa yakın) | `JIRA_MATCH_LABEL=off` | aynı türdeki etiketsiz bug'lar da eşleşebilir |

**Yanlış bağlantıyı silmek:** arayüzde düğmesi yoktur. divan'ı durdurun, `DATA_DIR/divan.db` dosyasını
yedekleyin, sonra `sqlite3` ile:

```sql
SELECT jira_key, COUNT(*) FROM bug_links WHERE source = 'eslesme' GROUP BY jira_key;   -- önce bakın
DELETE FROM bug_links WHERE source = 'eslesme' AND jira_key = 'ABC-12';                -- yanlış olan
```

`source` değerleri: `tag` (feature'daki tag), `eslesme` (otomatik), `otomatik` (divan'ın açtığı bug),
`admin` (elle açılan). Jira'ya eklenmiş ekran görüntüleri Jira'dan elle silinir.

## 6. Gereksinim kimliği (YTET) ve ek kimlikler (YGÖ)

Senaryo gereksinime **tag** ile bağlanır. Önek `.env`'de:

```
GEREKSINIM_ONEKI=YTET_      # varsayılan DOORS-
EK_KIMLIK_ONEKLERI=YGÖ-     # virgülle birden çok; boşsa yok
```

```gherkin
@YTET_1234 @YGÖ-12 @JIRA-STAJ-42
Senaryo: Yanlış şifrede özel mesaj
```

- `YTET_1234` gereksinim numarasıdır, yazıldığı gibi saklanır. Önek büyük/küçük harfe duyarlıdır;
  önekten sonrası harf, rakam, `-`, `_` (basamak sınırı yok). Bir senaryo birden çok gereksinimi
  karşılıyorsa birden çok tag taşır. Arayüzde ve CSV'de "DOORS" yerine önek adı (`YTET`) görünür.
- `YGÖ-12` senaryonun ek olarak bağlandığı gereksinimi gösterir (eşleşme kaybolmasın diye). Rozet
  olarak görünür, arama kutusunda aranır (tam eşleşme), CSV'de ayrı sütundur, Jira açıklamasına
  `{{YGÖ}}` ile yazılır. Saklanan tag'lerden okunduğu için eski koşumlarda da görünür.
- Önek değişmeden önce yüklenen koşumlarda `@YTET_…` tag'leri gereksinim numarası olarak değil,
  sıradan tag olarak durur (koşumlar yeniden işlenmez).
- İkisi de Jira'ya **label olarak gitmez**; yalnızca yer tutucuyla açıklamaya/özete yazılır.

**Aktarım CSV ile yapılır:** koşum sayfası → **YTET CSV** (`/runs/<koşum>/doors.csv`), kendi
aktarım betiğinizle alınır. Sütunlar (UTF-8 BOM, virgül; ek kimlik sütunları sonda):

```
YTET,Senaryo,Feature,Sonuç,Jira,Koşum,Tarih,YGÖ
YTET_1003,Yanlış şifrede özel mesaj,Giriş ekranı,Kaldı,STAJ-42,12,2026-10-30T02:14:05Z,YGÖ-12
```

Önceki sürümde ilk sütunun başlığı `DOORS ID` idi; aktarım betiğiniz başlığa bakıyorsa güncelleyin.
Bir senaryo birden çok gereksinim numarası taşıyorsa her numara ayrı satırdır; Jira sütunu tag'deki
ve bağlanan bug'ları boşlukla birleştirir. Arama sayfasındaki alan seçmeli CSV'de de gereksinim
sütunu `YTET`, ek kimlikler ayrı sütundur (`fields=senaryo,doors,ek:YGÖ-`).

Admin → Ayarlar → **Gereksinim (YTET)**:

| Ayar | Varsayılan | Açıklama |
|------|-----------|----------|
| Sonuç değerleri | `Geçti`, `Kaldı`, `Atlandı`, `Koşulmadı` | CSV'nin Sonuç sütununa yazılır; aktarım betiğinizin beklediği değerleri girin (ör. `Passed` / `Failed`) |
| Gereksinim linki şablonu | (boş) | rapordaki YTET rozetlerini tıklanabilir yapar; `{{no}}` = önekten sonrası (1234), `{{id}}` = YTET_1234 |

**DOORS linki:** DOORS'ta bir nesneye sağ tık → *Copy URL* ile alınan adreste nesne numarasının
geçtiği yeri `{{no}}` ile değiştirin. Ör.:

```
doors://doors.sirket.local:36677/?version=2&prodID=0&view=00000021&urn=urn:telelogic::1-432a1b2c3d4e5f60-O-{{no}}-0001a2b3
```

### DXL betiği (isteğe bağlı, varsayılan kapalı)

`.env`'de `DOORS_DXL=true` yapılırsa koşum sayfasında **DOORS DXL** düğmesi çıkar ve ayarlar
sayfasında DXL alanları görünür: modül yolu, kimlik özniteliği (`Absolute Number`), sonuç / tarih /
Jira / rapor linki öznitelik adları. Betik DOORS'ta *Tools → Edit DXL* ile çalıştırılır, kimlik
özniteliği eşleşen nesnelere yazar. Kapalıyken bu ayarlar korunur, yalnızca gizlenir. DXL gerçek bir
DOORS modülünde henüz denenmedi.
