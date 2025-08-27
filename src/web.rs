use actix_web::{
    delete, get, post,
    web::{self, Query},
    App, HttpServer, Responder, HttpResponse,
};
use actix_files::Files;
use serde::{Deserialize, Serialize};
use std::{fs, path::{PathBuf}, sync::Arc, io::ErrorKind, time::UNIX_EPOCH};
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
    author: String,    
    url: String,
}

#[get("/api/search")]
async fn api_search(query: Query<SearchQuery>, app_state: web::Data<AppState>) -> impl Responder {
    let dl = app_state.downloader.lock().await;
    let results = match dl.handle_input(&query.q).await {
        Ok(Some(items)) => items
            .into_iter()
            .map(|r| SearchResult {
                title: r.title,
                author: r.author, // Assicurati che `r` abbia questa proprietà
                url: format!("https://open.spotify.com/track/{}", r.track_id),
            })
            .collect(),
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

#[post("/api/login")]
async fn api_login() -> impl Responder {
    use std::path::PathBuf;
    use crate::spotify::run_spotify_login;

    let path = PathBuf::from("credentials.json");
    let force = true;

    match run_spotify_login(path, force) {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "status": "ok" })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

#[get("/api/status")]
async fn api_status(app_state: web::Data<AppState>) -> actix_web::HttpResponse {
    let downloads = app_state.downloader.lock().await.get_downloads().await;
    HttpResponse::Ok().json(downloads)
}

/* ------------------------- Nuovi endpoint qui sotto ------------------------- */

const DOWNLOAD_DIR: &str = "./downloads";

#[derive(Serialize)]
struct FileEntry {
    name: String,
    size: u64,
    modified: u64, // epoch seconds
    url: String,   // link diretto: /files/<name>
}

#[get("/api/downloads")]
async fn api_list_downloads() -> impl Responder {
    let base = PathBuf::from(DOWNLOAD_DIR);
    let mut entries: Vec<FileEntry> = Vec::new();

    match fs::read_dir(&base) {
        Ok(read_dir) => {
            for entry_res in read_dir {
                if let Ok(entry) = entry_res {
                    let path = entry.path();
                    if path.is_file() {
                        let name = entry
                            .file_name()
                            .to_string_lossy()
                            .to_string();

                        if let Ok(meta) = entry.metadata() {
                            let size = meta.len();
                            let modified = meta
                                .modified()
                                .ok()
                                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                                .map(|d| d.as_secs())
                                .unwrap_or(0);

                            // Nota: se i nomi possono contenere spazi o caratteri speciali,
                            // valuta di fare percent-encode lato UI quando costruisci l'URL.
                            let url = format!("/files/{}", name);

                            entries.push(FileEntry { name, size, modified, url });
                        }
                    }
                }
            }
            HttpResponse::Ok().json(entries)
        }
        Err(e) => {
            // Se la cartella non esiste, restituisci lista vuota (comportamento "gentile")
            if e.kind() == ErrorKind::NotFound {
                HttpResponse::Ok().json(entries)
            } else {
                HttpResponse::InternalServerError().body(format!("Errore lettura directory: {}", e))
            }
        }
    }
}

#[derive(Deserialize)]
struct DeleteQuery {
    file: String,
}

use std::path::Path;
use sanitize_filename::sanitize;

/// Verifica se un nome di file è valido e sicuro da usare su filesystem multipiattaforma.
pub fn is_valid_filename(name: &str) -> bool {
    // Deve essere non vuoto
    if name.trim().is_empty() {
        return false;
    }

    // Non deve contenere path traversal o separatori
    if name.contains('/') || name.contains('\\') {
        return false;
    }

    // Blocca solo ".." come directory, non come parte del nome
    if name.split('/').any(|part| part == "..") {
        return false;
    }

    // Deve essere uguale alla versione "sanitizzata"
    let sanitized = sanitize(name);
    if sanitized != name {
        return false;
    }

    // Deve essere un nome valido per il filesystem
    Path::new(&sanitized).file_name().is_some()
}

#[delete("/api/downloads")]
async fn api_delete_download(q: Query<DeleteQuery>) -> impl Responder {
    if !is_valid_filename(&q.file) {
        return HttpResponse::BadRequest().body("Nome file non valido");
    }

    // Usa canonicalize per evitare di uscire dalla dir base
    let base = match fs::canonicalize(DOWNLOAD_DIR) {
        Ok(p) => p,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Errore directory: {}", e)),
    };
    let target = base.join(&q.file);

    // Rimuovi il file in async
    match tokio::fs::remove_file(&target).await {
        Ok(_) => HttpResponse::Ok().body("File eliminato"),
        Err(e) if e.kind() == ErrorKind::NotFound => HttpResponse::NotFound().body("File non trovato"),
        Err(e) => HttpResponse::InternalServerError().body(format!("Errore eliminazione: {}", e)),
    }
}

/* --------------------------------------------------------------------------- */

pub async fn start_web_server(downloader: Downloader) -> std::io::Result<()> {
    // Assicurati che la cartella downloads esista
    let _ = fs::create_dir_all(DOWNLOAD_DIR);

    let state = web::Data::new(AppState {
        downloader: Arc::new(Mutex::new(downloader)),
    });
    println!("🚀 DownOnSpot Web Server avviato!");
    println!("🔗 UI disponibile su: http://localhost:8080/ui");
    println!("📡 API endpoint:");
    println!("   GET    /api/search?q=...");
    println!("   POST   /api/download");
    println!("   GET    /api/status");
    println!("   GET    /api/downloads");
    println!("   DELETE /api/downloads?file=<nome>");
    println!("   POST   /api/login");

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(api_search)
            .service(api_download)
            .service(api_status)
            .service(api_list_downloads)   // ← nuovo
            .service(api_delete_download)  // ← nuovo
            .service(api_login) // ← aggiunto qui
            // Servi file statici UI
            .service(Files::new("/static", "./ui/static").show_files_listing())
            .service(Files::new("/ui", "./ui").index_file("index.html"))
            // Servi i file scaricati per il download diretto
            .service(Files::new("/files", DOWNLOAD_DIR))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
