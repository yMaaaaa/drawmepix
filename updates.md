# DrawMePix — Journal des versions

Format inspiré de [Keep a Changelog](https://keepachangelog.com/fr/).
Ordre antichronologique : version la plus récente en haut.

## [1.5.1] — 2026-06-25

### Ajouté
- **Auto-update intégré** : nouveau bouton « Vérifier les mises à jour »
  dans le menu Aide qui interroge GitHub Releases et propose d'installer
  automatiquement la dernière version
  - Modale de confirmation avec affichage de la version actuelle et de la
    version disponible
  - Téléchargement et remplacement transparent du binaire via la crate
    `self_update`
  - Détection automatique de la plateforme (macOS, Windows, Linux) pour
    récupérer le bon asset
- **Pipeline GitHub Actions** déclenché à chaque tag `v*` qui compile et
  publie automatiquement les binaires pour macOS, Windows et Linux dans
  la release correspondante (`.github/workflows/release.yml`)

### Modifié
- **Renommage du package** de `pixel-project` à `drawmepix` dans
  `Cargo.toml` pour cohérence avec le nom du projet et la configuration
  `self_update`
- **Version Cargo.toml** alignée sur les tags Git (passe à `1.5.1`) pour
  que `env!("CARGO_PKG_VERSION")` retourne la valeur correcte au runtime
- Binaire Linux renommé en `DrawMePix` (au lieu de `drawmepix`) dans
  l'archive de release pour matcher la configuration `bin_name` de
  `self_update` sur tous les OS

### Notes
- Sur macOS, l'auto-update remplace le binaire à l'intérieur du
  `DrawMePix.app` installé. Le bundle `.app` reste intact, seul
  l'exécutable est mis à jour
- Pour les utilisateurs en v1.5.0 (ou antérieure), le bouton
  « Vérifier les mises à jour » n'existe pas — il faut télécharger
  manuellement v1.5.1 une fois pour bénéficier ensuite de l'auto-update

## [1.5.0] — 2026-06-24

### Ajouté
- **Fusion de calques** : bouton « Fusionner ↓ » dans la ligne de chaque
  calque non-bottom, qui compose le calque actif avec celui du dessous en
  respectant les opacités et le compositing alpha, puis le supprime
- **Déplacement de la sélection lasso** : l'outil Déplacer respecte
  désormais le masque lasso quand il est actif — seuls les pixels
  sélectionnés bougent, la zone source devient transparente
- Le masque lasso suit automatiquement le contenu à sa nouvelle position
  après un déplacement, ce qui permet d'enchaîner les opérations
  (re-déplacement, copier-coller, couper)

### Modifié
- **`paint_pixel` respecte les masques de clipping** : quand le calque
  actif est marqué comme clipping mask, les pixels peints hors de la
  silhouette du calque immédiatement en dessous sont ignorés. Tous les
  outils de dessin en héritent automatiquement (Pinceau, Gomme, Ligne,
  Rectangle, Cercle, Texte, Copier-coller, Déplacer). Évite les
  gribouilles invisibles qui apparaissaient à la fusion
- **`set_tool` préserve la sélection** lors du passage sur les outils
  Move et Eyedropper (en plus de Select et Lasso déjà couverts). Ces
  outils ne dessinent pas, donc n'invalident plus le contexte de
  sélection

### Limitations connues
- L'outil Pot de peinture ne respecte pas encore le clipping mask : il
  remplit la composante connexe dans le buffer brut, donc les pixels
  hors silhouette sont écrits même s'ils n'apparaissent pas. À corriger
  dans une future version si le besoin se présente

## [1.4.0] — 2026-06-23

### Ajouté

#### Outils de dessin
- **Outil Gomme** explicite dans la toolbar (force la peinture en transparent)
- **Outil Pipette** explicite dans la toolbar (clic = récupère la couleur d'un pixel)
- **Outil Déplacer** pour translater le calque actif au drag
- **Outil Lasso libre** avec polygone fermé au release et fill scanline
- **Outil Texte** avec bitmap font 5x7 hardcodée (espace, ponctuation, chiffres, majuscules), preview live et slider taille 1–5
- **Outil Flou** (box blur 3x3) appliqué à la sélection rectangulaire
- **Outil Règle** orientable avec poignées draggables aux extrémités et snap du Pinceau sur la ligne

#### Calques
- **Onion skin** (calque d'animation) avec radius et opacité réglables, frames précédentes en bleuté
- **Calques en clipping mask** : un calque marqué n'apparaît que dans la silhouette du calque immédiatement en dessous

#### Symétrie
- **Axes de symétrie déplaçables** au drag avec verrouillage par défaut pour éviter les déplacements accidentels
- Curseur visuel adapté au survol des axes (ResizeHorizontal / ResizeVertical)

#### Frames
- **Réordonner les frames** via flèches ◀ ▶
- Onion skin par défaut désactivé, opacité 0.4, radius 1 frame

#### Export
- **Export PNG zone utilisée** : découpe la bbox du contenu non-transparent, idéal pour les sprites de jeu
- Distinction entre export complet et export trimmed dans le menu Fichier

#### Performance
- **Affichage du frame time** en temps réel dans la barre de statut (code couleur vert/orange/rouge selon 60/30/<30 fps)
- Optimisation du damier d'arrière-plan : skip cell-by-cell quand pixel_size < 4, gain ~200x sur canvas 1024x1024 (passe de 2000 ms à 10 ms par frame en drag)

#### Personnalisation visuelle
- **16 thèmes** : Clair, Sombre, Contraste élevé, Cyberpunk, Océan, Pastel, Sépia, Forêt, Coucher de soleil, Lavande, Menthe, Monokai, Dracula, Sakura, Nord, Matrix
- **Taille de l'interface** ajustable de 75 % à 200 % via boutons discrets
- **Choix de typographie** entre proportionnelle et monospace
- **Tooltips hex** sur toutes les couleurs de la palette (preset, custom, récentes)

#### Panneaux
- **Panneaux redimensionnables** au drag de leur bord (palette, calques, aperçu, frames) avec bornes raisonnables
- **Toggle de visibilité** par panneau dans le menu Affichage → Panneaux

#### Accessibilité
- **Mode daltonien** : contour de sélection et indicateur de couleur active renforcés en blanc épais
- **Guide des commandes intégré F1** : modale avec raccourcis clavier, gestes souris, outils et astuces
- Menu **Aide** dans la barre de menu pour découverte du raccourci F1

#### UX
- **Aperçu live de la sélection rectangulaire** pendant le drag (avant c'était au release)
- **Boutons Loupe Zoom+ / 1:1 / Zoom-** dans la barre de menu
- **Auto-clear de la sélection** à la création d'un nouveau canvas et au chargement d'un PNG
- Auto-clear également au changement d'outil (sauf si la checkbox « Garder la sélection » est cochée)

### Modifié
- Le contour de sélection sur le canvas et le contour de couleur active dans la palette s'épaississent et passent en blanc quand le mode daltonien est activé
- `paste_at` ignore désormais les pixels transparents du presse-papier, respectant naturellement la forme du lasso et préservant le contenu sous-jacent
- Champ `dark_mode: bool` migré vers `theme: Theme` à 16 variantes
- Application du thème centralisée dans `Self::apply_theme`
- Refactor des outils géométriques en helpers purs `bresenham_pixels` / `rect_pixels` / `circle_pixels` retournant la liste de positions, partagés entre le commit et le preview live
- Format projet `.drawmepix` v1 étendu avec `is_clipping_mask` par calque (compatible ascendant via `#[serde(default)]`)
- L'auto-save (toutes les 60 secondes) prend en compte les nouveaux champs

### Corrigé
- Mitigation du clic droit + Pinceau qui effaçait accidentellement les pixels : curseur visuel `NotAllowed` affiché pendant l'action pour signaler le caractère destructeur
- Bug du faux double `axis_handled` qui rendait l'axe de symétrie attrapable seulement au clic droit
- Zoom centré sur la position du curseur via `ScrollArea::scroll_offset` plutôt que `scroll_with_delta` smoothé

## [1.3.0] — 2026-06-22

### Ajouté
- **Aperçu live des formes géométriques** Lors d'un drag avec
  les outils Ligne, Rectangle ou Cercle, les pixels qui seront peints au
  relâchement sont désormais affichés en temps réel et en semi-transparent
  par-dessus le canvas, supprimant les allers-retours undo / refaire.
- **Zoom centré sur le curseur** Cmd + molette zoome
  maintenant autour de la position du curseur plutôt qu'autour du centre du
  canvas. Le pixel sous le curseur reste sous le curseur après le changement
  de zoom, à l'instar d'Aseprite, Photoshop ou GIMP.
- **Dupliquer un calque** Nouveau bouton « Dupliquer » dans
  le panneau Calques, qui crée une copie du calque sélectionné juste
  au-dessus dans la pile, avec un nom suffixé « (copie) ».
- **Réordonner les calques** Flèches ⬆ et ⬇ dans la ligne
  de chaque calque pour modifier l'ordre de composition.
- **Renommer un calque** Double-clic sur le nom d'un calque
  bascule en mode édition, validation par Entrée ou clic ailleurs.

### Modifié
- **Refactor des outils géométriques** : `draw_line`, `draw_rect` et
  `draw_circle` ont été scindés en deux étapes — calcul pur de la liste des
  pixels via `bresenham_pixels`, `rect_pixels` et `circle_pixels`, puis
  application sur le calque actif. Single source of truth garantissant que
  l'aperçu et le commit dessinent exactement les mêmes pixels.

## [1.2.0] — 2026-06-22

### Ajouté
- **Format de projet propriétaire `.drawmepix`** Sérialisation binaire via Serde et Bincode de l'état complet de l'application (frames, calques, palette personnalisée, dimensions du
  canvas, fps). Deux nouvelles entrées dans le menu Fichier pour
  sauvegarder et rouvrir un projet à l'identique.
- **Auto-save** Sauvegarde automatique toutes les soixante
  secondes en cas de modifications non sauvegardées. Si un projet est
  ouvert, écrit dans un fichier `.autosave` à côté ; sinon dans
  `drawmepix_autosave.drawmepix` du dossier temporaire système.
- **Versionnement du format projet** : champ `version: u32` dans la
  sérialisation, permettant de gérer les évolutions futures sans casser
  les anciens fichiers.
- **Couleur blanche dans la palette par défaut** Ajout de
  `Color32::WHITE` entre le gris clair et le transparent.

### Modifié
- **Synchronisation du sélecteur de couleur personnalisée** Le `custom_picker` reflète 
désormais la couleur active. Cliquer sur une
  couleur de la palette (preset, personnelle, récente) ou pipetter un pixel
  met à jour le sélecteur, permettant d'ajuster finement une teinte
  existante.

## [1.1.0] — 2026-06-22

### Modifié
- **Curseur visuel d'interdiction sur clic droit + Pinceau** Mitigation temporaire : affichage de `CursorIcon::NotAllowed` lorsque
  l'utilisateur appuie sur le clic droit alors que l'outil Pinceau est
  actif, pour signaler le caractère destructeur de l'action (peinture en
  transparent). En attente de la migration vers un outil Gomme explicite
  prévue ultérieurement.

## [1.0.0] — version initiale

Première version stable, intégrant les chapitres 1 à 12 du guide
d'implémentation.

### Inclus
- Canvas rectangulaire de 4×4 jusqu'à 4096×4096 pixels avec rendu GPU via
  egui texture cache.
- Transparence native (canal alpha) avec damier d'arrière-plan.
- Outils : pinceau (taille variable de 1 à 20 pixels), pot de peinture
  (flood fill BFS), ligne (Bresenham), rectangle, cercle (midpoint
  circle), sélection rectangulaire.
- Palette prédéfinie de 24 couleurs, palette personnelle ajoutable au clic
  droit, historique des dix dernières couleurs utilisées, color picker
  sRGB.
- Effets : miroir horizontal et vertical.
- Historique undo / redo sur 100 étapes via pattern Memento.
- Sélection rectangulaire avec copier-coller cross-platform.
- Ouverture et sauvegarde de fichiers PNG avec préservation de l'alpha.
- Animation : multiples frames avec lecture automatique et FPS réglable.
- Export GIF animé.
- Calques avec opacité et alpha compositing (Porter-Duff « over »).
- Mode sombre et mode clair.
- Zoom de 25 % à 3200 % (pinch trackpad ou Cmd + molette, centré sur le
  canvas).
- Raccourcis clavier multi-plateformes (Cmd sur macOS, Ctrl ailleurs).