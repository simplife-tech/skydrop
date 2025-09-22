use warp::Filter;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::TcpListener;
use std::path::PathBuf;
use qrcode::QrCode;
use qrcode::render::unicode;
use local_ip_address::local_ip;
use uuid::Uuid;
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    let upload_dir = dirs::download_dir().unwrap_or_else(|| PathBuf::from("."));

    // 每次启动生成随机 token
    let token = Uuid::new_v4().to_string();
    println!("🔑 本次启动的访问 token: {}", token);

    // GET /?token=xxxx -> 返回上传页面
    let index_token = token.clone();
    let index = warp::get()
        .and(warp::query::<HashMap<String, String>>())
        .map(move |query: HashMap<String, String>| {
            if query.get("token") == Some(&index_token) {
                warp::reply::html(include_str!("upload.html"))
            } else {
                warp::reply::html("<h2>❌ 无效 token</h2>")
            }
        });

    // POST /upload?token=xxxx&filename=xxx&chunk_index=0&total_chunks=10
    let upload_dir_clone = upload_dir.clone();
    let upload_token = token.clone();
    let upload = warp::post()
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::body::bytes())
        .map(move |query: HashMap<String, String>, body: bytes::Bytes| {
            if query.get("token") != Some(&upload_token) {
                return "❌ 无效 token".to_string();
            }

            let filename = query.get("filename").unwrap();
            let chunk_index: usize = query.get("chunk_index").unwrap().parse().unwrap();
            let total_chunks: usize = query.get("total_chunks").unwrap().parse().unwrap();

            let temp_file = upload_dir_clone.join(format!("{}.part", filename));
            let mut f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&temp_file)
                .unwrap();
            f.write_all(&body).unwrap();

            if chunk_index + 1 == total_chunks {
                let final_path = upload_dir_clone.join(filename);
                fs::rename(temp_file, final_path).unwrap();
            }

            format!("✅ 收到 chunk {}/{}", chunk_index + 1, total_chunks)
        });

    let routes = index.or(upload);

    // 打印二维码（含 token）
    let ip = local_ip().unwrap();
    // 自动选择可用端口
    let mut port = 5000;
    loop {
        if TcpListener::bind(("0.0.0.0", port)).is_ok() {
            break;
        }
        port += 1;
    }
    let url = format!("http://{}:{}/?token={}", ip, port, token);
    let code = QrCode::new(url.as_bytes()).unwrap();
    let string = code.render::<unicode::Dense1x2>().dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark).build();
    println!("📡 扫描二维码上传文件到此设备:\n{}", string);
    println!("或者直接访问: {}", url);
    println!("文件将保存到: {:?}", upload_dir);

    warp::serve(routes).run(([0,0,0,0], port)).await;
}
