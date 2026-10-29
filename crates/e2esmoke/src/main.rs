//! Uçtan uca smoke (SW-10.2): sonuçlar CI yükleme yolundan (stajci upload → POST /api/ci/runs,
//! SW-2.1) gönderilir, rapor sayfaları gerçek sunucuda doğrulanır.
//!
//!   cargo run -p e2esmoke                       # kendi sunucusunu açar
//!   STAJ_SERVER=http://host:8080 CI_API_TOKEN=... cargo run -p e2esmoke   # çalışan sunucuya karşı

use anyhow::Result;
use rehearse::*;

fn main() -> Result<()> {
    println!("== stajproje uçtan uca smoke (SW-10.2) ==");
    build_binaries()?;
    let tmp = tempfile::tempdir()?;
    let root = tmp.path();
    let external = std::env::var("STAJ_SERVER").ok();
    let mut server = Server::new(&root.join("data"));
    let admin_pw = "Smoke-Admin-123";
    let (url, token) = match &external {
        Some(u) => (u.clone(), std::env::var("CI_API_TOKEN").unwrap_or_default()),
        None => {
            // SW-9.5: admin şifresi set-admin-password ile .env'e yazılır
            std::fs::create_dir_all(&server.data_dir)?;
            let envf = server.data_dir.join(".env");
            let st = std::process::Command::new(exe("stajproje"))
                .arg("set-admin-password")
                .env("ENV_FILE", &envf)
                .env("STAJ_NEW_ADMIN_PASSWORD", admin_pw)
                .status()?;
            anyhow::ensure!(st.success(), "set-admin-password başarısız");
            server.start()?;
            (server.url(), TOKEN.to_string())
        }
    };
    let mut ck = Checks::default();

    let (st, body) = get(&format!("{url}/api/health"))?;
    ck.check("SW-10.5", "sağlık ucu 200", st == 200 && body.contains("ok"), &body);

    // test projesi gibi: target/allure-results + stajci upload (run.json yok → yeni koşum)
    let proj = root.join("proje");
    copy_dir(&sample_dir(), &proj.join("target/allure-results"))?;
    let mut s = Stajci::new(&exe("stajci"), &proj, &url);
    s.set("CI_API_TOKEN", token.clone())
        .set("CI_PROJECT_PATH", "smoke/ornek")
        .set("CI_PROJECT_ID", "9090");
    let up = s.run(&["upload"])?;
    ck.check(
        "SW-2.1",
        "stajci upload ile CI yükleme yolu",
        up.code == 0 && up.text.contains("Rapor:"),
        &up.text,
    );
    let run_id: i64 = up.text.split("/runs/").last().and_then(|t| t.trim().parse().ok()).unwrap_or(0);

    let run = get_json(&format!("{url}/api/runs/{run_id}"))?;
    ck.check(
        "SW-2.1",
        "koşum listede: 7 senaryo, 4 geçti",
        run["kosum"]["total"] == 7 && run["kosum"]["passed"] == 4,
        run["kosum"].to_string(),
    );

    let (st, html) = get(&format!("{url}/runs/{run_id}"))?;
    ck.check(
        "SW-4.1",
        "koşum raporu sayfası",
        st == 200 && html.contains("Şifremi unuttum bağlantısı"),
        "",
    );
    let sid = run["senaryolar"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["senaryo"]["name"] == "Yanlış şifrede özel mesaj")
        .unwrap()["senaryo"]["id"]
        .as_i64()
        .unwrap();
    let (st, page) = get(&format!("{url}/scenarios/{sid}"))?;
    ck.check(
        "SW-4.1",
        "senaryo → adımlar → ekler (görüntü + video)",
        st == 200 && page.contains("<img") && page.contains("<video"),
        "",
    );
    ck.check(
        "SW-2.2",
        "@After ekran görüntüsü kalan senaryoda",
        page.contains("Ekran görüntüsü"),
        "",
    );
    let shot = run["senaryolar"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|x| x["ekler"].as_array().unwrap().clone())
        .find(|a| a["mime"] == "image/png")
        .unwrap();
    let (st, png) = get_bytes(&format!("{url}/files/{}", shot["id"]))?;
    ck.check(
        "SW-4.1",
        "ekran görüntüsü PNG olarak sunulur",
        st == 200 && png.starts_with(b"\x89PNG"),
        "",
    );
    let vid = run["senaryolar"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|x| x["ekler"].as_array().unwrap().clone())
        .find(|a| a["mime"] == "video/mp4")
        .unwrap();
    let (st, _, part) = get_with(&format!("{url}/files/{}", vid["id"]), &[("Range", "bytes=0-1023")])?;
    ck.check(
        "SW-10.6",
        "video HTTP range ile parça parça",
        st == 206 && part.len() == 1024,
        format!("{st} {}", part.len()),
    );

    let (st, csv) = get(&format!("{url}/export.csv?run={run_id}&fields=senaryo,durum,doors"))?;
    ck.check(
        "SW-4.8",
        "CSV dışa aktarma",
        st == 200 && csv.starts_with('\u{feff}') && csv.contains("Şifremi unuttum bağlantısı,broken,DOORS-1004"),
        &csv,
    );
    let (st, dx) = get(&format!("{url}/runs/{run_id}/doors.dxl"))?;
    ck.check("SW-6.4", "DOORS DXL", st == 200 && dx.contains("Module m = current"), "");
    let (st, srch) = get(&format!("{url}/search?doors=DOORS-1003"))?;
    ck.check("SW-4.7", "DOORS numarasıyla arama", st == 200 && srch.contains("1 sonuç"), "");

    if external.is_none() {
        // admin girişi + paylaşım linki
        let (st, hs, _) = post(
            &format!("{url}/login"),
            &[("Content-Type", "application/x-www-form-urlencoded")],
            b"password=yanlis&next=%2Fadmin",
        )?;
        ck.check(
            "SW-9.5",
            "yanlış şifreyle giriş reddedilir",
            st == 401 && !hs.iter().any(|(k, _)| k == "set-cookie"),
            "",
        );
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let (st, hs, _) = post(
            &format!("{url}/login"),
            &[("Content-Type", "application/x-www-form-urlencoded")],
            format!("password={admin_pw}&next=%2Fadmin").as_bytes(),
        )?;
        let cookie = hs
            .iter()
            .find(|(k, _)| k == "set-cookie")
            .map(|(_, v)| v.split(';').next().unwrap().to_string())
            .unwrap_or_default();
        ck.check(
            "SW-9.5",
            "doğru şifreyle giriş, HttpOnly çerez",
            st == 303 && cookie.starts_with("staj_oturum="),
            "",
        );
        let (st, hs, _) = post(&format!("{url}/admin/runs/{run_id}/share"), &[("Cookie", &cookie)], b"")?;
        let loc = hs.iter().find(|(k, _)| k == "location").map(|(_, v)| v.clone()).unwrap_or_default();
        let share = loc.split("share:").nth(1).unwrap_or("").to_string();
        let (st2, sh) = get(&format!("{url}/s/{share}"))?;
        ck.check(
            "SW-5.1",
            "paylaşım linki giriş olmadan açılır, Jira anahtarı yok",
            st == 303 && st2 == 200 && sh.contains("Şifremi unuttum") && !sh.contains("STAJ-42"),
            "",
        );
        let (_, adm) =
            get_with(&format!("{url}/admin"), &[("Cookie", &cookie)]).map(|(s, _, b)| (s, String::from_utf8_lossy(&b).to_string()))?;
        let sid_share = adm
            .split("/admin/shares/")
            .nth(1)
            .and_then(|x| x.split('/').next())
            .unwrap_or("0")
            .to_string();
        post(&format!("{url}/admin/shares/{sid_share}/revoke"), &[("Cookie", &cookie)], b"")?;
        let (st, _) = get(&format!("{url}/s/{share}"))?;
        ck.check("SW-5.2", "iptal edilen link açılmaz", st == 404, "");
        // webhook
        let hook = serde_json::json!({ "object_kind": "pipeline", "object_attributes": { "id": 31, "status": "running", "ref": "main", "source": "schedule" }, "project": { "id": 9090, "path_with_namespace": "smoke/ornek" } });
        let (st, _, _) = post(
            &format!("{url}/api/gitlab/webhook"),
            &[("X-Gitlab-Token", "prova-webhook"), ("Content-Type", "application/json")],
            hook.to_string().as_bytes(),
        )?;
        let (_, dash) = get(&format!("{url}/"))?;
        ck.check(
            "SW-3.6",
            "pipeline durumu panelde (webhook)",
            st == 200 && dash.contains("çalışıyor") && dash.contains("#31"),
            "",
        );
        // log dosyasında token ve şifre yok (SW-10.7)
        let mut logs = String::new();
        for e in std::fs::read_dir(server.data_dir.join("logs"))?.flatten() {
            logs.push_str(&std::fs::read_to_string(e.path()).unwrap_or_default());
        }
        ck.check(
            "SW-10.7",
            "log dosyaları JSON; token ve şifre yok",
            logs.lines().all(|l| serde_json::from_str::<serde_json::Value>(l).is_ok())
                && !logs.contains(TOKEN)
                && !logs.contains(admin_pw)
                && logs.contains("yükleme alındı"),
            "",
        );
        // yedek
        let st = std::process::Command::new(exe("stajproje"))
            .arg("backup")
            .env("DATA_DIR", &server.data_dir)
            .output()?;
        let bfile = String::from_utf8_lossy(&st.stdout).trim().to_string();
        ck.check(
            "SW-2.5",
            "sunucu çalışırken yedek alınır",
            st.status.success() && std::path::Path::new(&bfile).is_file(),
            &bfile,
        );
    }
    server.stop();
    std::process::exit(ck.result());
}
