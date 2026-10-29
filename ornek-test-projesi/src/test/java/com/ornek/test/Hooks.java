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
