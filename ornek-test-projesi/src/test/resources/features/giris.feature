# language: tr
@giris
Özellik: Giriş ekranı
  Kullanıcı doğru bilgilerle giriş yapabilmeli, yanlış bilgilerde açık bir mesaj görmeli.

  Geçmiş:
    Diyelim ki giriş ekranı açık

  @DOORS-1001 @smoke
  Senaryo: Geçerli bilgilerle giriş
    Eğer ki kullanıcı adı "admin" ve şifre "1234" girilirse
    Ve giriş butonuna basılırsa
    O zaman "Hoş geldin, admin" mesajı görünür

  @DOORS-1002
  Senaryo taslağı: Hatalı girişlerde mesaj
    Eğer ki kullanıcı adı "<kullanici>" ve şifre "<sifre>" girilirse
    Ve giriş butonuna basılırsa
    O zaman "<mesaj>" mesajı görünür

    Örnekler:
      | kullanici | sifre | mesaj                           |
      | admin     | 9999  | Kullanıcı adı veya şifre yanlış |
      |           | 1234  | Kullanıcı adı boş olamaz        |

  # Ürün hatası örneği: beklenen metin uygulamadakinden farklı -> AssertionError -> Allure "failed"
  @DOORS-1003 @JIRA-STAJ-42 @sample-fail
  Senaryo: Yanlış şifrede özel mesaj
    Eğer ki kullanıcı adı "admin" ve şifre "0000" girilirse
    Ve giriş butonuna basılırsa
    O zaman "Şifre hatalı" mesajı görünür

  # Test kodu hatası örneği: olmayan bir düğüm aranır -> istisna -> Allure "broken"
  @DOORS-1004
  Senaryo: Şifremi unuttum bağlantısı
    Eğer ki "Şifremi unuttum" bağlantısına tıklanırsa
    O zaman "Şifre sıfırlama" mesajı görünür

  # Flaky örneği: ilk denemede kalır, rerun'da geçer (aynı historyId ile iki sonuç)
  @DOORS-1005 @flaky
  Senaryo: Yavaş açılan karşılama mesajı
    Eğer ki kullanıcı adı "admin" ve şifre "1234" girilirse
    Ve giriş butonuna basılırsa
    O zaman karşılama mesajı ilk denemede gecikir

  # Tanımsız adım örneği -> Allure "skipped/unknown"
  @DOORS-1006 @wip
  Senaryo: Beni hatırla seçeneği
    Eğer ki beni hatırla kutusu işaretlenirse
    O zaman kullanıcı adı bir sonraki açılışta dolu gelir
