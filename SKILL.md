---
name: remote-mode
description: Lancer des tunnels sortants chiffrés, générer des QR d'appairage, produire le .exe d'appairage pour un deuxième PC, et préparer les certificats d'appairage.
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

## Compatibilité : où sont les lanceurs déjà générés

Le morph a été renommé (morph-travel-tunnel → morph-remote). Les lanceurs
générés avant le renommage vivent dans `~/.lochor/morph/travel-tunnel` ; la
v3.3 les dépose dans `~/.lochor/morph/remote`. **Au démarrage du serveur MCP
et avant chaque génération, le morph ramène automatiquement les lanceurs de
l'ancien dossier vers le nouveau** — sans jamais écraser un fichier plus
récent, et sans toucher aux autres fichiers. La personne n'a rien à faire :
un lanceur généré avec la v3.2 reste trouvable et réutilisable après la mise
à jour.

## Les certificats, embarqués dans le lien

Les trois outils embarquent d'eux-mêmes les URLs `cert` et `ca` du serveur
(`{serveur}/v1/pairing/cert` et `{serveur}/v1/pairing/ca`) : l'application de
destination télécharge et installe les certificats toute seule, après
consentement. Le paquet client est protégé par les identifiants du serveur —
c'est le paramètre `user`/`password` du lien qui les porte, et la modale de
consentement reste le lieu où la personne les a fournis.

Ne passez `cert`/`ca` que si la personne héberge elle-même ces fichiers
(reverse proxy, hébergement personnel) : un choix explicite n'est jamais
réécrit.

## Le mot de passe, jamais par défaut

`password` est un paramètre optionnel de ces trois outils. Ne le remplissez
**que si la personne le demande explicitement** : il reste en clair dans le
lien et dans le fichier généré, et la réponse le rappelle. Sans mot de passe,
l'application de destination le demande à la connexion — c'est le comportement
normal.
