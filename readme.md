# DrawMePix

Un éditeur de pixel art léger, rapide et cross-platform, écrit en Rust avec [egui](https://github.com/emilk/egui).

DrawMePix est conçu pour les artistes pixel qui veulent un outil simple, fluide et focalisé sur l'essentiel : un canvas, des couleurs, des outils. Pas de fioritures, pas de menus interminables — juste ce qu'il faut pour produire du sprite, de l'icône ou de l'illustration pixel art jusqu'à 4096×4096.

---

## Aperçu

- **Cross-platform** : binaires natifs pour macOS, Windows et Linux
- **Canvas rectangulaire** de 4×4 jusqu'à 4096×4096 pixels
- **Transparence native** (canal alpha) avec damier d'arrière-plan pour visualiser
- **Rendu GPU** : la grille est uploadée comme texture et dessinée en un seul appel par frame, ce qui permet de tenir 60 fps même en Full HD
- **Tout en mémoire**, démarrage instantané, aucun setup
- **Sans télémétrie**, sans compte, sans cloud — c'est ton bureau, point.

---

## Fonctionnalités

### Outils de dessin

- **Pinceau** avec taille variable (1 à 20 px, presets rapides 1/2/4/8/16)
- **Pot de peinture** (flood fill BFS)
- **Ligne**, **Rectangle**, **Cercle** (algorithmes de Bresenham et midpoint circle)
- **Sélection rectangulaire** (drag pour créer, Esc pour vider)
- **Pipette** (Alt + clic)
- **Gomme** (clic droit = peint en transparent)

### Canvas

- Création depuis presets : 16×16, 32×32, 64×64, 128×128, 256×256, 512×512, HD 1280×720, Full HD 1920×1080
- Ou dimensions personnalisées via sliders (4×4 à 4096×4096)
- Affichage à l'écran intelligent : zoom de 25 % à 3200 %
- Mode sombre / clair

### Palette

- **Palette prédéfinie** de 24 couleurs choisies (gris, rouges, oranges, verts, bleus, violets, bruns)
- **Palette personnelle** : ajoute tes propres couleurs, clic droit pour retirer
- **Historique des récentes** : les 10 dernières couleurs utilisées
- **Color picker** sRGB pour saisir une couleur précise (RGB ou hex)

### Effets

- **Miroir horizontal** / **vertical** : peint en symétrie pour les sprites symétriques
- **Transparence** : alpha channel complet, damier d'arrière-plan automatique pour visualiser les pixels transparents

### Historique

- **Undo / Redo** sur 100 étapes (pattern Memento)
- Snapshot de la grille à chaque action (début de tracé, flood fill, paste, etc.)

### Sélection & presse-papier

- Sélection rectangulaire avec contour bleu
- Copier / Coller en interne (préserve l'alpha)
- Cross-platform : utilise les events `Copy`/`Paste` macOS et `consume_key` sur Windows/Linux

### Fichiers

- **Ouvrir** un PNG (chargement avec alpha préservé)
- **Sauvegarder en PNG** (format RGBA)
- Dialogues système natifs via `rfd`

---

## Raccourcis clavier

Sur macOS : `Cmd` est la touche modificatrice principale.
Sur Windows / Linux : `Ctrl` joue le même rôle.

| Action | Raccourci |
|---|---|
| Annuler | `Cmd + Z` |
| Rétablir | `Cmd + Shift + Z` ou `Cmd + Y` |
| Copier la sélection | `Cmd + C` |
| Coller au coin haut-gauche de la sélection (ou en 0,0) | `Cmd + V` |
| Sélectionner tout le canvas | `Cmd + A` |
| Effacer la sélection | `Esc` |
| Zoom + | `Cmd + =` |
| Zoom - | `Cmd + -` |
| Réinitialiser le zoom | `Cmd + 0` |
| Afficher / masquer la grille | `G` |

### Souris / trackpad

| Action | Geste |
|---|---|
| Peindre | Clic gauche |
| Effacer (peindre en transparent) | Clic droit |
| Déplacer le canvas (pan) | Clic molette + drag |
| Pipette (récupérer la couleur d'un pixel) | Alt + clic gauche |
| Zoom | Pinch trackpad ou Cmd + molette |

---

## Installation

### Téléchargement (recommandé)

Les binaires pré-compilés pour macOS, Windows et Linux sont disponibles sur la page [Releases](https://github.com/yMaaaaa/drawmepix/releases) du repo.

- **macOS** : télécharge le `.app` ou le `.dmg`, glisse-le dans Applications
- **Windows** : télécharge le `.exe`, lance-le directement
- **Linux** : télécharge le binaire et rends-le exécutable (`chmod +x drawmepix && ./drawmepix`)

### Compilation depuis les sources

Prérequis : Rust >= 1.75 (installation via [rustup](https://rustup.rs/)).

```bash
git clone https://github.com/yMaaaaa/drawmepix.git
cd drawmepix
cargo run --release
```

Le binaire sera dans `target/release/`.

---

## Architecture technique

### Stack

- **Langage** : Rust (édition 2021)
- **Framework UI** : [egui 0.28](https://github.com/emilk/egui) + eframe (mode natif, wgpu backend)
- **Image** : crate [`image`](https://github.com/image-rs/image) pour PNG load/save
- **Dialogues système** : crate [`rfd`](https://github.com/PolyMeilex/rfd) pour les file pickers natifs
- **Modèle UI** : immediate mode (egui) — pas de scene graph, pas d'arbre persistant

### Performance

Le rendu naïf en mode immediate (un `rect_filled` par pixel) plafonne à environ 256×256 pixels avant de devenir injouable. DrawMePix utilise trois optimisations clés pour tenir 60 fps en Full HD :

1. **Texture GPU cache** : la grille est uploadée comme une `egui::TextureHandle` une seule fois par modification, puis dessinée en un seul appel `painter.image()` par frame, quelle que soit la taille
2. **Interpolation Bresenham** : entre deux frames, le pinceau relie automatiquement la dernière cellule peinte à la cellule actuelle (évite les pointillés sur les drags rapides)
3. **Culling de viewport** : les lignes de grille et le damier de transparence ne sont dessinés que sur les cellules effectivement visibles à l'écran via `ui.clip_rect()`

### Algorithmes

- **Flood fill** : BFS itératif avec `VecDeque`
- **Tracé de ligne** : Bresenham
- **Tracé de cercle** : midpoint circle algorithm (utilise les 8 octants par symétrie)
- **Undo/Redo** : pattern Memento (snapshots de la grille empilés dans deux Vec)

---

## Roadmap

### Implémenté

- Chapitres 1–9 du guide v2 : zoom, canvas rectangulaire, taille du pinceau, palette dynamique, mode sombre, outils géométriques, sélection, transparence

### À venir

- **Chapitre 10** — Curseur custom (forme de l'outil sélectionné dessinée à la position de la souris)
- **Chapitre 11** — Animation + export GIF
- **Chapitre 12** — Calques (layers)
- **Chapitre 13** — Format projet `.dpix` (sauvegarde/chargement de l'état complet incluant historique, calques, palette, etc.)

### Idées en vrac

- Aperçu live des formes géométriques pendant le drag
- Symétrie radiale (8 axes pour mandalas)
- Export en SVG (un `<rect>` par pixel non transparent)
- Export agrandi (canvas 32×32 vers un PNG 1024×1024 pour partage social)
- Mode "isometric grid" pour le pixel art iso
- Onion skin (voir la frame précédente en semi-transparent quand on anime)

---

## Licence

Ce projet est sous licence MIT — voir le fichier [LICENSE](./LICENSE) pour les détails.

En clair : tu peux copier, modifier, redistribuer, vendre, tout ce que tu veux. Seule obligation : garder le copyright et la licence dans les copies du code source.

---

## Auteur

Développé par [Matteo Douteaud](https://github.com/yMaaaaa) — étudiant BTS SIO, dans le cadre d'un apprentissage de Rust et de l'écosystème egui.

Si DrawMePix t'a été utile ou si tu veux juste dire bonjour, tu peux passer sur mon [portfolio](https://ymaaaaa.github.io/Portfolio/).
