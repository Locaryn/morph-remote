# morph-remote

L'extension d'accès distant de Locaryn : ouverture de port, tunnels chiffrés, et appairage des appareils et deuxièmes ordinateurs.

## Appairer un appareil

Le QR porte l'adresse et l'empreinte de la machine (côté application). Depuis
la v3.2, le morph sait aussi produire ce que la spécification
`locaryn://connect` promet :

- `build_connect_link` — le lien complet, à transmettre ;
- `build_connect_qr` — le même lien en QR (SVG) ;
- `build_pairing_launcher` — le `.exe` d'appairage : un lanceur minuscule qui
  embarque le lien et l'ouvre dans l'application déjà installée de la machine
  à connecter. La demande de confirmation s'affiche là-bas avant toute
  connexion — le fichier n'ouvre rien tout seul.

Depuis la v3.3, le lien embarque automatiquement les URLs des certificats
d'appairage du serveur (`/v1/pairing/cert` et `/v1/pairing/ca`) : l'application
de destination télécharge et installe les certificats toute seule, après
consentement. Ne passez `cert`/`ca` que si la personne héberge elle-même ces
fichiers (reverse proxy…) — un choix explicite n'est jamais réécrit.

Le mot de passe n'est embarqué que sur demande explicite de la personne : il
reste en clair dans le lien et le fichier, la réponse le rappelle. C'est aussi
lui qui protège le téléchargement du paquet certificat client sur le serveur.

## Compatibilité : les lanceurs déjà générés

Le morph a été renommé (morph-travel-tunnel → morph-remote). Depuis la v3.3,
les lanceurs sont déposés dans `~/.lochor/morph/remote` ; ceux générés avant
le renommage vivent encore dans `~/.lochor/morph/travel-tunnel`. **La v3.3
les ramène automatiquement** vers le nouveau dossier — au démarrage du serveur
MCP et avant chaque génération — sans jamais écraser un fichier plus récent.
Rien à faire : un lanceur de la v3.2 reste trouvable après la mise à jour.

Pour compiler le binaire du lanceur (à placer à côté du serveur MCP) :

```sh
cargo build --release -p locaryn-plugin-remote --bin locaryn-pair-launcher
```
