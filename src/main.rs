use eframe::egui;

const MAX_CANVAS_SIZE: f32 = 640.0;
const DEFAULT_GRID_SIZE: usize = 16;
const EXPORT_SCALE: u32 = 16;

#[derive(PartialEq, Clone, Copy)]
enum Tool {
    Brush,  //le pinceau actuel
    Bucket, //le pot de peinture
}

fn preset_palette() -> Vec<egui::Color32> {
    vec![
        // Gris
        egui::Color32::BLACK,
        egui::Color32::from_rgb(64, 64, 64),
        egui::Color32::from_rgb(128, 128, 128),
        egui::Color32::from_rgb(192, 192, 192),
        egui::Color32::WHITE,
        // Rouges
        egui::Color32::from_rgb(139, 0, 0),
        egui::Color32::from_rgb(255, 0, 0),
        egui::Color32::from_rgb(255, 105, 105),
        // Oranges / jaunes
        egui::Color32::from_rgb(255, 140, 0),
        egui::Color32::from_rgb(255, 200, 0),
        egui::Color32::from_rgb(255, 255, 0),
        // Verts
        egui::Color32::from_rgb(0, 100, 0),
        egui::Color32::from_rgb(0, 200, 0),
        egui::Color32::from_rgb(150, 255, 100),
        // Bleus
        egui::Color32::from_rgb(0, 0, 139),
        egui::Color32::from_rgb(0, 100, 255),
        egui::Color32::from_rgb(135, 206, 250),
        // Violets / roses
        egui::Color32::from_rgb(75, 0, 130),
        egui::Color32::from_rgb(150, 50, 200),
        egui::Color32::from_rgb(255, 105, 180),
        egui::Color32::from_rgb(255, 192, 203),
        // Bruns
        egui::Color32::from_rgb(101, 67, 33),
        egui::Color32::from_rgb(139, 69, 19),
        egui::Color32::from_rgb(210, 180, 140),
    ]
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("DrawMePix"),
        ..Default::default()
    };

    eframe::run_native(
        "DrawMePix",
        options,
        Box::new(|_cc| Ok(Box::new(DrawMePixApp::default()))),
    )
}

struct DrawMePixApp {
    grid: Vec<Vec<egui::Color32>>,
    grid_size: usize,
    current_color: egui::Color32,
    preset_palette: Vec<egui::Color32>,
    custom_palette: Vec<egui::Color32>,
    custom_picker: egui::Color32,
    last_status: Option<String>,
    show_new_dialog: bool,
    new_grid_size_input: usize,
    hovered_cell: Option<(usize, usize)>,

    //Pile des états passés (pour Ctrl+Z)
    history: Vec<Vec<Vec<egui::Color32>>>,

    //Pile des états annulés (pour Ctrl+Y / Cmd+Shift+Z)
    redo_stack: Vec<Vec<Vec<egui::Color32>>>,

    //Pour distinguer le premier clic d'un drag continu
    is_drawing: bool,

    //Pour afficher la grille ou non
    show_grid: bool,

    tool: Tool,

    mirror_horizontal: bool,
    mirror_vertical: bool,
}

impl Default for DrawMePixApp {
    fn default() -> Self {
        Self {
            grid: vec![vec![egui::Color32::WHITE; DEFAULT_GRID_SIZE]; DEFAULT_GRID_SIZE],
            grid_size: DEFAULT_GRID_SIZE,
            current_color: egui::Color32::BLACK,
            preset_palette: preset_palette(),
            custom_palette: Vec::new(),
            custom_picker: egui::Color32::from_rgb(255, 50, 50),
            last_status: None,
            show_new_dialog: false,
            new_grid_size_input: 16,
            hovered_cell: None,
            history: Vec::new(),
            redo_stack: Vec::new(),
            is_drawing: false,
            show_grid: true, //Grille visible au démarrage.
            tool: Tool::Brush,
            mirror_horizontal: false,
            mirror_vertical: false,
        }
    }
}

impl DrawMePixApp {
    fn create_new_canvas(&mut self, size: usize) {
        self.push_history();
        self.grid_size = size.clamp(4, 128);
        self.grid = vec![vec![egui::Color32::WHITE; self.grid_size]; self.grid_size];
        self.last_status = Some(format!(
            "Nouveau canvas {}×{}",
            self.grid_size, self.grid_size
        ));
    }

    fn clear_canvas(&mut self) {
        self.push_history();
        self.grid = vec![vec![egui::Color32::WHITE; self.grid_size]; self.grid_size];
        self.last_status = Some("Canvas effacé".to_string());
    }

    fn save_png(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG image", &["png"])
            .set_file_name("drawing.png")
            .save_file()
        else {
            return;
        };

        let width = self.grid_size as u32 * EXPORT_SCALE;
        let height = self.grid_size as u32 * EXPORT_SCALE;
        let mut img = image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::new(width, height);

        for y in 0..self.grid_size {
            for x in 0..self.grid_size {
                let c = self.grid[y][x];
                let pixel = image::Rgba([c.r(), c.g(), c.b(), c.a()]);
                for dy in 0..EXPORT_SCALE {
                    for dx in 0..EXPORT_SCALE {
                        img.put_pixel(
                            x as u32 * EXPORT_SCALE + dx,
                            y as u32 * EXPORT_SCALE + dy,
                            pixel,
                        );
                    }
                }
            }
        }

        match img.save(&path) {
            Ok(_) => self.last_status = Some(format!("Sauvegardé : {}", path.display())),
            Err(e) => self.last_status = Some(format!("Erreur : {}", e)),
        }
    }

    //Remplit toute la zone connectée à (start_x, start_y) qui a la
    //même couleur que le pixel de départ, en remplaçant par target_color.
    fn flood_fill(&mut self, start_x: usize, start_y: usize, target_color: egui::Color32) {
        //Borne de sécurité : si on clique en dehors de la grille, on quitte.
        if start_x >= self.grid_size || start_y >= self.grid_size {
            return;
        }

        let source_color = self.grid[start_y][start_x];

        //Cas trivial : la zone est déjà de la bonne couleur, rien à faire.
        //C'est aussi une protection essentielle : sinon l'algo boucle à l'infini.
        if source_color == target_color {
            return;
        }

        //VecDeque c'est une fille (queue) à double extrémité, idéale pour le BFS.
        //On la remplit par derrière, on la vide par devant.
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((start_x, start_y));

        while let Some((x, y)) = queue.pop_front() {
            //On vérifie que le pixel est toujours de la couleur source.
            //Important : un pixel peut être ajouté plusieurs fois à la queue
            //par ses voisins, on évite de le retraiter.
            if self.grid[y][x] != source_color {
                continue;
            }

            //On remplit ce pixel.
            self.grid[y][x] = target_color;

            //On ajoute les 4 voisins cardinaux à la queue.
            //L'algo classique du flood fill 4-connecté.
            if x > 0 {
                queue.push_back((x - 1, y));
            }
            if x + 1 < self.grid_size {
                queue.push_back((x + 1, y));
            }
            if y > 0 {
                queue.push_back((x, y - 1));
            }
            if y + 1 < self.grid_size {
                queue.push_back((x, y + 1));
            }
        }
    }

    fn load_png(&mut self) {
        //Étape 1 : demander un fichier à ouvrir
        let Some(path) = rfd::FileDialog::new()
        .add_filter("Image", &["png", "jpg", "jpeg", "bmp", "gif"])
        .pick_file()
        else {
            return;
        };
    

    //Étape 2 : ouvrir et lire l'image. Le ? propage l'erreur s'il y en a une,
    //mais comme on n'est pas dans une fonction qui retourne Result,
    //on utilise un match explicite.
    let img = match image::open(&path) {
            Ok(img) => img.to_rgba8(),
            Err(e) => {
                self.last_status = Some(format!("Erreur ouverture : {}", e));
                return;
            }
        };

        let (w, h) = (img.width(), img.height());

        // Étape 3 : sauvegarder l'état actuel dans l'historique avant de tout écraser
        self.push_history();

        // Étape 4 : reconstituer la grille en sous-échantillonnant l'image.
        // On parcourt chaque cellule de la nouvelle grille et on prend la couleur
        // du pixel correspondant au centre du bloc.
        let mut new_grid = vec![vec![egui::Color32::WHITE; self.grid_size]; self.grid_size];
        for y in 0..self.grid_size {
            for x in 0..self.grid_size {
                // Position du centre de la cellule dans l'image source
                let src_x = (x as u32 * w / self.grid_size as u32).min(w - 1);
                let src_y = (y as u32 * h / self.grid_size as u32).min(h - 1);
                let pixel = img.get_pixel(src_x, src_y);
                new_grid[y][x] = egui::Color32::from_rgba_unmultiplied(
                    pixel[0], pixel[1], pixel[2], pixel[3],
                );
            }
        }

    self.grid = new_grid;
    self.last_status = Some(format!("Importé : {}", path.display()));
}

    fn paint_pixel(&mut self, x:usize, y:usize, color: egui::Color32) {
        if x >= self.grid_size || y >= self.grid_size {
            return;
        }

        //Pixel principal
        self.grid[y][x] = color;

        //Calcul des données miroir
        let mx = self.grid_size - 1 - x;
        let my = self.grid_size - 1 - y;

        //Miroir horizontal : on dessine aussi à droite (symétrie gauche droite)
        if self.mirror_horizontal {
            self.grid[y][mx] = color;
        }

        //Miroir vertical : on dessine aussi en bas (symétrie haut-bas)
        if self.mirror_vertical {
            self.grid[my][x] = color;
        }

        //Si les deux miroirs sont actifs : symétrie centrale (4 pixels d'un coup)
        if self.mirror_horizontal && self.mirror_vertical {
            self.grid[my][mx] = color;
        }
    }

//Sauvegarde l'état actuel du canvas dans l'historique.
    //À appeler AVANT chaque modification
    fn push_history(&mut self) {
        //On clone la grille actuelle (deep copy) et on l'empile.
        self.history.push(self.grid.clone());

        //Toute nouvelle action invalide la pile de redo : si je peins
        //après avoir annulé, je ne peux plus "rétablir" l'ancienne action.
        self.redo_stack.clear();

        //On limite la taille de l'historique pour éviter de bouffer
        //toute la RAM. 100 entrée = largement assez.
        if self.history.len() > 100 {
            self.history.remove(0);
        }
    }

    //Annule la dernière action : restaure l'avant-dernier état.
    fn undo(&mut self) {
        if let Some(previous) = self.history.pop() {
            //L'état actuel devient un état "annulé" : on le pousse dans redo
            //pour pouvoir le rétablir avec Ctrl+Y plus tard.
            self.redo_stack.push(self.grid.clone());
            self.grid = previous;
        }
    }

    //Rétablit une action précédemment annulée.
    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            //Symétrique de undo : on archive l'état actuel avant de
            //restaurer celui qu'on avait annulé.
            self.history.push(self.grid.clone());
            self.grid = next;
        }
    }
}

impl eframe::App for DrawMePixApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // === Raccourcis clavier (à traiter en premier) ===
        ctx.input(|i| {
            if i.modifiers.command && i.key_pressed(egui::Key::Z) {
                if i.modifiers.shift {
                    self.redo();
                } else {
                    self.undo();
                }
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Y) {
                self.redo();
            }
        });
        ctx.input(|i| {
            if i.key_pressed(egui::Key::G) {
                self.show_grid = !self.show_grid;
            }
        });

        // === Barre de menu en haut ===
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Fichier", |ui| {
                    if ui.button("Nouveau canvas…").clicked() {
                        self.show_new_dialog = true;
                        self.new_grid_size_input = self.grid_size;
                        ui.close_menu();
                    }
                    if ui.button("Ouvrir...").clicked() {
                        self.load_png();
                        ui.close_menu();
                    }
                    if ui.button("Sauvegarder en PNG…").clicked() {
                        self.save_png();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quitter").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Affichage", |ui| {
                    //ui.checkbox crée une case à cocher liée directement à un booléen.
                    //L'utilisateur clique, ça toggle, on relit la valeur au prochain frame.
                    ui.checkbox(&mut self.show_grid, "Afficher la grille");
                    ui.separator();
                    ui.checkbox(&mut self.mirror_horizontal, "Mirroir horizontal");
                    ui.checkbox(&mut self.mirror_vertical, "Mirroir Vertical");
                });

                ui.menu_button("Édition", |ui| {
                    if ui
                        .add_enabled(
                            !self.history.is_empty(),
                            egui::Button::new("Annuler  (Ctrl+Z)"),
                        )
                        .clicked()
                    {
                        self.undo();
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            !self.redo_stack.is_empty(),
                            egui::Button::new("Rétablir  (Ctrl+Shift+Z)"),
                        )
                        .clicked()
                    {
                        self.redo();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Tout effacer").clicked() {
                        self.clear_canvas();
                        ui.close_menu();
                    }
                });

                ui.separator();
                ui.label(format!("Grille : {0}×{0}", self.grid_size));
            });
        });

        // === Barre de statut en bas ===
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some((x, y)) = self.hovered_cell {
                    ui.label(format!("Position : ({}, {})", x, y));
                } else {
                    ui.label("Position : —");
                }
                ui.separator();
                let c = self.current_color;
                ui.label(format!(
                    "Couleur : R{} G{} B{}  (#{:02X}{:02X}{:02X})",
                    c.r(),
                    c.g(),
                    c.b(),
                    c.r(),
                    c.g(),
                    c.b()
                ));
                ui.separator();
                ui.label("Astuce : Alt + clic = pipette");
                if let Some(status) = &self.last_status {
                    ui.label(status);
                }
            });
        });

        // === Panneau gauche : palette + actions ===
        egui::SidePanel::left("palette_panel")
            .resizable(false)
            .default_width(180.0)
            .show(ctx, |ui| {
                ui.heading("Palette");
                ui.separator();

                egui::Grid::new("preset_grid")
                    .spacing([4.0, 4.0])
                    .show(ui, |ui| {
                        for (i, color) in self.preset_palette.iter().enumerate() {
                            let is_selected = *color == self.current_color;
                            let stroke = if is_selected {
                                egui::Stroke::new(2.5, egui::Color32::from_rgb(255, 200, 0))
                            } else {
                                egui::Stroke::new(1.0, egui::Color32::from_gray(120))
                            };
                            let button = egui::Button::new("")
                                .fill(*color)
                                .min_size(egui::vec2(28.0, 28.0))
                                .stroke(stroke);
                            if ui.add(button).clicked() {
                                self.current_color = *color;
                            }
                            if (i + 1) % 5 == 0 {
                                ui.end_row();
                            }
                        }
                    });

                ui.add_space(10.0);
                ui.separator();
                ui.label("Couleur personnalisée");
                ui.horizontal(|ui| {
                    ui.color_edit_button_srgba(&mut self.custom_picker);
                    if ui.button("Sélectionner").clicked() {
                        self.current_color = self.custom_picker;
                    }
                });
                if ui.button("Ajouter à ma palette").clicked() {
                    if !self.custom_palette.contains(&self.custom_picker) {
                        self.custom_palette.push(self.custom_picker);
                    }
                }

                if !self.custom_palette.is_empty() {
                    ui.add_space(8.0);
                    ui.label("Ma palette :");
                    let mut to_remove: Option<usize> = None;
                    egui::Grid::new("custom_grid")
                        .spacing([4.0, 4.0])
                        .show(ui, |ui| {
                            for (i, color) in self.custom_palette.iter().enumerate() {
                                let is_selected = *color == self.current_color;
                                let stroke = if is_selected {
                                    egui::Stroke::new(2.5, egui::Color32::from_rgb(255, 200, 0))
                                } else {
                                    egui::Stroke::new(1.0, egui::Color32::from_gray(120))
                                };
                                let resp = ui.add(
                                    egui::Button::new("")
                                        .fill(*color)
                                        .min_size(egui::vec2(28.0, 28.0))
                                        .stroke(stroke),
                                );
                                if resp.clicked() {
                                    self.current_color = *color;
                                }
                                if resp.secondary_clicked() {
                                    to_remove = Some(i);
                                }
                                if (i + 1) % 5 == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                    if let Some(i) = to_remove {
                        self.custom_palette.remove(i);
                    }
                    ui.small("Clic droit pour retirer une couleur");
                }

                ui.separator();
                ui.label("Outil");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.tool, Tool::Brush, "Pinceau");
                    ui.selectable_value(&mut self.tool, Tool::Bucket, "Pot")
                })
            });

        // === Modale "Nouveau canvas" ===
        if self.show_new_dialog {
            let mut keep_open = true;
            let mut create_now = false;
            egui::Window::new("Nouveau canvas")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .open(&mut keep_open)
                .show(ctx, |ui| {
                    ui.label("Taille de la grille :");
                    ui.add(
                        egui::Slider::new(&mut self.new_grid_size_input, 4..=128).text("pixels"),
                    );
                    ui.horizontal(|ui| {
                        for preset in [8, 16, 32, 64, 128] {
                            if ui.button(format!("{0}×{0}", preset)).clicked() {
                                self.new_grid_size_input = preset;
                            }
                        }
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Créer").clicked() {
                            create_now = true;
                        }
                        if ui.button("Annuler").clicked() {
                            self.show_new_dialog = false;
                        }
                    });
                });
            if !keep_open {
                self.show_new_dialog = false;
            }
            if create_now {
                self.create_new_canvas(self.new_grid_size_input);
                self.show_new_dialog = false;
            }
        }

        // === Zone centrale : canvas ===
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                let pixel_size = (MAX_CANVAS_SIZE / self.grid_size as f32).floor();
                let canvas_size = egui::vec2(
                    self.grid_size as f32 * pixel_size,
                    self.grid_size as f32 * pixel_size,
                );
                let (response, painter) =
                    ui.allocate_painter(canvas_size, egui::Sense::click_and_drag());
                let canvas_rect = response.rect;

                for y in 0..self.grid_size {
                    for x in 0..self.grid_size {
                        let pixel_pos = canvas_rect.min
                            + egui::vec2(x as f32 * pixel_size, y as f32 * pixel_size);
                        let pixel_rect = egui::Rect::from_min_size(
                            pixel_pos,
                            egui::vec2(pixel_size, pixel_size),
                        );
                        painter.rect_filled(pixel_rect, 0.0, self.grid[y][x]);
                        if self.show_grid && pixel_size > 6.0 {
                            painter.rect_stroke(
                                pixel_rect,
                                0.0,
                                egui::Stroke::new(0.5, egui::Color32::from_gray(220)),
                            );
                        }
                    }
                }

                self.hovered_cell = response.hover_pos().and_then(|pos| {
                    let rel = pos - canvas_rect.min;
                    let x = (rel.x / pixel_size) as usize;
                    let y = (rel.y / pixel_size) as usize;
                    if x < self.grid_size && y < self.grid_size {
                        Some((x, y))
                    } else {
                        None
                    }
                });

                if response.is_pointer_button_down_on() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let rel = pos - canvas_rect.min;
                        let x = (rel.x / pixel_size) as usize;
                        let y = (rel.y / pixel_size) as usize;
                        if x < self.grid_size && y < self.grid_size {
                            // On lit l'état des touches modificatrices.
                            // En egui, ces booléens sont true tant que la touche est pressée.
                            let alt_pressed = ctx.input(|i| i.modifiers.alt);
                            let right_pressed = ctx.input(|i| i.pointer.secondary_down());

                            if alt_pressed {
                                // Mode pipette : on récupère la couleur du pixel
                                // au lieu de peindre. On ne modifie PAS la grille.
                                self.current_color = self.grid[y][x];
                            } else {
                                // Couleur à appliquer : blanche si clic droit (gomme),
                                // sinon la couleur active.
                                let color = if right_pressed {
                                    egui::Color32::WHITE
                                } else {
                                    self.current_color
                                };

                                // Aiguillage selon l'outil sélectionné
                                match self.tool {
                                    Tool::Brush => {
                                        // Début d'un nouveau trait, on snapshot l'historique
                                        // si ce n'est pas déjà fait pendant ce drag.
                                        if !self.is_drawing {
                                            self.push_history();
                                            self.is_drawing = true;
                                        }
                                        self.paint_pixel(x, y, color);
                                    }
                                    Tool::Bucket => {
                                        // Pot de peinture : on n'agit qu'au premier clic
                                        // d'un drag, pas pendant tout le mouvement.
                                        if !self.is_drawing {
                                            self.push_history();
                                            self.is_drawing = true;
                                            self.flood_fill(x, y, color);
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // Aucun bouton pressé : le trait est terminé, prochain clic = nouveau trait.
                    self.is_drawing = false;
                }
            });
        });
    }
}
