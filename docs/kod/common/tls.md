# common/src/tls.rs

**Amaç:** divan ve ulak'ın dış HTTPS bağlantıları (GitLab API, Jira, auto-fix push) için sertifika
doğrulaması. Şirket GitLab/Jira'ları çoğunlukla şirket CA'sıyla imzalıdır; ureq'in gömülü Mozilla kök
listesi onları tanımaz (`invalid peer certificate: UnknownIssuer`).

| Fonksiyon | Davranış |
|-----------|----------|
| `config(ca_envs)` | listedeki ilk dolu ortam değişkeninin gösterdiği PEM dosyasındaki CA'lar (`RootCerts::Specific`), hiçbiri yoksa işletim sisteminin deposu (`RootCerts::PlatformVerifier`, ureq `platform-verifier` özelliği: Windows CryptoAPI / Linux sistem sertifikaları). Dosya okunamazsa ya da sertifika yoksa hata |
| `config_or_platform(ca_envs)` | hata olursa sistem deposuyla devam eder, hatayı döndürür (çağıran loglar) |
| `ca_file(ca_envs)` | ilk dolu değişken ve dosya yolu (git'e `http.sslCAInfo` vermek için) |
| `load_pem(yol)` | uzantıdan bağımsız: PEM metni (birden çok sertifika) ya da tek ikili DER sertifika (`0x30 0x82` + uzunluk dosya boyuna eşit) |
| `pem_path_for_git(yol, klasör)` | git `http.sslCAInfo` yalnızca PEM okur: DER ise PEM'e çevrilip `klasör/sirket-ca.pem` yazılır (ulak: `.ulak/`) |
| `hint(hata, değişken)` | hata sertifikayla ilgiliyse "şirket CA'sı: sistem deposuna ekleyin ya da `<değişken>`" eki |

Kullanım: divan `gitlab::agent()` (Jira da bunu kullanır), `CA_ENV = DIVAN_CA_DOSYASI`; ulak
`fix::https_agent()`, `CA_ENVS = [ULAK_CA_DOSYASI, CI_SERVER_TLS_CA_FILE]`. Doğrulama kapatma seçeneği yoktur.

## Testler
`test_sw_9_1_ca_file_errors_are_clear`, `test_sw_9_1_ca_file_pem_and_der`, `test_sw_9_1_der_cert_converted_back_to_same_pem_for_git` (gerçek test CA'sıyla, OpenSSL çıktısıyla birebir). Elle: şirket CA'sı taklidiyle HTTPS sahte GitLab'a karşı ayarsız
(UnknownIssuer + ipucu), PEM ile (bağlanır), Linux sistem deposuna ekleyerek (bağlanır) denendi.

## Python karşılığı
`requests` için `verify=<pem yolu>`; sistem deposu için `truststore` paketi (`truststore.inject_into_ssl()`).
