//! Java destek eklentisi (eklentiler/java) tek kaynaktır. Örnek proje aynı sınıfların kopyalarını
//! kendi paketinde tutar (kopyalanacak örnek olsun diye); kopyalar eklentiden ayrışmamalı.

use std::path::PathBuf;

fn kok() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn oku(p: &str) -> String {
    std::fs::read_to_string(kok().join(p))
        .unwrap_or_else(|e| panic!("{p}: {e}"))
        .replace("\r\n", "\n")
}

#[test]
fn test_sw_11_3_ornek_proje_kopyalari_eklentiyle_ayni() {
    for ad in ["CanliOlayEklentisi.java", "VideoKaydi.java"] {
        let eklenti = oku(&format!("eklentiler/java/src/main/java/divan/destek/{ad}"));
        let ornek = oku(&format!("ornek-test-projesi/src/test/java/com/ornek/test/{ad}")).replace("com.ornek.test", "divan.destek");
        assert!(
            eklenti == ornek,
            "{ad}: eklentiler/java ile ornek-test-projesi kopyası farklı; değişikliği ikisine de uygulayın"
        );
    }
}

#[test]
fn test_sw_11_3_pom_eklentisi_surumu_eklentiyle_ayni() {
    let surum = |metin: &str| -> String {
        let i = metin.find("<artifactId>divan-test-destek</artifactId>").expect("artifactId yok");
        let s = &metin[i..];
        let a = s.find("<version>").unwrap() + "<version>".len();
        let b = s.find("</version>").unwrap();
        s[a..b].trim().to_string()
    };
    assert_eq!(
        surum(&oku("eklentiler/java/pom.xml")),
        surum(&oku("eklentiler/java/pom-eklentisi.xml")),
        "pom-eklentisi.xml'deki bağımlılık sürümü eklentinin sürümüyle aynı olmalı"
    );
}
