//! Sunucu tarafında üretilen SVG grafikler (JS gerekmez; ipucu ve çizgi takibi static/charts.js).
//!
//! Kurallar: ince işaretler (çizgi 2px, sütun en fazla 24px, veri ucu 4px yuvarlak, taban düz),
//! dokunan parçalar arasında 2px yüzey boşluğu, işaretçilerde 2px yüzey halkası, sönük ızgara,
//! tek eksen. Renkler CSS değişkenleriyle (`--c-gecti` ...) gelir; koyu tema kendi adımlarını kullanır.
//! Metinler hiçbir zaman veri renginde değildir.

use std::fmt::Write;

/// Durum dilimi: CSS sınıfı (`gecti`, `kaldi`, `broken`, `diger`), ad, değer.
#[derive(Clone, Debug)]
pub struct Slice {
    pub class: &'static str,
    pub name: &'static str,
    pub value: i64,
}

pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn pct(part: i64, total: i64) -> i64 {
    if total <= 0 {
        0
    } else {
        ((part as f64) * 100.0 / (total as f64)).round() as i64
    }
}

pub fn status_slices(passed: i64, failed: i64, broken: i64, other: i64) -> Vec<Slice> {
    vec![
        Slice {
            class: "gecti",
            name: "Geçti",
            value: passed,
        },
        Slice {
            class: "kaldi",
            name: "Kaldı",
            value: failed,
        },
        Slice {
            class: "broken",
            name: "Broken",
            value: broken,
        },
        Slice {
            class: "diger",
            name: "Bilinmiyor / atlandı",
            value: other,
        },
    ]
}

/// Genel rapor dilimleri: iç durum adları yerine doğrulama sınıfları.
pub fn genel_slices(dogrulanan: i64, urun: i64, inceleniyor: i64) -> Vec<Slice> {
    vec![
        Slice {
            class: "gecti",
            name: "Doğrulandı",
            value: dogrulanan,
        },
        Slice {
            class: "kaldi",
            name: "Ürün hatası",
            value: urun,
        },
        Slice {
            class: "incele",
            name: "İnceleniyor",
            value: inceleniyor,
        },
    ]
}

/// Halka grafik: dilimler arasında 2px boşluk, ortada toplam ve geçme oranı.
pub fn donut(slices: &[Slice], center_value: &str, center_label: &str, aria: &str) -> String {
    let total: i64 = slices.iter().map(|s| s.value).sum();
    let (size, r, w) = (200.0_f64, 78.0_f64, 22.0_f64);
    let c = size / 2.0;
    let mut out = String::new();
    let _ = write!(
        out,
        r#"<svg class="donut" viewBox="0 0 {size} {size}" role="img" aria-label="{}">"#,
        esc(aria)
    );
    let _ = write!(
        out,
        r#"<circle cx="{c}" cy="{c}" r="{r}" class="iz" stroke-width="{w}" fill="none"/>"#
    );
    let nonzero: Vec<&Slice> = slices.iter().filter(|s| s.value > 0).collect();
    if total > 0 {
        if nonzero.len() == 1 {
            let s = nonzero[0];
            let _ = write!(
                out,
                r#"<circle cx="{c}" cy="{c}" r="{r}" class="m-{}" stroke-width="{w}" fill="none" tabindex="0" data-tip="{}: {} (%100)"/>"#,
                s.class,
                esc(s.name),
                s.value
            );
        } else {
            // 2px yüzey boşluğu: yay uzunluğu cinsinden açıya çevir
            let gap = 2.0 / r;
            let mut a = -std::f64::consts::FRAC_PI_2;
            for s in &nonzero {
                let sweep = (s.value as f64) / (total as f64) * std::f64::consts::TAU;
                let (a0, a1) = (a + gap / 2.0, a + sweep - gap / 2.0);
                a += sweep;
                if a1 <= a0 {
                    continue;
                }
                let (x0, y0) = (c + r * a0.cos(), c + r * a0.sin());
                let (x1, y1) = (c + r * a1.cos(), c + r * a1.sin());
                let large = if a1 - a0 > std::f64::consts::PI { 1 } else { 0 };
                let _ = write!(
                    out,
                    r#"<path d="M{x0:.2} {y0:.2} A{r} {r} 0 {large} 1 {x1:.2} {y1:.2}" class="m-{}" stroke-width="{w}" fill="none" tabindex="0" data-tip="{}: {} (%{})"/>"#,
                    s.class,
                    esc(s.name),
                    s.value,
                    pct(s.value, total)
                );
            }
        }
    }
    let _ = write!(
        out,
        r#"<text x="{c}" y="{}" class="donut-deger" text-anchor="middle">{}</text><text x="{c}" y="{}" class="donut-etiket" text-anchor="middle">{}</text></svg>"#,
        c + 4.0,
        esc(center_value),
        c + 26.0,
        esc(center_label)
    );
    out
}

/// İnce segmentli çubuk (tablo satırları için): 8px, uçlar yuvarlak, parçalar arası 2px.
pub fn thin_bar(slices: &[Slice]) -> String {
    let total: i64 = slices.iter().map(|s| s.value).sum();
    let mut out = String::from(r#"<div class="ince-cubuk">"#);
    for s in slices.iter().filter(|s| s.value > 0) {
        let _ = write!(
            out,
            r#"<span class="m-{}" style="flex-grow:{}" tabindex="0" data-tip="{}: {} (%{})"></span>"#,
            s.class,
            s.value,
            esc(s.name),
            s.value,
            pct(s.value, total)
        );
    }
    out.push_str("</div>");
    out
}

/// Gidişat noktası: x ekseni etiketi, tıklanınca gidilecek adres, değer ve ipucu ayrıntısı.
#[derive(Clone, Debug)]
pub struct Point {
    pub label: String,
    pub href: String,
    pub value: f64,
    pub detail: String,
}

/// Çizgi grafik: y 0–100 (%), 2px çizgi, %10 alan dolgusu, uç değer etiketi, takip çizgisi (JS).
pub fn line_pct(points: &[Point], aria: &str) -> String {
    let (w, h) = (720.0_f64, 220.0_f64);
    let (l, r, t, b) = (40.0, 44.0, 12.0, 28.0);
    let (pw, ph) = (w - l - r, h - t - b);
    let mut out = String::new();
    let n = points.len();
    let xs: Vec<f64> = (0..n)
        .map(|i| if n <= 1 { l + pw / 2.0 } else { l + pw * i as f64 / (n - 1) as f64 })
        .collect();
    let y = |v: f64| t + ph * (1.0 - v.clamp(0.0, 100.0) / 100.0);
    let data: Vec<serde_json::Value> = points
        .iter()
        .zip(&xs)
        .map(|(p, x)| serde_json::json!({ "x": x, "y": y(p.value), "baslik": p.label, "deger": format!("%{}", p.value.round()), "ayrinti": p.detail, "href": p.href }))
        .collect();
    let _ = write!(
        out,
        r#"<svg class="cizgi-grafik" viewBox="0 0 {w} {h}" role="img" aria-label="{}" data-noktalar="{}">"#,
        esc(aria),
        esc(&serde_json::Value::Array(data).to_string())
    );
    for v in [0, 25, 50, 75, 100] {
        let yy = y(v as f64);
        let _ = write!(
            out,
            r#"<line x1="{l}" x2="{}" y1="{yy}" y2="{yy}" class="izgara"/><text x="{}" y="{}" class="eksen" text-anchor="end">%{v}</text>"#,
            w - r,
            l - 8.0,
            yy + 4.0
        );
    }
    if n == 0 {
        out.push_str("</svg>");
        return out;
    }
    // x etiketleri: en fazla ~8 tane
    let step = n.div_ceil(8).max(1);
    for (i, p) in points.iter().enumerate() {
        if i % step == 0 || i == n - 1 {
            let _ = write!(
                out,
                r#"<text x="{:.1}" y="{}" class="eksen" text-anchor="middle">{}</text>"#,
                xs[i],
                h - 8.0,
                esc(&p.label)
            );
        }
    }
    let path: String = points
        .iter()
        .zip(&xs)
        .enumerate()
        .map(|(i, (p, x))| format!("{}{x:.1} {:.1}", if i == 0 { "M" } else { "L" }, y(p.value)))
        .collect();
    let _ = write!(
        out,
        r#"<path d="{path} L{:.1} {:.1} L{:.1} {:.1} Z" class="alan"/>"#,
        xs[n - 1],
        y(0.0),
        xs[0],
        y(0.0)
    );
    let _ = write!(out, r#"<path d="{path}" class="cizgi" fill="none"/>"#);
    for (p, x) in points.iter().zip(&xs) {
        let _ = write!(out, r#"<circle cx="{x:.1}" cy="{:.1}" r="4" class="nokta"/>"#, y(p.value));
    }
    let last = &points[n - 1];
    let _ = write!(
        out,
        r#"<text x="{:.1}" y="{:.1}" class="uc-etiket">%{}</text>"#,
        xs[n - 1] + 8.0,
        y(last.value) + 4.0,
        last.value.round()
    );
    out.push_str(r#"<line class="takip" x1="0" x2="0" y1="0" y2="0" visibility="hidden"/></svg>"#);
    out
}

/// Yığılmış sütun: her grup bir sütun, parçalar arası 2px, en üst parça 4px yuvarlak, taban düz.
pub struct Column {
    pub label: String,
    pub href: Option<String>,
    pub title: String,
    pub parts: Vec<Slice>,
}

/// 4 eşit aralıklı, tam sayı ızgara için üst sınır: adım ∈ {1,2,3,4,5,6,8}×10ⁿ, 4×adım ≥ v.
fn nice_max(v: i64) -> i64 {
    if v <= 4 {
        return 4;
    }
    let need = v as f64 / 4.0;
    let mag = 10_f64.powf(need.log10().floor());
    for m in [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0] {
        if m * mag >= need {
            return (4.0 * m * mag).round() as i64;
        }
    }
    (40.0 * mag) as i64
}

/// Tam genişlik kart için.
pub fn stacked_columns(cols: &[Column], aria: &str) -> String {
    stacked_columns_sized(cols, aria, 1200.0, 260.0)
}

/// `w`, kartın ekrandaki genişliğine yakın seçilir; yazılar ekranda ~11px kalsın.
pub fn stacked_columns_sized(cols: &[Column], aria: &str, w: f64, h: f64) -> String {
    let (l, r, t, b) = (40.0, 12.0, 12.0, 28.0);
    let (pw, ph) = (w - l - r, h - t - b);
    let max = nice_max(cols.iter().map(|c| c.parts.iter().map(|p| p.value).sum::<i64>()).max().unwrap_or(0));
    let mut out = String::new();
    let _ = write!(
        out,
        r#"<svg class="sutun-grafik" viewBox="0 0 {w} {h}" role="img" aria-label="{}">"#,
        esc(aria)
    );
    for i in 0..=4 {
        let v = max * i / 4;
        let yy = t + ph * (1.0 - v as f64 / max as f64);
        let _ = write!(
            out,
            r#"<line x1="{l}" x2="{}" y1="{yy}" y2="{yy}" class="izgara"/><text x="{}" y="{}" class="eksen" text-anchor="end">{v}</text>"#,
            w - r,
            l - 8.0,
            yy + 4.0
        );
    }
    let n = cols.len().max(1);
    let band = pw / n as f64;
    let bw = (band * 0.6).min(24.0);
    let step = n.div_ceil(10).max(1);
    for (i, c) in cols.iter().enumerate() {
        let x = l + band * i as f64 + (band - bw) / 2.0;
        let mut ybase = t + ph;
        let visible: Vec<&Slice> = c.parts.iter().filter(|p| p.value > 0).collect();
        if let Some(href) = &c.href {
            let _ = write!(out, r#"<a href="{}">"#, esc(href));
        }
        for (k, p) in visible.iter().enumerate() {
            let hh = ph * p.value as f64 / max as f64;
            let top = k == visible.len() - 1;
            let gap = if k > 0 { 2.0 } else { 0.0 };
            let (y0, hh2) = (ybase - hh, (hh - gap).max(0.5));
            let tip = format!("{} · {}: {}", c.title, p.name, p.value);
            if top && hh2 > 4.0 {
                // üstte 4px yuvarlak, altta düz
                let rr = 4.0_f64.min(bw / 2.0);
                let _ = write!(
                    out,
                    r#"<path class="m-{}" tabindex="0" data-tip="{}" d="M{x:.1} {:.1} V{:.1} Q{x:.1} {:.1} {:.1} {:.1} H{:.1} Q{:.1} {:.1} {:.1} {:.1} V{:.1} Z"/>"#,
                    p.class,
                    esc(&tip),
                    y0 + hh2,
                    y0 + rr,
                    y0,
                    x + rr,
                    y0,
                    x + bw - rr,
                    x + bw,
                    y0,
                    x + bw,
                    y0 + rr,
                    y0 + hh2
                );
            } else {
                let _ = write!(
                    out,
                    r#"<rect class="m-{}" tabindex="0" data-tip="{}" x="{x:.1}" y="{y0:.1}" width="{bw:.1}" height="{hh2:.1}"/>"#,
                    p.class,
                    esc(&tip)
                );
            }
            ybase -= hh;
        }
        if c.href.is_some() {
            out.push_str("</a>");
        }
        if i % step == 0 || i == n - 1 {
            let _ = write!(
                out,
                r#"<text x="{:.1}" y="{}" class="eksen" text-anchor="middle">{}</text>"#,
                x + bw / 2.0,
                h - 8.0,
                esc(&c.label)
            );
        }
    }
    out.push_str("</svg>");
    out
}

/// Mini çizgi (tablo hücresi): son N koşumun geçme oranı, son nokta vurgulu.
pub fn sparkline(values: &[f64], aria: &str) -> String {
    let (w, h) = (110.0_f64, 28.0_f64);
    let n = values.len();
    let mut out = format!(
        r#"<svg class="mini-cizgi" viewBox="0 0 {w} {h}" role="img" aria-label="{}">"#,
        esc(aria)
    );
    if n == 0 {
        out.push_str("</svg>");
        return out;
    }
    let x = |i: usize| {
        if n == 1 {
            w / 2.0
        } else {
            4.0 + (w - 8.0) * i as f64 / (n - 1) as f64
        }
    };
    let y = |v: f64| 4.0 + (h - 8.0) * (1.0 - v.clamp(0.0, 100.0) / 100.0);
    let path: String = values
        .iter()
        .enumerate()
        .map(|(i, v)| format!("{}{:.1} {:.1}", if i == 0 { "M" } else { "L" }, x(i), y(*v)))
        .collect();
    let _ = write!(out, r#"<path d="{path}" class="cizgi ince" fill="none"/>"#);
    let _ = write!(
        out,
        r#"<circle cx="{:.1}" cy="{:.1}" r="3" class="nokta"/>"#,
        x(n - 1),
        y(values[n - 1])
    );
    out.push_str("</svg>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_4_2_donut_segments_and_center() {
        let s = donut(&status_slices(13, 3, 2, 3), "21", "%62 geçti", "Koşum dağılımı");
        assert_eq!(s.matches("<path").count(), 4);
        assert!(s.contains(">21<") && s.contains("%62 geçti"));
        assert!(s.contains("data-tip=\"Geçti: 13 (%62)\""));
        // tek durum: tam halka
        let s = donut(&status_slices(5, 0, 0, 0), "5", "%100 geçti", "x");
        assert_eq!(s.matches("<path").count(), 0);
        assert!(s.contains("class=\"m-gecti\""));
    }

    #[test]
    fn test_sw_4_2_line_and_columns_escape_and_scale() {
        let pts = vec![
            Point {
                label: "#1".into(),
                href: "/runs/1".into(),
                value: 50.0,
                detail: "<b>".into(),
            },
            Point {
                label: "#2".into(),
                href: "/runs/2".into(),
                value: 75.0,
                detail: "x".into(),
            },
        ];
        let s = line_pct(&pts, "Gidişat");
        assert!(!s.contains("<b>"), "veri metni kaçışlı");
        assert!(s.contains("%75"));
        let c = stacked_columns(
            &[Column {
                label: "#1".into(),
                href: None,
                title: "Koşum #1".into(),
                parts: status_slices(13, 3, 2, 3),
            }],
            "Sayılar",
        );
        assert!(c.contains(">24<"), "y ekseni yuvarlak üst sınır");
        assert_eq!(nice_max(21), 24);
        assert_eq!(nice_max(7), 8);
        assert_eq!(nice_max(150), 160);
        assert_eq!(nice_max(400), 400);
    }
}
