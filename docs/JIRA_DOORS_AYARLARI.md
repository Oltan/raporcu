# Jira ve DOORS ayarları

Bağlantı bilgileri (adres, token, proje) `.env`'de durur. **Bug'ın içeriği ve DOORS eşleşmesi**
arayüzden ayarlanır: **Admin → Ayarlar** (`/admin/ayarlar`). Ayarlar veritabanında saklanır,
sunucu yeniden başlasa da kalır, kaydedince hemen geçerli olur.

## 1. Jira bağlantısı (`.env`)

```
JIRA_URL=https://jira.sirket.local
JIRA_TOKEN=<kişisel erişim tokenı (PAT)>     # JIRA_USER verilirse Basic kimlik doğrulama
JIRA_PROJECT=STAJ
JIRA_ISSUE_TYPE=Bug
JIRA_AUTO_CREATE=observe                      # off | observe (yalnızca kayıt) | on
JIRA_MAX_BUGS_PER_RUN=3
JIRA_REQUIRE_RETRY=true
```

## 2. Zorunlu alanları bulma

Şirket Jira'larında bug ekranında genellikle zorunlu özel alanlar olur (faz, bulunduğu ortam,
sürüm, bileşen...). Bunlar eksikse Jira bug'ı reddeder.

1. **Admin → Ayarlar → "Jira alanlarını getir"**. raporcu, projenizin `JIRA_ISSUE_TYPE` ekranındaki
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
| DOORS label | DOORS numaraları label olarak eklensin mi |

**Yer tutucular** (iki süslü parantez; tek süslü parantez Jira biçimidir, dokunulmaz):

| | |
|---|---|
| `{{senaryo}}`, `{{feature}}` | senaryo ve feature adı |
| `{{doors}}` | DOORS numaraları (virgülle) |
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
| Label | `["a", "b"]` (DOORS label'larına eklenir) |

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

## 5. Otomatik bug açma

`JIRA_AUTO_CREATE=observe` (varsayılan) ile raporcu bug açmaz, yalnızca "açılacaktı" kaydı tutar.
Önce şablonu ve ek alanları bir elle bug açarak deneyin, sonra `on` yapın. Kurallar: yalnızca ürün
hatası (triage), retry sonrasında, koşum başına üst sınır, aynı hata için tek bug
(`docs/kod/raporcu/jira.md`).

## 6. DOORS

Senaryo DOORS gereksinimine **tag** ile bağlanır:

```gherkin
@DOORS-1003 @JIRA-STAJ-42
Senaryo: Yanlış şifrede özel mesaj
```

`1003`, DOORS nesnesinin **kimlik özniteliğinin** değeridir (varsayılan `Absolute Number`).
Bir senaryo birden çok gereksinimi karşılıyorsa birden çok `@DOORS-…` tag'i taşır.

Admin → Ayarlar → **DOORS**:

| Ayar | Varsayılan | Açıklama |
|------|-----------|----------|
| Modül yolu | (boş) | ör. `/Proje/Gereksinimler/SRS`. Boşsa DXL o an açık modülde çalışır; doluysa modülü düzenleme kipinde açar, günceller ve kaydeder |
| Kimlik özniteliği | `Absolute Number` | tag'deki numaranın karşılaştırıldığı öznitelik (ör. `Object Identifier`, `Req ID`) |
| Sonuç / Tarih / Jira / Rapor linki öznitelikleri | `Test Sonucu`, `Test Tarihi`, `Jira`, `Test Raporu` | modüldeki öznitelik adları; boş bırakılan yazılmaz |
| Sonuç değerleri | `Geçti`, `Kaldı`, `Atlandı`, `Koşulmadı` | sonuç özniteliği enum ise değerleri **birebir** aynı olmalı (ör. `Passed` / `Failed`) |
| DOORS linki şablonu | (boş) | rapordaki DOORS rozetlerini tıklanabilir yapar; `{{no}}` = 1003, `{{id}}` = DOORS-1003 |

**Aktarım:** koşum sayfası → **DOORS DXL** dosyasını indirin, DOORS'ta *Tools → Edit DXL* ile
çalıştırın. Betik her DOORS numarası için kimlik özniteliği eşleşen nesneyi bulur, sonuç, tarih,
Jira ve rapor linkini yazar; sonunda "N nesne güncellendi, M numara bulunamadı" der.
**DOORS CSV** aynı bilgiyi DOORS'un CSV içe aktarımı ya da Excel için verir.

**DOORS linki:** DOORS'ta bir nesneye sağ tık → *Copy URL* ile alınan adreste nesne numarasının
geçtiği yeri `{{no}}` ile değiştirin. Ör.:

```
doors://doors.sirket.local:36677/?version=2&prodID=0&view=00000021&urn=urn:telelogic::1-432a1b2c3d4e5f60-O-{{no}}-0001a2b3
```

Not: DXL betiği gerçek bir DOORS modülünde henüz denenmedi (pilot kontrol listesi).
