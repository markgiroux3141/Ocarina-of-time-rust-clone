mod common;

use common::*;
use oot_actors::PlayExt;
use eng_input::pad::BTN_A;
use glam::Vec3;

fn show(name: &str, pos: Vec3, yaw: i16, s: &[eng_input::pad::PadState], every: usize) {
    let Some(mut w) = world_at(pos, yaw) else { return };
    let f = run(&mut w, s);
    println!("=== {name}");
    let sel: Vec<_> = f.iter().enumerate().filter(|(i, _)| i % every == 0 || *i + 1 == f.len()).map(|(_, x)| x.clone()).collect();
    dump(&sel);
    for n in &w.player().notes {
        println!("  note: {n}");
    }
}

#[test]
#[ignore]
fn smoke_scenarios() {
    // Camera starts behind Link, so stick up = Link's facing.
    let mut s = repeat(stick(0, 0), 2);
    s.extend(repeat(stick(0, 80), 45));
    s.extend(repeat(stick(0, 0), 25));
    show("run off plateau (+x)", Vec3::new(-700.0, 150.0, -700.0), 0x4000, &s, 1);

    let mut s = repeat(stick(0, 0), 2);
    s.extend(repeat(stick(0, 30), 80));
    s.extend(repeat(stick(0, 0), 30));
    show("walk off plateau slowly", Vec3::new(-700.0, 150.0, -700.0), 0x4000, &s, 2);

    let mut s = repeat(stick(0, 80), 40);
    s.extend(repeat(stick(0, 0), 60));
    show("run into pit (fall 450)", Vec3::new(600.0, 0.0, -300.0), -0x8000, &s, 2);

    let s = repeat(stick(0, 80), 60);
    show("up ramp A", Vec3::new(-700.0, 0.0, 100.0), -0x8000, &s, 3);

    let s = repeat(stick(0, 80), 50);
    show("up stairs", Vec3::new(150.0, 0.0, -300.0), -0x8000, &s, 2);

    let s = repeat(stick(0, 80), 40);
    show("into tall block", Vec3::new(500.0, 0.0, 600.0), -0x8000, &s, 3);

    let s = repeat(stick(0, 80), 60);
    show("into diagonal wall", Vec3::new(-300.0, 0.0, 600.0), -0x8000, &s, 3);

    let mut s = repeat(stick(-30, 0), 20);
    s.extend(repeat(stick(0, 0), 20));
    show("small stick left (turn in place)", Vec3::ZERO, -0x8000, &s, 1);

    let mut s = repeat(stick(0, 80), 20);
    s.push(with(stick(0, 80), BTN_A));
    s.extend(repeat(stick(0, 80), 20));
    show("roll in open", Vec3::new(-300.0, 0.0, 900.0), -0x8000, &s, 1);
}
