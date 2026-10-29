package com.ornek.test;

import io.cucumber.java.en.Given;
import io.cucumber.java.en.When;
import io.cucumber.java.en.Then;
import io.qameta.allure.Allure;
import javafx.scene.control.Label;
import org.testfx.api.FxRobot;

import java.util.concurrent.atomic.AtomicInteger;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

public class GirisAdimlari {

    private static final AtomicInteger GECIKME_DENEMESI = new AtomicInteger();
    private final FxRobot robot = new FxRobot();

    @Given("giriş ekranı açık")
    public void girisEkraniAcik() {
        assertTrue(Hooks.stage.isShowing(), "Uygulama penceresi açılmadı");
    }

    @When("kullanıcı adı {string} ve şifre {string} girilirse")
    public void bilgileriGir(String kullanici, String sifre) {
        robot.clickOn("#kullaniciAdi");
        if (!kullanici.isEmpty()) {
            robot.write(kullanici);
        }
        robot.clickOn("#sifre").write(sifre);
        // Adım içinde ek örneği: Allure adım ekleri result.json içinde steps[].attachments altına düşer
        Allure.addAttachment("Girilen kullanıcı", "text/plain", kullanici.isEmpty() ? "(boş)" : kullanici);
    }

    @When("giriş butonuna basılırsa")
    public void giriseBas() {
        robot.clickOn("#girisButonu");
    }

    @Then("{string} mesajı görünür")
    public void mesajGorunur(String beklenen) {
        Label mesaj = robot.lookup("#mesaj").queryAs(Label.class);
        assertEquals(beklenen, mesaj.getText());
    }

    @When("{string} bağlantısına tıklanırsa")
    public void baglantiyaTikla(String metin) {
        // Kasıtlı test kodu hatası: böyle bir düğüm yok, TestFX istisna fırlatır.
        robot.clickOn("#sifremiUnuttum");
    }

    @Then("karşılama mesajı ilk denemede gecikir")
    public void karsilamaGecikir() {
        Label mesaj = robot.lookup("#mesaj").queryAs(Label.class);
        if (GECIKME_DENEMESI.incrementAndGet() == 1) {
            assertEquals("Hoş geldin, admin!", mesaj.getText(), "İlk denemede mesaj henüz güncellenmedi");
        }
        assertEquals("Hoş geldin, admin", mesaj.getText());
    }
}
