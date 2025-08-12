use actix_web::{
    get, post, web::{self, Query}, App, HttpServer,
    Responder, HttpResponse,
};
use actix_files::Files;
use serde::{Deserialize, Serialize}; // 💡 Serve per #[derive(Serialize)]
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::downloader::Downloader;

#[derive(Clone)]
struct AppState {
    downloader: Arc<Mutex<Downloader>>,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

#[derive(Serialize)]
struct SearchResult {
    title: String,
    url: String,
}

#[get("/api/search")]
async fn api_search(query: Query<SearchQuery>, app_state: web::Data<AppState>) -> impl Responder {
    let dl = app_state.downloader.lock().await;
    let results = match dl.handle_input(&query.q).await {
        Ok(Some(items)) => items.into_iter().map(|r| SearchResult {
            title: r.title,
            url: format!("https://open.spotify.com/track/{}", r.track_id),
        }).collect(),
        _ => Vec::new(),
    };
    HttpResponse::Ok().json(results)
}



#[derive(Deserialize)]
struct DownloadRequest {
    url: String,
}


#[post("/api/download")]
async fn api_download(payload: web::Json<DownloadRequest>, app_state: web::Data<AppState>) -> impl Responder {
    let mut dl = app_state.downloader.lock().await;
    match dl.search_and_download(&payload.url).await {
        Ok(_) => HttpResponse::Ok().body("Download avviato"),
        Err(e) => HttpResponse::InternalServerError().body(format!("Errore: {}", e)),
    }
}


#[get("/api/status")]
async fn api_status(app_state: web::Data<AppState>) -> impl Responder {
    let dl = app_state.downloader.lock().await;
    HttpResponse::Ok().json(dl.get_status())
}

pub async fn start_web_server(downloader: Downloader) -> std::io::Result<()> {
    let state = web::Data::new(AppState {
        downloader: Arc::new(Mutex::new(downloader)),
    });
    println!("🚀 DownOnSpot Web Server avviato!");
    println!("🔗 UI disponibile su: http://localhost:8080/ui");
    println!("📡 API endpoint:");
    println!("   GET    /api/search?q=...");
    println!("   POST   /api/download");
    println!("   GET    /api/status");


    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(api_search)
            .service(api_download)
            .service(api_status)
            // Servi tutto il contenuto della cartella `ui/static`
            .service(Files::new("/static", "./ui/static").show_files_listing())
            .service(Files::new("/ui", "./ui").index_file("index.html"))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}