---
name: remote-mode
description: Lancer des tunnels sortants chiffrés, générer des QR d'appairage, produire le .exe d'appairage pour un deuxième PC, et préparer les certificats pour une installation manuelle.
---

# Compétence Mode Distant & Tunnels

Utilisez `start_remote_tunnel` lorsque l'utilisateur demande d'activer l'accès
distant ou d'appairer son smartphone. Le tunnel appartient au service local :
ouvrez-le depuis Réglages → Serveur & fonctions, segment Tunnel — jamais par un
outil.

## Connecter un appareil ou un deuxième ordinateur

Trois outils construisent ce qu'il faut, à partir de l'adresse publique du
serveur (`https://…`, celle que le tunnel ou le port redirigé annonce) :

- `build_connect_link` — le lien `locaryn://connect` complet, à transmettre
  tel quel. Le format est celui de l'application
  (`docs/api/locaryn-deep-links.md` côté hôte).
- `build_connect_qr` — le même lien dessiné en QR (SVG), pour un téléphone
  qui est dans la pièce.
- `build_pairing_launcher` — le fichier `.exe` à exécuter sur un deuxième
  ordinateur : il ouvre l'application et une demande de connexion s'affiche
  là-bas avant toute connexion. Le fichier est déposé dans le dossier de
  données du morph ; la réponse dit où.

## Le mot de passe, jamais par défaut

`password` est un paramètre optionnel de ces trois outils. Ne le remplissez
**que si la personne le demande explicitement** : il reste en clair dans le
lien et dans le fichier généré, et la réponse le rappelle. Sans mot de passe,
l'application de destination le demande à la connexion — c'est le comportement
normal.

## Installation manuelle des certificats

Quand le serveur utilise une autorité auto-signée, le lien ne peut pas servir
les certificats (HTTPS strict côté application). Dans ce cas, donnez à la
personne les URLs `cert` et `ca` hébergées par son serveur : elle les ouvre
dans un navigateur, puis installe les fichiers téléchargés via Réglages →
Connexion → Installer… sur le poste client.
