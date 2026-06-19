use eframe::egui::{self};

const MAX_CANVAS_SIZE: f32 = 640.0;
const DEFAULT_GRID_SIZE: usize = 16;
const MIN_ZOOM: f32 = 0.25;
const MAX_ZOOM: f32 = 32.0;
const MAX_RECENT_COLORS: usize = 10;
const TRANSPARENT: egui::Color32 = egui::Color32::TRANSPARENT;

#[derive(PartialEq, Clone, Copy)]
enum Tool {
    Brush,
    Bucket,
    Line,
    Rect,
    Circle,
    Select,
}

fn preset_palette() -> Vec<egui::Color32> {
    vec![
        egui::Color32::BLACK,
        egui::Color32::from_rgb(64, 64, 64),
        egui::Color32::from_rgb(128, 128, 128),
        egui::Color32::from_rgb(192, 192, 192),
        egui::Color32::TRANSPARENT,
        egui::Color32::from_rgb(139, 0, 0),
        egui::Color32::from_rgb(255, 0, 0),
        egui::Color32::from_rgb(255, 105, 105),
        egui::Color32::from_rgb(255, 140, 0),
        egui::Color32::from_rgb(255, 200, 0),
        egui::Color32::from_rgb(255, 255, 0),
        egui::Color32::from_rgb(0, 100, 0),
        egui::Color32::from_rgb(0, 200, 0),
        egui::Color32::from_rgb(150, 255, 100),
        egui::Color32::from_rgb(0, 0, 139),
        egui::Color32::from_rgb(0, 100, 255),
        egui::Color32::from_rgb(135, 206, 250),
        egui::Color32::from_rgb(75, 0, 130),
        egui::Color32::from_rgb(150, 50, 200),
        egui::Color32::from_rgb(255, 105, 180),
        egui::Color32::from_rgb(255, 192, 203),
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
    frames: Vec<Vec<Vec<egui::Color32>>>,
    current_frame: usize,
    fps: u32,
    is_playing: bool,
    last_frame_advance: f64, //timestamp en secondes
    frames_width: usize,
    frames_height: usize,
    current_color: egui::Color32,
    preset_palette: Vec<egui::Color32>,
    custom_palette: Vec<egui::Color32>,
    custom_picker: egui::Color32,
    last_status: Option<String>,
    show_new_dialog: bool,
    new_grid_width_input: usize,
    new_grid_height_input: usize,
    hovered_cell: Option<(usize, usize)>,

    history: Vec<Vec<Vec<egui::Color32>>>,
    redo_stack: Vec<Vec<Vec<egui::Color32>>>,

    is_drawing: bool,
    last_paint_cell: Option<(usize, usize)>,

    show_grid: bool,
    tool: Tool,
    mirror_horizontal: bool,
    mirror_vertical: bool,
    zoom: f32,

    canvas_texture: Option<egui::TextureHandle>,
    texture_dirty: bool,

    brush_size: usize,
    recent_colors: Vec<egui::Color32>,
    dark_mode: bool,

    shape_start: Option<(usize, usize)>,

    selection: Option<(usize, usize, usize, usize)>,
    clipboard: Option<Vec<Vec<egui::Color32>>>,
}

impl Default for DrawMePixApp {
    fn default() -> Self {
        Self {
            frames: vec![vec![
                vec![egui::Color32::TRANSPARENT; DEFAULT_GRID_SIZE];
                DEFAULT_GRID_SIZE
            ]],
            current_frame: 0,
            fps: 8,
            is_playing: false,
            last_frame_advance: 0.0,
            frames_width: DEFAULT_GRID_SIZE,
            frames_height: DEFAULT_GRID_SIZE,
            current_color: egui::Color32::BLACK,
            preset_palette: preset_palette(),
            custom_palette: Vec::new(),
            custom_picker: egui::Color32::from_rgb(255, 50, 50),
            last_status: None,
            show_new_dialog: false,
            hovered_cell: None,
            history: Vec::new(),
            redo_stack: Vec::new(),
            is_drawing: false,
            last_paint_cell: None,
            show_grid: true,
            tool: Tool::Brush,
            mirror_horizontal: false,
            mirror_vertical: false,
            zoom: 1.0,
            new_grid_width_input: DEFAULT_GRID_SIZE,
            new_grid_height_input: DEFAULT_GRID_SIZE,
            canvas_texture: None,
            texture_dirty: true,
            brush_size: 1,
            recent_colors: Vec::new(),
            dark_mode: true,
            shape_start: None,
            selection: None,
            clipboard: None,
        }
    }
}

impl DrawMePixApp {
    fn fresh_frames(width: usize, height: usize) -> Vec<Vec<Vec<egui::Color32>>> {
        vec![Self::fresh_grid(width, height)]
    }

    fn create_new_canvas(&mut self, width: usize, height: usize) {
        self.push_history();
        self.frames_width = width.clamp(4, 4096);
        self.frames_height = height.clamp(4, 4096);
        self.frames = Self::fresh_frames(self.frames_width, self.frames_height);
        self.current_frame = 0;
        self.zoom = 1.0;
        self.hovered_cell = None;
        self.texture_dirty = true;
        self.last_status = Some(format!(
            "Nouveau canvas {}×{}",
            self.frames_width, self.frames_height
        ));
    }

    fn clear_canvas(&mut self) {
        self.push_history();
        self.frames[self.current_frame] = Self::fresh_grid(self.frames_width, self.frames_height);
        self.texture_dirty = true;
        self.last_status = Some("Canvas effacé".to_string());
    }

    fn save_png(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let w = self.frames_width as u32;
        let h = self.frames_height as u32;
        let mut img = image::RgbaImage::new(w, h);
        for y in 0..self.frames_height {
            for x in 0..self.frames_width {
                let c = self.frames[self.current_frame][y][x];
                img.put_pixel(
                    x as u32,
                    y as u32,
                    image::Rgba([c.r(), c.g(), c.b(), c.a()]),
                );
            }
        }
        img.save(path)?;
        Ok(())
    }

    fn paint_brush(&mut self, cx: usize, cy: usize, color: egui::Color32) {
        let half = self.brush_size as isize / 2;
        let size = self.brush_size as isize;
        for dy in 0..size {
            for dx in 0..size {
                let x = cx as isize + dx - half;
                let y = cy as isize + dy - half;
                if x >= 0 && y >= 0 {
                    self.paint_pixel(x as usize, y as usize, color);
                }
            }
        }
    }

    fn flood_fill(&mut self, start_x: usize, start_y: usize, new_color: egui::Color32) {
        let source_color = self.frames[self.current_frame][start_y][start_x];
        if source_color == new_color {
            return;
        }

        let mut queue = std::collections::VecDeque::new();
        queue.push_back((start_x, start_y));

        while let Some((x, y)) = queue.pop_front() {
            if self.frames[self.current_frame][y][x] != source_color {
                continue;
            }
            self.frames[self.current_frame][y][x] = new_color;

            if x > 0 {
                queue.push_back((x - 1, y));
            }
            if x < self.frames_width - 1 {
                queue.push_back((x + 1, y));
            }
            if y > 0 {
                queue.push_back((x, y - 1));
            }
            if y < self.frames_height - 1 {
                queue.push_back((x, y + 1));
            }
        }
        self.texture_dirty = true;
    }

    fn load_png(&mut self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let img = image::open(path)?.to_rgba8();
        let w = img.width() as usize;
        let h = img.height() as usize;

        let mut new_grid = vec![vec![egui::Color32::TRANSPARENT; w]; h];
        for y in 0..h {
            for x in 0..w {
                let p = img.get_pixel(x as u32, y as u32);
                new_grid[y][x] = egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]);
            }
        }

        self.frames_width = w;
        self.frames_height = h;
        self.frames = vec![new_grid];
        self.current_frame = 0;
        self.history.clear();
        self.redo_stack.clear();
        self.zoom = 1.0;
        self.hovered_cell = None;
        self.texture_dirty = true;
        Ok(())
    }

    fn paint_pixel(&mut self, x: usize, y: usize, color: egui::Color32) {
        self.frames[self.current_frame][y][x] = color;
        if self.mirror_horizontal {
            let mx = self.frames_width - 1 - x;
            self.frames[self.current_frame][y][mx] = color;
        }
        if self.mirror_vertical {
            let my = self.frames_height - 1 - y;
            self.frames[self.current_frame][my][x] = color;
        }
        if self.mirror_horizontal && self.mirror_vertical {
            let mx = self.frames_width - 1 - x;
            let my = self.frames_height - 1 - y;
            self.frames[self.current_frame][my][mx] = color;
        }
        self.texture_dirty = true;
    }

    fn paint_line(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, color: egui::Color32) {
        let (mut x0, mut y0) = (x0 as i32, y0 as i32);
        let (x1, y1) = (x1 as i32, y1 as i32);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x0 >= 0
                && y0 >= 0
                && (x0 as usize) < self.frames_width
                && (y0 as usize) < self.frames_height
            {
                self.paint_pixel(x0 as usize, y0 as usize, color);
            }
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    fn push_history(&mut self) {
        self.history.push(self.frames[self.current_frame].clone());
        self.redo_stack.clear();
        if self.history.len() > 100 {
            self.history.remove(0);
        }
    }

    fn undo(&mut self) {
        if let Some(previous) = self.history.pop() {
            self.redo_stack
                .push(self.frames[self.current_frame].clone());
            self.frames[self.current_frame] = previous;
            self.texture_dirty = true;
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.history.push(self.frames[self.current_frame].clone());
            self.frames[self.current_frame] = next;
            self.texture_dirty = true;
        }
    }

    fn fresh_grid(width: usize, height: usize) -> Vec<Vec<egui::Color32>> {
        vec![vec![egui::Color32::TRANSPARENT; width]; height] // ← TRANSPARENT au lieu de WHITE
    }

    fn rebuild_canvas_texture(&mut self, ctx: &egui::Context) {
        let mut pixels = Vec::with_capacity(self.frames_width * self.frames_height);
        for y in 0..self.frames_height {
            for x in 0..self.frames_width {
                pixels.push(self.frames[self.current_frame][y][x]);
            }
        }

        let image = egui::ColorImage {
            size: [self.frames_width, self.frames_height],
            pixels,
        };

        self.canvas_texture =
            Some(ctx.load_texture("drawmepix_canvas", image, egui::TextureOptions::NEAREST));
        self.texture_dirty = false;
    }

    fn remember_color(&mut self, color: egui::Color32) {
        self.recent_colors.retain(|c| *c != color);
        self.recent_colors.insert(0, color);
        if self.recent_colors.len() > MAX_RECENT_COLORS {
            self.recent_colors.truncate(MAX_RECENT_COLORS);
        }
    }

    fn draw_line(&mut self, x0: isize, y0: isize, x1: isize, y1: isize, color: egui::Color32) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut x = x0;
        let mut y = y0;
        loop {
            if x >= 0 && y >= 0 {
                self.paint_pixel(x as usize, y as usize, color);
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    fn draw_rect(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, color: egui::Color32) {
        let (x_min, x_max) = if x0 < x1 { (x0, x1) } else { (x1, x0) };
        let (y_min, y_max) = if y0 < y1 { (y0, y1) } else { (y1, y0) };
        for x in x_min..=x_max {
            self.paint_pixel(x, y_min, color);
            self.paint_pixel(x, y_max, color);
        }
        for y in y_min..=y_max {
            self.paint_pixel(x_min, y, color);
            self.paint_pixel(x_max, y, color);
        }
    }

    fn draw_circle(&mut self, cx: usize, cy: usize, r: usize, color: egui::Color32) {
        let r = r as isize;
        let cx = cx as isize;
        let cy = cy as isize;
        let mut x: isize = 0;
        let mut y = r;
        let mut d = 1 - r;
        while x <= y {
            for (px, py) in [
                (cx + x, cy + y),
                (cx - x, cy + y),
                (cx + x, cy - y),
                (cx - x, cy - y),
                (cx + y, cy + x),
                (cx - y, cy + x),
                (cx + y, cy - x),
                (cx - y, cy - x),
            ] {
                if px >= 0 && py >= 0 {
                    self.paint_pixel(px as usize, py as usize, color);
                }
            }
            if d < 0 {
                d += 2 * x + 3;
            } else {
                d += 2 * (x - y) + 5;
                y -= 1;
            }
            x += 1;
        }
    }

    fn copy_selection(&mut self) {
        if let Some((x0, y0, x1, y1)) = self.selection {
            let w = x1 - x0 + 1;
            let h = y1 - y0 + 1;
            let mut buf = vec![vec![egui::Color32::TRANSPARENT; w]; h];
            for dy in 0..h {
                for dx in 0..w {
                    buf[dy][dx] = self.frames[self.current_frame][y0 + dy][x0 + dx];
                }
            }
            self.clipboard = Some(buf);
            self.last_status = Some(format!("Copié {} × {}", w, h));
        } else {
            self.last_status = Some("Rien à copier — sélection vide".to_string());
        }
    }

    fn paste_at(&mut self, dx: usize, dy: usize) {
        if let Some(buf) = self.clipboard.clone() {
            self.push_history();
            let h = buf.len();
            let w = if h > 0 { buf[0].len() } else { 0 };
            for y in 0..h {
                for x in 0..w {
                    let gx = dx + x;
                    let gy = dy + y;
                    if gx < self.frames_width && gy < self.frames_height {
                        self.frames[self.current_frame][gy][gx] = buf[y][x];
                    }
                }
            }
            self.texture_dirty = true;
            self.last_status = Some("Collé".to_string());
        }
    }

    fn current_grid(&self) -> &Vec<Vec<egui::Color32>> {
        &self.frames[self.current_frame]
    }

    fn current_grid_mut(&mut self) -> &mut Vec<Vec<egui::Color32>> {
        &mut self.frames[self.current_frame]
    }

    fn add_frame(&mut self) {
        self.push_history();
        let new = Self::fresh_grid(self.frames_width, self.frames_height);
        self.frames.insert(self.current_frame + 1, new);
        self.current_frame += 1;
    }

    fn duplicate_frame(&mut self) {
        self.push_history();
        let copy = self.frames[self.current_frame].clone();
        self.frames.insert(self.current_frame + 1, copy);
        self.current_frame += 1;
    }

    fn remove_frame(&mut self) {
        if self.frames.len() > 1 {
            self.push_history();
            self.frames.remove(self.current_frame);
            if self.current_frame >= self.frames.len() {
                self.current_frame = self.frames.len() - 1;
            }
        }
    }
}

impl eframe::App for DrawMePixApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        if self.is_playing && self.frames.len() > 1 {
            let now = ctx.input(|i| i.time);
            let interval = 1.0 / self.fps as f64;
            if now - self.last_frame_advance >= interval {
                self.current_frame = (self.current_frame + 1) % self.frames.len();
                self.last_frame_advance = now;
                ctx.request_repaint();
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            }
        }

        if self.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }


        ctx.options_mut(|opts| opts.zoom_with_keyboard = false);

        // === Raccourcis qui ne sont pas interceptés par egui (lecture seule) ===
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
            if i.key_pressed(egui::Key::G) {
                self.show_grid = !self.show_grid;
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Equals) {
                self.zoom = (self.zoom * 1.25).min(MAX_ZOOM);
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Minus) {
                self.zoom = (self.zoom / 1.25).max(MIN_ZOOM);
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Num0) {
                self.zoom = 1.0;
            }
            if i.key_pressed(egui::Key::Escape) {
                self.selection = None;
            }
        });

        // === Raccourcis qui DOIVENT consumer la touche en priorité ===
        // (sinon egui les bouffe pour ses TextEdit / Sliders internes)

        // Cmd+C : copier la sélection
        let copy_pressed = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::C));
        if copy_pressed {
            self.copy_selection();
        }

        // Cmd+V : coller au coin haut-gauche de la sélection (ou en 0,0)
        let paste_pressed =
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::V));
        if paste_pressed {
            let (dx, dy) = self.selection.map(|s| (s.0, s.1)).unwrap_or((0, 0));
            self.paste_at(dx, dy);
        }

        // Cmd+A : sélectionner tout le canvas
        let select_all_pressed =
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::A));
        if select_all_pressed {
            self.selection = Some((0, 0, self.frames_width - 1, self.frames_height - 1));
        }

        // === Copy / Paste : sur Mac, egui les convertit en Event::Copy / Event::Paste ===
        // On scanne les events pour les attraper, parce que consume_key sur Key::C / Key::V
        // ne marche pas (egui les a déjà transformés avant qu'ils n'arrivent en queue).
        let mut should_copy = false;
        let mut should_paste = false;
        let mut should_select_all = false;
        ctx.input(|i| {
            for event in &i.events {
                match event {
                    egui::Event::Copy => should_copy = true,
                    egui::Event::Paste(_) => should_paste = true,
                    _ => {}
                }
            }
            // Cmd+A reste un event clavier classique (pas converti)
            if i.modifiers.command && i.key_pressed(egui::Key::A) {
                should_select_all = true;
            }
        });

        if should_copy {
            self.copy_selection();
        }
        if should_paste {
            let (dx, dy) = self.selection.map(|s| (s.0, s.1)).unwrap_or((0, 0));
            self.paste_at(dx, dy);
        }
        if should_select_all {
            self.selection = Some((0, 0, self.frames_width - 1, self.frames_height - 1));
        }

        // === Zoom au pinch trackpad ===
        let zoom_delta = ctx.input(|i| i.zoom_delta());
        if (zoom_delta - 1.0).abs() > 0.001 {
            self.zoom = (self.zoom * zoom_delta).clamp(MIN_ZOOM, MAX_ZOOM);
        }

        // === Zoom au Cmd + molette ===
        let (cmd_down, scroll_y) = ctx.input(|i| (i.modifiers.command, i.raw_scroll_delta.y));
        if cmd_down && scroll_y.abs() > 0.1 {
            let factor = (scroll_y * 0.005).exp();
            self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        }

        // === Barre de menu en haut ===
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Fichier", |ui| {
                    if ui.button("Nouveau canvas…").clicked() {
                        self.show_new_dialog = true;
                        self.new_grid_width_input = self.frames_width;
                        self.new_grid_height_input = self.frames_height;
                        ui.close_menu();
                    }
                    if ui.button("Ouvrir...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Images PNG", &["png"])
                            .pick_file()
                        {
                            if let Err(e) = self.load_png(&path) {
                                eprintln!("Erreur de chargement : {}", e);
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("Sauvegarder en PNG…").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Images PNG", &["png"])
                            .set_file_name("pixel_art.png")
                            .save_file()
                        {
                            if let Err(e) = self.save_png(&path) {
                                eprintln!("Erreur de sauvegarde : {}", e);
                            }
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quitter").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Affichage", |ui| {
                    ui.checkbox(&mut self.show_grid, "Afficher la grille");
                    ui.separator();
                    ui.checkbox(&mut self.mirror_horizontal, "Miroir horizontal");
                    ui.checkbox(&mut self.mirror_vertical, "Miroir vertical");
                    ui.separator();
                    if ui.button("Zoom +").clicked() {
                        self.zoom = (self.zoom * 1.25).min(MAX_ZOOM);
                    }
                    if ui.button("Zoom -").clicked() {
                        self.zoom = (self.zoom / 1.25).max(MIN_ZOOM);
                    }
                    if ui.button("Réinitialiser zoom").clicked() {
                        self.zoom = 1.0;
                    }
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
                    ui.separator();
                    if ui
                        .add_enabled(
                            self.selection.is_some(),
                            egui::Button::new("Copier (Ctrl + C)"),
                        )
                        .clicked()
                    {
                        self.copy_selection();
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            self.clipboard.is_some(),
                            egui::Button::new("Coller (Ctrl + V)"),
                        )
                        .clicked()
                    {
                        let (dx, dy) = self.selection.map(|s| (s.0, s.1)).unwrap_or((0, 0));
                        self.paste_at(dx, dy);
                        ui.close_menu();
                    }
                });

                ui.separator();
                ui.checkbox(&mut self.dark_mode, "Mode sombre");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Zoom : {:.0} %", self.zoom * 100.0));
                    ui.separator();
                    ui.label(format!(
                        "Canvas : {}×{}",
                        self.frames_width, self.frames_height
                    ));
                });
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
                ui.separator();
                ui.label(format!("Zoom : {:.0} %", self.zoom * 100.0));
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
                        for i in 0..self.preset_palette.len() {
                            let color = self.preset_palette[i];
                            let is_selected = color == self.current_color;
                            let stroke = if is_selected {
                                egui::Stroke::new(2.5, egui::Color32::from_rgb(255, 200, 0))
                            } else {
                                egui::Stroke::new(1.0, egui::Color32::from_gray(120))
                            };
                            let button = egui::Button::new("")
                                .fill(color)
                                .min_size(egui::vec2(28.0, 28.0))
                                .stroke(stroke);
                            if ui.add(button).clicked() {
                                self.current_color = color;
                                self.remember_color(color);
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
                        self.remember_color(self.custom_picker);
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
                            for i in 0..self.custom_palette.len() {
                                let color = self.custom_palette[i];
                                let is_selected = color == self.current_color;
                                let stroke = if is_selected {
                                    egui::Stroke::new(2.5, egui::Color32::from_rgb(255, 200, 0))
                                } else {
                                    egui::Stroke::new(1.0, egui::Color32::from_gray(120))
                                };
                                let resp = ui.add(
                                    egui::Button::new("")
                                        .fill(color)
                                        .min_size(egui::vec2(28.0, 28.0))
                                        .stroke(stroke),
                                );
                                if resp.clicked() {
                                    self.current_color = color;
                                    self.remember_color(color);
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

                if !self.recent_colors.is_empty() {
                    ui.add_space(8.0);
                    ui.label("Récentes :");
                    let mut clicked_color: Option<egui::Color32> = None;
                    egui::Grid::new("recent_grid")
                        .spacing([4.0, 4.0])
                        .show(ui, |ui| {
                            for (i, color) in self.recent_colors.iter().enumerate() {
                                let is_selected = *color == self.current_color;
                                let stroke = if is_selected {
                                    egui::Stroke::new(2.5, egui::Color32::from_rgb(255, 200, 0))
                                } else {
                                    egui::Stroke::new(1.0, egui::Color32::from_gray(120))
                                };
                                if ui
                                    .add(
                                        egui::Button::new("")
                                            .fill(*color)
                                            .min_size(egui::vec2(28.0, 28.0))
                                            .stroke(stroke),
                                    )
                                    .clicked()
                                {
                                    clicked_color = Some(*color);
                                }
                                if (i + 1) % 5 == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                    if let Some(c) = clicked_color {
                        self.current_color = c;
                        self.remember_color(c);
                    }
                }

                ui.separator();
                ui.label("Outil");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.tool, Tool::Brush, "Pinceau");
                    ui.selectable_value(&mut self.tool, Tool::Bucket, "Pot");
                });
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.tool, Tool::Line, "Ligne");
                    ui.selectable_value(&mut self.tool, Tool::Rect, "Carré");
                    ui.selectable_value(&mut self.tool, Tool::Circle, "Cercle");
                    ui.selectable_value(&mut self.tool, Tool::Select, "Sélectionner");
                });
                ui.separator();
                ui.label("Taille du pinceau");
                ui.add(egui::Slider::new(&mut self.brush_size, 1..=20).suffix(" px"));
                ui.horizontal(|ui| {
                    for size in [1, 2, 4, 8, 16] {
                        if ui.small_button(format!("{}", size)).clicked() {
                            self.brush_size = size;
                        }
                    }
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
                    ui.label("Largeur :");
                    ui.add(egui::Slider::new(&mut self.new_grid_width_input, 4..=4096));
                    ui.label("Hauteur :");
                    ui.add(egui::Slider::new(&mut self.new_grid_height_input, 4..=4096));

                    ui.separator();
                    ui.label("Presets :");
                    ui.horizontal_wrapped(|ui| {
                        for (label, w, h) in [
                            ("16×16", 16, 16),
                            ("32×32", 32, 32),
                            ("64×64", 64, 64),
                            ("128×128", 128, 128),
                            ("256×256", 256, 256),
                            ("512×512", 512, 512),
                            ("HD 1280×720", 1280, 720),
                            ("Full HD 1920×1080", 1920, 1080),
                        ] {
                            if ui.button(label).clicked() {
                                self.new_grid_width_input = w;
                                self.new_grid_height_input = h;
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
            if create_now {
                self.create_new_canvas(self.new_grid_width_input, self.new_grid_height_input);
                self.show_new_dialog = false;
            }
            if !keep_open {
                self.show_new_dialog = false;
            }
        }

        //Panel droite
        egui::SidePanel::right("preview_panel")
            .resizable(false)
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Aperçu");
                ui.label(format!("{}×{}", self.frames_width, self.frames_height));
                ui.separator();

                //On dessine une mini-version du canvas à taille native (1px = 1px)
                //ou avec un léger upscale si le canvas est très petit.
                let max_preview = 200.0;
                let preview_pixel = (max_preview
                    / self.frames_width.max(self.frames_height) as f32)
                    .floor()
                    .max(1.0);
                let preview_size = egui::vec2(
                    self.frames_width as f32 * preview_pixel,
                    self.frames_height as f32 * preview_pixel,
                );

                let (rect, _) = ui.allocate_exact_size(preview_size, egui::Sense::hover());
                let painter = ui.painter();
                for y in 0..self.frames_height {
                    for x in 0..self.frames_width {
                        let pos = rect.min
                            + egui::vec2(x as f32 * preview_pixel, y as f32 * preview_pixel);
                        let r = egui::Rect::from_min_size(
                            pos,
                            egui::vec2(preview_pixel, preview_pixel),
                        );
                        let c = self.frames[self.current_frame][y][x];
                        if c.a() > 0 {
                            painter.rect_filled(r, 0.0, c);
                        }
                    }
                }
            });

        // === Frise des frames (en bas, AVANT le CentralPanel) ===
        egui::TopBottomPanel::bottom("frames_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Frames");
                if ui.button("Nouvelle").clicked() {
                    self.add_frame();
                }
                if ui.button("Dupliquer").clicked() {
                    self.duplicate_frame();
                }
                if ui.button("Supprimer").clicked() {
                    self.remove_frame();
                }
                ui.separator();
                let play_label = if self.is_playing {
                    "⏸ Pause"
                } else {
                    "▶ Play"
                };
                if ui.button(play_label).clicked() {
                    self.is_playing = !self.is_playing;
                }
                ui.add(egui::Slider::new(&mut self.fps, 1..=60).text("FPS"));
            });

            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    let count = self.frames.len();
                    for i in 0..count {
                        let is_current = i == self.current_frame;
                        let label = format!("{}{}", i + 1, if is_current { " ◀" } else { "" });
                        if ui.button(label).clicked() {
                            self.current_frame = i;
                            self.texture_dirty = true; // important pour refresh le canvas
                        }
                    }
                });
            });
        });

        // === Zone centrale : canvas ===
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let max_dim = self.frames_width.max(self.frames_height) as f32;
                    let base_pixel_size = (MAX_CANVAS_SIZE / max_dim).floor().max(1.0);
                    let pixel_size = (base_pixel_size * self.zoom).max(1.0);

                    let canvas_size = egui::vec2(
                        self.frames_width as f32 * pixel_size,
                        self.frames_height as f32 * pixel_size,
                    );
                    let (response, painter) =
                        ui.allocate_painter(canvas_size, egui::Sense::click_and_drag());
                    let canvas_rect = response.rect;

                    // === Damier d'arrière-plan (pour visualiser la transparence) ===
                    // Dessiné AVANT la texture : les pixels transparents le laisseront voir.
                    // Culling sur les cellules visibles pour rester rapide même en gros canvas.
                    {
                        const CHECKER_LIGHT: egui::Color32 = egui::Color32::from_rgb(220, 220, 220);
                        const CHECKER_DARK: egui::Color32 = egui::Color32::from_rgb(180, 180, 180);

                        let visible = ui.clip_rect();
                        let sx = (((visible.min.x - canvas_rect.min.x) / pixel_size).floor() as i32)
                            .max(0) as usize;
                        let sy = (((visible.min.y - canvas_rect.min.y) / pixel_size).floor() as i32)
                            .max(0) as usize;
                        let ex = (((visible.max.x - canvas_rect.min.x) / pixel_size).ceil() as i32)
                            .max(0) as usize;
                        let ey = (((visible.max.y - canvas_rect.min.y) / pixel_size).ceil() as i32)
                            .max(0) as usize;
                        let ex = ex.min(self.frames_width);
                        let ey = ey.min(self.frames_height);

                        for y in sy..ey {
                            for x in sx..ex {
                                let p = canvas_rect.min
                                    + egui::vec2(x as f32 * pixel_size, y as f32 * pixel_size);
                                let r = egui::Rect::from_min_size(
                                    p,
                                    egui::vec2(pixel_size, pixel_size),
                                );
                                let checker = if (x + y) % 2 == 0 {
                                    CHECKER_LIGHT
                                } else {
                                    CHECKER_DARK
                                };
                                painter.rect_filled(r, 0.0, checker);
                            }
                        }
                    }

                    // 1. Rendu de la texture
                    if self.texture_dirty || self.canvas_texture.is_none() {
                        self.rebuild_canvas_texture(ctx);
                    }
                    if let Some(tex) = &self.canvas_texture {
                        painter.image(
                            tex.id(),
                            canvas_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }

                    if let Some((hx, hy)) = self.hovered_cell {
                        let hover_pos = canvas_rect.min
                            + egui::vec2(hx as f32 * pixel_size, hy as f32 * pixel_size);
                        let hover_rect = egui::Rect::from_min_size(
                            hover_pos,
                            egui::vec2(pixel_size, pixel_size),
                        );
                        //Un contour épais de la couleur active pour prévisualiser
                        painter.rect_stroke(
                            hover_rect,
                            0.0,
                            egui::Stroke::new(1.0, egui::Color32::BLACK),
                        );
                    }

                    // 2. Grille
                    if self.show_grid && pixel_size > 6.0 {
                        let visible = ui.clip_rect();
                        let sx = (((visible.min.x - canvas_rect.min.x) / pixel_size).floor() as i32)
                            .max(0) as usize;
                        let sy = (((visible.min.y - canvas_rect.min.y) / pixel_size).floor() as i32)
                            .max(0) as usize;
                        let ex = (((visible.max.x - canvas_rect.min.x) / pixel_size).ceil() as i32)
                            .max(0) as usize;
                        let ey = (((visible.max.y - canvas_rect.min.y) / pixel_size).ceil() as i32)
                            .max(0) as usize;
                        let ex = ex.min(self.frames_width);
                        let ey = ey.min(self.frames_height);

                        let stroke = egui::Stroke::new(0.5, egui::Color32::from_gray(220));
                        for y in sy..ey {
                            for x in sx..ex {
                                let p = canvas_rect.min
                                    + egui::vec2(x as f32 * pixel_size, y as f32 * pixel_size);
                                let r = egui::Rect::from_min_size(
                                    p,
                                    egui::vec2(pixel_size, pixel_size),
                                );
                                painter.rect_stroke(r, 0.0, stroke);
                            }
                        }
                    }

                    // 3. Rendu de la sélection (par-dessus la grille pour rester visible)
                    if let Some((x0, y0, x1, y1)) = self.selection {
                        let sel_pos = canvas_rect.min
                            + egui::vec2(x0 as f32 * pixel_size, y0 as f32 * pixel_size);
                        let sel_size = egui::vec2(
                            (x1 - x0 + 1) as f32 * pixel_size,
                            (y1 - y0 + 1) as f32 * pixel_size,
                        );
                        painter.rect_stroke(
                            egui::Rect::from_min_size(sel_pos, sel_size),
                            0.0,
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 150, 255)),
                        );
                    }

                    // 4. Hover detection
                    self.hovered_cell = response.hover_pos().and_then(|pos| {
                        let rel = pos - canvas_rect.min;
                        let x = (rel.x / pixel_size) as usize;
                        let y = (rel.y / pixel_size) as usize;
                        if x < self.frames_width && y < self.frames_height {
                            Some((x, y))
                        } else {
                            None
                        }
                    });

                    // 5. Pan / Peinture / Formes / Sélection
                    let middle_down = ctx.input(|i| i.pointer.middle_down());
                    if middle_down {
                        let delta = ctx.input(|i| i.pointer.delta());
                        ui.scroll_with_delta(-delta);
                        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
                        self.is_drawing = false;
                        self.last_paint_cell = None;
                    } else {
                        let drag_started = response.drag_started();
                        let drag_released = response.drag_stopped();
                        let pressed = response.is_pointer_button_down_on();

                        if let Some(pos) = response.interact_pointer_pos() {
                            let rel = pos - canvas_rect.min;
                            let cx = (rel.x / pixel_size) as usize;
                            let cy = (rel.y / pixel_size) as usize;

                            if cx < self.frames_width && cy < self.frames_height {
                                let alt_pressed = ctx.input(|i| i.modifiers.alt);
                                let right_pressed = ctx.input(|i| i.pointer.secondary_down());
                                let color = if right_pressed {
                                    egui::Color32::TRANSPARENT
                                } else {
                                    self.current_color
                                };

                                if alt_pressed && pressed {
                                    self.current_color = self.frames[self.current_frame][cy][cx];
                                    self.remember_color(self.frames[self.current_frame][cy][cx]);
                                } else {
                                    match self.tool {
                                        Tool::Brush => {
                                            if pressed {
                                                if !self.is_drawing {
                                                    self.push_history();
                                                    self.is_drawing = true;
                                                    self.last_paint_cell = None;
                                                }
                                                if let Some((px, py)) = self.last_paint_cell {
                                                    self.paint_line(px, py, cx, cy, color);
                                                }
                                                self.paint_brush(cx, cy, color);
                                                self.last_paint_cell = Some((cx, cy));
                                            }
                                        }
                                        Tool::Bucket => {
                                            if pressed && !self.is_drawing {
                                                self.push_history();
                                                self.is_drawing = true;
                                                self.flood_fill(cx, cy, color);
                                            }
                                        }
                                        Tool::Select => {
                                            if drag_started {
                                                self.shape_start = Some((cx, cy));
                                            }
                                            if drag_released {
                                                if let Some((x0, y0)) = self.shape_start.take() {
                                                    let (x_min, x_max) =
                                                        if x0 < cx { (x0, cx) } else { (cx, x0) };
                                                    let (y_min, y_max) =
                                                        if y0 < cy { (y0, cy) } else { (cy, y0) };
                                                    self.selection =
                                                        Some((x_min, y_min, x_max, y_max));
                                                    self.last_status = Some(format!(
                                                        "Sélection {}×{}",
                                                        x_max - x_min + 1,
                                                        y_max - y_min + 1
                                                    ));
                                                }
                                            }
                                        }
                                        Tool::Line | Tool::Rect | Tool::Circle => {
                                            if drag_started {
                                                self.shape_start = Some((cx, cy));
                                                self.push_history();
                                            }
                                            if drag_released {
                                                if let Some((x0, y0)) = self.shape_start.take() {
                                                    match self.tool {
                                                        Tool::Line => self.draw_line(
                                                            x0 as isize,
                                                            y0 as isize,
                                                            cx as isize,
                                                            cy as isize,
                                                            color,
                                                        ),
                                                        Tool::Rect => {
                                                            self.draw_rect(x0, y0, cx, cy, color)
                                                        }
                                                        Tool::Circle => {
                                                            let dx = cx as isize - x0 as isize;
                                                            let dy = cy as isize - y0 as isize;
                                                            let r = ((dx * dx + dy * dy) as f32)
                                                                .sqrt()
                                                                as usize;
                                                            self.draw_circle(x0, y0, r, color);
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if !pressed {
                            self.is_drawing = false;
                            self.last_paint_cell = None;
                        }
                    }
                });
        });
    }
}
