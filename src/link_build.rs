//! Construire ce que la spécification `locaryn://connect` promet : le lien,
//! son QR, et le `.exe` qui embarque le lien.
//!
//! Le format du lien est un contrat, pas une convention locale : il est
//! spécifié dans le dépôt principal
//! (`docs/api/locaryn-deep-links.md`) et lu par
//! `apps/desktop/src/lib/deepLink.ts` (action `connect`). Le morph ne décide
//! de rien : il assemble les paramètres que l'application accepte, les encode,
//! et les sert.
//!
//! Le mot de passe est le seul paramètre sensible. Il n'est présent que si la
//! personne le demande explicitement (`password` dans l'outil), et l'outil le
//! dit dans sa réponse — un lien à mot de passe ne se transmet pas.

use serde::Serialize;

/// La marque qui sépare le code du lanceur du lien embarqué.
pub const MARKER: &[u8] = b"LCAIRN1\0";

#[derive(Debug, Clone, Serialize)]
pub struct BuiltLink {
    /// Le lien complet, encodé, prêt pour un QR ou un fichier.
    pub link: String,
    /// Ce que la réponse MCP doit rappeler à la personne.
    pub warning: Option<String>,
}

/// Encoder un paramètre comme le ferait `encodeURIComponent` : la table des
/// caractères non encodés vient de la spécification MDN, appliquée à la main —
/// le morph n'a pas de dépendance de % pour ce service.
fn encode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => out.push(b as char),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Construire le lien `locaryn://connect` — cf. spécification §4.
///
/// `server` est obligatoire et doit commencer par `https://` ou `http://` :
/// c'est le parseur de l'application qui refuse, autant échouer ici avec le
/// vrai message. `cert`/`ca` sont des URLs **HTTPS strict** (la commande Rust
/// de l'hôte refuse autre chose) — on le vérifie aussi ici, pour l'erreur
/// immédiate plutôt qu'au scan du QR.
pub fn build_connect_link(
    server: &str,
    user: Option<&str>,
    password: Option<&str>,
    cert: Option<&str>,
    ca: Option<&str>,
) -> Result<BuiltLink, String> {
    let server = server.trim().trim_end_matches('/');
    if server.is_empty() {
        return Err("Adresse du serveur manquante (paramètre server).".into());
    }
    if !(server.starts_with("https://") || server.starts_with("http://")) {
        return Err(format!(
            "L'adresse du serveur doit commencer par https:// ou http:// — obtenu : {server}"
        ));
    }
    let cert = cert.map(str::trim).filter(|s| !s.is_empty());
    let ca = ca.map(str::trim).filter(|s| !s.is_empty());
    if let Some(u) = cert.or(ca) {
        if !u.starts_with("https://") {
            return Err(format!(
                "Les URLs de certificat sont HTTPS strict (refusée par l'hôte) — obtenu : {u}"
            ));
        }
    }
    let password = password.filter(|p| !p.is_empty());
    let user = user.map(str::trim).filter(|s| !s.is_empty());

    let mut link = format!("locaryn://connect?server={}", encode_uri_component(server));
    if let Some(u) = user {
        link.push_str(&format!("&user={}", encode_uri_component(u)));
    }
    if let Some(p) = password {
        link.push_str(&format!("&password={}", encode_uri_component(p)));
    }
    if let Some(c) = cert {
        link.push_str(&format!("&cert={}", encode_uri_component(c)));
    }
    if let Some(a) = ca {
        link.push_str(&format!("&ca={}", encode_uri_component(a)));
    }
    let warning = password
        .map(|_| "Ce lien contient le mot de passe en clair. Ne le transmettez qu'à vos propres machines.".to_string());
    Ok(BuiltLink { link, warning })
}

/// Produire le `.exe` d'appairage : le binaire du lanceur, puis le lien après
/// la marque.
///
/// `launcher_bytes` vient du binaire `locaryn-pair-launcher` compilé à côté du
/// serveur MCP — le morph le cherche à son côté (`current_exe`), comme le fait
/// tout outil embarqué. Rien d'autre n'est écrit : pas d'identifiant, pas de
/// certificat — le lanceur n'a que le lien, et c'est la spécification.
pub fn build_pairing_launcher(launcher_bytes: &[u8], link: &BuiltLink) -> Result<Vec<u8>, String> {
    if launcher_bytes.is_empty() {
        return Err("Binaire du lanceur introuvable (locaryn-pair-launcher).".into());
    }
    let mut out = launcher_bytes.to_vec();
    out.extend_from_slice(MARKER);
    out.extend_from_slice(link.link.as_bytes());
    // Terminer par deux octets nuls : la lecture tronque au premier '\0'
    // parasite, et un buffer aligné fait un fichier bien formé.
    out.extend_from_slice(&[0, 0]);
    Ok(out)
}

/// Dérive les URLs `cert`/`ca` de l'adresse du serveur quand la personne ne
/// les a pas fournies. Le daemon hôte expose ces deux routes (voir la
/// spécification §4 et `routes/pairing.rs`) ; le lien les embarque pour que
/// la machine distante installe ses certificats toute seule — sans elles,
/// l'appairage par lien resterait bloqué derrière l'installation manuelle.
///
/// Si la personne a passé une URL explicite (reverse proxy, hébergement
/// personnel), elle gagne : on ne réécrit jamais ce qui est explicite.
/// Une valeur explicitement vide vaut absence — même règle que dans
/// `build_connect_link`, pour que les deux chemins se répondent.
fn sans_vide(s: Option<&str>) -> Option<&str> {
    match s {
        Some(v) => {
            let t = v.trim();
            if t.is_empty() {
                None
            } else {
                Some(t)
            }
        }
        None => None,
    }
}

pub fn derive_certificate_urls(
    server: &str,
    cert: Option<&str>,
    ca: Option<&str>,
) -> (Option<String>, Option<String>) {
    let cert = sans_vide(cert);
    let ca = sans_vide(ca);
    if cert.is_some() || ca.is_some() {
        // Une des deux au moins est explicite : l'assemblage doit rester
        // exactement ce qu'on lui a donné — et `build_connect_link` validera
        // le HTTPS strict.
        return (cert.map(str::to_string), ca.map(str::to_string));
    }
    let base = server.trim().trim_end_matches('/');
    if !(base.starts_with("https://") || base.starts_with("http://")) {
        // `build_connect_link` refusera cette adresse avec le vrai message :
        // pas de dérivation à moitié juste ici.
        return (None, None);
    }
    (
        Some(format!("{base}/v1/pairing/cert")),
        Some(format!("{base}/v1/pairing/ca")),
    )
}

/// Retrouver le binaire du lanceur à côté du serveur MCP. Le chemin des
/// binaires du morph est standard : même dossier que `current_exe`.
pub fn find_launcher_bytes() -> Option<Vec<u8>> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let name = if cfg!(windows) {
        "locaryn-pair-launcher.exe"
    } else {
        "locaryn-pair-launcher"
    };
    std::fs::read(dir.join(name)).ok()
}

/// Dessiner un QR (SVG) pour ce lien — même encodeur que le daemon hôte
/// (`packages/travel/src/qr.rs`) : mêmes entrées, même sortie, une seule
/// dépendance ajoutée au morph.
pub fn qr_svg(uri: &str) -> Result<String, String> {
    let bits = qrcodegen::QrCode::encode_text(uri, qrcodegen::QrCodeEcc::Medium)
        .map_err(|e| format!("QR : {e:?}"))?;
    let n = bits.size() as usize;
    // Deux modules de marge : un QR imprimé ou affiché de nuit doit rester
    // lisible même avec un bord de photo coupé.
    let quiet = 2usize;
    let dim = (n + 2 * quiet) * 4;
    let mut path = String::new();
    for y in 0..n {
        for x in 0..n {
            if bits.get_module(x as i32, y as i32) {
                let px = x + quiet;
                let py = y + quiet;
                path.push_str(&format!("M{px},{py}h4v4h-4z"));
            }
        }
    }
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {dim} {dim}\" \
         shape-rendering=\"crispEdges\" width=\"{dim}\" height=\"{dim}\" \
         fill=\"currentColor\"><path d=\"{path}\"/></svg>"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC_SIMPLE: &str = "locaryn://connect?server=https%3A%2F%2F192.168.1.10%3A7474";

    #[test]
    fn minimal_link_matches_the_spec_example() {
        let l = build_connect_link("https://192.168.1.10:7474", None, None, None, None).unwrap();
        assert_eq!(l.link, SPEC_SIMPLE);
        assert!(l.warning.is_none());
    }

    #[test]
    fn user_password_and_certs_are_encoded_in_order() {
        let l = build_connect_link(
            "https://locaryn.example.net:7474/",
            Some("dev"),
            Some("s3crét"),
            Some("https://locaryn.example.net/pair/a1b2c3/client.pem"),
            Some("https://locaryn.example.net/pair/a1b2c3/ca.pem"),
        )
        .unwrap();
        assert_eq!(
            l.link,
            "locaryn://connect?server=https%3A%2F%2Flocaryn.example.net%3A7474\
             &user=dev&password=s3cr%C3%A9t\
             &cert=https%3A%2F%2Flocaryn.example.net%2Fpair%2Fa1b2c3%2Fclient.pem\
             &ca=https%3A%2F%2Flocaryn.example.net%2Fpair%2Fa1b2c3%2Fca.pem"
        );
        assert!(l.warning.is_some());
    }

    #[test]
    fn http_server_is_allowed_but_certs_stay_https_strict() {
        let l = build_connect_link("http://192.168.1.10:7474", None, None, None, None).unwrap();
        assert!(l.link.starts_with("locaryn://connect?server=http%3A%2F%2F"));
        let e = build_connect_link(
            "https://ok.example",
            None,
            None,
            Some("http://pas-tls.example/client.pem"),
            None,
        )
        .unwrap_err();
        assert!(e.contains("HTTPS strict"));
    }

    #[test]
    fn missing_or_bare_server_is_rejected_here_already() {
        assert!(build_connect_link("", None, None, None, None).is_err());
        assert!(build_connect_link("192.168.1.10:7474", None, None, None, None).is_err());
    }
    #[test]
    fn cert_and_ca_are_derived_from_the_server_url() {
        let (cert, ca) = derive_certificate_urls("https://192.168.1.10:7474/", None, None);
        assert_eq!(
            cert.as_deref(),
            Some("https://192.168.1.10:7474/v1/pairing/cert")
        );
        assert_eq!(
            ca.as_deref(),
            Some("https://192.168.1.10:7474/v1/pairing/ca")
        );
        let (cert, ca) = derive_certificate_urls("http://192.168.1.10:7474", None, None);
        assert_eq!(
            cert.as_deref(),
            Some("http://192.168.1.10:7474/v1/pairing/cert")
        );
        assert_eq!(
            ca.as_deref(),
            Some("http://192.168.1.10:7474/v1/pairing/ca")
        );
    }

    #[test]
    fn explicit_cert_wins_and_partial_explicit_wins_too() {
        let (cert, ca) = derive_certificate_urls(
            "https://a.example",
            Some("https://relay.example/ca.pem"),
            None,
        );
        assert_eq!(cert.as_deref(), Some("https://relay.example/ca.pem"));
        assert!(
            ca.is_none(),
            "rien n'est inventé à côté d'un choix explicite"
        );
    }

    #[test]
    fn empty_or_bare_server_derives_nothing() {
        let (cert, ca) = derive_certificate_urls("   ", None, None);
        assert!(cert.is_none() && ca.is_none());
        let (cert, ca) = derive_certificate_urls("192.168.1.10:7474", None, None);
        assert!(cert.is_none() && ca.is_none());
    }

    #[test]
    fn empty_optional_params_are_treated_as_absent() {
        let l =
            build_connect_link("https://a.example", Some(""), Some(""), None, Some("")).unwrap();
        assert_eq!(l.link, "locaryn://connect?server=https%3A%2F%2Fa.example");
        assert!(l.warning.is_none());
    }

    #[test]
    fn launcher_carries_the_link_after_the_marker() {
        let l =
            build_connect_link("https://192.168.1.10:7474", Some("dev"), None, None, None).unwrap();
        let fake_launcher = b"MZ fake binary".to_vec();
        let out = build_pairing_launcher(&fake_launcher, &l).unwrap();
        let expected_tail = {
            let mut t = MARKER.to_vec();
            t.extend_from_slice(l.link.as_bytes());
            t.extend_from_slice(&[0, 0]);
            t
        };
        assert!(out.ends_with(&expected_tail));
        // Le lanceur reste exécutable : son code est intact devant.
        assert!(out.starts_with(b"MZ fake binary"));
    }
}
