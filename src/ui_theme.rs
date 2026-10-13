//! Shared monochrome native UI, bundled Vietnamese font and thinking feedback.
use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, FontId, Pos2, Response, Sense, Stroke,
    Vec2,
    text::{LayoutJob, TextFormat},
};

pub const PANEL: Color32 = Color32::from_rgb(23, 23, 25);
pub const SIDEBAR: Color32 = Color32::from_rgb(29, 29, 31);
pub const SURFACE: Color32 = Color32::from_rgb(38, 38, 41);
pub const BORDER: Color32 = Color32::from_rgb(65, 65, 69);
pub const TEXT: Color32 = Color32::from_rgb(242, 242, 243);
pub const MUTED: Color32 = Color32::from_rgb(168, 168, 174);

pub fn install(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "noto-vietnamese".into(),
        FontData::from_static(include_bytes!("../assets/fonts/NotoSans-Regular.ttf")).into(),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "noto-vietnamese".into());
    }
    ctx.set_fonts(fonts);
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    style.spacing.button_padding = Vec2::new(12.0, 7.0);
    style.visuals.panel_fill = PANEL;
    style.visuals.window_fill = PANEL;
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.extreme_bg_color = SIDEBAR;
    style.visuals.selection.bg_fill = Color32::from_gray(65);
    style.visuals.selection.stroke = Stroke::new(1.0, TEXT);
    for widgets in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
    ] {
        widgets.corner_radius = 8.into();
        widgets.bg_stroke = Stroke::new(1.0, BORDER);
    }
    style.visuals.widgets.inactive.bg_fill = SURFACE;
    style.visuals.widgets.hovered.bg_fill = Color32::from_gray(52);
    style.visuals.widgets.active.bg_fill = Color32::from_gray(62);
    style
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, FontId::proportional(13.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, FontId::proportional(11.0));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, FontId::proportional(22.0));
    ctx.set_style_of(egui::Theme::Dark, style);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum Glyph {
    Search,
    Plus,
    Close,
    Minimize,
    Maximize,
    Restore,
    Arrow,
    Spark,
    Folder,
    File,
    Attach,
    Code,
    Book,
    Check,
    Pin,
    Tag,
    Camera,
    Stop,
    Settings,
    Refresh,
    Undo,
    Save,
    Font,
    Align,
    Scope,
    Word,
    Excel,
    Presentation,
}

pub const NOTE_ICONS: &[(&str, Glyph, &str)] = &[
    ("file", Glyph::File, "Văn bản"),
    ("folder", Glyph::Folder, "Thư mục"),
    ("code", Glyph::Code, "Lập trình"),
    ("book", Glyph::Book, "Giáo án / Sách"),
    ("spark", Glyph::Spark, "Ý tưởng"),
    ("check", Glyph::Check, "Công việc"),
    ("pin", Glyph::Pin, "Ghim quan trọng"),
    ("tag", Glyph::Tag, "Nhãn phân loại"),
];

pub fn glyph_from_name(name: &str) -> Glyph {
    match name {
        "folder" => Glyph::Folder,
        "code" => Glyph::Code,
        "book" => Glyph::Book,
        "spark" => Glyph::Spark,
        "check" => Glyph::Check,
        "pin" => Glyph::Pin,
        "tag" => Glyph::Tag,
        _ => Glyph::File,
    }
}

pub fn icon(ui: &mut egui::Ui, kind: Glyph, label: &str, enabled: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::click());
    let mut color = if enabled { TEXT } else { MUTED };
    if response.hovered() && enabled {
        ui.painter().rect_filled(rect, 16, SURFACE);
    }
    if matches!(kind, Glyph::Arrow | Glyph::Plus) {
        let fill = if matches!(kind, Glyph::Arrow) && enabled {
            TEXT
        } else {
            SURFACE
        };
        ui.painter().circle_filled(rect.center(), 15.0, fill);
        if matches!(kind, Glyph::Arrow) && enabled {
            color = PANEL;
        }
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    draw_icon(ui.painter(), rect.center(), kind, color);
    response.on_hover_text(label)
}

pub fn draw_icon(p: &egui::Painter, c: Pos2, kind: Glyph, color: Color32) {
    let s = Stroke::new(1.5, color);
    let line = |a: (f32, f32), b: (f32, f32)| {
        p.line_segment([c + Vec2::new(a.0, a.1), c + Vec2::new(b.0, b.1)], s);
    };
    match kind {
        Glyph::Settings => {
            p.circle_stroke(c, 6.0, s);
            p.circle_stroke(c, 2.0, s);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let v = Vec2::angled(a);
                p.line_segment([c + v * 6.0, c + v * 9.0], s);
            }
        }
        Glyph::Refresh => {
            p.add(egui::Shape::line(
                (0..20)
                    .map(|i| {
                        let a = i as f32 * 0.25 + 0.4;
                        c + Vec2::angled(a) * 7.0
                    })
                    .collect(),
                s,
            ));
            line((7.0, -2.0), (7.0, 3.0));
            line((7.0, 3.0), (2.0, 3.0));
        }
        Glyph::Undo => {
            line((-7.0, -3.0), (5.0, -3.0));
            line((-7.0, -3.0), (-3.0, -7.0));
            line((-7.0, -3.0), (-3.0, 1.0));
            line((5.0, -3.0), (7.0, 2.0));
            line((7.0, 2.0), (3.0, 6.0));
            line((3.0, 6.0), (-3.0, 6.0));
        }
        Glyph::Save => {
            p.rect_stroke(
                egui::Rect::from_center_size(c, Vec2::splat(14.0)),
                1,
                s,
                egui::StrokeKind::Inside,
            );
            line((-3.0, -7.0), (-3.0, -1.0));
            line((-3.0, -1.0), (4.0, -1.0));
            line((4.0, -1.0), (4.0, -7.0));
            p.rect_stroke(
                egui::Rect::from_center_size(c + Vec2::new(0.0, 4.0), Vec2::new(8.0, 6.0)),
                0,
                s,
                egui::StrokeKind::Inside,
            );
        }
        Glyph::Font => {
            line((-7.0, 7.0), (0.0, -7.0));
            line((0.0, -7.0), (7.0, 7.0));
            line((-4.0, 2.0), (4.0, 2.0));
        }
        Glyph::Align => {
            for (y, w) in [(-6.0, 7.0), (-2.0, 4.0), (2.0, 7.0), (6.0, 4.0)] {
                line((-w, y), (w, y));
            }
        }
        Glyph::Scope => {
            p.rect_stroke(
                egui::Rect::from_center_size(c, Vec2::splat(12.0)),
                1,
                s,
                egui::StrokeKind::Inside,
            );
            line((-9.0, 0.0), (-4.0, 0.0));
            line((4.0, 0.0), (9.0, 0.0));
            line((0.0, -9.0), (0.0, -4.0));
            line((0.0, 4.0), (0.0, 9.0));
        }
        Glyph::Stop => {
            p.rect_filled(
                egui::Rect::from_center_size(c, Vec2::splat(9.0)),
                1.5,
                color,
            );
        }
        Glyph::Search => {
            p.circle_stroke(c + Vec2::new(-2.0, -2.0), 5.5, s);
            line((2.0, 2.0), (7.0, 7.0));
        }
        Glyph::Plus => {
            line((-7.0, 0.0), (7.0, 0.0));
            line((0.0, -7.0), (0.0, 7.0));
        }
        Glyph::Close => {
            line((-5.0, -5.0), (5.0, 5.0));
            line((5.0, -5.0), (-5.0, 5.0));
        }
        Glyph::Minimize => line((-6.0, 4.0), (6.0, 4.0)),
        Glyph::Maximize => {
            p.rect_stroke(
                egui::Rect::from_center_size(c, Vec2::splat(12.0)),
                0,
                s,
                egui::StrokeKind::Inside,
            );
        }
        Glyph::Restore => {
            p.rect_stroke(
                egui::Rect::from_center_size(c + Vec2::new(2.0, -2.0), Vec2::splat(10.0)),
                0,
                s,
                egui::StrokeKind::Inside,
            );
            p.rect_filled(
                egui::Rect::from_center_size(c + Vec2::new(-2.0, 2.0), Vec2::splat(10.0)),
                0,
                PANEL,
            );
            p.rect_stroke(
                egui::Rect::from_center_size(c + Vec2::new(-2.0, 2.0), Vec2::splat(10.0)),
                0,
                s,
                egui::StrokeKind::Inside,
            );
        }
        Glyph::Arrow => {
            line((0.0, 7.0), (0.0, -7.0));
            line((-5.0, -2.0), (0.0, -7.0));
            line((0.0, -7.0), (5.0, -2.0));
        }
        Glyph::Spark => {
            p.add(egui::Shape::closed_line(
                vec![
                    c + Vec2::new(0.0, -8.0),
                    c + Vec2::new(2.5, -2.5),
                    c + Vec2::new(8.0, 0.0),
                    c + Vec2::new(2.5, 2.5),
                    c + Vec2::new(0.0, 8.0),
                    c + Vec2::new(-2.5, 2.5),
                    c + Vec2::new(-8.0, 0.0),
                    c + Vec2::new(-2.5, -2.5),
                ],
                s,
            ));
        }
        Glyph::Folder => {
            p.add(egui::Shape::closed_line(
                vec![
                    c + Vec2::new(-8.0, -5.0),
                    c + Vec2::new(-2.0, -5.0),
                    c + Vec2::new(0.0, -2.0),
                    c + Vec2::new(8.0, -2.0),
                    c + Vec2::new(8.0, 6.0),
                    c + Vec2::new(-8.0, 6.0),
                ],
                s,
            ));
        }
        Glyph::Word | Glyph::Excel | Glyph::Presentation => {
            p.rect_stroke(
                egui::Rect::from_center_size(c, Vec2::new(18.0, 20.0)),
                2,
                s,
                egui::StrokeKind::Inside,
            );
            let letter = match kind {
                Glyph::Word => "W",
                Glyph::Excel => "X",
                _ => "P",
            };
            p.text(
                c,
                egui::Align2::CENTER_CENTER,
                letter,
                egui::FontId::proportional(13.0),
                s.color,
            );
        }
        Glyph::File => {
            p.rect_stroke(
                egui::Rect::from_center_size(c, Vec2::new(12.0, 16.0)),
                2,
                s,
                egui::StrokeKind::Inside,
            );
            line((-3.0, -2.0), (3.0, -2.0));
            line((-3.0, 2.0), (3.0, 2.0));
        }
        Glyph::Attach => {
            p.add(egui::Shape::line(
                vec![
                    c + Vec2::new(5.0, -6.0),
                    c + Vec2::new(-3.0, 2.0),
                    c + Vec2::new(-3.0, 5.0),
                    c + Vec2::new(0.0, 7.0),
                    c + Vec2::new(3.0, 6.0),
                    c + Vec2::new(8.0, 1.0),
                ],
                s,
            ));
        }
        Glyph::Code => {
            line((-6.0, 0.0), (-2.0, -5.0));
            line((-6.0, 0.0), (-2.0, 5.0));
            line((6.0, 0.0), (2.0, -5.0));
            line((6.0, 0.0), (2.0, 5.0));
            line((-1.0, 5.0), (1.0, -5.0));
        }
        Glyph::Book => {
            p.add(egui::Shape::line(
                vec![
                    c + Vec2::new(0.0, 6.0),
                    c + Vec2::new(-7.0, 4.0),
                    c + Vec2::new(-7.0, -5.0),
                    c + Vec2::new(0.0, -3.0),
                    c + Vec2::new(7.0, -5.0),
                    c + Vec2::new(7.0, 4.0),
                    c + Vec2::new(0.0, 6.0),
                ],
                s,
            ));
            line((0.0, -3.0), (0.0, 6.0));
        }
        Glyph::Check => {
            line((-6.0, 0.0), (-2.0, 4.0));
            line((-2.0, 4.0), (6.0, -5.0));
        }
        Glyph::Pin => {
            line((-4.0, -6.0), (4.0, -6.0));
            line((-3.0, -2.0), (3.0, -2.0));
            line((0.0, -2.0), (0.0, 6.0));
            line((-3.0, -6.0), (-3.0, -2.0));
            line((3.0, -6.0), (3.0, -2.0));
        }
        Glyph::Tag => {
            p.add(egui::Shape::closed_line(
                vec![
                    c + Vec2::new(-6.0, -5.0),
                    c + Vec2::new(1.0, -5.0),
                    c + Vec2::new(6.0, 0.0),
                    c + Vec2::new(1.0, 5.0),
                    c + Vec2::new(-6.0, 5.0),
                ],
                s,
            ));
            p.circle_filled(c + Vec2::new(-3.0, 0.0), 1.2, color);
        }
        Glyph::Camera => {
            p.rect_stroke(
                egui::Rect::from_center_size(c + Vec2::new(0.0, 1.0), Vec2::new(14.0, 10.0)),
                2,
                s,
                egui::StrokeKind::Inside,
            );
            p.circle_stroke(c + Vec2::new(0.0, 1.0), 2.5, s);
            line((-3.0, -4.0), (3.0, -4.0));
        }
    }
}

pub fn caption(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(egui::RichText::new(text.into()).size(11.0).color(MUTED));
}

/// Short fade in/out around a real background job, independent of shape pause.
#[derive(Default)]
pub struct ThinkingLight {
    pub strength: f32,
    phase: f32,
}
impl ThinkingLight {
    pub fn update(&mut self, dt: f32, busy: bool) -> f32 {
        self.phase += dt;
        let target = if busy {
            0.7 + 0.3 * (self.phase * 2.5).sin().powi(2)
        } else {
            0.0
        };
        let speed = if busy { 8.0 } else { 5.0 };
        self.strength += (target - self.strength) * (1.0 - (-speed * dt).exp());
        if !busy && self.strength < 0.002 {
            self.strength = 0.0;
        }
        self.strength
    }
    pub fn active(&self) -> bool {
        self.strength > 0.0
    }
}

/// Hiển thị văn bản Markdown tiêu chuẩn:
/// - Chuyển đổi **chữ đậm** (bold) thành chữ sáng trắng nổi bật, loại bỏ dấu **
/// - Hỗ trợ **`tên_tệp`** và `mã_nguồn` thành badge monospace
/// - Hỗ trợ *chữ nghiêng* (italic)
/// - Hỗ trợ Tiêu đề (#, ##, ###)
/// - Hỗ trợ Danh sách (- hoặc * hoặc 1.)
/// - Hỗ trợ Khối mã nguồn (```code```)
pub fn render_markdown(ui: &mut egui::Ui, text: &str) {
    let mut in_code_block = false;
    let mut code_block_content = String::new();
    let mut code_block_lang = String::new();

    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            if in_code_block {
                render_code_block(ui, &code_block_lang, &code_block_content);
                code_block_content.clear();
                code_block_lang.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
                code_block_lang = trimmed.trim_start_matches('`').trim().to_string();
            }
            i += 1;
            continue;
        }

        if in_code_block {
            code_block_content.push_str(line);
            code_block_content.push('\n');
            i += 1;
            continue;
        }

        if trimmed.is_empty() {
            ui.add_space(4.0);
            i += 1;
            continue;
        }

        // Headings: #, ##, ###
        if let Some(rest) = trimmed.strip_prefix("### ") {
            let job = parse_inline_markdown(rest, 15.0, true);
            ui.add(egui::Label::new(job).wrap());
            ui.add_space(2.0);
            i += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("## ") {
            let job = parse_inline_markdown(rest, 16.5, true);
            ui.add(egui::Label::new(job).wrap());
            ui.add_space(3.0);
            i += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("# ") {
            let job = parse_inline_markdown(rest, 18.0, true);
            ui.add(egui::Label::new(job).wrap());
            ui.add_space(4.0);
            i += 1;
            continue;
        }

        // List item: - or * or •
        if let Some(rest) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("• "))
        {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("•").color(MUTED));
                let job = parse_inline_markdown(rest, 14.0, false);
                ui.add(egui::Label::new(job).wrap());
            });
            i += 1;
            continue;
        }

        // Numbered list item: 1. , 2.
        if let Some(dot_idx) = trimmed.find(". ")
            && trimmed[..dot_idx].chars().all(|c| c.is_ascii_digit())
            && dot_idx > 0
            && dot_idx <= 3
        {
            let num = &trimmed[..dot_idx + 1];
            let rest = &trimmed[dot_idx + 2..];
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(num).color(MUTED));
                let job = parse_inline_markdown(rest, 14.0, false);
                ui.add(egui::Label::new(job).wrap());
            });
            i += 1;
            continue;
        }

        // Regular paragraph / line
        let job = parse_inline_markdown(line, 14.0, false);
        ui.add(egui::Label::new(job).wrap());
        i += 1;
    }

    if in_code_block && !code_block_content.is_empty() {
        render_code_block(ui, &code_block_lang, &code_block_content);
    }
}

fn render_code_block(ui: &mut egui::Ui, lang: &str, content: &str) {
    egui::Frame::new()
        .fill(Color32::from_rgb(18, 18, 20))
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(8)
        .inner_margin(8)
        .show(ui, |ui| {
            if !lang.is_empty() {
                caption(ui, lang);
            }
            ui.add(
                egui::Label::new(
                    egui::RichText::new(content.trim_end())
                        .monospace()
                        .size(12.5)
                        .color(Color32::from_rgb(220, 220, 225)),
                )
                .wrap()
                .selectable(true),
            );
        });
    ui.add_space(2.0);
}

pub fn parse_inline_markdown(text: &str, base_size: f32, is_heading: bool) -> LayoutJob {
    let mut job = LayoutJob::default();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    let base_color = if is_heading { Color32::WHITE } else { TEXT };
    let font_id = FontId::proportional(base_size);

    while i < len {
        // Underscores inside identifiers and paths are not emphasis delimiters.
        let underscore_can_open =
            i == 0 || (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '_');
        // 1. Check for inline code `...`
        if chars[i] == '`'
            && let Some(end) = chars[i + 1..].iter().position(|&c| c == '`')
        {
            let code_end = i + 1 + end;
            let code_str: String = chars[i + 1..code_end].iter().collect();
            job.append(
                &code_str,
                0.0,
                TextFormat {
                    font_id: FontId::monospace(base_size - 1.0),
                    color: Color32::from_rgb(235, 235, 240),
                    background: Color32::from_gray(48),
                    ..Default::default()
                },
            );
            i = code_end + 1;
            continue;
        }

        // 2. Check for bold `**...**` or `__...__`
        if i + 1 < len
            && ((chars[i] == '*' && chars[i + 1] == '*')
                || (chars[i] == '_' && chars[i + 1] == '_' && underscore_can_open))
        {
            let marker = chars[i];
            let rest = &chars[i + 2..];
            if let Some(pos) = rest
                .windows(2)
                .position(|w| w[0] == marker && w[1] == marker)
            {
                let bold_end = i + 2 + pos;
                let bold_inner: String = chars[i + 2..bold_end].iter().collect();

                // Nếu bên trong bold có backtick code (ví dụ: **`file.png`**)
                if bold_inner.starts_with('`') && bold_inner.ends_with('`') && bold_inner.len() >= 2
                {
                    let code_inner = &bold_inner[1..bold_inner.len() - 1];
                    job.append(
                        code_inner,
                        0.0,
                        TextFormat {
                            font_id: FontId::monospace(base_size - 1.0),
                            color: Color32::WHITE,
                            background: Color32::from_gray(55),
                            ..Default::default()
                        },
                    );
                } else {
                    job.append(
                        &bold_inner,
                        0.0,
                        TextFormat {
                            font_id: FontId::proportional(base_size),
                            color: Color32::WHITE,
                            ..Default::default()
                        },
                    );
                }
                i = bold_end + 2;
                continue;
            }
        }

        // 3. Check for italic `*...*` or `_..._`
        if chars[i] == '*' || (chars[i] == '_' && underscore_can_open) {
            let marker = chars[i];
            if let Some(pos) = chars[i + 1..].iter().position(|&c| c == marker) {
                let italic_end = i + 1 + pos;
                let italic_str: String = chars[i + 1..italic_end].iter().collect();
                let closing_boundary = marker != '_'
                    || italic_end + 1 == len
                    || !chars[italic_end + 1].is_alphanumeric();
                if !italic_str.trim().is_empty() && closing_boundary {
                    job.append(
                        &italic_str,
                        0.0,
                        TextFormat {
                            font_id: FontId::proportional(base_size),
                            color: Color32::from_rgb(220, 220, 225),
                            italics: true,
                            ..Default::default()
                        },
                    );
                    i = italic_end + 1;
                    continue;
                }
            }
        }

        // A marker without a valid closing pair is literal text. Always consume
        // at least one character; starting this search at i would loop forever.
        let next_special = chars[i + 1..]
            .iter()
            .position(|c| matches!(c, '*' | '_' | '`'))
            .map_or(len, |offset| i + 1 + offset);
        let chunk: String = chars[i..next_special].iter().collect();
        job.append(
            &chunk,
            0.0,
            TextFormat {
                font_id: font_id.clone(),
                color: base_color,
                ..Default::default()
            },
        );
        i = next_special;
    }

    job
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_markdown_preserves_literal_text_and_finishes() {
        for text in [
            "*",
            "_",
            "`",
            "Đang đọc *nội dung",
            "C:\\du_an\\file_name",
            "mã `chưa đóng",
            "* *",
            "__",
        ] {
            assert_eq!(parse_inline_markdown(text, 14.0, false).text, text);
        }
        assert_eq!(
            parse_inline_markdown("**đậm** và `code`", 14.0, false).text,
            "đậm và code"
        );
    }
    #[test]
    fn dark_interface_survives_a_light_system_theme() {
        let ctx = egui::Context::default();
        install(&ctx);
        let mut output = ctx.run_ui(
            egui::RawInput {
                system_theme: Some(egui::Theme::Light),
                ..Default::default()
            },
            |ui| {
                assert!(ui.visuals().dark_mode);
                assert_eq!(ui.visuals().override_text_color, Some(TEXT));
            },
        );
        output.textures_delta.clear();
    }
    #[test]
    fn thinking_light_fades_after_completion_at_different_frame_rates() {
        for fps in [30, 60, 120] {
            let mut light = ThinkingLight::default();
            for _ in 0..fps {
                light.update(1.0 / fps as f32, true);
            }
            assert!(light.strength > 0.5 && light.strength <= 1.0);
            for _ in 0..fps * 2 {
                light.update(1.0 / fps as f32, false);
            }
            assert!(!light.active());
        }
    }
}
