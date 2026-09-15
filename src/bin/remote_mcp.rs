//! Stdio MCP server shipped by plugin-travel-tunnel.
use locaryn_plugin_remote::link_build::{
    build_connect_link, build_pairing_launcher, find_launcher_bytes,
};
use locaryn_plugin_remote::list_providers;
use serde_json::{json, Value};
use std::io::Write;
use tokio::io::{AsyncBufReadExt, BufReader};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(request) => handle_request(request).await,
            Err(error) => error_response(Value::Null, -32700, format!("JSON invalide : {error}")),
        };
        if let Ok(serialized) = serde_json::to_string(&response) {
            println!("{serialized}");
            let _ = std::io::stdout().flush();
        }
    }
}

async fn handle_request(request: Value) -> Value {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match method {
        "initialize" => success(
            id,
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "plugin-travel-tunnel", "version": VERSION }
            }),
        ),
        "tools/list" => success(id, tools_list()),
        "tools/call" => {
            let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            match call_tool(name, args).await {
                Ok(value) => success(id, text_content(value)),
                Err(error) => error_response(id, -32000, error),
            }
        }
        notification if notification.starts_with("notifications/") => Value::Null,
        _ => error_response(id, -32601, format!("méthode MCP inconnue : {method}")),
    }
}

/// Ce que ce serveur sait dire.
///
/// Ouvrir et fermer un tunnel n'en font plus partie. Le service local en tient
/// deja un, et c'est le sien que porte le code d'appairage : un tunnel ouvert
/// ici aurait donne une adresse que le QR n'annoncait pas. L'ouverture se fait
/// donc par le panneau des reglages, qui pilote le service. Reste ce que le
/// service ne dit pas : quels relais sont installes sur cette machine, et
/// comment obtenir celui qui manque.
fn tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "list_providers",
                "description": "Les relais de tunnel connus, et lesquels sont installes ici. `needs_account` dit lequel exige une inscription avant de servir ; `install_hint` dit comment obtenir celui qui manque.",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "build_connect_link",
                "description": "Construit un lien locaryn://connect — le format de l'application (voir docs/api/locaryn-deep-links.md côté hôte). `server` est obligatoire (https:// ou http://) ; `user`, `password`, `cert` et `ca` sont optionnels. Le mot de passe ne s'embarque que sur demande explicite de la personne, et la réponse le rappelle.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "server": { "type": "string", "description": "Adresse publique du serveur, ex. https://maison.exemple:7474" },
                        "user": { "type": "string", "description": "Identifiant pré-rempli (optionnel)" },
                        "password": { "type": "string", "description": "Mot de passe à embarquer — uniquement si la personne le demande explicitement (optionnel, reste en clair dans le fichier)" },
                        "cert": { "type": "string", "description": "URL HTTPS du paquet certificat client+clé hébergé par le serveur (optionnel)" },
                        "ca": { "type": "string", "description": "URL HTTPS de l'autorité locale, si le serveur n'a pas d'autorité publique (optionnel)" }
                    },
                    "required": ["server"]
                }
            },
            {
                "name": "build_connect_qr",
                "description": "Le même lien locaryn://connect, dessiné en QR (SVG) pour un téléphone qui est dans la pièce.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "server": { "type": "string" },
                        "user": { "type": "string" },
                        "password": { "type": "string" },
                        "cert": { "type": "string" },
                        "ca": { "type": "string" }
                    },
                    "required": ["server"]
                }
            },
            {
                "name": "build_pairing_launcher",
                "description": "Génère le .exe d'appairage : le lanceur minuscule qui embarque le lien locaryn://connect et l'ouvre dans l'application déjà installée. Le .exe est déposé dans le dossier de données, et la réponse dit où. Ne transmettez le fichier qu'à vos propres machines si le mot de passe y est.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "server": { "type": "string" },
                        "user": { "type": "string" },
                        "password": { "type": "string" },
                        "cert": { "type": "string" },
                        "ca": { "type": "string" },
                        "filename": { "type": "string", "description": "Nom du fichier produit (défaut : Appairage-Locaryn.exe)" }
                    },
                    "required": ["server"]
                }
            }
        ]
    })
}

/// Les paramètres communs aux trois outils de lien, extraits d'un appel.
struct LinkParams<'a> {
    server: &'a str,
    user: Option<&'a str>,
    password: Option<&'a str>,
    cert: Option<&'a str>,
    ca: Option<&'a str>,
}

fn extract_link_params(args: &Value) -> Result<LinkParams<'_>, String> {
    let server = args
        .get("server")
        .and_then(Value::as_str)
        .ok_or("Paramètre server manquant.")?;
    let opt = |k: &str| args.get(k).and_then(Value::as_str);
    Ok(LinkParams {
        server,
        user: opt("user"),
        password: opt("password"),
        cert: opt("cert"),
        ca: opt("ca"),
    })
}

fn link_payload(l: locaryn_plugin_remote::link_build::BuiltLink) -> Value {
    json!({ "link": l.link, "warning": l.warning })
}

async fn call_tool(name: &str, args: Value) -> Result<Value, String> {
    match name {
        "list_providers" => Ok(json!({ "providers": list_providers() })),
        "build_connect_link" => {
            let p = extract_link_params(&args)?;
            build_connect_link(p.server, p.user, p.password, p.cert, p.ca)
                .map(link_payload)
        }
        "build_connect_qr" => {
            let p = extract_link_params(&args)?;
            let l = build_connect_link(p.server, p.user, p.password, p.cert, p.ca)?;
            let svg = locaryn_plugin_remote::link_build::qr_svg(&l.link)?;
            Ok(json!({
                "qr_svg": svg,
                "link": l.link,
                "warning": l.warning,
                "say": "Scannez avec le téléphone : l'application demande confirmation avant toute connexion."
            }))
        }
        "build_pairing_launcher" => {
            let p = extract_link_params(&args)?;
            let filename = args
                .get("filename")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("Appairage-Locaryn.exe");
            let filename = if filename.to_lowercase().ends_with(".exe") {
                filename.to_string()
            } else {
                format!("{filename}.exe")
            };
            let l = build_connect_link(p.server, p.user, p.password, p.cert, p.ca)?;
            let bytes = find_launcher_bytes().ok_or(
                "Le binaire locaryn-pair-launcher est introuvable à côté du serveur MCP. \
                 Compilez-le (cargo build --release -p locaryn-plugin-remote --bin locaryn-pair-launcher) \
                 et placez-le dans le dossier bin du morph.",
            )?;
            let exe = build_pairing_launcher(&bytes, &l)?;
            // Le fichier est déposé dans le dossier de données du morph : un
            // endroit que l'application sait lire, et que la personne peut
            // récupérer par ses propres moyens.
            let dir = locaryn_config_shim::data_dir()?;
            std::fs::create_dir_all(&dir)
                .map_err(|e| format!("dossier de données : {e}"))?;
            let path = dir.join(&filename);
            std::fs::write(&path, &exe)
                .map_err(|e| format!("écriture du lanceur : {e}"))?;
            Ok(json!({
                "path": path.display().to_string(),
                "filename": filename,
                "size_bytes": exe.len(),
                "link": l.link,
                "warning": l.warning,
                "say": "Transmettez ce fichier à la machine à connecter. En l'exécutant, elle ouvrira l'application et une demande de connexion s'affichera avant toute connexion."
            }))
        }
        "start_remote_tunnel" | "stop_remote_tunnel" | "tunnel_status" => Err(
            "Le tunnel appartient au service local, pas a ce morph : ouvrez-le depuis Reglages -> Serveur & fonctions, segment Tunnel."
                .to_string(),
        ),
        _ => Err(format!("Outil tunnel inconnu : {name}")),
    }
}

/// Le dossier où déposer ce que l'outil produit. Petit shim local : le morph
/// ne dépend pas de la config de l'hôte, et un dossier de données par défaut
/// suffit — la personne récupère le fichier ensuite par ses moyens.
mod locaryn_config_shim {
    pub fn data_dir() -> Result<std::path::PathBuf, String> {
        if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
            let mut p = std::path::PathBuf::from(home);
            p.push(".lochor");
            p.push("morph");
            p.push("travel-tunnel");
            return Ok(p);
        }
        Err("Aucun répertoire utilisateur connu (USERPROFILE/HOME).".into())
    }
}

fn text_content(value: Value) -> Value {
    json!({ "content": [{ "type": "text", "text": serde_json::to_string(&value).unwrap_or_else(|_| "{}".into()) }] })
}
fn success(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}
fn error_response(id: Value, code: i64, message: String) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}
