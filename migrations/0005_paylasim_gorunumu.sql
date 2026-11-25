-- SW-5.1: paylaşım linkinin görünümü: 'ekip' (ayrıntılı rapor) ya da 'genel' (yalnızca ürün sonucu)
ALTER TABLE share_links ADD COLUMN gorunum TEXT NOT NULL DEFAULT 'ekip';
