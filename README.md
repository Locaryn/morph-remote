# plugin-travel-tunnel

Extension de mode distant et tunnels chiffrés pour Locaryn.

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

Le mot de passe n'est embarqué que sur demande explicite de la personne : il
reste en clair dans le lien et le fichier, la réponse le rappelle.

Pour compiler le binaire du lanceur (à placer à côté du serveur MCP) :

```sh
cargo build --release -p locaryn-plugin-remote --bin locaryn-pair-launcher
```
