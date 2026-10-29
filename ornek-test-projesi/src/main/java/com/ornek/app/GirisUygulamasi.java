package com.ornek.app;

import javafx.application.Application;
import javafx.geometry.Insets;
import javafx.scene.Scene;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.PasswordField;
import javafx.scene.control.TextField;
import javafx.scene.layout.VBox;
import javafx.stage.Stage;

/** Test edilen küçük JavaFX uygulaması: kullanıcı adı + şifre ile giriş ekranı. */
public class GirisUygulamasi extends Application {

    public static final String GECERLI_KULLANICI = "admin";
    public static final String GECERLI_SIFRE = "1234";

    @Override
    public void start(Stage stage) {
        TextField kullanici = new TextField();
        kullanici.setId("kullaniciAdi");
        kullanici.setPromptText("Kullanıcı adı");

        PasswordField sifre = new PasswordField();
        sifre.setId("sifre");
        sifre.setPromptText("Şifre");

        Label mesaj = new Label();
        mesaj.setId("mesaj");

        Button giris = new Button("Giriş yap");
        giris.setId("girisButonu");
        giris.setOnAction(e -> {
            if (kullanici.getText().isBlank()) {
                mesaj.setText("Kullanıcı adı boş olamaz");
            } else if (GECERLI_KULLANICI.equals(kullanici.getText()) && GECERLI_SIFRE.equals(sifre.getText())) {
                mesaj.setText("Hoş geldin, " + kullanici.getText());
            } else {
                mesaj.setText("Kullanıcı adı veya şifre yanlış");
            }
        });

        VBox kok = new VBox(10, kullanici, sifre, giris, mesaj);
        kok.setPadding(new Insets(20));
        stage.setTitle("Örnek Giriş");
        stage.setScene(new Scene(kok, 360, 220));
        stage.show();
    }

    public static void main(String[] args) {
        launch(args);
    }
}
