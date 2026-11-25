//! SW-10.6: büyük zip yüklenirken sunucu bellek kullanımı hedefin altında kalır.
//! Ölçüm Linux'ta /proc üzerinden yapılır (tepe RSS sıfırlanıp yükleme sonrası okunur).
mod util;

#[cfg(target_os = "linux")]
fn status_kb(field: &str) -> u64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap();
    s.lines()
        .find(|l| l.starts_with(field))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse().ok())
        .unwrap()
}

/// Sıkıştırılamayan sözde rastgele veri (xorshift)
#[cfg(target_os = "linux")]
fn noise(seed: u64, len: usize) -> Vec<u8> {
    let mut x = seed | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u8
        })
        .collect()
}

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_sw_10_6_large_zip_upload_stays_under_memory_target() {
    use axum::body::Body;
    use axum::http::Request;
    use std::io::Write;

    let app = util::app();
    let target_mb = app.state.cfg.memory_target_mb;

    // ~300 senaryo + 150 adet 1 MB ek (sıkıştırmasız) ≈ 150 MB zip
    let zip_path = app.dir.path().join("buyuk.zip");
    {
        let f = std::fs::File::create(&zip_path).unwrap();
        let mut w = zip::ZipWriter::new(std::io::BufWriter::new(f));
        let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let template: serde_json::Value = serde_json::from_slice(
            &std::fs::read(util::sample_dir().join("0b36ff12-0a0a-4ca4-960b-50de3d2e71b6-result.json"))
                .unwrap_or_else(|_| std::fs::read(first_result()).unwrap()),
        )
        .unwrap();
        for i in 0..300 {
            let mut r = template.clone();
            let uuid = format!("00000000-0000-0000-0000-{i:012}");
            r["uuid"] = uuid.clone().into();
            r["historyId"] = format!("h{i}").into();
            r["name"] = format!("Senaryo {i}").into();
            let att = format!("{uuid}-attachment.png");
            r["attachments"] = serde_json::json!([{ "name": "ekran", "source": att, "type": "image/png" }]);
            w.start_file(format!("allure-results/{uuid}-result.json"), stored).unwrap();
            w.write_all(r.to_string().as_bytes()).unwrap();
            if i < 150 {
                w.start_file(format!("allure-results/{att}"), stored).unwrap();
                w.write_all(&noise(i as u64 + 7, 1024 * 1024)).unwrap();
            }
        }
        w.finish().unwrap();
    }
    let zip_size = std::fs::metadata(&zip_path).unwrap().len();
    assert!(zip_size > 140 * 1024 * 1024);

    let baseline_kb = status_kb("VmRSS:");
    std::fs::write("/proc/self/clear_refs", "5").unwrap(); // tepe RSS'yi sıfırla

    let file = tokio::fs::File::open(&zip_path).await.unwrap();
    let body = Body::from_stream(tokio_util::io::ReaderStream::with_capacity(file, 64 * 1024));
    let req = Request::post("/api/ci/runs?project_path=ekip%2Fbuyuk&branch=main")
        .header("authorization", format!("Bearer {}", util::CI_TOKEN))
        .body(body)
        .unwrap();
    let resp = app.send(req).await;
    let status = resp.status();
    let text = util::body_string(resp).await;
    assert_eq!(status, 200, "{text}");
    assert!(text.contains("\"accepted_results\":300"));

    let peak_kb = status_kb("VmHWM:");
    let growth_mb = peak_kb.saturating_sub(baseline_kb) / 1024;
    eprintln!(
        "zip {} MB, başlangıç RSS {} MB, tepe {} MB, artış {} MB (hedef {} MB)",
        zip_size / 1024 / 1024,
        baseline_kb / 1024,
        peak_kb / 1024,
        growth_mb,
        target_mb
    );
    assert!(growth_mb < target_mb, "bellek artışı {growth_mb} MB, hedef {target_mb} MB");
}

#[allow(dead_code)]
fn first_result() -> std::path::PathBuf {
    std::fs::read_dir(util::sample_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().ends_with("-result.json"))
        .unwrap()
}
