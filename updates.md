# DrawMePix — Journal des versions

Format inspiré de [Keep a Changelog](https://keepachangelog.com/fr/).
Ordre antichronologique : version la plus récente en haut.

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