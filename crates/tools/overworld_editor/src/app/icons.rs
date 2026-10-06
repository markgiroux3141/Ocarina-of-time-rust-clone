//! Line icons, drawn with egui shapes on a 24-unit grid scaled into any square: the tools, the
//! document's kinds of thing, and the panels' small buttons.

use eframe::egui::{epaint::CubicBezierShape, Color32, Painter, Pos2, Rect, Shape, Stroke};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Select,
    Region,
    Path,
    Brush,
    Prop,
    Dirt,
    Fence,
    Bridge,
    Hedge,
    Tunnel,
    Outline,
    Water,
    Trash,
    Warn,
    Chevron,
    Reset,
    Info,
    Search,
    Duplicate,
    Raise,
    Lower,
    Smooth,
    Flatten,
    Bumps,
    Erase,
    Play,
    Undo,
    Redo,
    Keys,
    Leaf,
}

/// Draws `icon` into `rect` (its square middle) in `color`, `width` thick at 24 units.
pub fn paint(p: &Painter, rect: Rect, icon: Icon, color: Color32, width: f32) {
    let side = rect.width().min(rect.height());
    let k = side / 24.0;
    let o = rect.center() - eframe::egui::Vec2::splat(side / 2.0);
    let at = |x: f32, y: f32| Pos2::new(o.x + x * k, o.y + y * k);
    let st = Stroke::new(width * k.max(0.6), color);
    let faint = Stroke::new(width * k.max(0.6), color.gamma_multiply(0.38));
    let line = |pts: &[(f32, f32)], s: Stroke| {
        p.add(Shape::line(pts.iter().map(|&(x, y)| at(x, y)).collect(), s));
    };
    let closed = |pts: &[(f32, f32)], s: Stroke| {
        p.add(Shape::closed_line(pts.iter().map(|&(x, y)| at(x, y)).collect(), s));
    };
    let fill = |pts: &[(f32, f32)]| {
        p.add(Shape::convex_polygon(pts.iter().map(|&(x, y)| at(x, y)).collect(), color, Stroke::NONE));
    };
    let seg = |a: (f32, f32), b: (f32, f32)| {
        p.line_segment([at(a.0, a.1), at(b.0, b.1)], st);
    };
    let dot = |x: f32, y: f32, r: f32| {
        p.circle_filled(at(x, y), r * k, color);
    };
    let ring = |x: f32, y: f32, r: f32| {
        p.circle_stroke(at(x, y), r * k, st);
    };
    let cubic = |a: (f32, f32), b: (f32, f32), c: (f32, f32), d: (f32, f32), s: Stroke| {
        p.add(CubicBezierShape::from_points_stroke([at(a.0, a.1), at(b.0, b.1), at(c.0, c.1), at(d.0, d.1)], false, Color32::TRANSPARENT, s));
    };
    // an arc round (cx, cy) from a0 to a1 degrees (screen angles: y down)
    let arc = |cx: f32, cy: f32, r: f32, a0: f32, a1: f32| {
        let n = 16;
        let pts: Vec<Pos2> = (0..=n)
            .map(|i| {
                let a = (a0 + (a1 - a0) * i as f32 / n as f32).to_radians();
                at(cx + r * a.cos(), cy + r * a.sin())
            })
            .collect();
        p.add(Shape::line(pts, st));
    };
    let wave = |y: f32, s: Stroke| {
        let pts: Vec<(f32, f32)> = (0..=24).map(|i| (3.0 + i as f32 * 0.75, y - 1.3 * (i as f32 * 0.75 / 6.0 * std::f32::consts::TAU).sin())).collect();
        line(&pts, s);
    };
    match icon {
        Icon::Select => closed(&[(5.0, 3.5), (18.0, 10.7), (12.4, 12.3), (10.1, 17.9)], st),
        Icon::Region => {
            let pts = [(4.5, 9.0), (11.5, 4.0), (20.0, 7.0), (18.0, 18.0), (7.0, 20.0)];
            closed(&pts, st);
            for (x, y) in pts {
                dot(x, y, 1.9);
            }
        }
        Icon::Path => {
            line(&[(2.5, 19.0), (8.5, 19.0), (15.5, 9.0), (21.5, 9.0)], st);
            line(&[(8.5, 19.0), (21.5, 19.0)], faint);
        }
        Icon::Brush => {
            cubic((2.0, 20.0), (6.0, 20.0), (7.0, 11.0), (12.0, 11.0), st);
            cubic((12.0, 11.0), (17.0, 11.0), (18.0, 20.0), (22.0, 20.0), st);
            seg((12.0, 8.0), (12.0, 2.5));
            line(&[(9.5, 5.0), (12.0, 2.5), (14.5, 5.0)], st);
        }
        Icon::Prop => {
            line(&[(6.5, 20.0), (6.5, 11.0)], st);
            arc(12.0, 11.0, 5.5, 180.0, 360.0);
            line(&[(17.5, 11.0), (17.5, 20.0)], st);
            line(&[(10.5, 20.0), (10.5, 16.0)], st);
            arc(12.0, 16.0, 1.5, 180.0, 360.0);
            line(&[(13.5, 16.0), (13.5, 20.0)], st);
            seg((3.5, 20.0), (20.5, 20.0));
        }
        Icon::Dirt => {
            let thick = Stroke::new(width * 2.6 * k, color.gamma_multiply(0.35));
            cubic((3.0, 20.0), (7.0, 13.0), (13.0, 17.0), (15.0, 11.0), thick);
            cubic((15.0, 11.0), (16.2, 7.5), (18.0, 5.5), (21.0, 4.0), thick);
            for (x, y) in [(3.5, 19.3), (6.3, 16.3), (9.6, 15.6), (13.0, 14.6), (15.3, 10.3), (17.2, 7.0), (20.2, 4.5)] {
                dot(x, y, 1.15);
            }
        }
        Icon::Fence => {
            for x in [5.0, 12.0, 19.0] {
                seg((x, 5.0), (x, 20.0));
            }
            seg((2.5, 9.5), (21.5, 9.5));
            seg((2.5, 15.0), (21.5, 15.0));
        }
        Icon::Bridge => {
            seg((3.0, 6.0), (3.0, 18.0));
            seg((21.0, 6.0), (21.0, 18.0));
            cubic((3.0, 8.5), (9.0, 15.0), (15.0, 15.0), (21.0, 8.5), st);
            for (x, y0, y1) in [(6.5, 12.6, 15.8), (9.5, 14.2, 17.4), (12.5, 14.7, 17.9), (15.5, 14.2, 17.4), (18.0, 12.9, 15.9)] {
                seg((x, y0), (x, y1));
            }
        }
        Icon::Hedge => {
            line(&[(2.5, 19.0), (4.5, 12.0), (6.5, 17.0), (8.5, 8.0), (10.5, 16.0), (12.5, 6.0), (14.5, 15.0), (16.5, 8.0), (18.5, 14.0), (20.5, 9.0), (22.0, 14.0)], st);
            seg((2.0, 19.5), (22.0, 19.5));
        }
        Icon::Tunnel => {
            // a mouth in a wall: its arch, and the passage going in
            line(&[(5.0, 20.0), (5.0, 12.0)], st);
            arc(12.0, 12.0, 7.0, 180.0, 360.0);
            line(&[(19.0, 12.0), (19.0, 20.0)], st);
            line(&[(9.0, 20.0), (9.0, 14.0)], faint);
            line(&[(15.0, 14.0), (15.0, 20.0)], faint);
            seg((2.0, 20.0), (22.0, 20.0));
        }
        Icon::Outline => {
            cubic((5.0, 6.0), (9.0, 3.0), (15.0, 3.0), (19.0, 6.0), st);
            cubic((19.0, 6.0), (22.0, 9.0), (21.0, 15.0), (18.0, 18.0), st);
            cubic((18.0, 18.0), (14.0, 21.0), (8.0, 21.0), (5.0, 18.0), st);
            cubic((5.0, 18.0), (2.0, 15.0), (2.0, 9.0), (5.0, 6.0), st);
        }
        Icon::Water => {
            for y in [8.0, 13.0, 18.0] {
                wave(y, st);
            }
        }
        Icon::Trash => {
            seg((4.0, 7.0), (20.0, 7.0));
            line(&[(9.0, 7.0), (9.0, 4.0), (15.0, 4.0), (15.0, 7.0)], st);
            line(&[(6.5, 7.0), (7.5, 20.0), (16.5, 20.0), (17.5, 7.0)], st);
            seg((10.0, 11.0), (10.0, 17.0));
            seg((14.0, 11.0), (14.0, 17.0));
        }
        Icon::Warn => {
            closed(&[(12.0, 3.0), (22.0, 20.0), (2.0, 20.0)], st);
            seg((12.0, 9.5), (12.0, 14.0));
            dot(12.0, 17.0, 1.2);
        }
        Icon::Chevron => line(&[(6.0, 9.0), (12.0, 15.0), (18.0, 9.0)], st),
        Icon::Reset => {
            line(&[(4.0, 4.0), (4.0, 10.0), (10.0, 10.0)], st);
            arc(12.0, 12.0, 8.0, 200.0, 520.0);
        }
        Icon::Info => {
            ring(12.0, 12.0, 9.0);
            seg((12.0, 11.0), (12.0, 17.0));
            dot(12.0, 7.6, 1.2);
        }
        Icon::Search => {
            ring(11.0, 11.0, 6.5);
            seg((16.0, 16.0), (21.0, 21.0));
        }
        Icon::Duplicate => {
            closed(&[(8.0, 8.0), (20.0, 8.0), (20.0, 20.0), (8.0, 20.0)], st);
            line(&[(16.0, 8.0), (16.0, 4.0), (4.0, 4.0), (4.0, 16.0), (8.0, 16.0)], st);
        }
        Icon::Raise => {
            cubic((2.0, 20.0), (6.0, 20.0), (7.0, 13.0), (12.0, 13.0), st);
            cubic((12.0, 13.0), (17.0, 13.0), (18.0, 20.0), (22.0, 20.0), st);
            seg((12.0, 10.0), (12.0, 3.0));
            line(&[(9.0, 6.0), (12.0, 3.0), (15.0, 6.0)], st);
        }
        Icon::Lower => {
            cubic((2.0, 13.0), (6.0, 13.0), (7.0, 20.0), (12.0, 20.0), st);
            cubic((12.0, 20.0), (17.0, 20.0), (18.0, 13.0), (22.0, 13.0), st);
            seg((12.0, 3.0), (12.0, 10.0));
            line(&[(9.0, 7.0), (12.0, 10.0), (15.0, 7.0)], st);
        }
        Icon::Smooth => {
            wave(12.0, faint);
            cubic((2.0, 19.0), (8.0, 17.0), (16.0, 17.0), (22.0, 19.0), st);
        }
        Icon::Flatten => {
            seg((2.0, 18.0), (22.0, 18.0));
            wave(7.0, faint);
            seg((12.0, 9.0), (12.0, 15.0));
            line(&[(9.5, 12.5), (12.0, 15.0), (14.5, 12.5)], st);
        }
        Icon::Bumps => {
            cubic((2.0, 19.0), (3.5, 19.0), (4.0, 14.0), (6.0, 14.0), st);
            cubic((6.0, 14.0), (8.0, 14.0), (8.5, 19.0), (10.0, 19.0), st);
            cubic((10.0, 19.0), (11.5, 19.0), (12.0, 12.0), (14.0, 12.0), st);
            cubic((14.0, 12.0), (16.0, 12.0), (16.5, 19.0), (18.0, 19.0), st);
            cubic((18.0, 19.0), (19.5, 19.0), (20.0, 15.0), (22.0, 15.0), st);
        }
        Icon::Erase => {
            seg((8.0, 20.0), (20.0, 20.0));
            closed(&[(4.5, 14.5), (13.5, 5.5), (16.3, 5.5), (18.5, 7.7), (18.5, 10.5), (11.0, 18.0), (7.5, 18.0)], st);
            seg((9.0, 10.0), (14.0, 15.0));
        }
        Icon::Play => fill(&[(7.0, 4.0), (20.0, 12.0), (7.0, 20.0)]),
        Icon::Undo => {
            line(&[(9.0, 14.0), (4.0, 9.0), (9.0, 4.0)], st);
            seg((4.0, 9.0), (15.0, 9.0));
            arc(15.0, 14.0, 5.0, -90.0, 90.0);
            seg((15.0, 19.0), (12.0, 19.0));
        }
        Icon::Redo => {
            line(&[(15.0, 14.0), (20.0, 9.0), (15.0, 4.0)], st);
            seg((20.0, 9.0), (9.0, 9.0));
            arc(9.0, 14.0, 5.0, 270.0, 90.0);
            seg((9.0, 19.0), (12.0, 19.0));
        }
        Icon::Keys => {
            closed(&[(2.0, 6.0), (22.0, 6.0), (22.0, 19.0), (2.0, 19.0)], st);
            for x in [6.0, 10.0, 14.0, 18.0] {
                dot(x, 10.5, 1.1);
            }
            seg((7.0, 15.0), (17.0, 15.0));
        }
        Icon::Leaf => {
            cubic((4.0, 20.0), (3.0, 9.0), (11.0, 3.5), (21.0, 3.0), st);
            cubic((21.0, 3.0), (21.5, 13.5), (14.5, 21.0), (4.0, 20.0), st);
            seg((4.0, 20.0), (14.0, 10.0));
        }
    }
}
