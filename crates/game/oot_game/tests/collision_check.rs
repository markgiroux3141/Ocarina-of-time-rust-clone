//! The collision check (`oot_game::collision_check`) against `z_collision_check.c`: OC push-out
//! by mass (`CollisionCheck_SetOCvsOC`), AT vs AC hits and what they record on both sides
//! (`CollisionCheck_SetATvsAC`), damage with and without a damage table
//! (`CollisionCheck_ApplyDamage`), `TOUCH_NEAREST` quads (`Collider_QuadSetNearestAC`), and the
//! rules that skip a pair. Expected values are worked out from the C in the comments. No asset
//! pack needed: the checks run on an `ActorContext`.

use eng_collision::math3d::Cylinder16;
use glam::Vec3;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_NPC, ACTORCAT_PLAYER, ACTORCAT_PROP, ActorContext, ActorHandle, ActorImpl};
use oot_game::collision_check::*;

/// A test actor with one collider.
struct Holder {
    actor: Actor,
    col: Collider,
}

impl ActorImpl for Holder {
    fn name(&self) -> &'static str {
        "Holder"
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    fn update(&mut self, _play: &mut oot_game::play::PlayState) {}
    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        if id != 0 {
            return None;
        }
        Some(match &mut self.col {
            Collider::JntSph(c) => ColliderMut::JntSph(c),
            Collider::Cylinder(c) => ColliderMut::Cylinder(c),
            Collider::Tris(c) => ColliderMut::Tris(c),
            Collider::Quad(c) => ColliderMut::Quad(c),
        })
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

fn holder(actors: &mut ActorContext, cat: usize, pos: Vec3, mass: u8, col: Collider) -> ActorHandle {
    let mut actor = Actor::new(pos, 0);
    actor.category = cat;
    actor.col_chk_info.mass = mass;
    actors.insert(Box::new(Holder { actor, col })).unwrap()
}

fn col(actors: &ActorContext, h: ActorHandle) -> &Collider {
    &actors.downcast::<Holder>(h).unwrap().col
}

fn cyl(c: &Collider) -> &ColliderCylinder {
    match c {
        Collider::Cylinder(c) => c,
        _ => panic!("not a cylinder"),
    }
}

fn quad(c: &Collider) -> &ColliderQuad {
    match c {
        Collider::Quad(c) => c,
        _ => panic!("not a quad"),
    }
}

/// Registers actor `h`'s collider as AT, AC and/or OC, as its update would.
fn register(ctx: &mut CollisionCheckContext, actors: &mut ActorContext, h: ActorHandle, at: bool, ac: bool, oc: bool) -> [i32; 3] {
    let mut a = actors.take(h).unwrap();
    let base = a.base().clone();
    let holder = a.as_any_mut().downcast_mut::<Holder>().unwrap();
    let mut out = [-1; 3];
    macro_rules! reg {
        ($c:expr) => {{
            if at {
                out[0] = ctx.set_at(Some(h), &base, 0, $c);
            }
            if ac {
                out[1] = ctx.set_ac(Some(h), &base, 0, $c);
            }
            if oc {
                out[2] = ctx.set_oc(Some(h), &base, 0, $c);
            }
        }};
    }
    match &mut holder.col {
        Collider::Cylinder(c) => reg!(c),
        Collider::Quad(c) => reg!(c),
        Collider::JntSph(c) => reg!(c),
        Collider::Tris(c) => reg!(c),
    }
    actors.put_back(h, a);
    out
}

/// En_Ko's body (`sCylinderInit` in `z_en_ko.c`): OC only, `OC1_TYPE_ALL`, `OC2_TYPE_2`.
fn en_ko_cylinder(pos: [i16; 3]) -> Collider {
    let init = ColliderCylinderInit {
        base: ColliderInit { col_type: COLTYPE_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
        info: ColliderInfoInit {
            elem_type: ELEMTYPE_UNK0,
            toucher: ColliderTouch { dmg_flags: 0, effect: 0, damage: 0 },
            bumper: ColliderBumpInit { dmg_flags: 0, effect: 0, defense: 0 },
            toucher_flags: TOUCH_NONE,
            bumper_flags: BUMP_NONE,
            oc_elem_flags: OCELEM_ON,
        },
        dim: Cylinder16 { radius: 20, height: 46, y_shift: 0, pos },
    };
    Collider::Cylinder(ColliderCylinder::new(&init))
}

/// Player's body (`D_80854624` in `z_player.c`): AC from enemies, OC with everything,
/// `OC2_TYPE_PLAYER`.
fn player_cylinder(pos: [i16; 3]) -> Collider {
    let init = ColliderCylinderInit {
        base: ColliderInit { col_type: COLTYPE_HIT5, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_ENEMY, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_PLAYER, shape: COLSHAPE_CYLINDER },
        info: ColliderInfoInit {
            elem_type: ELEMTYPE_UNK1,
            toucher: ColliderTouch { dmg_flags: 0, effect: 0, damage: 0 },
            bumper: ColliderBumpInit { dmg_flags: 0xFFCF_FFFF, effect: 0, defense: 0 },
            toucher_flags: TOUCH_NONE,
            bumper_flags: BUMP_ON,
            oc_elem_flags: OCELEM_ON,
        },
        dim: Cylinder16 { radius: 12, height: 60, y_shift: 0, pos },
    };
    Collider::Cylinder(ColliderCylinder::new(&init))
}

/// En_Kusa's bush (`sCylinderInit` in `z_en_kusa.c`): AC from Player, bumper `0x4FC00758`.
fn bush_cylinder(pos: [i16; 3]) -> Collider {
    let init = ColliderCylinderInit {
        base: ColliderInit {
            col_type: COLTYPE_NONE,
            at_flags: AT_NONE,
            ac_flags: AC_ON | AC_TYPE_PLAYER,
            oc_flags1: OC1_ON | OC1_TYPE_PLAYER | OC1_TYPE_2,
            oc_flags2: OC2_TYPE_2,
            shape: COLSHAPE_CYLINDER,
        },
        info: ColliderInfoInit {
            elem_type: ELEMTYPE_UNK0,
            toucher: ColliderTouch { dmg_flags: 0, effect: 0, damage: 0 },
            bumper: ColliderBumpInit { dmg_flags: 0x4FC0_0758, effect: 0, defense: 0 },
            toucher_flags: TOUCH_NONE,
            bumper_flags: BUMP_ON,
            oc_elem_flags: OCELEM_ON,
        },
        dim: Cylinder16 { radius: 12, height: 44, y_shift: 0, pos },
    };
    Collider::Cylinder(ColliderCylinder::new(&init))
}

/// Player's sword quad (`D_80854650`), with the Kokiri Sword's flags as `func_80837918` sets
/// them for a slash: `DMG_SLASH_KOKIRI`, damage 1, `TOUCH_ON | TOUCH_NEAREST`.
fn sword_quad(q: [Vec3; 4]) -> Collider {
    let init = ColliderQuadInit {
        base: ColliderInit { col_type: COLTYPE_NONE, at_flags: AT_ON | AT_TYPE_PLAYER, ac_flags: AC_NONE, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_PLAYER, shape: COLSHAPE_QUAD },
        info: ColliderInfoInit {
            elem_type: ELEMTYPE_UNK2,
            toucher: ColliderTouch { dmg_flags: DMG_SLASH_KOKIRI, effect: 0, damage: 1 },
            bumper: ColliderBumpInit { dmg_flags: 0xFFCF_FFFF, effect: 0, defense: 0 },
            toucher_flags: TOUCH_ON | TOUCH_NEAREST,
            bumper_flags: BUMP_NONE,
            oc_elem_flags: OCELEM_NONE,
        },
        quad: q,
    };
    Collider::Quad(ColliderQuad::new(&init))
}

/// A blade 100 long along x at height y, 20 tall, in the plane z = 0: quad[0], [1] the base
/// edge (y), [2], [3] the top edge (y + 20), as `func_80090480` passes new base / tip and old
/// base / tip.
fn blade(x0: f32, x1: f32, y: f32) -> [Vec3; 4] {
    [Vec3::new(x0, y, 0.0), Vec3::new(x1, y, 0.0), Vec3::new(x0, y + 20.0, 0.0), Vec3::new(x1, y + 20.0, 0.0)]
}

#[test]
fn oc_pushes_equal_masses_apart_by_half_the_overlap_each() {
    let mut actors = ActorContext::default();
    let mut ctx = CollisionCheckContext::default();
    // Two mass-50 bodies 30 apart with radii 20 and 12: Math3D_CylOutsideCylDist gives deadSpace
    // 20 + 12 - 30 = 2. Both MASSTYPE_NORMAL: leftDispRatio = rightMass / total = 0.5, and the
    // same for the right. xDelta = 30 * (2 / 30) = 2, so left moves -1 and right +1 in x.
    let a = holder(&mut actors, ACTORCAT_NPC, Vec3::ZERO, 50, en_ko_cylinder([0, 0, 0]));
    let b = holder(&mut actors, ACTORCAT_PLAYER, Vec3::new(30.0, 0.0, 0.0), 50, player_cylinder([30, 0, 0]));
    assert_eq!(register(&mut ctx, &mut actors, a, false, false, true), [-1, -1, 0]);
    assert_eq!(register(&mut ctx, &mut actors, b, false, false, true), [-1, -1, 1]);
    ctx.check(&mut actors);
    assert_eq!(actors.actor(a).unwrap().col_chk_info.displacement, Vec3::new(-1.0, 0.0, 0.0));
    assert_eq!(actors.actor(b).unwrap().col_chk_info.displacement, Vec3::new(1.0, 0.0, 0.0));
    // Both marked, each pointing at the other; the NPC saw Player (OC2_TYPE_PLAYER).
    let (ca, cb) = (cyl(col(&actors, a)), cyl(col(&actors, b)));
    assert_eq!((ca.base.oc_flags1 & OC1_HIT, ca.base.oc, ca.base.oc_flags2 & OC2_HIT_PLAYER), (OC1_HIT, Some(b), OC2_HIT_PLAYER));
    assert_eq!((cb.base.oc_flags1 & OC1_HIT, cb.base.oc, cb.base.oc_flags2 & OC2_HIT_PLAYER), (OC1_HIT, Some(a), 0));
    assert_eq!(ca.info.oc_elem_flags & OCELEM_HIT, OCELEM_HIT);
    // Registering again (the next frame's update) clears the hit (Collider_ResetCylinderOC).
    register(&mut ctx, &mut actors, a, false, false, true);
    let ca = cyl(col(&actors, a));
    assert_eq!((ca.base.oc_flags1 & OC1_HIT, ca.base.oc, ca.info.oc_elem_flags & OCELEM_HIT), (0, None, 0));
}

#[test]
fn oc_moves_only_the_movable_side_of_an_immovable_pair() {
    let mut actors = ActorContext::default();
    let mut ctx = CollisionCheckContext::default();
    // En_Ko is MASS_IMMOVABLE (sColChkInfoInit). The left actor is immovable, so
    // rightMassType (read from the left actor) is IMMOVABLE and the ratios are 0 and 1: Player
    // takes the whole overlap, 2.
    let a = holder(&mut actors, ACTORCAT_NPC, Vec3::ZERO, MASS_IMMOVABLE, en_ko_cylinder([0, 0, 0]));
    let b = holder(&mut actors, ACTORCAT_PLAYER, Vec3::new(0.0, 0.0, 30.0), 50, player_cylinder([0, 0, 30]));
    register(&mut ctx, &mut actors, a, false, false, true);
    register(&mut ctx, &mut actors, b, false, false, true);
    ctx.check(&mut actors);
    assert_eq!(actors.actor(a).unwrap().col_chk_info.displacement, Vec3::ZERO);
    assert_eq!(actors.actor(b).unwrap().col_chk_info.displacement, Vec3::new(0.0, 0.0, 2.0));
    // Two immovable bodies are marked but not pushed.
    let mut actors = ActorContext::default();
    let mut ctx = CollisionCheckContext::default();
    let a = holder(&mut actors, ACTORCAT_NPC, Vec3::ZERO, MASS_IMMOVABLE, en_ko_cylinder([0, 0, 0]));
    let b = holder(&mut actors, ACTORCAT_NPC, Vec3::new(10.0, 0.0, 0.0), MASS_IMMOVABLE, en_ko_cylinder([10, 0, 0]));
    register(&mut ctx, &mut actors, a, false, false, true);
    register(&mut ctx, &mut actors, b, false, false, true);
    ctx.check(&mut actors);
    assert_eq!(actors.actor(a).unwrap().col_chk_info.displacement, Vec3::ZERO);
    assert_eq!(cyl(col(&actors, b)).base.oc_flags1 & OC1_HIT, OC1_HIT);
}

#[test]
fn oc_skips_incompatible_types_and_bodies_apart_in_height() {
    let mut actors = ActorContext::default();
    let mut ctx = CollisionCheckContext::default();
    // A bush's OC1 has TYPE_PLAYER | TYPE_2, and En_Ko's OC2 is TYPE_2: compatible. But a bush
    // 100 above: En_Ko's top is 46, below the bush's bottom, so Math3D_CylOutsideCylDist fails.
    let a = holder(&mut actors, ACTORCAT_NPC, Vec3::ZERO, 50, en_ko_cylinder([0, 0, 0]));
    let b = holder(&mut actors, ACTORCAT_PROP, Vec3::new(0.0, 100.0, 0.0), 50, bush_cylinder([0, 100, 0]));
    register(&mut ctx, &mut actors, a, false, false, true);
    register(&mut ctx, &mut actors, b, false, false, true);
    ctx.check(&mut actors);
    assert_eq!(cyl(col(&actors, a)).base.oc_flags1 & OC1_HIT, 0);
    // Player's OC2 is TYPE_PLAYER; a collider without OC1_TYPE_PLAYER can't touch it
    // (CollisionCheck_Incompatible): a bush that cleared the flag, as En_Kusa_Regrow does.
    let mut actors = ActorContext::default();
    let mut ctx = CollisionCheckContext::default();
    let mut bush = bush_cylinder([0, 0, 0]);
    bush.base_mut().oc_flags1 &= !OC1_TYPE_PLAYER;
    let a = holder(&mut actors, ACTORCAT_PROP, Vec3::ZERO, MASS_IMMOVABLE, bush);
    let b = holder(&mut actors, ACTORCAT_PLAYER, Vec3::new(10.0, 0.0, 0.0), 50, player_cylinder([10, 0, 0]));
    register(&mut ctx, &mut actors, a, false, false, true);
    register(&mut ctx, &mut actors, b, false, false, true);
    ctx.check(&mut actors);
    assert_eq!(actors.actor(b).unwrap().col_chk_info.displacement, Vec3::ZERO);
}

#[test]
fn a_sword_quad_hits_a_bush_and_both_sides_record_it() {
    let mut actors = ActorContext::default();
    let mut ctx = CollisionCheckContext::default();
    let bush = holder(&mut actors, ACTORCAT_PROP, Vec3::ZERO, MASS_IMMOVABLE, bush_cylinder([0, 0, 0]));
    let link = holder(&mut actors, ACTORCAT_PLAYER, Vec3::new(0.0, 0.0, -30.0), 50, sword_quad(blade(-50.0, 50.0, 10.0)));
    register(&mut ctx, &mut actors, link, true, false, false);
    register(&mut ctx, &mut actors, bush, false, true, false);
    ctx.check(&mut actors);
    // The quad's first triangle (quad 2, 3, 1) has the edge from (-50, 30, 0) to (50, 30, 0),
    // which crosses the cylinder's wall (radius 12, height 0..44) at x = -12 and 12:
    // Math3D_CylTriVsIntersect returns one of those points.
    let b = cyl(col(&actors, bush));
    assert_eq!(b.base.ac_flags & AC_HIT, AC_HIT);
    assert_eq!(b.base.ac, Some(link));
    assert_eq!(b.info.bumper_flags & BUMP_HIT, BUMP_HIT);
    let hp = b.info.bumper.hit_pos;
    assert!(hp[1] == 30 && hp[0].abs() == 12 && hp[2] == 0, "hit at {hp:?}");
    let hit = b.info.ac_hit_info.expect("acHitInfo");
    assert_eq!(hit.toucher.dmg_flags, DMG_SLASH_KOKIRI);
    assert_eq!(hit.elem.col.actor, link);
    // COLTYPE_NONE isn't METAL, WOOD or HARD, and the sword has no TOUCH_AT_HITMARK: the hit
    // mark is left to CollisionCheck_SetHitEffects (BUMP_DRAW_HITMARK), which then marks the
    // sword's element TOUCH_DREW_HITMARK.
    assert_eq!(b.info.bumper_flags & BUMP_DRAW_HITMARK, BUMP_DRAW_HITMARK);
    let q = quad(col(&actors, link));
    assert_eq!((q.base.at_flags & AT_HIT, q.base.at), (AT_HIT, Some(bush)));
    assert_eq!(q.info.toucher_flags & (TOUCH_HIT | TOUCH_DREW_HITMARK), TOUCH_HIT | TOUCH_DREW_HITMARK);
    assert_eq!(q.info.at_hit.map(|r| r.actor), Some(bush));
    // No damage table: damage = toucher.damage 1 - defense 0.
    assert_eq!(actors.actor(bush).unwrap().col_chk_info.damage, 1);
}

#[test]
fn damage_comes_from_the_table_entry_of_the_toucher_bit() {
    static TABLE: DamageTable = {
        let mut t = [0u8; 32];
        // DMG_ENTRY(2, 0x1) for bit 8, DMG_SLASH_KOKIRI.
        t[8] = 2 | (1 << 4);
        DamageTable { table: t }
    };
    let mut actors = ActorContext::default();
    let mut ctx = CollisionCheckContext::default();
    let bush = holder(&mut actors, ACTORCAT_PROP, Vec3::ZERO, MASS_IMMOVABLE, bush_cylinder([0, 0, 0]));
    actors.actor_mut(bush).unwrap().col_chk_info.damage_table = Some(&TABLE);
    let link = holder(&mut actors, ACTORCAT_PLAYER, Vec3::new(0.0, 0.0, -30.0), 50, sword_quad(blade(-50.0, 50.0, 10.0)));
    register(&mut ctx, &mut actors, link, true, false, false);
    register(&mut ctx, &mut actors, bush, false, true, false);
    ctx.check(&mut actors);
    let info = actors.actor(bush).unwrap().col_chk_info;
    assert_eq!((info.damage, info.damage_effect), (2, 1));
}

#[test]
fn a_nearest_quad_keeps_only_its_nearest_hit() {
    // The blade crosses two bushes, one at x = -30 and one at x = 30. Its dcMid (the midpoint of
    // quad[3] and quad[2]) is at x = 40, nearer the second. Whichever order the bushes come in,
    // Collider_QuadSetNearestAC leaves only the nearer one hit: registered first, the far bush
    // is hit and then reset when the near one is closer; registered second, it isn't nearer.
    for near_first in [false, true] {
        let mut actors = ActorContext::default();
        let mut ctx = CollisionCheckContext::default();
        let far = holder(&mut actors, ACTORCAT_PROP, Vec3::new(-30.0, 0.0, 0.0), MASS_IMMOVABLE, bush_cylinder([-30, 0, 0]));
        let near = holder(&mut actors, ACTORCAT_PROP, Vec3::new(30.0, 0.0, 0.0), MASS_IMMOVABLE, bush_cylinder([30, 0, 0]));
        let link = holder(&mut actors, ACTORCAT_PLAYER, Vec3::new(0.0, 0.0, -30.0), 50, sword_quad([Vec3::new(-70.0, 10.0, 0.0), Vec3::new(50.0, 10.0, 0.0), Vec3::new(30.0, 30.0, 0.0), Vec3::new(50.0, 30.0, 0.0)]));
        register(&mut ctx, &mut actors, link, true, false, false);
        let order = if near_first { [near, far] } else { [far, near] };
        for h in order {
            register(&mut ctx, &mut actors, h, false, true, false);
        }
        ctx.check(&mut actors);
        let (f, n) = (cyl(col(&actors, far)), cyl(col(&actors, near)));
        assert_eq!(n.base.ac_flags & AC_HIT, AC_HIT, "near bush hit (near first: {near_first})");
        assert_eq!(f.base.ac_flags & AC_HIT, 0, "far bush not hit (near first: {near_first})");
        assert!(f.info.ac_hit_info.is_none());
        assert_eq!(quad(col(&actors, link)).base.at, Some(near));
        assert_eq!(actors.actor(far).unwrap().col_chk_info.damage, 0);
    }
}

#[test]
fn pairs_are_skipped_by_type_by_owner_and_by_death() {
    // AT_TYPE_PLAYER against Player's own body (AC_TYPE_ENEMY): no shared type.
    let mut actors = ActorContext::default();
    let mut ctx = CollisionCheckContext::default();
    let body = holder(&mut actors, ACTORCAT_PLAYER, Vec3::ZERO, 50, player_cylinder([0, 0, 0]));
    let link = holder(&mut actors, ACTORCAT_PLAYER, Vec3::ZERO, 50, sword_quad(blade(-50.0, 50.0, 10.0)));
    register(&mut ctx, &mut actors, link, true, false, false);
    register(&mut ctx, &mut actors, body, false, true, false);
    ctx.check(&mut actors);
    assert_eq!(cyl(col(&actors, body)).base.ac_flags & AC_HIT, 0);

    // A killed actor's registration is refused (actor->update == NULL).
    let mut actors = ActorContext::default();
    let mut ctx = CollisionCheckContext::default();
    let bush = holder(&mut actors, ACTORCAT_PROP, Vec3::ZERO, 50, bush_cylinder([0, 0, 0]));
    actors.actor_mut(bush).unwrap().kill();
    assert_eq!(register(&mut ctx, &mut actors, bush, false, true, true), [-1, -1, -1]);
    assert!(ctx.col_ac.is_empty() && ctx.col_oc.is_empty());

    // An AC collider killed after registering is skipped by CollisionCheck_AC.
    let mut actors = ActorContext::default();
    let mut ctx = CollisionCheckContext::default();
    let bush = holder(&mut actors, ACTORCAT_PROP, Vec3::ZERO, 50, bush_cylinder([0, 0, 0]));
    let link = holder(&mut actors, ACTORCAT_PLAYER, Vec3::ZERO, 50, sword_quad(blade(-50.0, 50.0, 10.0)));
    register(&mut ctx, &mut actors, link, true, false, false);
    register(&mut ctx, &mut actors, bush, false, true, false);
    actors.actor_mut(bush).unwrap().kill();
    ctx.check(&mut actors);
    assert_eq!(cyl(col(&actors, bush)).base.ac_flags & AC_HIT, 0);
}

#[test]
fn the_context_holds_at_most_the_game_s_counts() {
    let mut actors = ActorContext::default();
    let mut ctx = CollisionCheckContext::default();
    let hs: Vec<_> = (0..COLLISION_CHECK_OC_MAX + 1).map(|i| holder(&mut actors, ACTORCAT_PROP, Vec3::new(i as f32 * 100.0, 0.0, 0.0), 50, bush_cylinder([i as i16 * 100, 0, 0]))).collect();
    for (i, &h) in hs.iter().enumerate() {
        let r = register(&mut ctx, &mut actors, h, false, false, true);
        assert_eq!(r[2], if i < COLLISION_CHECK_OC_MAX { i as i32 } else { -1 });
    }
    ctx.clear();
    assert!(ctx.col_oc.is_empty());
}
