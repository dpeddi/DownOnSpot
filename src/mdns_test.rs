use clap::Parser;
use futures::StreamExt;
use librespot_core::authentication::Credentials;
use librespot_core::config::{DeviceType, SessionConfig};
use librespot_discovery::Discovery;
use sha1::Sha1;

use serde_json;
use std::fs::File;
use std::io::Write;
use std::process::exit;
use std::str::FromStr;
use log::warn;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "Speaker")]
    name: String,
    #[arg(short, long, default_value = "credentials.json")]
    path: String,
    #[arg(short, long, default_value = "speaker")]
    class: String,
}

pub fn save_credentials_and_exit(location: &str, cred: &Credentials) {
    let result = File::create(location).and_then(|mut file| {
        let data = serde_json::to_string(cred)?;
        write!(file, "{data}")
    });

    if let Err(e) = result {
        warn!("Cannot save credentials to cache: {}", e);
        exit(1);
    } else {
        println!("Credentials saved: {}", location);
        exit(0);
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = Args::parse();
    let name = args.name;
    let credentials_location = args.path;

    let device_type = match DeviceType::from_str(&args.class) {
        Ok(device_type) => device_type,
        Err(_) => {
            eprintln!("Invalid device type: {}", args.class);
            exit(1);
        }
    };

    // Genera device_id con SHA1
    let mut hasher = Sha1::new();
    hasher.update(name.as_bytes());
    let hash = hasher.digest().bytes();
    let device_id = hex::encode(hash);

    // Avvia il demone mDNS
    let mdns = ServiceDaemon::new().expect("Failed to create mDNS daemon");

    let port = 8080;
    let ttl = 60;


let username = std::env::var("USER").unwrap_or("unknown".to_string());
    let device_name = format!("spotify-quickauth-{}", username);


let mut txt_props = HashMap::new();
txt_props.insert("version".to_string(), "1.0".to_string());
txt_props.insert("platform".to_string(), "windows".to_string());
txt_props.insert("type".to_string(), "speaker".to_string());
txt_props.insert("device".to_string(), device_name.clone());
txt_props.insert("device_id".to_string(), device_id.clone());

use std::net::Ipv4Addr;
    let ip = Ipv4Addr::new(192, 168, 179, 75); // Sostituisci con il tuo IP
let service_info = ServiceInfo::new(
    "_spotify-connect._tcp.local.",
    &name,
    "RustSpeaker.local.",
    ip,
    8080,
    Some(txt_props), // ✅ ora è del tipo corretto
).expect("Failed to create mDNS service info");

    mdns.register(service_info).expect("Failed to register mDNS service");

    println!("Servizio mDNS registrato come '{}'", name);
    println!("Apri Spotify e seleziona il dispositivo: {}", name);

    let mut server = Discovery::builder(device_id.clone())
        .name(name.clone())
        .device_type(device_type)
        .launch()
        .unwrap();

    while let Some(credentials) = server.next().await {
        save_credentials_and_exit(&credentials_location, &credentials);
    }
}
