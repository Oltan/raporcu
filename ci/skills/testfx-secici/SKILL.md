---
name: testfx-secici
description: TestFX ile JavaFX düğümü bulma, tıklama, yazma ve bekleme. Seçici (#fxId) hatası, "Query returned no nodes", zaman aşımı ya da yanlış ekranda kalma gibi TestFX hatalarını düzeltirken yükle.
---

# TestFX seçici ve bekleme kuralları

## Seçici seçimi
1. İstemdeki **"Sahne ağacı (TestFX)"** eki hata anında ekranda olan düğümleri verir:
   `Tür #fxId "metin" değer="..." (gizli) (pasif)`. Seçiciyi **buradan** al, tahmin etme.
2. Öncelik: `#fxId` > CSS sınıfı (`.giris-butonu`) > metin. Metne bağlı seçici (`"Giriş"`)
   dil/çeviri değişince kırılır; yalnızca başka yol yoksa kullan.
3. Sahne ağacında düğüm `(gizli)` ya da `(pasif)` ise sorun seçici değil zamanlamadır
   (aşağıdaki bekleme) ya da ürün davranışıdır (bkz. `hata-siniflandirma`).
4. Düğüm sahne ağacında hiç yoksa ve ekran görüntüsü başka bir ekranı gösteriyorsa, önceki
   adım beklenen ekrana geçmemiştir: hatayı o adımda ara.

## Doğru kullanım
```java
robot.clickOn("#girisButonu");
robot.clickOn("#kullaniciAdi").write("admin");
Label mesaj = robot.lookup("#mesaj").queryAs(Label.class);
// birden çok eşleşme olabiliyorsa:
Button b = robot.lookup(".tamam-butonu").queryAllAs(Button.class).stream().filter(Node::isVisible).findFirst().orElseThrow();
```

## Bekleme
- `Thread.sleep` **kullanma**. Koşullu bekle:
```java
import org.testfx.util.WaitForAsyncUtils;
import java.util.concurrent.TimeUnit;

WaitForAsyncUtils.waitFor(5, TimeUnit.SECONDS,
    () -> robot.lookup("#mesaj").tryQuery().isPresent());
WaitForAsyncUtils.waitFor(5, TimeUnit.SECONDS,
    () -> "Hoş geldin, admin".equals(robot.lookup("#mesaj").queryAs(Label.class).getText()));
WaitForAsyncUtils.waitForFxEvents();   // FX kuyruğundaki işler bitsin
```
- Bekleme süresi senaryonun mantıklı süresini aşmasın (genelde ≤ 10 sn).
- Bekleme koşulu **beklenen değerin kendisi** olabilir; bu assertion'ı gevşetmek değildir,
  ama bekleme sonunda yine aynı `assertEquals` çalışmalıdır.

## Yapma
- `src/main/` altındaki uygulama koduna `fx:id` ekleme ya da değiştirme (yasak; ürün değişikliği gerekir → `urun`).
- Headless/Monocle ayarı ekleme; testler gerçek ekranda koşar.
- `lookup(...).query()` sonucunu null kontrolsüz kullanma; `tryQuery()` ile kontrol et.
