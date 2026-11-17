# Konuşma notları

Toplam ~25–30 dakika + sorular. Teknik olmayan dinleyici için 13–15 ve 17–18 kısaltılabilir.

## 1. divan
Tek cümle: gece koşan JavaFX e2e testlerinin sonuçlarını toplayan, kalanların nedenine karar veren ve
uygun olanları otomatik düzelten bir merkez. İki parça var: divan (sunucu) ve ulak (CI aracı).

## 2. Neden?
Sabah rutini: hangi testler kaldı, neden kaldı, ürün mü bozuk test mi, Jira'da bunun bug'ı var mı?
Bunların hepsi elle yapılıyordu. Sağ sütundaki her madde bir sonraki slaytların konusu.

## 3. İki parça
divan rapor sitesini ve verileri tutar; ulak pipeline'ın içinde çalışır. Pipeline dosyasında ulak
dışında komut yok; bu yüzden şablon iki işletim sisteminde aynı. Kurulum: iki exe dosyası.

## 4. Mimari
Önemli nokta: testleri divan koşmuyor, GitLab runner koşuyor. divan aynı makinede durup sonuçları
alıyor. Dış bağlantılar: tarayıcı, Jira, GitLab, e-posta. Dil modeli şirket içinde (vLLM); veri dışarı
çıkmıyor.

## 5. Bir gece koşusu
Beş adım. Vurgulanacaklar: test kalsa bile rapor ve düzeltme aşamaları çalışır; sunucu o an kapalı
olsa bile sonuçlar kuyruğa alınıp sonra gönderilir. Çıkış kodları sorunu GitLab'da tek bakışta söyler
(ör. 4 = Windows runner masaüstü oturumunda değil).

## 6. Canlı izleme
İki kanal: anlık olaylar ("şu an bu adımda") ve sonuç dosyaları. Gece koşusunu sabah izlemek gerekmez
ama gündüz uzun bir koşuyu takip etmek için kullanışlı. Mümkünse canlı ekranı gösterin.

## 7. Rapor
Ekran görüntüleri üzerinden: dashboard (gidişat, flaky) → koşum (feature bazlı) → senaryo (hata anı:
görüntü, video, sahne ağacı). Mümkünse bunu canlı sistemde gösterin.

## 8. Arama, dışa aktarma, paylaşım
Paylaşım linklerinin üç türü: ekip (ayrıntılı), genel (yalnızca doğrulananlar ve ürün hataları),
genel ana sayfa (proje/branch için kalıcı). Yöneticiye "genel" link gönderilir.

## 9. Ara başlık
Buradan sonrası divan'ı rapor aracından ayıran kısım: kalan senaryoya ne olduğu.

## 10. Triage
Öncelik sırası: admin > ajan (güven ≥ 0,70) > kural. Kural basit ve her zaman var; ajan onu
iyileştiriyor; insan her zaman son sözü söylüyor. Değerlendiren ajan dosya değiştiremiyor.

## 11. Jira
Dört adımlı eşleşme; amaç aynı hata için ikinci bug açmamak. Varsayılan "gözlem modu": bug açılmaz,
taslak hazırlanır, admin tek tıkla açar. Arama yalnızca divan'ın açtığı (`divan` label'lı) Bug'lara
bakar; story'lere fotoğraf eklenmez.

## 12. Gereksinim izlenebilirliği
`@YTET_…` gereksinim numarası, `@YGÖ-…` ek bağ. Raporda rozet, CSV'de sütun, Jira açıklamasında yer
tutucu. Gereksinim aracına aktarım CSV ile ve ekibin kendi betiğiyle.

## 13. Self-healing
Akış: branch → ajan → yol denetimi → tek senaryo doğrulama → MR. Hiçbir şey kendiliğinden
birleşmez; MR'ı insan inceler. Doğrulama geçmezse MR Draft açılır. Açık MR'ı olan senaryoya ikinci
MR açılmaz. Birleşen düzeltmeler sonraki benzer hatalarda örnek olarak kullanılır.

## 14. Güvenlik
Sık gelen soru: "yapay zekâ kodumuzu bozar mı?" Cevap: yalnızca `src/test/` değişebilir, komut
çalıştıramaz, internete çıkamaz, her değişiklik MR'dan geçer. Test çıktısındaki metinlere (prompt
enjeksiyonu) uymaz. Tokenlar loglara yazılmaz; bu testle denetleniyor.

## 15. Ajanlar ve skill'ler
İki ajan: değerlendir (okur) ve fix (düzeltir). Skill'ler ajanın el kitabı; projeye özel skill
eklenebilir (`.claude/skills/<ad>/SKILL.md`). OpenCode ya da Claude Code aynı dosyaları kullanır.

## 16. Test yazarken
Ekipten istenen: `fx:id`, bağımsız senaryolar, koşullu bekleme, gereksinim tag'i. En önemlisi
bağımsızlık: doğrulama ve auto-fix pipeline'ı senaryoyu tek başına koşturur; başka senaryoya bağlı
olan orada kalır. Koşum sırası dosya yoluna göredir; sıraya güvenilmemeli.

## 17. Teknik özet
Tek dosya, düşük bellek, ek çalışma ortamı yok. Ekipte Rust bilen yoksa: her dosyanın dilden bağımsız
açıklaması ve çalışan bir Python iskeleti var.

## 18. Nasıl doğrulanıyor?
Her gereksinimin numarası var, her testin adı numarayı söylüyor. Prova, gerçek divan ve ulak
süreçlerini sahte Maven/GitLab/ajanla uçtan uca çalıştırıyor.

## 19. Durum
"Kısmen"lerin çoğunun kodu hazır; eksik olan gerçek ortamda doğrulama. Sıradakileri kısaca geçin.

## 20. Projenizi bağlamak
Beş adım; çalışan örnek proje var. Ayrıntı rehberlerde.

## 21. Sorular
