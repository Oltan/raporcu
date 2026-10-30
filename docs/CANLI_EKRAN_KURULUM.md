# Canlı ekran: test projesine eklenecekler

raporcu'nun canlı ekranında **şu an koşan senaryo ve adım** görünsün diye test projenize
eklenecek her şey burada. Toplam **1 Java dosyası + 1 satır ayar**. Başka bağımlılık, CI değişkeni
ya da kod değişikliği gerekmez.

| # | Ne | Nereye |
|---|----|--------|
| 1 | `CanliOlayEklentisi.java` (aşağıda tamamı) | `src/test/java/<paketiniz>/CanliOlayEklentisi.java` |
| 2 | `cucumber.plugin` satırına eklentinin adı | `src/test/resources/junit-platform.properties` |
| 3 | (zaten var) Allure eklentisi ve CI şablonu | — |

Gereksinim: Cucumber 7 (`cucumber-java`, JUnit Platform). Java 17+ (kodda `instanceof` desen
eşleştirmesi ve `switch ->` kullanılıyor; Java 11 ise aşağıdaki nota bakın).

---

## 1. Java dosyası

`src/test/java/com/sirket/test/CanliOlayEklentisi.java` — **ilk satırdaki paket adını kendi test
paketinizle değiştirin** (adım tanımlarınızın olduğu paket ya da altı; glue içinde olması şart değil).

```java
package com.sirket.test;   // ← kendi test paketinizin adı

import io.cucumber.plugin.ConcurrentEventListener;
import io.cucumber.plugin.event.EventPublisher;
import io.cucumber.plugin.event.HookTestStep;
import io.cucumber.plugin.event.PickleStepTestStep;
import io.cucumber.plugin.event.Result;
import io.cucumber.plugin.event.TestCase;
import io.cucumber.plugin.event.TestCaseFinished;
import io.cucumber.plugin.event.TestCaseStarted;
import io.cucumber.plugin.event.TestSourceRead;
import io.cucumber.plugin.event.TestStepFinished;
import io.cucumber.plugin.event.TestStepStarted;

import java.io.IOException;
import java.io.Writer;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

/**
 * raporcu canlı aktarım eklentisi.
 *
 * Allure sonuç dosyası ancak senaryo BİTİNCE yazılır. Canlı ekranda "şu an hangi senaryo, hangi
 * adımda" bilgisini göstermek için bu eklenti Cucumber olaylarını anında bir JSON-lines dosyasına
 * yazar; {@code stajci watch} dosyayı izleyip raporcu'ya gönderir.
 *
 * Dosya: {@code -Dstajci.canli.dosya=...} (stajci start bunu verir), yoksa
 * {@code target/stajci-canli/olaylar.jsonl}. Her satır bir olay:
 *
 * <pre>
 * {"tur":"senaryo_basladi","zaman":1793304316815,"ad":"...","feature":"...","uri":"...","satir":10}
 * {"tur":"adim_basladi","zaman":...,"metin":"Eğer ki kullanıcı adı ...","sira":2}
 * {"tur":"adim_bitti","zaman":...,"metin":"...","sira":2,"durum":"PASSED","sure_ms":1321,"hata":null}
 * {"tur":"senaryo_bitti","zaman":...,"ad":"...","durum":"FAILED","sure_ms":2400}
 * </pre>
 *
 * Yazma hatası testleri asla etkilemez. Kayıt: junit-platform.properties içindeki
 * {@code cucumber.plugin} listesine {@code com.sirket.test.CanliOlayEklentisi} eklenir.
 */
public class CanliOlayEklentisi implements ConcurrentEventListener {

    private final Map<URI, String> featureAdlari = new ConcurrentHashMap<>();
    private final Path dosya;
    private Writer yazici;
    private int adimSirasi;

    public CanliOlayEklentisi() {
        String yol = System.getProperty("stajci.canli.dosya");
        if (yol == null || yol.isBlank()) {
            yol = System.getenv().getOrDefault("STAJ_CANLI_DOSYA", "target/stajci-canli/olaylar.jsonl");
        }
        dosya = Path.of(yol);
    }

    @Override
    public void setEventPublisher(EventPublisher yayinci) {
        yayinci.registerHandlerFor(TestSourceRead.class, this::kaynakOkundu);
        yayinci.registerHandlerFor(TestCaseStarted.class, this::senaryoBasladi);
        yayinci.registerHandlerFor(TestStepStarted.class, this::adimBasladi);
        yayinci.registerHandlerFor(TestStepFinished.class, this::adimBitti);
        yayinci.registerHandlerFor(TestCaseFinished.class, this::senaryoBitti);
    }

    /** Feature adı Cucumber olaylarında yok; kaynak metindeki "Özellik:/Feature:" satırından alınır. */
    private void kaynakOkundu(TestSourceRead olay) {
        for (String satir : olay.getSource().split("\n")) {
            String s = satir.trim();
            for (String k : new String[] {"Özellik:", "Feature:", "Ability:", "Business Need:"}) {
                if (s.startsWith(k)) {
                    featureAdlari.put(olay.getUri(), s.substring(k.length()).trim());
                    return;
                }
            }
        }
    }

    private void senaryoBasladi(TestCaseStarted olay) {
        TestCase tc = olay.getTestCase();
        adimSirasi = 0;
        yaz("{\"tur\":\"senaryo_basladi\",\"zaman\":" + olay.getInstant().toEpochMilli()
                + ",\"ad\":" + js(tc.getName())
                + ",\"feature\":" + js(featureAdlari.getOrDefault(tc.getUri(), ""))
                + ",\"uri\":" + js(tc.getUri().toString())
                + ",\"satir\":" + tc.getLocation().getLine() + "}");
    }

    private void adimBasladi(TestStepStarted olay) {
        if (olay.getTestStep() instanceof PickleStepTestStep adim) {
            adimSirasi++;
            yaz("{\"tur\":\"adim_basladi\",\"zaman\":" + olay.getInstant().toEpochMilli()
                    + ",\"metin\":" + js(adim.getStep().getKeyword() + adim.getStep().getText())
                    + ",\"sira\":" + adimSirasi + "}");
        }
    }

    private void adimBitti(TestStepFinished olay) {
        if (olay.getTestStep() instanceof HookTestStep) {
            return;
        }
        if (olay.getTestStep() instanceof PickleStepTestStep adim) {
            Result r = olay.getResult();
            yaz("{\"tur\":\"adim_bitti\",\"zaman\":" + olay.getInstant().toEpochMilli()
                    + ",\"metin\":" + js(adim.getStep().getKeyword() + adim.getStep().getText())
                    + ",\"sira\":" + adimSirasi
                    + ",\"durum\":" + js(r.getStatus().name())
                    + ",\"sure_ms\":" + r.getDuration().toMillis()
                    + ",\"hata\":" + (r.getError() == null ? "null" : js(kisalt(String.valueOf(r.getError().getMessage()), 500))) + "}");
        }
    }

    private void senaryoBitti(TestCaseFinished olay) {
        Result r = olay.getResult();
        yaz("{\"tur\":\"senaryo_bitti\",\"zaman\":" + olay.getInstant().toEpochMilli()
                + ",\"ad\":" + js(olay.getTestCase().getName())
                + ",\"durum\":" + js(r.getStatus().name())
                + ",\"sure_ms\":" + r.getDuration().toMillis() + "}");
    }

    /** Bir satır yazar ve hemen diske iter (stajci saniyede bir okur). Hata testi etkilemez. */
    private synchronized void yaz(String satir) {
        try {
            if (yazici == null) {
                if (dosya.getParent() != null) {
                    Files.createDirectories(dosya.getParent());
                }
                yazici = Files.newBufferedWriter(dosya, StandardCharsets.UTF_8,
                        StandardOpenOption.CREATE, StandardOpenOption.APPEND);
            }
            yazici.write(satir);
            yazici.write('\n');
            yazici.flush();
        } catch (IOException e) {
            System.err.println("[raporcu] canlı olay yazılamadı: " + e.getMessage());
        }
    }

    private static String kisalt(String s, int n) {
        return s.length() <= n ? s : s.substring(0, n) + "…";
    }

    private static String js(String s) {
        if (s == null) {
            return "null";
        }
        StringBuilder b = new StringBuilder("\"");
        for (char c : s.toCharArray()) {
            switch (c) {
                case '"' -> b.append("\\\"");
                case '\\' -> b.append("\\\\");
                case '\n' -> b.append("\\n");
                case '\r' -> b.append("\\r");
                case '\t' -> b.append("\\t");
                default -> {
                    if (c < 0x20) {
                        b.append(String.format("\\u%04x", (int) c));
                    } else {
                        b.append(c);
                    }
                }
            }
        }
        return b.append('"').toString();
    }
}
```

Ne yapar:

- Cucumber'ın olaylarını dinler; her senaryo başında/sonunda ve her **Gherkin adımının** (hook'lar
  hariç) başında/sonunda dosyaya bir JSON satırı yazar ve hemen diske iter.
- Dosya yolu: `stajci start` bunu `-Dstajci.canli.dosya=...` ile verir. Elle koşarken
  `target/stajci-canli/olaylar.jsonl`.
- Yazamazsa yalnızca `System.err`'e bir satır basar; **testin sonucunu asla etkilemez**.
- Test kodunuza, hook'larınıza, Allure'a dokunmaz; yanlarında çalışır.

## 2. junit-platform.properties

`src/test/resources/junit-platform.properties` dosyasındaki `cucumber.plugin` satırının **sonuna**
virgülle ekleyin:

```properties
cucumber.plugin=pretty, io.qameta.allure.cucumber7jvm.AllureCucumber7Jvm, com.sirket.test.CanliOlayEklentisi
```

- Paket adı 1. adımdakiyle aynı olmalı.
- Satırda `AllureCucumber7Jvm` zaten olmalı (rapor ve ekler için); yoksa onu da ekleyin.
- Eklentileri `@ConfigurationParameter(key = PLUGIN_PROPERTY_NAME, value = "...")` ile runner
  sınıfında veriyorsanız aynı adı oraya ekleyin.
- `cucumber.execution.parallel.enabled=true` **olmamalı**: canlı ekran aynı anda tek senaryo
  koştuğunu varsayar (TestFX zaten tek ekranda sırayla koşar).

## 3. Başka bir şey gerekmiyor

CI tarafında her şey hazır: `stajci start` olay dosyasının yolunu Maven'a verir, `stajci watch`
dosyayı saniyede bir okuyup raporcu'ya gönderir, canlı ekran WebSocket ile güncellenir.
Canlı ekranın adresi `stajci start` çıktısında yazar:

```
[stajci run_id=12] koşum açıldı: 48 senaryo bekleniyor. Canlı durum: http://raporcu:8080/runs/12/live
```

ya da koşum sayfasındaki **Canlı izle** düğmesi.

---

## Yerelde doğrulama (CI olmadan)

1. Testleri normal koşturun (Linux'ta `xvfb-run -a mvn test`, Windows'ta `mvn test`).
2. Dosya oluşmuş olmalı:

   ```bash
   head -5 target/stajci-canli/olaylar.jsonl
   ```

   ```json
   {"tur":"senaryo_basladi","zaman":1793340892175,"ad":"Geçerli bilgilerle giriş","feature":"Giriş ekranı","uri":"classpath:features/giris.feature","satir":10}
   {"tur":"adim_basladi","zaman":1793340895673,"metin":"Diyelim ki giriş ekranı açık","sira":1}
   {"tur":"adim_bitti","zaman":1793340895710,"metin":"Diyelim ki giriş ekranı açık","sira":1,"durum":"PASSED","sure_ms":37,"hata":null}
   ```

3. raporcu ile birlikte görmek için: `deneme/ev-denemesi.sh` (ya da Windows'ta `ev-denemesi.cmd`)
   içindeki `ornek-test-projesi` yerine kendi projenizi koyun; canlı ekran tarayıcıda açılır.

## Sorun giderme

| Belirti | Neden / çözüm |
|---------|---------------|
| `olaylar.jsonl` hiç oluşmuyor | Eklenti yüklenmemiş: `cucumber.plugin` satırındaki paket.sınıf adı yanlış ya da dosya başka bir `junit-platform.properties`'te. sınıf adı yanlışsa Cucumber testler başlamadan eklentiyi yükleyemediğini söyleyen bir hatayla durur |
| Canlı ekranda "şu an koşuyor" kartı yok, yalnızca senaryolar bitince güncelleniyor | Olaylar gelmiyor: CI'da `stajci start` kullanılıyor mu (doğrudan `mvn test` değil)? `.stajci/watch.log` dosyasına bakın |
| Feature adı boş | Feature dosyası `Feature:` / `Özellik:` / `Ability:` / `Business Need:` ile başlamıyor (eklenti adı bu satırdan okur) |
| Kart "Hazırlık: @Before hook'ları" diyor | Normal: senaryo başladı, uygulama açılıyor; ilk Gherkin adımı başlayınca adımlar görünür |
| Derleme hatası `instanceof ... adim` | Java 11: `if (olay.getTestStep() instanceof PickleStepTestStep) { PickleStepTestStep adim = (PickleStepTestStep) olay.getTestStep(); ... }` biçimine çevirin; `switch (c) { case '"' -> ... }` bloğunu klasik `switch` + `break` ile yazın |

Olay biçiminin ve sunucu tarafının ayrıntısı: `docs/ANLIK_AKTARIM.md`.
