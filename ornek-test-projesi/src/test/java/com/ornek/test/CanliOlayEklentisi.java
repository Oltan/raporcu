package com.ornek.test;

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
 * {@code cucumber.plugin} listesine {@code com.ornek.test.CanliOlayEklentisi} eklenir.
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
