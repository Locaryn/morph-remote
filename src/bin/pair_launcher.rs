//! Le lanceur d'appairage : un exécutable minuscule qui ne fait qu'ouvrir un
//! lien `locaryn://connect?…`.
//!
//! Pourquoi un fichier, et pas juste le lien : un lien ne se transmet pas bien
//! — par clé USB, par mail, en double-clic depuis un dossier partagé, c'est un
//! `.exe` que l'on donne. Le fichier porte le lien **embarqué à sa fin** :
//! quand on l'exécute, il se relit, trouve la marque, et demande au système
//! d'ouvrir ce qui suit. Toute la logique de connexion reste dans l'application
//! — le lanceur n'a ni identifiant, ni mot de passe, ni certificat à gérer.
//!
//! La marque qui sépare le code du lien : les liens ne contiennent pas
//! d'octets nuls, un exécutable Windows non plus. Chercher le premier
//! `LCAIRN1\0` suffit donc, sans table de sections.
//!
//! Généré par le morph (outil MCP `build_pairing_launcher`) : le binaire du
//! lanceur est recopié, puis le lien est écrit après la marque. Voir la
//! spécification `docs/api/locaryn-deep-links.md` dans le dépôt principal.

/// La marque, suivie d'un octet nul : les deux moitiés du piquage.
const MARKER: &[u8] = b"LCAIRN1\0";
fn main() {
    // Le chemin de ce fichier : l'exécutable lui-même, où qu'il soit.
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return,
    };
    let me = match std::fs::read(&exe) {
        Ok(b) => b,
        Err(_) => return,
    };
    // Retrouver la marque, puis le lien juste après. Dernière occurrence, pas
    // première : la constante de la marque vit aussi dans le code de ce
    // binaire — chercher depuis la fin évite de se piéger soi-même.
    let mut start = None;
    if me.len() >= MARKER.len() {
        for i in (0..=me.len() - MARKER.len()).rev() {
            if &me[i..i + MARKER.len()] == MARKER {
                start = Some(i + MARKER.len());
                break;
            }
        }
    }
    let Some(start) = start else { return };
    let link = String::from_utf8_lossy(&me[start..]);
    let link = link.trim_end_matches('\0').trim();
    if link.is_empty() {
        return;
    }
    // ShellExecuteA ouvre le lien avec le gestionnaire du schéma : c'est
    // Windows qui sait que `locaryn://` appartient à Locaryn (le schéma est
    // enregistré par l'application, voir tauri.conf.json / deep-link plugin).
    open_with_shell(link);
}

/// Ouvrir un URI via ShellExecuteA — sans dépendance, sans manifeste.
#[cfg(windows)]
fn open_with_shell(link: &str) {
    use std::os::windows::ffi::OsStrExt;

    let wide: Vec<u16> = std::ffi::OsStr::new(link)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let operation: Vec<u16> = std::ffi::OsStr::new("open")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        ShellExecuteW(
            0,
            operation.as_ptr(),
            wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}

#[cfg(not(windows))]
fn open_with_shell(link: &str) {
    // Sur Linux/macOS, le schéma appartient à xdg-open / open quand
    // l'application s'est enregistrée. Le lanceur généré sur Windows reste
    // le cas principal ; ici, on tente l'outil du système.
    let _ = std::process::Command::new("xdg-open").arg(link).spawn();
}

#[cfg(windows)]
const SW_SHOWNORMAL: i32 = 1;

#[cfg(windows)]
#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(
        hwnd: isize,
        operation: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show: i32,
    ) -> isize;
}
