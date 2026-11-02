# common/src/tls.rs

**Amaç:** divan ve ulak'ın dış HTTPS bağlantıları (GitLab API, Jira, auto-fix push) için sertifika
doğrulaması. Şirket GitLab/Jira'ları çoğunlukla şirket CA'sıyla imzalıdır; ureq'in gömülü Mozilla kök
listesi onları tanımaz (`invalid peer certificate: UnknownIssuer`).

| Fonksiyon | Davranış |
|-----------|----------|
| `config(ca_envs)` | listedeki ilk dolu ortam değişkeninin gösterdiği PEM dosyasındaki CA'lar (`RootCerts::Specific`), hiçbiri yoksa işletim sisteminin deposu (`RootCerts::PlatformVerifier`, ureq `platform-verifier` özelliği: Windows CryptoAPI / Linux sistem sertifikaları). Dosya okunamazsa ya da sertifika yoksa hata |
| `config_or_platform(ca_envs)` | hata olursa sistem deposuyla devam eder, hatayı döndürür (çağıran loglar) |
| `ca_file(ca_envs)` | ilk dolu değişken ve dosya yolu (git'e `http.sslCAInfo` vermek için) |
| `load_pem(yol)` | PEM'den sertifikalar; boşsa ".cer ise Base-64 olarak dışa aktarın" ipucu |
| `hint(hata, değişken)` | hata sertifikayla ilgiliyse "şirket CA'sı: sistem deposuna ekleyin ya da `<değişken>`" eki |

Kullanım: divan `gitlab::agent()` (Jira da bunu kullanır), `CA_ENV = DIVAN_CA_DOSYASI`; ulak
`fix::https_agent()`, `CA_ENVS = [ULAK_CA_DOSYASI, CI_SERVER_TLS_CA_FILE]`. Doğrulama kapatma seçeneği yoktur.

## Testler
`test_sw_9_1_ca_file_errors_are_clear`. Elle: şirket CA'sı taklidiyle HTTPS sahte GitLab'a karşı ayarsız
(UnknownIssuer + ipucu), PEM ile (bağlanır), Linux sistem deposuna ekleyerek (bağlanır) denendi.

## Python karşılığı
`requests` için `verify=<pem yolu>`; sistem deposu için `truststore` paketi (`truststore.inject_into_ssl()`).
