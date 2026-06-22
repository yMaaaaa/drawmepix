use eframe::egui::{self};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MAX_CANVAS_SIZE: f32 = 640.0;
const DEFAULT_GRID_SIZE: usize = 16;
const MIN_ZOOM: f32 = 0.25;
const MAX_ZOOM: f32 = 32.0;
const MAX_RECENT_COLORS: usize = 10;
const PROJECT_VERSION: u32 = 1;
const AUTOSAVE_INTERVAL_SECS: f64 = 60.0;

#[derive(PartialEq, Clone, Copy)]
enum Tool {
    Brush,
    Bucket,
    Line,
    Rect,
    Circle,
    Select,
}

#[derive(Clone)]
struct Layer {
    name: String,
    visible: bool,
    opacity: f32,
    pixels: Vec<Vec<egui::Color32>>,
}

impl Layer {
    fn new(name: String, width: usize, height: usize) -> Self {
        Self {
            name,
            visible: true,
            opacity: 1.0,
            pixels: vec![vec![egui::Color32::TRANSPARENT; width]; height],
        }
    }
}

#[derive(Clone)]
struct Frame {
    layers: Vec<Layer>,
    active_layer: usize,
}

impl Frame {
    fn new(width: usize, height: usize) -> Self {
        Self {
            layers: vec![Layer::new("Calque 1".to_string(), width, height)],
            active_layer: 0,
        }
    }

    /// Compose tous les calques visibles avec leur opacity en une seule grille.
    fn flatten(&self, width: usize, height: usize) -> Vec<Vec<egui::Color32>> {
        let mut out = vec![vec![egui::Color32::TRANSPARENT; width]; height];
        for layer in &self.layers {
            if !layer.visible {
                continue;
            }
            for y in 0..height {
                for x in 0..width {
                    let src = layer.pixels[y][x];
                    if src.a() == 0 {
                        continue;
                    }
                    let src_a = (src.a() as f32 / 255.0) * layer.opacity;
                    let dst = out[y][x];
                    let dst_a = dst.a() as f32 / 255.0;
                    let out_a = src_a + dst_a * (1.0 - src_a);
                    if out_a == 0.0 {
                        continue;
                    }
                    let blend = |s: u8, d: u8| -> u8 {
                        let s = s as f32 / 255.0;
                        let d = d as f32 / 255.0;
                        (((s * src_a + d * dst_a * (1.0 - src_a)) / out_a) * 255.0) as u8
                    };
                    out[y][x] = egui::Color32::from_rgba_unmultiplied(
                        blend(src.r(), dst.r()),
                        blend(src.g(), dst.g()),
                        blend(src.b(), dst.b()),
                        (out_a * 255.0) as u8,
                    );
                }
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Format projet sérialisable (.drawmepix)
// ---------------------------------------------------------------------------
// egui::Color32 n'implémente pas Serialize, donc on stocke les pixels en
// [u8; 4] (RGBA) et on convertit au save/load.

#[derive(Serialize, Deserialize)]
struct ProjectLayer {
    name: String,
    visible: bool,
    opacity: f32,
    pixels: Vec<Vec<[u8; 4]>>,
}

#[derive(Serialize, Deserialize)]
struct ProjectFrame {
    layers: Vec<ProjectLayer>,
    active_layer: usize,
}

#[derive(Serialize, Deserialize)]
struct Project {
    version: u32,
    width: usize,
    height: usize,
    fps: u32,
    current_frame: usize,
    frames: Vec<ProjectFrame>,
    custom_palette: Vec<[u8; 4]>,
}

#[inline]
fn color_to_array(c: egui::Color32) -> [u8; 4] {
    [c.r(), c.g(), c.b(), c.a()]
}

#[inline]
fn array_to_color(a: [u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(a[0], a[1], a[2], a[3])
}

fn preset_palette() -> Vec<egui::Color32> {
    vec![
        egui::Color32::BLACK,
        egui::Color32::from_rgb(64, 64, 64),
        egui::Color32::from_rgb(128, 128, 128),
        egui::Color32::from_rgb(192, 192, 192),
        egui::Color32::WHITE,
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
    frames: Vec<Frame>,
    current_frame: usize,
    fps: u32,
    is_playing: bool,
    last_frame_advance: f64,
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

    history: Vec<Frame>,
    redo_stack: Vec<Frame>,

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

    selection: Option<(usize, usize, usize, usize)>,
    clipboard: Option<Vec<Vec<egui::Color32>>>,

    // --- État projet / auto-save ---
    current_project_path: Option<PathBuf>,
    dirty: bool,
    last_autosave_time: f64,

    keep_selection_on_tool_change: bool,

    shape_start: Option<(usize, usize)>,
    shape_current: Option<(usize, usize)>,

    pending_zoom_focus: Option<(egui::Pos2, f32, f32)>,
    last_canvas_min: Option<egui::Pos2>,
    scroll_offset: egui::Vec2,

    renaming_layer: Option<usize>,
}

impl Default for DrawMePixApp {
    fn default() -> Self {
        Self {
            frames: vec![Frame::new(DEFAULT_GRID_SIZE, DEFAULT_GRID_SIZE)],
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
            selection: None,
            clipboard: None,
            current_project_path: None,
            dirty: false,
            last_autosave_time: 0.0,
            keep_selection_on_tool_change: false,
            shape_start: None,
            shape_current: None,
            pending_zoom_focus: None,
            last_canvas_min: None,
            scroll_offset: egui::Vec2::ZERO,
            renaming_layer: None,
        }
    }
}

impl DrawMePixApp {
    fn create_new_canvas(&mut self, width: usize, height: usize) {
        self.push_history();
        self.frames_width = width.clamp(4, 4096);
        self.frames_height = height.clamp(4, 4096);
        self.frames = vec![Frame::new(self.frames_width, self.frames_height)];
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
        let w = self.frames_width;
        let h = self.frames_height;
        let cf = self.current_frame;
        let frame = &mut self.frames[cf];
        let al = frame.active_layer;
        frame.layers[al].pixels = vec![vec![egui::Color32::TRANSPARENT; w]; h];
        self.texture_dirty = true;
        self.last_status = Some("Calque effacé".to_string());
    }

    fn save_png(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let flat = self.frames[self.current_frame].flatten(self.frames_width, self.frames_height);
        let w = self.frames_width as u32;
        let h = self.frames_height as u32;
        let mut img = image::RgbaImage::new(w, h);
        for y in 0..self.frames_height {
            for x in 0..self.frames_width {
                let c = flat[y][x];
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
                if x >= 0
                    && y >= 0
                    && (x as usize) < self.frames_width
                    && (y as usize) < self.frames_height
                {
                    self.paint_pixel(x as usize, y as usize, color);
                }
            }
        }
    }

    fn flood_fill(&mut self, start_x: usize, start_y: usize, new_color: egui::Color32) {
        let w = self.frames_width;
        let h = self.frames_height;
        let cf = self.current_frame;
        let frame = &mut self.frames[cf];
        let al = frame.active_layer;
        let pixels = &mut frame.layers[al].pixels;

        let source_color = pixels[start_y][start_x];
        if source_color == new_color {
            return;
        }

        let mut queue = std::collections::VecDeque::new();
        queue.push_back((start_x, start_y));

        while let Some((x, y)) = queue.pop_front() {
            if pixels[y][x] != source_color {
                continue;
            }
            pixels[y][x] = new_color;
            if x > 0 {
                queue.push_back((x - 1, y));
            }
            if x < w - 1 {
                queue.push_back((x + 1, y));
            }
            if y > 0 {
                queue.push_back((x, y - 1));
            }
            if y < h - 1 {
                queue.push_back((x, y + 1));
            }
        }
        self.texture_dirty = true;
    }

    fn load_png(&mut self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let img = image::open(path)?.to_rgba8();
        let w = img.width() as usize;
        let h = img.height() as usize;

        let mut layer_pixels = vec![vec![egui::Color32::TRANSPARENT; w]; h];
        for y in 0..h {
            for x in 0..w {
                let p = img.get_pixel(x as u32, y as u32);
                layer_pixels[y][x] = egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]);
            }
        }

        let mut layer = Layer::new("Calque 1".to_string(), w, h);
        layer.pixels = layer_pixels;
        let frame = Frame {
            layers: vec![layer],
            active_layer: 0,
        };

        self.frames_width = w;
        self.frames_height = h;
        self.frames = vec![frame];
        self.current_frame = 0;
        self.history.clear();
        self.redo_stack.clear();
        self.zoom = 1.0;
        self.hovered_cell = None;
        self.texture_dirty = true;
        Ok(())
    }

    fn paint_pixel(&mut self, x: usize, y: usize, color: egui::Color32) {
        if x >= self.frames_width || y >= self.frames_height {
            return;
        }
        let w = self.frames_width;
        let h = self.frames_height;
        let mh = self.mirror_horizontal;
        let mv = self.mirror_vertical;
        let cf = self.current_frame;
        let frame = &mut self.frames[cf];
        let al = frame.active_layer;
        let pixels = &mut frame.layers[al].pixels;

        pixels[y][x] = color;
        if mh {
            pixels[y][w - 1 - x] = color;
        }
        if mv {
            pixels[h - 1 - y][x] = color;
        }
        if mh && mv {
            pixels[h - 1 - y][w - 1 - x] = color;
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
        self.dirty = true;
    }

    fn undo(&mut self) {
        if let Some(previous) = self.history.pop() {
            self.redo_stack
                .push(self.frames[self.current_frame].clone());
            self.frames[self.current_frame] = previous;
            self.texture_dirty = true;
            self.dirty = true;
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.history.push(self.frames[self.current_frame].clone());
            self.frames[self.current_frame] = next;
            self.texture_dirty = true;
            self.dirty = true;
        }
    }

    fn rebuild_canvas_texture(&mut self, ctx: &egui::Context) {
        let flat = self.frames[self.current_frame].flatten(self.frames_width, self.frames_height);
        let mut pixels = Vec::with_capacity(self.frames_width * self.frames_height);
        for y in 0..self.frames_height {
            for x in 0..self.frames_width {
                pixels.push(flat[y][x]);
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
        self.custom_picker = color;
    }

    fn draw_line(&mut self, x0: isize, y0: isize, x1: isize, y1: isize, color: egui::Color32) {
        for (x, y) in Self::bresenham_pixels(x0, y0, x1, y1) {
            self.paint_pixel(x, y, color);
        }
    }

    fn draw_rect(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, color: egui::Color32) {
        for (x, y) in Self::rect_pixels(x0, y0, x1, y1) {
            self.paint_pixel(x, y, color);
        }
    }

    fn draw_circle(&mut self, cx: usize, cy: usize, r: usize, color: egui::Color32) {
        for (x, y) in Self::circle_pixels(cx as isize, cy as isize, r as isize) {
            self.paint_pixel(x, y, color);
        }
    }

    fn copy_selection(&mut self) {
        if let Some((x0, y0, x1, y1)) = self.selection {
            let w = x1 - x0 + 1;
            let h = y1 - y0 + 1;
            let mut buf = vec![vec![egui::Color32::TRANSPARENT; w]; h];
            let frame = &self.frames[self.current_frame];
            let pixels = &frame.layers[frame.active_layer].pixels;
            for dy in 0..h {
                for dx in 0..w {
                    buf[dy][dx] = pixels[y0 + dy][x0 + dx];
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
            let fw = self.frames_width;
            let fh = self.frames_height;
            let cf = self.current_frame;
            let frame = &mut self.frames[cf];
            let al = frame.active_layer;
            let pixels = &mut frame.layers[al].pixels;
            for y in 0..h {
                for x in 0..w {
                    let gx = dx + x;
                    let gy = dy + y;
                    if gx < fw && gy < fh {
                        pixels[gy][gx] = buf[y][x];
                    }
                }
            }
            self.texture_dirty = true;
            self.last_status = Some("Collé".to_string());
        }
    }

    fn add_frame(&mut self) {
        self.push_history();
        let new = Frame::new(self.frames_width, self.frames_height);
        self.frames.insert(self.current_frame + 1, new);
        self.current_frame += 1;
        self.texture_dirty = true;
    }

    fn duplicate_frame(&mut self) {
        self.push_history();
        let copy = self.frames[self.current_frame].clone();
        self.frames.insert(self.current_frame + 1, copy);
        self.current_frame += 1;
        self.texture_dirty = true;
    }

    fn remove_frame(&mut self) {
        if self.frames.len() > 1 {
            self.push_history();
            self.frames.remove(self.current_frame);
            if self.current_frame >= self.frames.len() {
                self.current_frame = self.frames.len() - 1;
            }
            self.texture_dirty = true;
        }
    }

    fn add_layer(&mut self) {
        self.push_history();
        let w = self.frames_width;
        let h = self.frames_height;
        let cf = self.current_frame;
        let frame = &mut self.frames[cf];
        let name = format!("Calque {}", frame.layers.len() + 1);
        frame.layers.push(Layer::new(name, w, h));
        frame.active_layer = frame.layers.len() - 1;
        self.texture_dirty = true;
    }

    fn remove_layer(&mut self, idx: usize) {
        let cf = self.current_frame;
        if self.frames[cf].layers.len() <= 1 {
            return;
        }
        self.push_history();
        let frame = &mut self.frames[cf];
        frame.layers.remove(idx);
        if frame.active_layer >= frame.layers.len() {
            frame.active_layer = frame.layers.len() - 1;
        }
        self.texture_dirty = true;
    }

    fn save_gif(&mut self, path: &std::path::Path) {
        use std::fs::File;
        let width = self.frames_width as u16;
        let height = self.frames_height as u16;
        let delay = (100.0 / self.fps as f32).max(1.0) as u16;

        let file = match File::create(path) {
            Ok(f) => f,
            Err(e) => {
                self.last_status = Some(format!("Erreur création fichier : {}", e));
                return;
            }
        };

        let mut encoder = match gif::Encoder::new(file, width, height, &[]) {
            Ok(e) => e,
            Err(e) => {
                self.last_status = Some(format!("Erreur encoder : {}", e));
                return;
            }
        };

        if let Err(e) = encoder.set_repeat(gif::Repeat::Infinite) {
            self.last_status = Some(format!("Erreur set_repeat : {}", e));
            return;
        }

        for frame in &self.frames {
            let flat = frame.flatten(self.frames_width, self.frames_height);
            let mut rgba: Vec<u8> = Vec::with_capacity(self.frames_width * self.frames_height * 4);
            for y in 0..self.frames_height {
                for x in 0..self.frames_width {
                    let c = flat[y][x];
                    rgba.push(c.r());
                    rgba.push(c.g());
                    rgba.push(c.b());
                    rgba.push(c.a());
                }
            }
            let mut gif_frame = gif::Frame::from_rgba_speed(width, height, &mut rgba, 10);
            gif_frame.delay = delay;
            gif_frame.dispose = gif::DisposalMethod::Background;
            if let Err(e) = encoder.write_frame(&gif_frame) {
                self.last_status = Some(format!("Erreur écriture : {}", e));
                return;
            }
        }
        self.last_status = Some(format!("GIF sauvegardé : {}", path.display()));
    }

    fn set_tool(&mut self, new_tool: Tool) {
        if new_tool != self.tool && new_tool != Tool::Select {
            if !self.keep_selection_on_tool_change {
                self.selection = None;
            }
        }
        self.tool = new_tool;
    }

    fn cut_selection(&mut self) {
        self.copy_selection();
        if let Some((x0, y0, x1, y1)) = self.selection {
            self.push_history();
            let cf = self.current_frame;
            let frame = &mut self.frames[cf];
            let al = frame.active_layer;
            let pixels = &mut frame.layers[al].pixels;
            for y in y0..=y1 {
                for x in x0..=x1 {
                    pixels[y][x] = egui::Color32::TRANSPARENT;
                }
            }
            self.texture_dirty = true;
            self.last_status = Some("Coupé".to_string());
        }
    }

    fn bresenham_pixels(x0: isize, y0: isize, x1: isize, y1: isize) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut x, mut y) = (x0, y0);
        loop {
            if x >= 0 && y >= 0 {
                out.push((x as usize, y as usize));
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
        out
    }

    fn rect_pixels(x0: usize, y0: usize, x1: usize, y1: usize) -> Vec<(usize, usize)> {
        let (x_min, x_max) = if x0 < x1 { (x0, x1) } else { (x1, x0) };
        let (y_min, y_max) = if y0 < y1 { (y0, y1) } else { (y1, y0) };
        let mut out = Vec::new();
        for x in x_min..=x_max {
            out.push((x, y_min));
            out.push((x, y_max));
        }
        for y in y_min..=y_max {
            out.push((x_min, y));
            out.push((x_max, y));
        }
        out
    }

    fn circle_pixels(cx: isize, cy: isize, r: isize) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
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
                    out.push((px as usize, py as usize));
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
        out
    }

    fn duplicate_layer(&mut self, idx: usize) {
        self.push_history();
        let cf = self.current_frame;
        let frame = &mut self.frames[cf];
        let mut copy = frame.layers[idx].clone();
        copy.name = format!("{} (copie)", copy.name);
        frame.layers.insert(idx + 1, copy);
        frame.active_layer = idx + 1;
        self.texture_dirty = true;
    }

    fn move_layer_up(&mut self, idx: usize) {
        let cf = self.current_frame;
        if idx + 1 < self.frames[cf].layers.len() {
            self.push_history();
            let frame = &mut self.frames[cf];
            frame.layers.swap(idx, idx + 1);
            if frame.active_layer == idx {
                frame.active_layer = idx + 1;
            } else if frame.active_layer == idx + 1 {
                frame.active_layer = idx;
            }
            self.texture_dirty = true;
        }
    }

    fn move_layer_down(&mut self, idx: usize) {
        if idx > 0 {
            self.push_history();
            let frame = &mut self.frames[self.current_frame];
            frame.layers.swap(idx, idx - 1);
            if frame.active_layer == idx {
                frame.active_layer = idx - 1;
            } else if frame.active_layer == idx - 1 {
                frame.active_layer = idx;
            }
            self.texture_dirty = true;
        }
    }

    // -----------------------------------------------------------------------
    // Format projet .drawmepix + auto-save
    // -----------------------------------------------------------------------

    fn to_project(&self) -> Project {
        Project {
            version: PROJECT_VERSION,
            width: self.frames_width,
            height: self.frames_height,
            fps: self.fps,
            current_frame: self.current_frame,
            frames: self
                .frames
                .iter()
                .map(|f| ProjectFrame {
                    active_layer: f.active_layer,
                    layers: f
                        .layers
                        .iter()
                        .map(|l| ProjectLayer {
                            name: l.name.clone(),
                            visible: l.visible,
                            opacity: l.opacity,
                            pixels: l
                                .pixels
                                .iter()
                                .map(|row| row.iter().map(|c| color_to_array(*c)).collect())
                                .collect(),
                        })
                        .collect(),
                })
                .collect(),
            custom_palette: self
                .custom_palette
                .iter()
                .map(|c| color_to_array(*c))
                .collect(),
        }
    }

    fn from_project(&mut self, p: Project) {
        self.frames_width = p.width;
        self.frames_height = p.height;
        self.fps = p.fps;
        self.frames = p
            .frames
            .into_iter()
            .map(|pf| Frame {
                active_layer: pf.active_layer,
                layers: pf
                    .layers
                    .into_iter()
                    .map(|pl| Layer {
                        name: pl.name,
                        visible: pl.visible,
                        opacity: pl.opacity,
                        pixels: pl
                            .pixels
                            .into_iter()
                            .map(|row| row.into_iter().map(array_to_color).collect())
                            .collect(),
                    })
                    .collect(),
            })
            .collect();
        self.current_frame = p.current_frame.min(self.frames.len().saturating_sub(1));
        self.custom_palette = p.custom_palette.into_iter().map(array_to_color).collect();
        self.history.clear();
        self.redo_stack.clear();
        self.zoom = 1.0;
        self.hovered_cell = None;
        self.texture_dirty = true;
        self.dirty = false;
    }

    fn save_project(&mut self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let project = self.to_project();
        let bytes = bincode::serialize(&project)?;
        std::fs::write(path, bytes)?;
        self.current_project_path = Some(path.to_path_buf());
        self.dirty = false;
        Ok(())
    }

    fn load_project(&mut self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let bytes = std::fs::read(path)?;
        let project: Project = bincode::deserialize(&bytes)?;
        if project.version != PROJECT_VERSION {
            return Err(format!(
                "Version de projet non supportée : {} (attendu {})",
                project.version, PROJECT_VERSION
            )
            .into());
        }
        self.from_project(project);
        self.current_project_path = Some(path.to_path_buf());
        Ok(())
    }

    fn autosave_path(&self) -> PathBuf {
        match &self.current_project_path {
            Some(p) => {
                let mut s = p.clone();
                let name = format!(
                    "{}.autosave",
                    p.file_name().and_then(|n| n.to_str()).unwrap_or("project")
                );
                s.set_file_name(name);
                s
            }
            None => std::env::temp_dir().join("drawmepix_autosave.drawmepix"),
        }
    }

    fn autosave(&mut self) {
        let path = self.autosave_path();
        let project = self.to_project();
        match bincode::serialize(&project) {
            Ok(bytes) => {
                if let Err(e) = std::fs::write(&path, bytes) {
                    self.last_status = Some(format!("Auto-save KO : {}", e));
                } else {
                    self.last_status = Some(format!("Auto-save : {}", path.display()));
                }
            }
            Err(e) => {
                self.last_status = Some(format!("Auto-save KO : {}", e));
            }
        }
    }
}

impl eframe::App for DrawMePixApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // === Auto-save toutes les AUTOSAVE_INTERVAL_SECS si modifs ===
        let now_time = ctx.input(|i| i.time);
        if self.dirty && now_time - self.last_autosave_time >= AUTOSAVE_INTERVAL_SECS {
            self.autosave();
            self.last_autosave_time = now_time;
        }

        // === Lecture automatique des frames ===
        if self.is_playing && self.frames.len() > 1 {
            let now = ctx.input(|i| i.time);
            let interval = 1.0 / self.fps as f64;
            if now - self.last_frame_advance >= interval {
                self.current_frame = (self.current_frame + 1) % self.frames.len();
                self.last_frame_advance = now;
                self.texture_dirty = true;
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

        let copy_pressed = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::C));
        if copy_pressed {
            self.copy_selection();
        }

        let paste_pressed =
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::V));
        if paste_pressed {
            let (dx, dy) = self.selection.map(|s| (s.0, s.1)).unwrap_or((0, 0));
            self.paste_at(dx, dy);
        }

        let select_all_pressed =
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::A));
        if select_all_pressed {
            self.selection = Some((0, 0, self.frames_width - 1, self.frames_height - 1));
        }

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

        let cursor_pos = ctx.input(|i| i.pointer.hover_pos());

        // Pinch trackpad
        let zoom_delta = ctx.input(|i| i.zoom_delta());
        if (zoom_delta - 1.0).abs() > 0.001 {
            let old_zoom = self.zoom;
            let new_zoom = (self.zoom * zoom_delta).clamp(MIN_ZOOM, MAX_ZOOM);
            if let Some(mouse) = cursor_pos {
                self.pending_zoom_focus = Some((mouse, old_zoom, new_zoom));
            }
            self.zoom = new_zoom;
        }

        // Cmd + molette
        let (cmd_down, scroll_y) = ctx.input(|i| (i.modifiers.command, i.raw_scroll_delta.y));
        if cmd_down && scroll_y.abs() > 0.1 {
            let old_zoom = self.zoom;
            let factor = (scroll_y * 0.005).exp();
            let new_zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
            if let Some(mouse) = cursor_pos {
                self.pending_zoom_focus = Some((mouse, old_zoom, new_zoom));
                println!("zoom: mouse={:?} old={} new={}", mouse, old_zoom, new_zoom);
            } else {
                println!("zoom sans curseur");
            }
            self.zoom = new_zoom;
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
                    if ui.button("Exporter en GIF…").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("GIF animé", &["gif"])
                            .set_file_name("animation.gif")
                            .save_file()
                        {
                            self.save_gif(&path);
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Sauvegarder le projet… (.drawmepix)").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Projet DrawMePix", &["drawmepix"])
                            .set_file_name("projet.drawmepix")
                            .save_file()
                        {
                            match self.save_project(&path) {
                                Ok(()) => {
                                    self.last_status =
                                        Some(format!("Projet sauvegardé : {}", path.display()));
                                }
                                Err(e) => {
                                    self.last_status =
                                        Some(format!("Erreur sauvegarde projet : {}", e));
                                }
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("Ouvrir un projet… (.drawmepix)").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Projet DrawMePix", &["drawmepix"])
                            .pick_file()
                        {
                            match self.load_project(&path) {
                                Ok(()) => {
                                    self.last_status =
                                        Some(format!("Projet chargé : {}", path.display()));
                                }
                                Err(e) => {
                                    self.last_status =
                                        Some(format!("Erreur ouverture projet : {}", e));
                                }
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
                    ui.checkbox(&mut self.mirror_horizontal, "Miroir vertical");
                    ui.checkbox(&mut self.mirror_vertical, "Miroir horizontal");
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
                    ui.separator();
                    ui.checkbox(
                        &mut self.keep_selection_on_tool_change,
                        "Garder la sélection en changeant d'outil",
                    );
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
                    if ui.button("Effacer le calque").clicked() {
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
                ui.label("Boutons");
                ui.horizontal(|ui| {
                    let has_sel = self.selection.is_some();
                    let has_clip = self.clipboard.is_some();
                    if ui
                        .add_enabled(has_sel, egui::Button::new("Couper"))
                        .clicked()
                    {
                        self.cut_selection();
                    }
                    if ui
                        .add_enabled(has_sel, egui::Button::new("Copier"))
                        .clicked()
                    {
                        self.copy_selection();
                    }
                    if ui
                        .add_enabled(has_clip, egui::Button::new("Coller"))
                        .clicked()
                    {
                        let (dx, dy) = self.selection.map(|s| (s.0, s.1)).unwrap_or((0, 0));
                        self.paste_at(dx, dy);
                    }
                });

                ui.separator();
                ui.label("Outil");
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(self.tool == Tool::Brush, "Pinceau")
                        .clicked()
                    {
                        self.set_tool(Tool::Brush);
                    }
                    if ui
                        .selectable_label(self.tool == Tool::Bucket, "Pot")
                        .clicked()
                    {
                        self.set_tool(Tool::Bucket);
                    }
                });
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(self.tool == Tool::Line, "Ligne")
                        .clicked()
                    {
                        self.set_tool(Tool::Line);
                    }
                    if ui
                        .selectable_label(self.tool == Tool::Rect, "Carré")
                        .clicked()
                    {
                        self.set_tool(Tool::Rect);
                    }
                    if ui
                        .selectable_label(self.tool == Tool::Circle, "Cercle")
                        .clicked()
                    {
                        self.set_tool(Tool::Circle);
                    }
                    if ui
                        .selectable_label(self.tool == Tool::Select, "Sélectionner")
                        .clicked()
                    {
                        self.set_tool(Tool::Select);
                    }
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
                self.selection = None;
            }
            if !keep_open {
                self.show_new_dialog = false;
            }
        }

        // === Panneau droite : Calques (rightmost) ===
        egui::SidePanel::right("layers_panel")
            .resizable(false)
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Calques");
                if ui.button("➕ Nouveau calque").clicked() {
                    self.add_layer();
                }
                ui.separator();

                let mut dirty = false;
                let mut to_delete: Option<usize> = None;
                let mut to_duplicate: Option<usize> = None;
                let mut to_move_up: Option<usize> = None;
                let mut to_move_down: Option<usize> = None;
                let mut new_active: Option<usize> = None;

                let frame_idx = self.current_frame;
                let active_layer = self.frames[frame_idx].active_layer;
                let layer_count = self.frames[frame_idx].layers.len();

                for i in (0..layer_count).rev() {
                    ui.horizontal(|ui| {
                        if ui
                            .checkbox(&mut self.frames[frame_idx].layers[i].visible, "")
                            .changed()
                        {
                            dirty = true;
                        }
                        let is_active = i == active_layer;
                        let name_resp = if self.renaming_layer == Some(i) {
                            let r = ui.add(
                                egui::TextEdit::singleline(
                                    &mut self.frames[frame_idx].layers[i].name,
                                )
                                .desired_width(100.0),
                            );
                            if r.lost_focus() || ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
                                self.renaming_layer = None;
                            }
                            r
                        } else {
                            let name = self.frames[frame_idx].layers[i].name.clone();
                            let r = ui.selectable_label(is_active, &name);
                            if r.clicked() {
                                new_active = Some(i);
                            }
                            if r.double_clicked() {
                                self.renaming_layer = Some(i);
                            }
                            r
                        };
                        let _ = name_resp;
                        if ui.small_button("Dupliquer").clicked() {
                            to_duplicate = Some(i);
                        }
                        // Boutons monter / descendre
                        if i < layer_count - 1 && ui.small_button("⬆").clicked() {
                            to_move_up = Some(i);
                        }
                        if i > 0 && ui.small_button("⬇").clicked() {
                            to_move_down = Some(i);
                        }

                        if layer_count > 1 && ui.small_button("🗑").clicked() {
                            to_delete = Some(i);
                        }
                    });
                    if ui
                        .add(
                            egui::Slider::new(
                                &mut self.frames[frame_idx].layers[i].opacity,
                                0.0..=1.0,
                            )
                            .text("Opacité"),
                        )
                        .changed()
                    {
                        dirty = true;
                    }
                    ui.separator();
                }

                if let Some(idx) = new_active {
                    self.frames[frame_idx].active_layer = idx;
                }
                if let Some(idx) = to_delete {
                    self.remove_layer(idx);
                    dirty = true;
                }
                if let Some(idx) = to_duplicate {
                    self.duplicate_layer(idx);
                    dirty = true;
                }
                if let Some(idx) = to_move_up {
                    self.move_layer_up(idx);
                    dirty = true;
                }
                if let Some(idx) = to_move_down {
                    self.move_layer_down(idx);
                    dirty = true;
                }
                if dirty {
                    self.texture_dirty = true;
                    self.dirty = true;
                }
            });

        // === Panneau droite : Aperçu (à gauche des calques) ===
        egui::SidePanel::right("preview_panel")
            .resizable(false)
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Aperçu");
                ui.label(format!("{}×{}", self.frames_width, self.frames_height));
                ui.separator();

                // On réutilise la texture du canvas (déjà composite des calques)
                // et on la dessine en mini via un seul appel GPU.
                // Beaucoup plus rapide que d'itérer W×H pixels à la main.
                let max_preview = 200.0;
                let scale = (max_preview / self.frames_width.max(self.frames_height) as f32)
                    .min(max_preview / self.frames_width.max(self.frames_height) as f32);
                let preview_w = self.frames_width as f32 * scale;
                let preview_h = self.frames_height as f32 * scale;
                let preview_size = egui::vec2(preview_w, preview_h);

                let (rect, _) = ui.allocate_exact_size(preview_size, egui::Sense::hover());
                let painter = ui.painter();

                // Damier d'arrière-plan (pour visualiser la transparence)
                const CHECKER_LIGHT: egui::Color32 = egui::Color32::from_rgb(220, 220, 220);
                const CHECKER_DARK: egui::Color32 = egui::Color32::from_rgb(180, 180, 180);
                let checker_size = 8.0_f32.min(scale.max(2.0));
                let mut cy = 0;
                let mut y = rect.min.y;
                while y < rect.max.y {
                    let mut cx = 0;
                    let mut x = rect.min.x;
                    while x < rect.max.x {
                        let r = egui::Rect::from_min_size(
                            egui::pos2(x, y),
                            egui::vec2(
                                checker_size.min(rect.max.x - x),
                                checker_size.min(rect.max.y - y),
                            ),
                        );
                        let c = if (cx + cy) % 2 == 0 {
                            CHECKER_LIGHT
                        } else {
                            CHECKER_DARK
                        };
                        painter.rect_filled(r, 0.0, c);
                        x += checker_size;
                        cx += 1;
                    }
                    y += checker_size;
                    cy += 1;
                }

                // Image du canvas (composite des calques) en un seul appel
                if let Some(tex) = &self.canvas_texture {
                    painter.image(
                        tex.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            });

        // === Frise des frames (en bas) ===
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
                            self.texture_dirty = true;
                        }
                    }
                });
            });
        });

        // === Zone centrale : canvas ===
        egui::CentralPanel::default().show(ctx, |ui| {
            // Compensation du zoom AVANT la ScrollArea
            let mut force_offset = false;
            if let Some((mouse, old_z, new_z)) = self.pending_zoom_focus.take() {
                if let Some(canvas_min) = self.last_canvas_min {
                    let scale = new_z / old_z;
                    let rel = mouse - canvas_min;
                    let delta = rel * (scale - 1.0);
                    self.scroll_offset += delta;
                    force_offset = true;
                }
            }

            let mut scroll_area = egui::ScrollArea::both().auto_shrink([false; 2]);
            if force_offset {
                scroll_area = scroll_area.scroll_offset(self.scroll_offset);
            }

            let output = scroll_area.show(ui, |ui| {
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

                self.last_canvas_min = Some(canvas_rect.min);

                // Damier d'arrière-plan
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
                            let r =
                                egui::Rect::from_min_size(p, egui::vec2(pixel_size, pixel_size));
                            let checker = if (x + y) % 2 == 0 {
                                CHECKER_LIGHT
                            } else {
                                CHECKER_DARK
                            };
                            painter.rect_filled(r, 0.0, checker);
                        }
                    }
                }

                // Texture (composite des calques visibles)
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

                // Preview live des formes (Line / Rect / Circle)
                if let (Some(start), Some(current)) = (self.shape_start, self.shape_current) {
                    let pixels: Vec<(usize, usize)> = match self.tool {
                        Tool::Line => Self::bresenham_pixels(
                            start.0 as isize,
                            start.1 as isize,
                            current.0 as isize,
                            current.1 as isize,
                        ),
                        Tool::Rect => Self::rect_pixels(start.0, start.1, current.0, current.1),
                        Tool::Circle => {
                            let dx = current.0 as isize - start.0 as isize;
                            let dy = current.1 as isize - start.1 as isize;
                            let r = ((dx * dx + dy * dy) as f32).sqrt() as isize;
                            Self::circle_pixels(start.0 as isize, start.1 as isize, r)
                        }
                        _ => vec![],
                    };
                    let preview_color = self.current_color.gamma_multiply(0.6);
                    for (px, py) in pixels {
                        if px >= self.frames_width || py >= self.frames_height {
                            continue;
                        }
                        let p = canvas_rect.min
                            + egui::vec2(px as f32 * pixel_size, py as f32 * pixel_size);
                        let r = egui::Rect::from_min_size(p, egui::vec2(pixel_size, pixel_size));
                        painter.rect_filled(r, 0.0, preview_color);
                    }
                }

                // Hover
                if let Some((hx, hy)) = self.hovered_cell {
                    let hover_pos = canvas_rect.min
                        + egui::vec2(hx as f32 * pixel_size, hy as f32 * pixel_size);
                    let hover_rect =
                        egui::Rect::from_min_size(hover_pos, egui::vec2(pixel_size, pixel_size));
                    painter.rect_stroke(
                        hover_rect,
                        0.0,
                        egui::Stroke::new(1.0, egui::Color32::BLACK),
                    );
                }

                // Grille
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
                            let r =
                                egui::Rect::from_min_size(p, egui::vec2(pixel_size, pixel_size));
                            painter.rect_stroke(r, 0.0, stroke);
                        }
                    }
                }

                // Sélection
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

                // Hover detection
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

                // Pan / Peinture / Formes / Sélection
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

                            if right_pressed && self.tool == Tool::Brush {
                                ctx.set_cursor_icon(egui::CursorIcon::NotAllowed);
                            }

                            if alt_pressed && pressed {
                                let flat = self.frames[self.current_frame]
                                    .flatten(self.frames_width, self.frames_height);
                                let c = flat[cy][cx];
                                self.current_color = c;
                                self.remember_color(c);
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
                                                self.selection = Some((x_min, y_min, x_max, y_max));
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
                                        if pressed {
                                            self.shape_current = Some((cx, cy));
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
                                                        let r = ((dx * dx + dy * dy) as f32).sqrt()
                                                            as usize;
                                                        self.draw_circle(x0, y0, r, color);
                                                    }
                                                    _ => {}
                                                }
                                                self.shape_current = None;
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

            self.scroll_offset = output.state.offset;
        });
    }
}
