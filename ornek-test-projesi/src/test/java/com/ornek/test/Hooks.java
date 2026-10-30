package com.ornek.test;

import com.ornek.app.GirisUygulamasi;
import io.cucumber.java.After;
import io.cucumber.java.Before;
import io.cucumber.java.Scenario;
import javafx.embed.swing.SwingFXUtils;
import javafx.geometry.Rectangle2D;
import javafx.scene.image.Image;
import javafx.stage.Screen;
import javafx.stage.Stage;
import org.testfx.api.FxRobot;
import org.testfx.api.FxToolkit;

import javax.imageio.ImageIO;
import java.io.ByteArrayOutputStream;

/**
 * Her senaryoda uygulamayı gerçek ekranda (Linux: Xvfb, Windows: masaüstü oturumu) açar.
 * Senaryo kalırsa ekran görüntüsü alınır ve Cucumber ekine konur; allure-cucumber7-jvm bunu
 * ilgili *-container.json dosyasındaki @After fixture'ına yazar (SW-2.2).
 * Kalan senaryonun videosu da aynı yolla eklenir; geçen senaryonun videosu silinir.
 */
public class Hooks {

    static Stage stage;
    private VideoKaydi video;

    @Before
    public void uygulamayiAc() throws Exception {
        video = VideoKaydi.baslat();
        stage = FxToolkit.registerPrimaryStage();
        FxToolkit.setupApplication(GirisUygulamasi.class);
    }

    @After
    public void kapat(Scenario scenario) throws Exception {
        try {
            if (scenario.isFailed()) {
                scenario.attach(ekranGoruntusu(), "image/png", "Ekran görüntüsü");
                // Self-healing için: ekrandaki gerçek düğümler (fx:id, tür, metin)
                scenario.attach(sahneAgaci().getBytes(java.nio.charset.StandardCharsets.UTF_8), "text/plain", "Sahne ağacı (TestFX)");
            }
        } finally {
            if (video != null) {
                byte[] kayit = video.durdur(scenario.isFailed());
                if (kayit != null) {
                    scenario.attach(kayit, "video/mp4", "Video");
                }
            }
            FxToolkit.cleanupApplication(new GirisUygulamasi());
            FxToolkit.cleanupStages();
        }
    }

    /** Açık pencerelerin düğüm ağacı: her satır "tür #fxId \"metin\"" biçiminde, girintili. */
    private static String sahneAgaci() {
        StringBuilder sb = new StringBuilder();
        for (javafx.stage.Window w : javafx.stage.Window.getWindows()) {
            if (w.getScene() != null) {
                sb.append("Pencere: ").append(w instanceof Stage st ? st.getTitle() : w.getClass().getSimpleName()).append('\n');
                yaz(w.getScene().getRoot(), 1, sb);
            }
        }
        return sb.toString();
    }

    private static void yaz(javafx.scene.Node n, int derinlik, StringBuilder sb) {
        sb.append("  ".repeat(derinlik)).append(n.getClass().getSimpleName());
        if (n.getId() != null) {
            sb.append(" #").append(n.getId());
        }
        if (n instanceof javafx.scene.control.Labeled l && l.getText() != null && !l.getText().isEmpty()) {
            sb.append(" \"").append(l.getText()).append('"');
        }
        if (n instanceof javafx.scene.control.TextInputControl t && !(n instanceof javafx.scene.control.PasswordField)) {
            sb.append(" değer=\"").append(t.getText()).append('"');
        }
        if (!n.isVisible()) {
            sb.append(" (gizli)");
        }
        if (n.isDisabled()) {
            sb.append(" (pasif)");
        }
        sb.append('\n');
        if (n instanceof javafx.scene.Parent p && !(n instanceof javafx.scene.control.Control)) {
            for (javafx.scene.Node c : p.getChildrenUnmodifiable()) {
                yaz(c, derinlik + 1, sb);
            }
        }
    }

    /** Uygulama penceresinin bulunduğu ekran bölgesinin gerçek görüntüsü (robot ile). */
    private static byte[] ekranGoruntusu() throws Exception {
        Rectangle2D bolge = stage != null && stage.isShowing()
                ? new Rectangle2D(stage.getX(), stage.getY(), stage.getWidth(), stage.getHeight())
                : Screen.getPrimary().getBounds();
        Image goruntu = new FxRobot().capture(bolge).getImage();
        ByteArrayOutputStream cikti = new ByteArrayOutputStream();
        ImageIO.write(SwingFXUtils.fromFXImage(goruntu, null), "png", cikti);
        return cikti.toByteArray();
    }
}
