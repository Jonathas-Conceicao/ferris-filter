//! Pure drag geometry for the RULES list and the CONFIG style table (drag-and-drop reorder).
//!
//! No GUI-toolkit types outside the tests: slot maths, auto-scroll speed and shadow clamping are
//! plain arithmetic so every rule of the old arrows (economy boundary, banners, collapsed headers,
//! built-ins first) is testable without a window.

/// Distance from the viewport's top / bottom edge (pt) inside which a drag auto-scrolls.
pub const EDGE_ZONE: f32 = 40.0;
/// Auto-scroll speed at the very edge (pt/s); linear down to 0 at [`EDGE_ZONE`].
pub const AUTOSCROLL_MAX: f32 = 900.0;
/// Thickness of the gold drop marker line (pt).
pub const MARKER_THICKNESS: f32 = 2.0;

/// List-agnostic copy of what one visible rule-list entry is. `ordinal` is the section's banner
/// ordinal (the one `RowAction::RenameSection` and friends use).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotEntry {
    /// A user rule row (a whole `# ff:group` run is one entry).
    Rule(usize),
    /// A user section header.
    UserHeader { ordinal: usize, collapsed: bool },
    /// The locked economy section header.
    EconomyHeader,
    /// A read-only economy rule row.
    EconomyRule,
}

/// Where a dropped rule lands, in document terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotTarget {
    /// Directly above rule entry `i`.
    BeforeRule(usize),
    /// First rule of the section whose banner has this ordinal.
    AfterBanner(usize),
    /// End of the section above the banner with this ordinal (the preamble for the first banner).
    BeforeBanner(usize),
    /// Just above the economy banner, else the end of the file.
    End,
}

/// Nearest boundary of `offsets` (content-space row tops, `n + 1` of them) to `y` among the
/// boundaries `valid` accepts. `None` when none is valid. Never panics on NaN.
pub fn nearest_slot(offsets: &[f32], y: f32, valid: impl Fn(usize) -> bool) -> Option<usize> {
    offsets
        .iter()
        .enumerate()
        .filter(|(b, _)| valid(*b))
        .min_by(|(_, a), (_, c)| (y - **a).abs().total_cmp(&(y - **c).abs()))
        .map(|(b, _)| b)
}

/// Signed auto-scroll speed in pt/s (negative = up, positive = down): 0 farther than `zone` from
/// both edges, linear up to `max` at the edge, clamped beyond it. In a viewport shorter than
/// `2 * zone` the bands overlap and the nearer edge wins.
pub fn autoscroll_speed(y: f32, top: f32, bottom: f32, zone: f32, max: f32) -> f32 {
    if zone <= 0.0 {
        return 0.0;
    }
    let up = ((top + zone - y) / zone).clamp(0.0, 1.0);
    let down = ((y - (bottom - zone)) / zone).clamp(0.0, 1.0);
    if up > 0.0 && down > 0.0 {
        return if y - top <= bottom - y {
            -up * max
        } else {
            down * max
        };
    }
    (down - up) * max
}

/// Top of the drag shadow: centred on the pointer, clamped to `[top, bottom - h]` (`top` when the
/// viewport is shorter than the row).
pub fn shadow_top(pointer_y: f32, h: f32, top: f32, bottom: f32) -> f32 {
    (pointer_y - h / 2.0).clamp(top, (bottom - h).max(top))
}

/// What dropping on boundary `b` (between `entries[b - 1]` and `entries[b]`, `0..=len`) means.
/// The first matching rule wins:
/// 1. the entry above is the economy header or an economy rule: `None` (in or below economy);
/// 2. the entry above is a user header, collapsed or not: `AfterBanner` (a drop on a collapsed
///    header makes the rule that section's first rule, and the section stays collapsed; this also
///    claims the boundary above the next header, which is geometrically the same place);
/// 3. the entry below is a rule: `BeforeRule`;
/// 4. the entry below is a user header: `BeforeBanner`;
/// 5. the entry below is the economy header or the list ends: `End`.
pub fn slot_target(entries: &[SlotEntry], b: usize) -> Option<SlotTarget> {
    if b > entries.len() {
        return None;
    }
    if b > 0 {
        match entries[b - 1] {
            SlotEntry::EconomyHeader | SlotEntry::EconomyRule => return None,
            SlotEntry::UserHeader { ordinal, .. } => return Some(SlotTarget::AfterBanner(ordinal)),
            SlotEntry::Rule(_) => {}
        }
    }
    match entries.get(b) {
        Some(SlotEntry::Rule(i)) => Some(SlotTarget::BeforeRule(*i)),
        Some(SlotEntry::UserHeader { ordinal, .. }) => Some(SlotTarget::BeforeBanner(*ordinal)),
        Some(SlotEntry::EconomyHeader) | None => Some(SlotTarget::End),
        Some(SlotEntry::EconomyRule) => None,
    }
}

/// True when dropping the rule at entry index `src` on boundary `b` changes nothing: `b == src`
/// (just above it) and `b == src + 1` (just below it), which also covers a `BeforeBanner` / `End`
/// target directly after the last rule of its section. A non-rule `src` is always a no-op.
pub fn is_noop(entries: &[SlotEntry], src: usize, b: usize) -> bool {
    !matches!(entries.get(src), Some(SlotEntry::Rule(_))) || b == src || b == src + 1
}

/// Nearest gap `0..=n` among the custom style rows. `bounds` holds the custom rows' tops plus the
/// last custom row's bottom (`n + 1` values); empty bounds give gap 0.
pub fn style_gap_for_y(bounds: &[f32], y: f32) -> usize {
    nearest_slot(bounds, y, |_| true).unwrap_or(0)
}

// The custom-style gap maths lives beside `StyleSet::move_custom_to` in the model layer, which
// must not depend on the overlay; the style table uses it through this re-export.
pub use crate::models::config::{style_gap_to_index, style_noop};

#[cfg(test)]
mod tests {
    use super::*;
    use SlotEntry::*;

    fn user(ordinal: usize, collapsed: bool) -> SlotEntry {
        UserHeader { ordinal, collapsed }
    }

    #[test]
    fn nearest_slot_picks_closest_valid_boundary() {
        let o = [0., 52., 104., 156.];
        assert_eq!(nearest_slot(&o, 60.0, |_| true), Some(1));
        assert_eq!(nearest_slot(&o, 60.0, |b| b != 1), Some(2));
        assert_eq!(nearest_slot(&o, 60.0, |_| false), None);
        assert!(nearest_slot(&o, f32::NAN, |_| true).is_some());
    }

    #[test]
    fn autoscroll_speed_ramps_and_clamps() {
        let s = |y| autoscroll_speed(y, 0.0, 600.0, EDGE_ZONE, AUTOSCROLL_MAX);
        assert_eq!(s(300.0), 0.0);
        assert_eq!(s(40.0), 0.0);
        assert_eq!(s(560.0), 0.0);
        assert_eq!(s(0.0), -900.0);
        assert_eq!(s(600.0), 900.0);
        assert_eq!(s(580.0), 450.0);
        assert_eq!(s(20.0), -450.0);
        assert_eq!(s(-50.0), -900.0);
        assert_eq!(s(700.0), 900.0);
    }

    #[test]
    fn autoscroll_in_a_short_viewport_prefers_the_nearer_edge() {
        let s = |y| autoscroll_speed(y, 0.0, 60.0, EDGE_ZONE, AUTOSCROLL_MAX);
        assert!(s(10.0) < 0.0);
        assert!(s(50.0) > 0.0);
    }

    #[test]
    fn shadow_top_centres_and_clamps() {
        assert_eq!(shadow_top(300.0, 52.0, 0.0, 600.0), 274.0);
        assert_eq!(shadow_top(5.0, 52.0, 0.0, 600.0), 0.0);
        assert_eq!(shadow_top(599.0, 52.0, 0.0, 600.0), 548.0);
        assert_eq!(shadow_top(30.0, 52.0, 10.0, 40.0), 10.0);
    }

    #[test]
    fn slot_target_follows_the_table() {
        // 0 R0, 1 H0, 2 R1, 3 R2, 4 H1 (collapsed), 5 H2 (collapsed), 6 Econ, 7 E
        let e = [
            Rule(0),
            user(0, false),
            Rule(1),
            Rule(2),
            user(1, true),
            user(2, true),
            EconomyHeader,
            EconomyRule,
        ];
        assert_eq!(slot_target(&e, 0), Some(SlotTarget::BeforeRule(0)));
        assert_eq!(slot_target(&e, 1), Some(SlotTarget::BeforeBanner(0)));
        assert_eq!(slot_target(&e, 2), Some(SlotTarget::AfterBanner(0)));
        assert_eq!(slot_target(&e, 3), Some(SlotTarget::BeforeRule(2)));
        assert_eq!(slot_target(&e, 4), Some(SlotTarget::BeforeBanner(1)));
        assert_eq!(slot_target(&e, 99), None);
    }

    #[test]
    fn collapsed_header_followed_by_header_yields_after_banner_of_the_collapsed_one() {
        let e = [user(1, true), user(2, true), EconomyHeader];
        assert_eq!(slot_target(&e, 1), Some(SlotTarget::AfterBanner(1)));
        assert_eq!(slot_target(&e, 2), Some(SlotTarget::AfterBanner(2)));
    }

    #[test]
    fn economy_boundary_is_never_a_slot_and_end_sits_above_it() {
        let e = [Rule(0), EconomyHeader, EconomyRule, EconomyRule];
        assert_eq!(slot_target(&e, 1), Some(SlotTarget::End));
        assert_eq!(slot_target(&e, 2), None);
        assert_eq!(slot_target(&e, 3), None);
        assert_eq!(slot_target(&e, 4), None);
        // No economy section: the end of the list is the end of the file.
        let f = [Rule(0), Rule(1)];
        assert_eq!(slot_target(&f, 2), Some(SlotTarget::End));
        assert_eq!(slot_target(&[], 0), Some(SlotTarget::End));
    }

    #[test]
    fn noop_covers_the_gaps_around_the_source() {
        let e = [Rule(0), Rule(1), user(0, false), Rule(2), EconomyHeader];
        assert!(is_noop(&e, 1, 1));
        assert!(is_noop(&e, 1, 2)); // BeforeBanner right below the last rule of its section
        assert!(!is_noop(&e, 1, 0));
        assert!(!is_noop(&e, 1, 3));
        assert!(is_noop(&e, 3, 4)); // End right below the last rule
        assert!(is_noop(&e, 2, 0)); // a header is not a rule: nothing to move
    }

    #[test]
    fn style_gaps() {
        let bounds = [100.0, 150.0, 200.0, 250.0];
        assert_eq!(style_gap_for_y(&bounds, 90.0), 0);
        assert_eq!(style_gap_for_y(&bounds, 160.0), 1);
        assert_eq!(style_gap_for_y(&bounds, 900.0), 3);
        assert_eq!(style_gap_for_y(&[], 10.0), 0);
        assert!(style_noop(1, 1) && style_noop(1, 2));
        assert!(!style_noop(1, 0) && !style_noop(1, 3));
        assert_eq!(style_gap_to_index(0, 3), 2);
        assert_eq!(style_gap_to_index(2, 0), 0);
        assert_eq!(style_gap_to_index(1, 1), 1);
    }
}

/// Spike tests (phase 49, plan 01): headless egui experiments that settle how the list scrolls
/// while a handle is being dragged and how the drag shadow is drawn. Results are recorded in
/// `49-01-SUMMARY.md`.
#[cfg(test)]
mod spike {
    use egui::style::ScrollAnimation;
    use egui::{
        Area, Context, Event, Id, Key, Modifiers, MouseWheelUnit, Order, PointerButton, Pos2,
        RawInput, Rect, ScrollArea, Sense, TouchPhase, Vec2, pos2, vec2,
    };

    const ROW_H: f32 = 52.0;
    const ROWS: usize = 60;
    const VIEW_W: f32 = 660.0;
    const VIEW_H: f32 = 600.0;
    const DT: f32 = 1.0 / 60.0;

    /// Per-frame knobs of the test list.
    #[derive(Clone, Copy)]
    struct Opts {
        /// Virtualized (`show_viewport`, only visible rows drawn) or plain `show`.
        virtualized: bool,
        /// Forward the wheel while a drag is active (`scroll_with_delta_animation`).
        forward_wheel: bool,
        /// Edge auto-scroll while a drag is active.
        autoscroll: bool,
        /// Id salt of the rows (simulates the epoch bump that changes a row's ids).
        salt: u32,
    }

    /// What one frame recorded.
    #[derive(Default, Clone)]
    struct Seen {
        /// `viewport.min.y` (virtualized) or the top of row 0 negated (plain).
        offset: f32,
        visible: std::ops::Range<usize>,
        /// Screen rects of the rows drawn this frame.
        rows: Vec<(usize, Rect)>,
        handle3_id: Option<Id>,
        handle3_drag_started: bool,
        handle3_clicked: bool,
        row3_clicked: bool,
        dragged_id: Option<Id>,
        drag_stopped_id: Option<Id>,
        primary_released: bool,
        escape_after_consume: bool,
    }

    struct Rig {
        ctx: Context,
        time: f64,
    }

    impl Rig {
        fn new() -> Self {
            let ctx = Context::default();
            crate::skin::fonts::install(&ctx);
            Rig { ctx, time: 0.0 }
        }

        fn frame(&mut self, events: Vec<Event>, o: Opts) -> Seen {
            self.time += DT as f64;
            let raw = RawInput {
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    vec2(VIEW_W, VIEW_H + 100.0),
                )),
                time: Some(self.time),
                predicted_dt: DT,
                events,
                ..Default::default()
            };
            let mut seen = Seen::default();
            let ctx = self.ctx.clone();
            ctx.run_ui(raw, |ui| {
                seen.dragged_id = ui.ctx().dragged_id();
                seen.drag_stopped_id = ui.ctx().drag_stopped_id();
                seen.primary_released = ui.input(|i| i.pointer.primary_released());
                if ui.input(|i| i.key_pressed(Key::Escape)) {
                    ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
                    seen.escape_after_consume = ui.input(|i| i.key_pressed(Key::Escape));
                }
                let area = Rect::from_min_size(Pos2::ZERO, vec2(VIEW_W, VIEW_H));
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(area));
                let ui = &mut child;
                let row = |ui: &mut egui::Ui, i: usize, rect: Rect, seen: &mut Seen| {
                    let row_resp = ui.interact(rect, Id::new(("row", o.salt, i)), Sense::click());
                    let handle = Rect::from_min_size(
                        pos2(rect.right() - 60.0, rect.top() + 12.0),
                        Vec2::splat(28.0),
                    );
                    let h = ui.interact(
                        handle,
                        Id::new(("handle", o.salt, i)),
                        Sense::click_and_drag(),
                    );
                    seen.rows.push((i, rect));
                    if i == 3 {
                        seen.handle3_id = Some(h.id);
                        seen.handle3_drag_started = h.drag_started();
                        seen.handle3_clicked = h.clicked();
                        seen.row3_clicked = row_resp.clicked();
                    }
                };
                let forward = |ui: &mut egui::Ui, top: f32, bottom: f32| {
                    if ui.ctx().dragged_id().is_none() {
                        return;
                    }
                    let wheel = if o.forward_wheel {
                        ui.input(|i| i.smooth_scroll_delta.y)
                    } else {
                        0.0
                    };
                    let auto = if o.autoscroll {
                        let y = ui.input(|i| i.pointer.latest_pos()).map_or(0.0, |p| p.y);
                        super::autoscroll_speed(
                            y,
                            top,
                            bottom,
                            super::EDGE_ZONE,
                            super::AUTOSCROLL_MAX,
                        )
                    } else {
                        0.0
                    };
                    ui.scroll_with_delta_animation(
                        vec2(0.0, wheel - auto * DT),
                        ScrollAnimation::none(),
                    );
                };
                if o.virtualized {
                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show_viewport(ui, |ui, viewport| {
                            seen.offset = viewport.min.y;
                            let origin = ui.max_rect().min.y;
                            ui.set_height(ROWS as f32 * ROW_H);
                            let offsets: Vec<f32> = (0..=ROWS).map(|i| i as f32 * ROW_H).collect();
                            let vis = crate::overlay::virtual_list::visible_range(
                                &offsets,
                                viewport.min.y,
                                viewport.max.y,
                            );
                            seen.visible = vis.clone();
                            for i in vis {
                                let rect = Rect::from_min_size(
                                    pos2(0.0, origin + offsets[i]),
                                    vec2(VIEW_W - 20.0, ROW_H),
                                );
                                row(ui, i, rect, &mut seen);
                            }
                            forward(ui, 0.0, VIEW_H);
                        });
                } else {
                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for i in 0..ROWS {
                                let (rect, _) = ui.allocate_exact_size(
                                    vec2(VIEW_W - 20.0, ROW_H),
                                    Sense::hover(),
                                );
                                if i == 0 {
                                    seen.offset = -rect.top();
                                }
                                row(ui, i, rect, &mut seen);
                            }
                            forward(ui, 0.0, VIEW_H);
                        });
                }
            })
            .drop_without_applying_deltas();
            seen
        }
    }

    fn press(pos: Pos2) -> Vec<Event> {
        vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::default(),
            },
        ]
    }

    fn release(pos: Pos2) -> Vec<Event> {
        vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::default(),
        }]
    }

    fn wheel(dy: f32) -> Vec<Event> {
        vec![Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: vec2(0.0, dy),
            phase: TouchPhase::Move,
            modifiers: Modifiers::default(),
        }]
    }

    fn escape() -> Vec<Event> {
        vec![Event::Key {
            key: Key::Escape,
            physical_key: Some(Key::Escape),
            pressed: true,
            repeat: false,
            modifiers: Modifiers::default(),
        }]
    }

    const BASE: Opts = Opts {
        virtualized: true,
        forward_wheel: false,
        autoscroll: false,
        salt: 0,
    };

    /// Row 3's handle centre on screen at scroll offset 0.
    fn handle3() -> Pos2 {
        pos2(VIEW_W - 20.0 - 60.0 + 14.0, 3.0 * ROW_H + 12.0 + 14.0)
    }

    /// Start a drag on row 3's handle; returns the rig after the drag began.
    fn start_drag(o: Opts) -> (Rig, Seen) {
        let mut rig = Rig::new();
        rig.frame(vec![], o);
        rig.frame(press(handle3()), o);
        let moved = handle3() + vec2(0.0, 10.0);
        let mut s = rig.frame(vec![Event::PointerMoved(moved)], o);
        for _ in 0..2 {
            if s.dragged_id.is_some() {
                break;
            }
            s = rig.frame(vec![], o);
        }
        (rig, s)
    }

    #[test]
    fn s1a_wheel_is_ignored_by_scroll_area_during_a_drag() {
        let (mut rig, s) = start_drag(BASE);
        assert!(s.dragged_id.is_some(), "the drag must be active");
        let mut last = s;
        for _ in 0..6 {
            last = rig.frame(wheel(-60.0), BASE);
        }
        assert_eq!(
            last.offset, 0.0,
            "egui gates the wheel off while a widget is dragged"
        );
    }

    #[test]
    fn s1b_forwarded_wheel_scrolls_during_a_drag() {
        let o = Opts {
            forward_wheel: true,
            ..BASE
        };
        let (mut rig, s) = start_drag(o);
        assert!(s.dragged_id.is_some());
        let mut last = s;
        for _ in 0..8 {
            last = rig.frame(wheel(-60.0), o);
        }
        assert!(
            last.offset > 50.0,
            "forwarded wheel must scroll, offset {}",
            last.offset
        );
    }

    #[test]
    fn s1c_edge_autoscroll_moves_about_speed_times_dt() {
        let o = Opts {
            autoscroll: true,
            ..BASE
        };
        let (mut rig, s) = start_drag(o);
        assert!(s.dragged_id.is_some());
        let y = VIEW_H - 5.0;
        let before = rig
            .frame(vec![Event::PointerMoved(pos2(300.0, y))], o)
            .offset;
        let n = 20;
        let mut last = before;
        for _ in 0..n {
            last = rig.frame(vec![], o).offset;
        }
        let speed =
            super::autoscroll_speed(y, 0.0, VIEW_H, super::EDGE_ZONE, super::AUTOSCROLL_MAX);
        let expected = speed * DT * n as f32;
        let got = last - before;
        assert!(
            (got - expected).abs() <= expected * 0.25,
            "moved {got}, expected about {expected}"
        );
    }

    #[test]
    fn s2_drag_survives_virtualization_and_id_changes() {
        let o = Opts {
            autoscroll: true,
            ..BASE
        };
        let (mut rig, s) = start_drag(o);
        let captured = s.handle3_id.expect("row 3 drawn");
        assert_eq!(s.dragged_id, Some(captured));
        let mut o2 = o;
        rig.frame(vec![Event::PointerMoved(pos2(300.0, VIEW_H - 5.0))], o2);
        let mut out_of_range = false;
        for n in 0..60 {
            if n == 5 {
                o2.salt = 1; // the epoch bump: every row id changes
            }
            let s = rig.frame(vec![], o2);
            assert_eq!(s.dragged_id, Some(captured), "drag lost at frame {n}");
            assert_eq!(s.drag_stopped_id, None);
            if !s.visible.contains(&3) {
                out_of_range = true;
            }
        }
        assert!(out_of_range, "row 3 must have been virtualized out");
        let end = pos2(300.0, VIEW_H - 5.0);
        let mut stops = 0;
        for ev in [release(end), vec![], vec![]] {
            let s = rig.frame(ev, o2);
            if s.drag_stopped_id == Some(captured) {
                stops += 1;
                assert!(s.primary_released);
            }
        }
        assert_eq!(
            stops, 1,
            "drag_stopped_id must report the captured id exactly once"
        );
    }

    #[test]
    fn s3_tap_starts_no_drag_and_does_not_click_the_row() {
        let mut rig = Rig::new();
        rig.frame(vec![], BASE);
        rig.frame(press(handle3()), BASE);
        let mut any_row_click = false;
        let mut any_drag = false;
        let mut handle_clicked = false;
        for ev in [release(handle3()), vec![], vec![]] {
            let s = rig.frame(ev, BASE);
            any_row_click |= s.row3_clicked;
            any_drag |= s.handle3_drag_started;
            handle_clicked |= s.handle3_clicked;
        }
        assert!(!any_drag, "a tap must not start a drag");
        assert!(
            !any_row_click,
            "a tap on the handle must not reach the row's click"
        );
        assert!(
            handle_clicked,
            "the handle owns the click (ignored by the list)"
        );
    }

    #[test]
    fn s3_escape_aborts_the_drag_without_a_release() {
        let (mut rig, s) = start_drag(BASE);
        let captured = s.dragged_id.expect("dragging");
        let s = rig.frame(escape(), BASE);
        assert_eq!(s.dragged_id, None);
        assert_eq!(s.drag_stopped_id, Some(captured));
        assert!(!s.primary_released);
        assert!(
            !s.escape_after_consume,
            "consume_key hides Escape from later widgets"
        );
    }

    /// S4: a disabled, 60% opaque Area on `Order::Tooltip` neither steals clicks nor paints
    /// opaque. Returns (clicked underneath, alpha of the white shadow fill).
    fn shadow_probe() -> (bool, Option<u8>) {
        let ctx = Context::default();
        crate::skin::fonts::install(&ctx);
        let spot = Rect::from_min_size(pos2(40.0, 40.0), vec2(200.0, 52.0));
        let click_at = spot.center();
        let mut clicked = false;
        let mut alpha = None;
        let mut t = 0.0;
        let frames: Vec<Vec<Event>> = vec![vec![], press(click_at), release(click_at), vec![]];
        for ev in frames {
            t += DT as f64;
            let raw = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))),
                time: Some(t),
                predicted_dt: DT,
                events: ev,
                ..Default::default()
            };
            let out = ctx.run_ui(raw, |ui| {
                let r = ui.interact(spot, Id::new("under"), Sense::click());
                clicked |= r.clicked();
                Area::new(Id::new("shadow"))
                    .order(Order::Tooltip)
                    .fixed_pos(spot.min)
                    .interactable(false)
                    .constrain(false)
                    .show(ui.ctx(), |ui| {
                        ui.disable();
                        ui.set_opacity(0.6);
                        let (rect, _) = ui.allocate_exact_size(spot.size(), Sense::click());
                        ui.painter().rect_filled(rect, 0.0, egui::Color32::WHITE);
                    });
            });
            for cs in &out.shapes {
                if let egui::epaint::Shape::Rect(r) = &cs.shape {
                    if r.fill.r() > 0 && r.fill.a() < 255 && r.fill.a() > 0 {
                        alpha = Some(r.fill.a());
                    }
                }
            }
            out.drop_without_applying_deltas();
        }
        (clicked, alpha)
    }

    #[test]
    fn s4_shadow_area_does_not_steal_hits_and_is_about_60_percent() {
        let (clicked, alpha) = shadow_probe();
        assert!(
            clicked,
            "a click on the shadow region must reach the widget underneath"
        );
        let a = alpha.expect("a translucent white fill was emitted");
        assert!(
            (i32::from(a) - 153).abs() <= 3,
            "set_opacity(0.6) gave alpha {a}, expected about 153"
        );
    }

    #[test]
    fn s5_plain_scroll_area_scrolls_with_forwarded_wheel() {
        let o = Opts {
            virtualized: false,
            forward_wheel: true,
            ..BASE
        };
        let (mut rig, s) = start_drag(o);
        assert!(s.dragged_id.is_some());
        let r3_before = s.rows.iter().find(|(i, _)| *i == 3).unwrap().1.top();
        let mut last = s;
        for _ in 0..8 {
            last = rig.frame(wheel(-60.0), o);
        }
        assert!(
            last.offset > 50.0,
            "forwarding must scroll the plain area, {}",
            last.offset
        );
        let r3_after = last.rows.iter().find(|(i, _)| *i == 3).unwrap().1.top();
        assert!(
            ((r3_before - r3_after) - last.offset).abs() < 1.0,
            "recorded rects shift by the scrolled amount"
        );
    }
}
