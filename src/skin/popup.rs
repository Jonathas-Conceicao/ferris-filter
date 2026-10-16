//! The skinned modal popup: a dimmed window, the game's box art centred, game buttons.
//!
//! [`ff_modal`] is stateless: the caller keeps a "popup open" flag, calls it once per frame
//! while the flag is set, and acts on the returned button index. The last button is always the
//! safe dismiss: Escape and a click on the dimmed backdrop both answer with it.

use egui::{Align, Color32, Context, Id, Layout, Margin, Rect, Shape, Ui, vec2};

use super::Skin;
use super::fonts::{regular, small_caps};
use super::palette as pal;
use super::slice::{self, Margins};
use super::widgets::ff_button_ex;

/// One button of a modal.
#[derive(Debug, Clone, Copy)]
pub struct PopupButton<'a> {
    pub label: &'a str,
    /// Red caption, for the destructive choice.
    pub danger: bool,
    /// Hover tooltip.
    pub tip: Option<&'a str>,
}

impl<'a> PopupButton<'a> {
    /// A plain button.
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            danger: false,
            tip: None,
        }
    }
}

/// Dim colour behind the box.
const BACKDROP: Color32 = Color32::from_black_alpha(160);
/// Gap between buttons.
const GAP: f32 = 8.0;
/// Inner padding of the box.
const PAD: f32 = 24.0;
/// Body text is capped at about four lines, then scrolls.
const BODY_MAX_HEIGHT: f32 = 84.0;

/// Shows the modal and returns the index of the button chosen this frame, if any. Escape, a
/// click on the backdrop or the close request answers `Some(buttons.len() - 1)`. Empty
/// `buttons` shows nothing and returns `None`.
pub fn ff_modal(
    ctx: &Context,
    skin: &Skin,
    id: Id,
    title: &str,
    body: &str,
    buttons: &[PopupButton],
) -> Option<usize> {
    if buttons.is_empty() {
        return None;
    }
    let mut choice = None;
    let width = 580.0_f32.min(ctx.content_rect().width() - 80.0).max(200.0);
    let modal = egui::Modal::new(id)
        .backdrop_color(BACKDROP)
        .frame(egui::Frame::NONE.inner_margin(Margin::same(PAD as i8)))
        .show(ctx, |ui| {
            ui.set_width(width - 2.0 * PAD);
            let bg = ui.painter().add(Shape::Noop);
            ui.spacing_mut().item_spacing.y = 0.0;
            title_block(ui, skin, title);
            ui.add_space(12.0);
            egui::ScrollArea::vertical()
                .max_height(BODY_MAX_HEIGHT)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(body)
                                .font(regular(15.0))
                                .color(pal::VALUE),
                        )
                        .wrap(),
                    );
                });
            ui.add_space(16.0);
            choice = button_row(ui, skin, id, buttons);
            let frame_rect = ui.min_rect().expand(PAD);
            let mut mesh = egui::Mesh::with_texture(skin.box_9slice.id());
            for p in slice::nine_slice(
                frame_rect,
                skin.box_9slice.size(),
                Margins::new(8.0, 8.0, 8.0, 8.0),
                1.0,
                ui.painter().pixels_per_point(),
            ) {
                mesh.add_rect_with_uv(p.dest, p.uv, Color32::WHITE);
            }
            ui.painter().set(bg, Shape::mesh(mesh));
        });
    modal
        .response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Window, true, title));
    if choice.is_none() && modal.should_close() {
        choice = Some(buttons.len() - 1);
    }
    choice
}

/// The title in small caps over the faint gold rule.
fn title_block(ui: &mut Ui, skin: &Skin, title: &str) {
    let galley = ui
        .painter()
        .layout_no_wrap(title.to_owned(), small_caps(21.0), pal::HEADING);
    let (rect, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), galley.size().y + 12.0),
        egui::Sense::hover(),
    );
    let painter = ui.painter();
    painter.galley(
        egui::pos2(rect.min.x, rect.min.y + 2.0),
        galley,
        pal::HEADING,
    );
    let rule = Rect::from_min_size(
        egui::pos2(rect.min.x, rect.max.y - 5.0),
        vec2(rect.width(), 3.0),
    );
    slice::paint_image(
        painter,
        &skin.title_underline,
        rule,
        Color32::from_white_alpha(150),
    );
}

/// The centred row of buttons; returns the one clicked.
fn button_row(ui: &mut Ui, skin: &Skin, id: Id, buttons: &[PopupButton]) -> Option<usize> {
    let widths: Vec<f32> = buttons
        .iter()
        .map(|b| {
            let w = ui
                .painter()
                .layout_no_wrap(b.label.to_owned(), small_caps(17.0), pal::BUTTON_TEXT)
                .size()
                .x;
            (w + 44.0).max(120.0)
        })
        .collect();
    let total: f32 = widths.iter().sum::<f32>() + GAP * (buttons.len() - 1) as f32;
    let avail = ui.available_width();
    let mut chosen = None;
    let mut rects = Vec::with_capacity(buttons.len());
    let mut draw = |ui: &mut Ui| {
        ui.spacing_mut().item_spacing = vec2(GAP, GAP);
        for (i, b) in buttons.iter().enumerate() {
            let mut r = ff_button_ex(ui, skin, b.label, 120.0, true, b.danger);
            if let Some(tip) = b.tip {
                r = r.on_hover_text(tip);
            }
            rects.push(r.rect);
            if r.clicked() {
                chosen = Some(i);
            }
        }
    };
    if total <= avail {
        ui.with_layout(Layout::top_down(Align::Center), |ui| {
            ui.horizontal(|ui| {
                ui.add_space(((avail - total) / 2.0).max(0.0));
                draw(ui);
            });
        });
    } else {
        ui.horizontal_wrapped(|ui| draw(ui));
    }
    // Test hook: where each button ended up.
    ui.ctx()
        .data_mut(|d| d.insert_temp(id.with("ff_modal_rects"), rects));
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Key, Modifiers, PointerButton, RawInput, pos2};

    fn setup() -> (Context, Skin) {
        let ctx = Context::default();
        super::super::fonts::install(&ctx);
        let skin = Skin::load(&ctx);
        (ctx, skin)
    }

    fn raw(events: Vec<Event>) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0))),
            events,
            ..Default::default()
        }
    }

    const BUTTONS: [PopupButton; 3] = [
        PopupButton {
            label: "Apply",
            danger: false,
            tip: None,
        },
        PopupButton {
            label: "Quit",
            danger: true,
            tip: Some("Quit without applying"),
        },
        PopupButton {
            label: "Cancel",
            danger: false,
            tip: None,
        },
    ];

    fn frame(ctx: &Context, skin: &Skin, events: Vec<Event>) -> Option<usize> {
        let mut out = None;
        ctx.run_ui(raw(events), |_ui| {
            out = ff_modal(
                ctx,
                skin,
                Id::new("t"),
                "Quit?",
                "Unapplied changes.",
                &BUTTONS,
            );
        })
        .drop_without_applying_deltas();
        out
    }

    fn settle(ctx: &Context, skin: &Skin) -> Vec<Rect> {
        for _ in 0..3 {
            frame(ctx, skin, vec![]);
        }
        ctx.data(|d| d.get_temp::<Vec<Rect>>(Id::new("t").with("ff_modal_rects")))
            .expect("rects recorded")
    }

    #[test]
    fn clicking_a_button_returns_its_index() {
        let (ctx, skin) = setup();
        let rects = settle(&ctx, &skin);
        assert_eq!(rects.len(), 3);
        let pos = rects[1].center();
        let got = frame(
            &ctx,
            &skin,
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::default(),
                },
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::default(),
                },
            ],
        );
        assert_eq!(got, Some(1));
    }

    #[test]
    fn escape_answers_the_last_button() {
        let (ctx, skin) = setup();
        settle(&ctx, &skin);
        let got = frame(
            &ctx,
            &skin,
            vec![Event::Key {
                key: Key::Escape,
                physical_key: Some(Key::Escape),
                pressed: true,
                repeat: false,
                modifiers: Modifiers::default(),
            }],
        );
        assert_eq!(got, Some(2));
    }

    #[test]
    fn a_quiet_frame_returns_none_and_empty_buttons_never_panic() {
        let (ctx, skin) = setup();
        assert_eq!(frame(&ctx, &skin, vec![]), None);
        ctx.run_ui(raw(vec![]), |_ui| {
            assert_eq!(ff_modal(&ctx, &skin, Id::new("e"), "t", "b", &[]), None);
        })
        .drop_without_applying_deltas();
    }
}
