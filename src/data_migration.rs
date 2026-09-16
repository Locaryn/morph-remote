//! Migration des données du morph quand son dossier de données bouge.
//!
//! Le morph a changé de nom (morph-travel-tunnel → morph-remote) : un jour,
//! le dossier de données `~/.lochor/morph/travel-tunnel` pourrait devenir
//! `~/.lochor/morph/remote`. Les lanceurs déjà générés y sont — la personne
//! n'a pas à aller les chercher plus loin parce que le morph a été renommé.
//!
//! La règle : **déplacer, jamais écraser, ne rien détruire**. Un fichier de
//! même nom déjà présent dans le dossier courant est le plus récent — il
//! gagne. Tout ce qui n'est pas un lanceur reste où il est.

use std::path::Path;

/// Un lanceur d'appairage porte l'extension `.exe` (casse indifférente).
/// Tout le reste — notes, archives, ce que la personne a posé là — ne nous
/// appartient pas : la migration ne le touche pas.
pub fn est_lanceur(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
}

/// Déplace chaque `.exe` de `legacy` vers `current`, sans jamais écraser :
/// un fichier de même nom déjà présent dans `current` est la version la plus
/// récente — il gagne. Les autres fichiers restent où ils sont.
///
/// Retourne `(déplacés, laissés sur place)`. Un ancien dossier absent n'est
/// pas une erreur : c'est le cas le plus courant.
pub fn migrate_launchers(legacy: &Path, current: &Path) -> std::io::Result<(usize, usize)> {
    let mut moved = 0usize;
    let mut left = 0usize;
    let entries = match std::fs::read_dir(legacy) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((0, 0)),
        Err(e) => return Err(e),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || !est_lanceur(&path) {
            continue;
        }
        let dest = current.join(entry.file_name());
        if dest.exists() {
            left += 1;
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // `rename` échoue entre deux volumes : on retombe sur copie puis
        // suppression — et si la suppression échoue, l'original reste en
        // place plutôt que perdu.
        std::fs::rename(&path, &dest)
            .or_else(|_| std::fs::copy(&path, &dest).and_then(|_| std::fs::remove_file(&path)))?;
        moved += 1;
    }
    Ok((moved, left))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un dossier temporaire unique par test, sans dépendance de plus.
    fn tmpdir(tag: &str) -> std::path::PathBuf {
        let d =
            std::env::temp_dir().join(format!("locaryn-morph-remote-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn seuls_les_exe_se_deplacent() {
        let d = tmpdir("move");
        let legacy = d.join("legacy");
        let current = d.join("current");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("Appairage.exe"), b"MZ").unwrap();
        std::fs::write(legacy.join("notes.txt"), b"x").unwrap();
        let (moved, left) = migrate_launchers(&legacy, &current).unwrap();
        assert_eq!((moved, left), (1, 0));
        assert!(current.join("Appairage.exe").exists());
        assert!(
            legacy.join("notes.txt").exists(),
            "les autres fichiers restent"
        );
        assert!(!legacy.join("Appairage.exe").exists());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn ancien_dossier_absent_n_est_pas_une_erreur() {
        let d = tmpdir("absent");
        let (moved, left) = migrate_launchers(&d.join("inexistant"), &d.join("current")).unwrap();
        assert_eq!((moved, left), (0, 0));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn le_fichier_courant_gagne_jamais_ecrase() {
        let d = tmpdir("keep");
        let legacy = d.join("legacy");
        let current = d.join("current");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::create_dir_all(&current).unwrap();
        std::fs::write(legacy.join("Appairage-Locaryn.exe"), b"old").unwrap();
        std::fs::write(current.join("Appairage-Locaryn.exe"), b"new").unwrap();
        let (moved, left) = migrate_launchers(&legacy, &current).unwrap();
        assert_eq!((moved, left), (0, 1));
        assert_eq!(
            std::fs::read(current.join("Appairage-Locaryn.exe")).unwrap(),
            b"new"
        );
        assert!(
            legacy.join("Appairage-Locaryn.exe").exists(),
            "l'original reste"
        );
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn extension_exe_sans_casse() {
        let d = tmpdir("case");
        let legacy = d.join("legacy");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("PAIRING.EXE"), b"MZ").unwrap();
        let (moved, _) = migrate_launchers(&legacy, &d.join("current")).unwrap();
        assert_eq!(moved, 1);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn est_lanceur_juge_par_extension() {
        assert!(est_lanceur(Path::new("a/Appairage.exe")));
        assert!(est_lanceur(Path::new("a/PAIRING.EXE")));
        assert!(!est_lanceur(Path::new("a/notes.txt")));
        assert!(!est_lanceur(Path::new("a/notes")));
    }
}
