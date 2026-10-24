//! Pure drag geometry for the RULES list and the CONFIG style table (drag-and-drop reorder).
//!
//! No GUI-toolkit types outside the tests: slot maths, auto-scroll speed and shadow clamping are
//! plain arithmetic so every rule of the old arrows (economy boundary, banners, collapsed headers,
//! built-ins first) is testable without a window.

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
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(VIEW_W, VIEW_H + 100.0))),
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
                    let row_resp = ui.interact(
                        rect,
                        Id::new(("row", o.salt, i)),
                        Sense::click(),
                    );
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
                    let wheel = if o.forward_wheel { ui.input(|i| i.smooth_scroll_delta.y) } else { 0.0 };
                    let auto = if o.autoscroll {
                        let y = ui.input(|i| i.pointer.latest_pos()).map_or(0.0, |p| p.y);
                        auto_speed(y, top, bottom)
                    } else {
                        0.0
                    };
                    ui.scroll_with_delta_animation(
                        vec2(0.0, wheel - auto * DT),
                        ScrollAnimation::none(),
                    );
                };
                if o.virtualized {
                    ScrollArea::vertical().auto_shrink([false, false]).show_viewport(
                        ui,
                        |ui, viewport| {
                            seen.offset = viewport.min.y;
                            let origin = ui.max_rect().min.y;
                            ui.set_height(ROWS as f32 * ROW_H);
                            let offsets: Vec<f32> =
                                (0..=ROWS).map(|i| i as f32 * ROW_H).collect();
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
                        },
                    );
                } else {
                    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        for i in 0..ROWS {
                            let (rect, _) =
                                ui.allocate_exact_size(vec2(VIEW_W - 20.0, ROW_H), Sense::hover());
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

    /// Inline auto-scroll speed (signed pt/s, positive = down) until `autoscroll_speed` lands.
    fn auto_speed(y: f32, top: f32, bottom: f32) -> f32 {
        let zone = 40.0;
        let up = ((top + zone - y) / zone).clamp(0.0, 1.0);
        let down = ((y - (bottom - zone)) / zone).clamp(0.0, 1.0);
        (down - up) * 900.0
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

    const BASE: Opts = Opts { virtualized: true, forward_wheel: false, autoscroll: false, salt: 0 };

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
        assert_eq!(last.offset, 0.0, "egui gates the wheel off while a widget is dragged");
    }

    #[test]
    fn s1b_forwarded_wheel_scrolls_during_a_drag() {
        let o = Opts { forward_wheel: true, ..BASE };
        let (mut rig, s) = start_drag(o);
        assert!(s.dragged_id.is_some());
        let mut last = s;
        for _ in 0..8 {
            last = rig.frame(wheel(-60.0), o);
        }
        assert!(last.offset > 50.0, "forwarded wheel must scroll, offset {}", last.offset);
    }

    #[test]
    fn s1c_edge_autoscroll_moves_about_speed_times_dt() {
        let o = Opts { autoscroll: true, ..BASE };
        let (mut rig, s) = start_drag(o);
        assert!(s.dragged_id.is_some());
        let y = VIEW_H - 5.0;
        let before = rig.frame(vec![Event::PointerMoved(pos2(300.0, y))], o).offset;
        let n = 20;
        let mut last = before;
        for _ in 0..n {
            last = rig.frame(vec![], o).offset;
        }
        let speed = auto_speed(y, 0.0, VIEW_H);
        let expected = speed * DT * n as f32;
        let got = last - before;
        assert!(
            (got - expected).abs() <= expected * 0.25,
            "moved {got}, expected about {expected}"
        );
    }

    #[test]
    fn s2_drag_survives_virtualization_and_id_changes() {
        let o = Opts { autoscroll: true, ..BASE };
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
        assert_eq!(stops, 1, "drag_stopped_id must report the captured id exactly once");
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
        assert!(!any_row_click, "a tap on the handle must not reach the row's click");
        assert!(handle_clicked, "the handle owns the click (ignored by the list)");
    }

    #[test]
    fn s3_escape_aborts_the_drag_without_a_release() {
        let (mut rig, s) = start_drag(BASE);
        let captured = s.dragged_id.expect("dragging");
        let s = rig.frame(escape(), BASE);
        assert_eq!(s.dragged_id, None);
        assert_eq!(s.drag_stopped_id, Some(captured));
        assert!(!s.primary_released);
        assert!(!s.escape_after_consume, "consume_key hides Escape from later widgets");
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
        let frames: Vec<Vec<Event>> = vec![
            vec![],
            press(click_at),
            release(click_at),
            vec![],
        ];
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
        assert!(clicked, "a click on the shadow region must reach the widget underneath");
        let a = alpha.expect("a translucent white fill was emitted");
        assert!(
            (i32::from(a) - 153).abs() <= 3,
            "set_opacity(0.6) gave alpha {a}, expected about 153"
        );
    }

    #[test]
    fn s5_plain_scroll_area_scrolls_with_forwarded_wheel() {
        let o = Opts { virtualized: false, forward_wheel: true, ..BASE };
        let (mut rig, s) = start_drag(o);
        assert!(s.dragged_id.is_some());
        let r3_before = s.rows.iter().find(|(i, _)| *i == 3).unwrap().1.top();
        let mut last = s;
        for _ in 0..8 {
            last = rig.frame(wheel(-60.0), o);
        }
        assert!(last.offset > 50.0, "forwarding must scroll the plain area, {}", last.offset);
        let r3_after = last.rows.iter().find(|(i, _)| *i == 3).unwrap().1.top();
        assert!(
            ((r3_before - r3_after) - last.offset).abs() < 1.0,
            "recorded rects shift by the scrolled amount"
        );
    }
}
