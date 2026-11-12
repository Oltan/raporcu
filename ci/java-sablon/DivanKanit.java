package com.ornek.test;

import javafx.application.Platform;
import javafx.embed.swing.SwingFXUtils;
import javafx.geometry.Rectangle2D;
import javafx.scene.Node;
import javafx.scene.Parent;
import javafx.scene.control.Accordion;
import javafx.scene.control.ButtonBar;
import javafx.scene.control.Control;
import javafx.scene.control.Labeled;
import javafx.scene.control.PasswordField;
import javafx.scene.control.ScrollPane;
import javafx.scene.control.SplitPane;
import javafx.scene.control.Tab;
import javafx.scene.control.TabPane;
import javafx.scene.control.TextInputControl;
import javafx.scene.control.TitledPane;
import javafx.scene.control.ToolBar;
import javafx.scene.image.Image;
import javafx.stage.Screen;
import javafx.stage.Stage;
import javafx.stage.Window;
import org.testfx.api.FxRobot;

import javax.imageio.ImageIO;
import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.FutureTask;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;

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
        // iki kanıt birbirinden bağımsız: ekran görüntüsü alınamasa da sahne ağacı eklenir
        try {
            senaryo.attach(ekranGoruntusu(stage), "image/png", "Ekran görüntüsü");
        } catch (Exception e) {
            System.err.println("[divan] ekran görüntüsü alınamadı: " + e);
        }
        try {
            String agac = sahneAgaci();
            if (agac.isBlank()) {
                System.err.println("[divan] sahne ağacı boş: açık pencere yok. DivanKanit.ekle pencere kapatılmadan önce çağrılmalı.");
            } else {
                senaryo.attach(agac.getBytes(StandardCharsets.UTF_8), "text/plain", "Sahne ağacı (TestFX)");
            }
        } catch (Exception e) {
            System.err.println("[divan] sahne ağacı alınamadı: " + e);
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

    /** Sahne JavaFX iş parçacığında okunur; o meşgulse (10 sn) test iş parçacığından okunur. */
    static String sahneAgaci() throws Exception {
        if (Platform.isFxApplicationThread()) {
            return pencereler();
        }
        FutureTask<String> is = new FutureTask<>(DivanKanit::pencereler);
        Platform.runLater(is);
        try {
            return is.get(10, TimeUnit.SECONDS);
        } catch (TimeoutException e) {
            is.cancel(false);
            return pencereler();
        }
    }

    private static String pencereler() {
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
        if (n instanceof TabPane sekmeler) {
            for (Tab t : sekmeler.getTabs()) {
                sb.append("  ".repeat(derinlik + 1)).append("Tab");
                if (t.getId() != null) {
                    sb.append(" #").append(t.getId());
                }
                if (t.getText() != null && !t.getText().isEmpty()) {
                    sb.append(" \"").append(t.getText()).append('"');
                }
                sb.append(t.isSelected() ? " (seçili)" : " (seçili değil)").append('\n');
                if (t.getContent() != null) {
                    yaz(t.getContent(), derinlik + 2, sb);
                }
            }
            return;
        }
        for (Node c : cocuklar(n)) {
            yaz(c, derinlik + 1, sb);
        }
    }

    /**
     * Yazılacak alt düğümler. İçerik taşıyan kontrollerin (ScrollPane, TitledPane, SplitPane...) içeriğine
     * inilir; diğer kontrollerin iç yapısı (skin: düğmenin içindeki Text vb.) yazılmaz.
     */
    private static List<Node> cocuklar(Node n) {
        List<Node> liste = new ArrayList<>();
        if (n instanceof ScrollPane s) {
            ekle(liste, s.getContent());
        } else if (n instanceof TitledPane t) {
            ekle(liste, t.getContent());
        } else if (n instanceof SplitPane s) {
            liste.addAll(s.getItems());
        } else if (n instanceof Accordion a) {
            liste.addAll(a.getPanes());
        } else if (n instanceof ToolBar t) {
            liste.addAll(t.getItems());
        } else if (n instanceof ButtonBar b) {
            liste.addAll(b.getButtons());
        } else if (n instanceof Parent p && !(n instanceof Control)) {
            liste.addAll(p.getChildrenUnmodifiable());
        }
        return liste;
    }

    private static void ekle(List<Node> liste, Node n) {
        if (n != null) {
            liste.add(n);
        }
    }
}
