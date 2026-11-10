/**
 * Les panneaux d'appairage qu'ajoute ce morph.
 *
 * L'application ne connait que le reseau local. Les deux transports que ce
 * morph apporte — un port joignable de l'exterieur, et un tunnel sortant — se
 * declarent comme segments du meme selecteur, et se dessinent ici.
 *
 * Light DOM et classes de l'hote, jamais de shadow DOM : un panneau qui
 * emporte ses propres couleurs se voit tout de suite, et cesse de suivre le
 * theme des que l'application change le sien. Aucune valeur n'est inventee —
 * quand le service ne repond pas, on le dit.
 */
(function () {
  "use strict";

  /** Ce que l'hote a prete au panneau, ou rien s'il est monte hors contexte. */
  function hote(el) {
    const c = el.context;
    return c && typeof c.demanderCode === "function" ? c : null;
  }

  // Une preference d'affichage, gardee sur cette machine seulement. Enveloppee
  // parce qu'un navigateur en navigation privee refuse d'ecrire, et qu'un
  // panneau ne doit pas se briser pour si peu.
  function lire(cle, defaut) {
    try {
      const v = window.localStorage.getItem(cle);
      return v === null ? defaut : v;
    } catch {
      return defaut;
    }
  }

  function ecrire(cle, valeur) {
    try {
      window.localStorage.setItem(cle, valeur);
    } catch {
      // Stockage indisponible : la valeur vaut pour cette session, c'est tout.
    }
  }

  function vider(el) {
    while (el.firstChild) el.removeChild(el.firstChild);
  }

  function creer(tag, classe, texte) {
    const el = document.createElement(tag);
    if (classe) el.className = classe;
    if (texte !== undefined) el.textContent = texte;
    return el;
  }

  /** Une note explicative, avec le style des notices de l'hote. */
  function notice(texte) {
    const d = creer("div", "locaryn-pairing-notice");
    d.appendChild(creer("span", null, texte));
    return d;
  }

  function avertissement(texte) {
    return creer("div", "locaryn-pairing-warning", texte);
  }

  /**
   * Le bloc du QR, identique a celui de l'hote.
   *
   * Le SVG vient du service local par la fonction que l'hote a pretee : il
   * n'est pas fabrique ici, et rien ne s'affiche tant qu'il n'est pas arrive.
   */
  function blocQr(code, surCopie, h) {
    const bloc = creer("div", "locaryn-travel-code");

    const bouton = creer("button", "locaryn-travel-qr");
    bouton.type = "button";
    const agrandir = h && typeof h.agrandirCode === "function";
    bouton.title = agrandir ? "Afficher le code en grand" : "Code d'appairage";
    if (agrandir) bouton.addEventListener("click", () => h.agrandirCode(code.qr_svg));
    // Le SVG est produit par le service local de la machine, pas par le reseau.
    bouton.innerHTML = code.qr_svg;
    bloc.appendChild(bouton);

    const dire = creer("div", "locaryn-travel-say");
    dire.appendChild(creer("p", "locaryn-travel-title", "Scannez avec le téléphone"));
    dire.appendChild(
      creer("p", "locaryn-travel-sub", "Dans l'app Locaryn : « Scanner un QR code »."),
    );

    const actions = creer("div", "locaryn-pairing-actions");
    const copier = creer("button", "locaryn-btn-ghost", "Copier l'adresse");
    copier.type = "button";
    copier.addEventListener("click", () => {
      navigator.clipboard
        .writeText(code.url)
        .then(() => {
          copier.textContent = "Adresse copiée";
          window.setTimeout(() => {
            copier.textContent = "Copier l'adresse";
          }, 1500);
        })
        .catch((e) => surCopie(String(e)));
    });
    actions.appendChild(copier);

    // Le même serveur pour un PC d'un autre réseau : le lien s'ouvre dans
    // Locaryn, qui propose la connexion. Copié, jamais affiché.
    if (code.connect_link) {
      const pc = creer("button", "locaryn-btn-ghost", "Lien pour un PC");
      pc.type = "button";
      pc.title = "Copier le lien à ouvrir sur un PC où Locaryn est installé";
      pc.addEventListener("click", () => {
        navigator.clipboard
          .writeText(code.connect_link)
          .then(() => {
            pc.textContent = "Lien copié";
            window.setTimeout(() => {
              pc.textContent = "Lien pour un PC";
            }, 1500);
          })
          .catch((e) => surCopie(String(e)));
      });
      actions.appendChild(pc);
    }
    dire.appendChild(actions);

    bloc.appendChild(dire);
    return bloc;
  }

  /**
   * Une fenêtre de l'hôte (voile + carte), montée sur `document.body` pour ne
   * pas être recouverte par la colonne de réglages. Échap ou un clic sur le
   * voile la ferment.
   */
  function ouvrirFenetre(titre, texte, boutons) {
    const voile = creer("div", "locaryn-modal-overlay");
    voile.setAttribute("role", "presentation");
    const carte = creer("dialog", "locaryn-card");
    carte.open = true;
    carte.setAttribute("aria-modal", "true");
    carte.setAttribute("aria-label", titre);
    carte.appendChild(creer("h3", "locaryn-section-title", titre));
    carte.appendChild(creer("p", "locaryn-field-hint", texte));
    const actions = creer("div", "locaryn-pairing-actions");
    function surTouche(e) {
      if (e.key === "Escape") fermer();
    }
    function fermer() {
      document.removeEventListener("keydown", surTouche);
      voile.remove();
    }
    for (const b of boutons) {
      const el = creer("button", b.principal ? "locaryn-btn-primary" : "locaryn-btn-ghost", b.texte);
      el.type = "button";
      el.addEventListener("click", () => {
        fermer();
        if (b.action) b.action();
      });
      actions.appendChild(el);
    }
    carte.appendChild(actions);
    voile.appendChild(carte);
    voile.addEventListener("click", (e) => {
      if (e.target === voile) fermer();
    });
    document.addEventListener("keydown", surTouche);
    document.body.appendChild(voile);
    const premier = actions.querySelector("button");
    if (premier) premier.focus();
  }

  function messageDe(err) {
    return String(err && err.message ? err.message : err);
  }

  // ── Accès distant : l'IP publique de cette machine, port par défaut ────
  //
  // Ouvrir l'onglet suffit : le service lit l'IP publique (la box d'abord,
  // qu'il tente d'ouvrir par UPnP) et dessine le code avec le port du
  // service. Changer de port, ou donner un nom de domaine, reste possible
  // dans un champ secondaire. L'adresse n'est jamais écrite à l'écran :
  // elle n'est que dans le code et dans le lien copié.

  /** Le port d'une URL, sans afficher le reste. */
  function portDe(url) {
    try {
      const u = new URL(url);
      return u.port || (u.protocol === "https:" ? "443" : "80");
    } catch {
      return "";
    }
  }

  class PanneauPublic extends HTMLElement {
    constructor() {
      super();
      // Vide : l'adresse par défaut. Un port seul, ou une adresse complète.
      this.saisie = "";
      this.code = null;
      this.erreur = null;
      this.occupe = false;
      this.reglages = false;
    }

    connectedCallback() {
      const h = hote(this);
      this.rendre();
      if (h && h.serveurActif) void this.generer();
    }

    async generer() {
      const h = hote(this);
      if (!h) {
        this.erreur = "Ce panneau n'a pas reçu le contexte de l'application.";
        this.rendre();
        return;
      }
      this.occupe = true;
      this.erreur = null;
      this.code = null;
      this.rendre();
      try {
        this.code = await h.demanderCode("public", this.saisie.trim());
      } catch (e) {
        this.code = null;
        this.erreur = messageDe(e);
      } finally {
        this.occupe = false;
        this.rendre();
      }
    }

    /** Une ligne rouge, « Utiliser le tunnel », et le détail replié. */
    rendreAvertissement(h) {
      const c = this.code;
      const bloc = creer("div", "locaryn-vp-error");
      bloc.setAttribute("role", "status");
      bloc.appendChild(creer("strong", null, c.warning));
      const actions = creer("div", "locaryn-pairing-actions");
      if (h && typeof h.choisirSegment === "function") {
        const tunnel = creer("button", "locaryn-btn-ghost", "Utiliser le tunnel");
        tunnel.type = "button";
        tunnel.addEventListener("click", () => h.choisirSegment("pairing-tunnel"));
        actions.appendChild(tunnel);
      }
      if (c.warning_detail) {
        const details = creer("details", "locaryn-field-hint");
        const resume = creer("summary", null, "Détails");
        // Même hauteur que le bouton voisin, texte centré sur sa ligne.
        resume.style.display = "flex";
        resume.style.alignItems = "center";
        resume.style.minHeight = "44px";
        resume.style.cursor = "pointer";
        details.style.margin = "0";
        details.appendChild(resume);
        details.appendChild(creer("p", null, c.warning_detail));
        actions.appendChild(details);
      }
      actions.style.alignItems = "flex-start";
      if (actions.firstChild) bloc.appendChild(actions);
      this.appendChild(bloc);
    }

    rendreReglages() {
      const bloc = creer("div", "locaryn-pairing-public");
      const etiquette = creer("label", "locaryn-pairing-select-label", "Port");
      etiquette.htmlFor = "morph-remote-port";
      bloc.appendChild(etiquette);

      const ligne = creer("div", "locaryn-pairing-public-row");
      const champ = creer("input", "locaryn-input");
      champ.id = "morph-remote-port";
      champ.inputMode = "numeric";
      champ.placeholder = (this.code && portDe(this.code.url)) || "7474";
      champ.value = this.saisie;
      champ.addEventListener("input", (e) => {
        this.saisie = e.target.value;
      });
      champ.addEventListener("keydown", (e) => {
        if (e.key === "Enter") void this.generer();
      });
      ligne.appendChild(champ);

      const bouton = creer("button", "locaryn-btn-ghost", this.occupe ? "…" : "Appliquer");
      bouton.type = "button";
      bouton.disabled = this.occupe;
      bouton.addEventListener("click", () => void this.generer());
      ligne.appendChild(bouton);
      bloc.appendChild(ligne);
      bloc.appendChild(
        creer(
          "p",
          "locaryn-field-hint",
          "Vide : le port du service. Un autre port doit être redirigé par la box.",
        ),
      );
      this.appendChild(bloc);
    }

    rendre() {
      const h = hote(this);
      vider(this);

      if (h && !h.serveurActif) {
        this.appendChild(
          notice(
            "Serveur arrêté : choisissez ce mode dans Service & Mode Serveur.",
          ),
        );
        return;
      }

      if (this.occupe && !this.code) {
        this.appendChild(
          creer("p", "locaryn-field-hint", "Recherche de l'adresse publique de cette machine…"),
        );
      }
      if (this.erreur) this.appendChild(avertissement(this.erreur));
      if (this.code && this.code.warning) this.rendreAvertissement(h);
      if (this.code && this.code.qr_svg) {
        this.appendChild(
          blocQr(
            this.code,
            (m) => {
              this.erreur = m;
              this.rendre();
            },
            h,
          ),
        );
        // Le port Internet exige le certificat remis à l'appairage : un
        // téléphone neuf le reçoit en scannant ce code sur le même Wi‑Fi.
        this.appendChild(
          creer(
            "p",
            "locaryn-field-hint",
            "Un téléphone neuf scanne ce code une première fois sur le même Wi‑Fi.",
          ),
        );
      }

      const plus = creer(
        "button",
        "locaryn-btn-ghost",
        this.reglages ? "Masquer le port" : "Changer le port",
      );
      plus.type = "button";
      plus.setAttribute("aria-expanded", String(this.reglages));
      plus.addEventListener("click", () => {
        this.reglages = !this.reglages;
        this.rendre();
      });
      this.appendChild(plus);
      if (this.reglages) this.rendreReglages();
    }
  }

  // ── Tunnel : un relais appelé depuis l'intérieur ───────────────────────
  //
  // Rien n'est ouvert sur la box : c'est la machine qui appelle le relais.
  // Aucun relais n'est livré avec le morph — on regarde lesquels sont là, et
  // on dit comment obtenir celui qui manque plutôt que d'échouer sans raison.

  /** Le relais peut-il s'ouvrir tout de suite ? */
  function pret(r) {
    return Boolean(r.installed && (!r.needs_account || r.account_ready !== false));
  }

  /** Ce que dit l'option du sélecteur, avant le choix. */
  function libelle(r) {
    if (!r.installed) return `${r.label} — non installé`;
    if (r.needs_account && r.account_ready === false) return `${r.label} — compte à connecter`;
    if (r.needs_account && r.account_ready === undefined) return `${r.label} — compte requis`;
    return r.label;
  }

  class PanneauTunnel extends HTMLElement {
    constructor() {
      super();
      this.relais = [];
      this.choisi = "";
      this.etat = null;
      this.code = null;
      this.erreur = null;
      this.occupe = false;
      this.charge = false;
      // Le serveur vers lequel renvoyer, quand le relais en reclame un. Garde
      // d'une session a l'autre : personne ne veut retaper son serveur.
      this.cible = lire("morph-remote-cible", "");
      // Le jeton que la personne colle pour un relais qui en demande un. Il
      // part tel quel vers l'outil du relais, jamais ailleurs.
      this.jeton = "";
      // Le relais dont on attend la connexion dans le navigateur.
      this.attenteConnexion = "";
      this.minuterie = 0;
    }

    connectedCallback() {
      this.rendre();
      void this.rafraichir();
      // L'état vient du service : un tunnel ouvert ou tombé ailleurs se voit
      // ici sans recharger la page.
      this.minuterie = window.setInterval(() => void this.suivre(), 4000);
    }

    disconnectedCallback() {
      window.clearInterval(this.minuterie);
    }

    /** Relire l'état du service ; les relais aussi quand on attend une connexion. */
    async suivre() {
      const h = hote(this);
      if (!h || this.occupe || !this.charge) return;
      try {
        const etat = await h.etatPartage();
        const avant = Boolean(this.etat && this.etat.active);
        const apres = Boolean(etat && etat.active);
        this.etat = etat || null;
        let change = avant !== apres;
        if (this.attenteConnexion) {
          this.relais = (await h.relaisDisponibles()) || [];
          const r = this.relais.find((x) => x.id === this.attenteConnexion);
          if (r && pret(r)) {
            this.attenteConnexion = "";
            change = true;
          }
        }
        if (apres && !avant) {
          if (etat.provider) this.choisi = etat.provider;
          void this.produireCode();
        }
        if (!apres && avant) this.code = null;
        if (change) this.rendre();
      } catch (err) {
        this.erreur = messageDe(err);
        this.rendre();
      }
    }

    /** Ce qu'on propose pour le relais choisi, au moment où on le choisit. */
    guider() {
      const h = hote(this);
      const r = this.relais.find((x) => x.id === this.choisi);
      if (!h || !r || typeof h.ouvrirPageRelais !== "function") return;
      const echec = (e) => {
        this.erreur = messageDe(e);
        this.rendre();
      };
      if (!r.installed) {
        if (!r.download_url) return;
        ouvrirFenetre(
          `${r.label} n'est pas installé`,
          `Le tunnel passe par l'outil de ${r.label}, absent de cette machine. ` +
            "Locaryn n'installe rien à votre place : la page officielle de téléchargement " +
            "s'ouvre dans votre navigateur. Installez-le, puis revenez ici.",
          [
            {
              texte: "Ouvrir la page de téléchargement",
              principal: true,
              action: () => h.ouvrirPageRelais(r.id, "download").catch(echec),
            },
            { texte: "Annuler" },
          ],
        );
        return;
      }
      if (r.needs_account && r.account_ready === false && r.sign_in === "paste_token") {
        h.ouvrirPageRelais(r.id, "login").catch(echec);
      }
    }

    async connecter() {
      const h = hote(this);
      const r = this.relais.find((x) => x.id === this.choisi);
      if (!h || !r) return;
      this.occupe = true;
      this.erreur = null;
      this.rendre();
      try {
        await h.connecterRelais(r.id, r.sign_in === "paste_token" ? this.jeton.trim() : undefined);
        this.jeton = "";
        if (r.sign_in === "command") this.attenteConnexion = r.id;
        this.relais = (await h.relaisDisponibles()) || [];
      } catch (err) {
        this.erreur = messageDe(err);
      } finally {
        this.occupe = false;
        this.rendre();
      }
    }

    async rafraichir() {
      const h = hote(this);
      if (!h || typeof h.relaisDisponibles !== "function") {
        this.erreur = "Ce panneau n'a pas reçu le contexte de l'application.";
        this.charge = true;
        this.rendre();
        return;
      }
      try {
        const [relais, etat] = await Promise.all([h.relaisDisponibles(), h.etatPartage()]);
        this.relais = relais || [];
        this.etat = etat || null;
        if (!this.choisi) {
          // Celui qui ne demande rien d'abord : découvrir qu'il faut un compte
          // au moment de partir est le pire moment pour l'apprendre.
          // Le tunnel déjà ouvert d'abord : l'écran montre ce qui tourne.
          const ouvert = etat && etat.active && etat.provider;
          const simple = this.relais.find(pret);
          this.choisi =
            ouvert || (simple || this.relais.find((r) => r.installed) || this.relais[0] || {}).id || "";
        }
        this.erreur = null;
      } catch (err) {
        this.erreur = String(err && err.message ? err.message : err);
      } finally {
        this.charge = true;
        this.rendre();
        if (this.etat && this.etat.active) void this.produireCode();
      }
    }

    async produireCode() {
      const h = hote(this);
      if (!h) return;
      try {
        this.code = await h.demanderCode("tunnel");
        this.erreur = null;
      } catch (e) {
        this.code = null;
        this.erreur = String(e && e.message ? e.message : e);
      }
      this.rendre();
    }

    async basculer() {
      const h = hote(this);
      if (!h) return;
      this.occupe = true;
      this.erreur = null;
      this.rendre();
      try {
        const actif = Boolean(this.etat && this.etat.active);
        const relais = this.relais.find((r) => r.id === this.choisi);
        // Un seul champ pour le service : le relais, puis ce qu'il lui faut.
        const demande =
          relais && relais.needs_target ? `${this.choisi}:${this.cible.trim()}` : this.choisi;
        this.etat = await h.reglerPartage(actif ? null : demande);
        if (!this.etat || !this.etat.active) this.code = null;
        await this.rafraichir();
      } catch (err) {
        this.erreur = String(err && err.message ? err.message : err);
      } finally {
        this.occupe = false;
        this.rendre();
      }
    }

    /** Ce qu'il reste à faire pour le relais choisi, avec de quoi le faire. */
    rendreGuide(r) {
      const h = hote(this);
      const guide = Boolean(h && typeof h.ouvrirPageRelais === "function");
      if (!r.installed) {
        this.appendChild(notice(r.install_hint));
        if (guide && r.download_url) {
          const b = creer("button", "locaryn-btn-ghost", `Installer ${r.label}…`);
          b.type = "button";
          b.addEventListener("click", () => this.guider());
          this.appendChild(b);
        }
        return;
      }
      if (!guide || !r.needs_account || r.account_ready !== false) return;

      const bloc = creer("div", "locaryn-pairing-public");
      if (r.sign_in === "paste_token") {
        const et = creer("label", "locaryn-pairing-select-label", `Jeton ${r.label}`);
        et.htmlFor = "morph-remote-jeton";
        bloc.appendChild(et);
        bloc.appendChild(
          creer(
            "p",
            "locaryn-field-hint",
            `Collez le jeton (« authtoken ») de la page ${r.label} ouverte dans votre navigateur.`,
          ),
        );
        const ligne = creer("div", "locaryn-pairing-public-row");
        const champ = creer("input", "locaryn-input");
        champ.id = "morph-remote-jeton";
        champ.type = "password";
        champ.autocomplete = "off";
        champ.spellcheck = false;
        champ.value = this.jeton;
        champ.disabled = this.occupe;
        champ.addEventListener("input", (e) => {
          this.jeton = e.target.value;
          const b = this.querySelector("[data-role=connecter]");
          if (b) b.disabled = this.occupe || this.jeton.trim() === "";
        });
        champ.addEventListener("keydown", (e) => {
          if (e.key === "Enter" && this.jeton.trim() !== "") void this.connecter();
        });
        ligne.appendChild(champ);
        const ok = creer("button", "locaryn-btn-primary", this.occupe ? "…" : "Enregistrer");
        ok.type = "button";
        ok.setAttribute("data-role", "connecter");
        ok.disabled = this.occupe || this.jeton.trim() === "";
        ok.addEventListener("click", () => void this.connecter());
        ligne.appendChild(ok);
        bloc.appendChild(ligne);
        const page = creer("button", "locaryn-btn-ghost", "Rouvrir la page du jeton");
        page.type = "button";
        page.addEventListener("click", () =>
          h.ouvrirPageRelais(r.id, "login").catch((e) => {
            this.erreur = messageDe(e);
            this.rendre();
          }),
        );
        bloc.appendChild(page);
      } else if (r.sign_in === "command") {
        const attend = this.attenteConnexion === r.id;
        bloc.appendChild(
          creer(
            "p",
            "locaryn-field-hint",
            attend
              ? "Connectez-vous dans le navigateur : cet écran se met à jour seul."
              : `${r.label} demande un compte, à connecter dans votre navigateur.`,
          ),
        );
        const b = creer(
          "button",
          "locaryn-btn-primary",
          attend ? "Rouvrir la connexion" : `Se connecter à ${r.label}`,
        );
        b.type = "button";
        b.disabled = this.occupe;
        b.addEventListener("click", () => void this.connecter());
        bloc.appendChild(b);
      }
      this.appendChild(bloc);
    }

    rendre() {
      const h = hote(this);
      vider(this);

      if (h && !h.serveurActif) {
        this.appendChild(
          notice(
            "Serveur arrêté : choisissez ce mode dans Service & Mode Serveur.",
          ),
        );
        return;
      }

      if (!this.charge) {
        this.appendChild(creer("p", "locaryn-field-hint", "Lecture des relais disponibles…"));
        return;
      }

      const actif = Boolean(this.etat && this.etat.active);

      const bloc = creer("div", "locaryn-pairing-public");
      const etiquette = creer("label", "locaryn-pairing-select-label", "Relais");
      etiquette.htmlFor = "morph-remote-relais";
      bloc.appendChild(etiquette);

      const ligne = creer("div", "locaryn-pairing-public-row");
      const choix = creer("select", "locaryn-input");
      choix.id = "morph-remote-relais";
      choix.disabled = this.occupe || actif;
      for (const r of this.relais) {
        const o = document.createElement("option");
        o.value = r.id;
        // Ce qui manque est dit avant le choix, pas découvert comme un échec.
        o.textContent = libelle(r);
        if (r.id === this.choisi) o.selected = true;
        choix.appendChild(o);
      }
      choix.addEventListener("change", (e) => {
        this.choisi = e.target.value;
        this.erreur = null;
        this.rendre();
        this.guider();
      });
      ligne.appendChild(choix);

      const bouton = creer(
        "button",
        actif ? "locaryn-btn-ghost" : "locaryn-btn-primary",
        this.occupe ? "…" : actif ? "Fermer le tunnel" : "Ouvrir le tunnel",
      );
      bouton.type = "button";
      bouton.setAttribute("data-role", "basculer");
      const relaisChoisi = this.relais.find((r) => r.id === this.choisi);
      const cibleManquante =
        Boolean(relaisChoisi && relaisChoisi.needs_target) && this.cible.trim() === "";
      const incomplet = Boolean(
        relaisChoisi && relaisChoisi.account_ready !== undefined && !pret(relaisChoisi),
      );
      bouton.disabled =
        this.occupe || (!actif && (this.choisi === "" || cibleManquante || incomplet));
      bouton.addEventListener("click", () => void this.basculer());
      ligne.appendChild(bouton);

      bloc.appendChild(ligne);
      this.appendChild(bloc);

      const relais = this.relais.find((r) => r.id === this.choisi);

      if (relais && relais.needs_target) {
        const bloc2 = creer("div", "locaryn-pairing-public");
        const et2 = creer("label", "locaryn-pairing-select-label", "Votre serveur");
        et2.htmlFor = "morph-remote-cible";
        bloc2.appendChild(et2);
        const champ = creer("input", "locaryn-input");
        champ.id = "morph-remote-cible";
        champ.placeholder = "moi@serveur.fr:8443";
        champ.value = this.cible;
        champ.disabled = this.occupe || actif;
        champ.addEventListener("input", (e) => {
          this.cible = e.target.value;
          ecrire("morph-remote-cible", this.cible);
          const b = this.querySelector("[data-role=basculer]");
          if (b) b.disabled = this.occupe || this.cible.trim() === "";
        });
        bloc2.appendChild(champ);
        bloc2.appendChild(
          creer(
            "p",
            "locaryn-field-hint",
            "Ajoutez « /2222 » si SSH n'écoute pas sur 22.",
          ),
        );
        this.appendChild(bloc2);
      }

      if (relais && !actif) this.rendreGuide(relais);

      if (this.etat && this.etat.blocker) this.appendChild(notice(this.etat.blocker));
      if (this.erreur) this.appendChild(avertissement(this.erreur));

      if (actif && !this.code && !this.erreur) {
        this.appendChild(creer("p", "locaryn-field-hint", "Tunnel ouvert — préparation du code…"));
      }

      if (this.code && this.code.qr_svg) {
        this.appendChild(
          blocQr(
            this.code,
            (m) => {
              this.erreur = m;
              this.rendre();
            },
            h,
          ),
        );
      }
    }
  }

  if (!customElements.get("locaryn-pairing-public")) {
    customElements.define("locaryn-pairing-public", PanneauPublic);
  }
  if (!customElements.get("locaryn-pairing-tunnel")) {
    customElements.define("locaryn-pairing-tunnel", PanneauTunnel);
  }
})();
