package com.ornek.test;

import javafx.embed.swing.SwingFXUtils;
import javafx.geometry.Rectangle2D;
import javafx.scene.Node;
import javafx.scene.Parent;
import javafx.scene.control.Control;
import javafx.scene.control.Labeled;
import javafx.scene.control.PasswordField;
import javafx.scene.control.TextInputControl;
import javafx.scene.image.Image;
import javafx.stage.Screen;
import javafx.stage.Stage;
import javafx.stage.Window;
import org.testfx.api.FxRobot;

import javax.imageio.ImageIO;
import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;

/**
 * Kalan senaryonun kanıtları: gerçek ekran görüntüsü ve TestFX sahne ağacı.
 * Hook'a yalnızca {@link #ekle(io.cucumber.java.Scenario, Stage)} çağrısı eklenir.
 * Kanıt alınamazsa senaryonun gerçek hatası gölgelenmez; yalnızca System.err'e yazılır.
 */
public final class DivanKanit {

    private DivanKanit() {
    }

    /** Senaryo kaldıysa ekran görüntüsünü ve sahne ağacını Cucumber ekine koyar. */
    public static void ekle(io.cucumber.java.Scenario senaryo, Stage stage) {
        if (!senaryo.isFailed()) {
            return;
        }
        try {
            senaryo.attach(ekranGoruntusu(stage), "image/png", "Ekran görüntüsü");
            senaryo.attach(sahneAgaci().getBytes(StandardCharsets.UTF_8), "text/plain", "Sahne ağacı (TestFX)");
        } catch (Exception e) {
            System.err.println("[divan] kanıt alınamadı: " + e.getMessage());
        }
    }

    private static byte[] ekranGoruntusu(Stage stage) throws Exception {
        Rectangle2D bolge = stage != null && stage.isShowing()
                ? new Rectangle2D(stage.getX(), stage.getY(), stage.getWidth(), stage.getHeight())
                : Screen.getPrimary().getBounds();
        Image goruntu = new FxRobot().capture(bolge).getImage();
        ByteArrayOutputStream cikti = new ByteArrayOutputStream();
        ImageIO.write(SwingFXUtils.fromFXImage(goruntu, null), "png", cikti);
        return cikti.toByteArray();
    }

    private static String sahneAgaci() {
        StringBuilder sb = new StringBuilder();
        for (Window w : Window.getWindows()) {
            if (w.getScene() != null) {
                sb.append("Pencere: ").append(w instanceof Stage st ? st.getTitle() : w.getClass().getSimpleName()).append('\n');
                yaz(w.getScene().getRoot(), 1, sb);
            }
        }
        return sb.toString();
    }

    private static void yaz(Node n, int derinlik, StringBuilder sb) {
        sb.append("  ".repeat(derinlik)).append(n.getClass().getSimpleName());
        if (n.getId() != null) {
            sb.append(" #").append(n.getId());
        }
        if (n instanceof Labeled l && l.getText() != null && !l.getText().isEmpty()) {
            sb.append(" \"").append(l.getText()).append('"');
        }
        if (n instanceof TextInputControl t && !(n instanceof PasswordField)) {
            sb.append(" değer=\"").append(t.getText()).append('"');
        }
        if (!n.isVisible()) {
            sb.append(" (gizli)");
        }
        if (n.isDisabled()) {
            sb.append(" (pasif)");
        }
        sb.append('\n');
        if (n instanceof Parent p && !(n instanceof Control)) {
            for (Node c : p.getChildrenUnmodifiable()) {
                yaz(c, derinlik + 1, sb);
            }
        }
    }
}
