package com.ornek.test;

import java.io.IOException;
import java.io.OutputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.TimeUnit;

/**
 * Senaryo boyunca ekranı ffmpeg ile kaydeder (SW-1.2, SW-1.6, SW-1.7).
 * Linux: Xvfb ekranı (x11grab, $DISPLAY). Windows: açık masaüstü oturumu (gdigrab).
 * Düşük kare hızı + sıkıştırılmış H.264; video yalnızca senaryo kalırsa saklanır.
 * İşletim sistemine özgü tek kısım burasıdır.
 *
 * Ayarlar (system property ya da ortam değişkeni):
 *   video.enabled / ULAK_VIDEO      true|false (varsayılan: ffmpeg varsa true)
 *   video.fps     / ULAK_VIDEO_FPS  varsayılan 5
 *   video.crf     / ULAK_VIDEO_CRF  varsayılan 35 (büyüdükçe dosya küçülür)
 */
public final class VideoKaydi {

    private final Process surec;
    private final Path dosya;

    private VideoKaydi(Process surec, Path dosya) {
        this.surec = surec;
        this.dosya = dosya;
    }

    public static VideoKaydi baslat() {
        if (!Boolean.parseBoolean(ayar("video.enabled", "ULAK_VIDEO", "true"))) {
            return null;
        }
        try {
            Path dosya = Files.createTempFile("senaryo-", ".mp4");
            List<String> komut = new ArrayList<>(List.of("ffmpeg", "-hide_banner", "-loglevel", "error", "-y"));
            String fps = ayar("video.fps", "ULAK_VIDEO_FPS", "5");
            if (System.getProperty("os.name").toLowerCase().contains("win")) {
                komut.addAll(List.of("-f", "gdigrab", "-framerate", fps, "-i", "desktop"));
            } else {
                String ekran = System.getenv().getOrDefault("DISPLAY", ":0");
                komut.addAll(List.of("-f", "x11grab", "-framerate", fps, "-i", ekran));
            }
            komut.addAll(List.of("-c:v", "libx264", "-preset", "ultrafast",
                    "-crf", ayar("video.crf", "ULAK_VIDEO_CRF", "35"), "-pix_fmt", "yuv420p", dosya.toString()));
            Process surec = new ProcessBuilder(komut).redirectErrorStream(true)
                    .redirectOutput(ProcessBuilder.Redirect.DISCARD).start();
            return new VideoKaydi(surec, dosya);
        } catch (IOException e) {
            // ffmpeg yoksa test yine koşar; yalnızca video olmaz.
            System.err.println("Video kaydı başlatılamadı: " + e.getMessage());
            return null;
        }
    }

    /** Kaydı durdurur; sakla=true ise video baytlarını döner, değilse dosyayı siler. */
    public byte[] durdur(boolean sakla) {
        try {
            try (OutputStream girdi = surec.getOutputStream()) {
                girdi.write('q');
                girdi.flush();
            } catch (IOException ignored) {
                // süreç zaten bitmiş olabilir
            }
            if (!surec.waitFor(10, TimeUnit.SECONDS)) {
                surec.destroyForcibly();
            }
            return sakla && Files.size(dosya) > 0 ? Files.readAllBytes(dosya) : null;
        } catch (IOException | InterruptedException e) {
            return null;
        } finally {
            try {
                Files.deleteIfExists(dosya);
            } catch (IOException ignored) {
                // geçici dosya
            }
        }
    }

    private static String ayar(String ozellik, String ortam, String varsayilan) {
        String deger = System.getProperty(ozellik);
        if (deger == null) {
            deger = System.getenv(ortam);
        }
        return deger == null || deger.isBlank() ? varsayilan : deger;
    }
}
