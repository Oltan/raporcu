# e2esmoke/src/main.rs

**Amaç:** SW-10.2 uçtan uca smoke. Ya kendi sunucusunu açar (admin şifresini
`divan set-admin-password` ile `.env`'e yazarak) ya da `ULAK_SERVER` + `CI_API_TOKEN` verilirse
**kurulu sunucuya** karşı koşar (kurulum sonrası doğrulama).

## Kontroller (sırayla)
| SW | Kontrol |
|----|---------|
| 10.5 | `/api/health` 200 |
| 2.1 | `ulak upload` örnek sonuçlarla koşum oluşturur, rapor linki basar |
| 4.1 | koşum sayfası, senaryo sayfası (adımlar, ekran görüntüsü) |
| 2.2 | ek dosyası indirilebilir |
| 10.6 | sunucu RSS hedefin altında (Linux) |
| 4.8 / 6.4 | CSV, DOORS CSV ve DXL (ayarlardan öznitelik değişkenleri) |
| 4.7 | DOORS numarasıyla arama |
| 9.5 | yanlış şifre reddedilir, doğru şifreyle admin girişi (yalnızca kendi sunucusunda) |
| 5.1 / 5.2 | paylaşım linki açılır; iptal edilince 404 |
| 3.6 | dashboard |
| 10.7 | log dosyasında run_id'li satırlar |
| 2.5 | veritabanı dosyası `DATA_DIR`'de kendiliğinden oluşmuş |
