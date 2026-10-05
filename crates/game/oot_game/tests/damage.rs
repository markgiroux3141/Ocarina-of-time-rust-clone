//! Damage and health against the C (GAME-05 milestone 2): `Health_ChangeBy` (`z_parameter.c`),
//! `Actor_ApplyDamage` and `Actor_SetColorFilter` (`z_actor.c`), the damage table's lookup and
//! `CollisionCheck_ApplyDamage`'s per-shape calls (`z_collision_check.c`), the fog of
//! `Gfx_SetFog` (`z_rcp.c`) and `gSPFogPosition` (`gbi.h`) that the hit flash and the colour
//! filter draw with, and `Inventory_ConsumeFairy` and `Inventory_DeleteEquipment`. Each
//! expectation is worked out from the C in its comment. No asset pack needed.

use eng_collision::math3d::{Cylinder16, Sphere16};
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_PLAYER, ActorContext, ActorHandle, ActorImpl};
use oot_game::collision_check::*;
use oot_game::item::*;
use oot_game::save::SaveContext;

/// A save with `capacity` and `health` (in sixteenths of a heart).
fn save(capacity: i16, health: i16) -> SaveContext {
    let mut s = SaveContext::new(0, false, 0);
    s.health_capacity = capacity;
    s.health = health;
    s
}

#[test]
fn health_change_by_adds_and_clamps_and_says_when_its_gone() {
    // Health_ChangeBy: health += amount, then health > healthCapacity → healthCapacity, and
    // health <= 0 → 0 and false.
    let mut s = save(0x30, 0x30);
    assert!(health_change_by(&mut s, None, -0x10));
    assert_eq!(s.health, 0x20);
    assert!(health_change_by(&mut s, None, -4));
    assert_eq!(s.health, 0x1C);
    // A gain past the capacity is clamped (and plays NA_SE_SY_HP_RECOVER).
    assert!(health_change_by(&mut s, None, 0x40));
    assert_eq!(s.health, 0x30);
    // Exactly to 0: false.
    assert!(!health_change_by(&mut s, None, -0x30));
    assert_eq!(s.health, 0);
    // Past 0: still 0.
    let mut s = save(0x30, 8);
    assert!(!health_change_by(&mut s, None, -0x10));
    assert_eq!(s.health, 0);
}

#[test]
fn double_defense_halves_damage_by_an_arithmetic_shift() {
    // amount >>= 1 on an s16 with isDoubleDefenseAcquired and amount < 0: -16 → -8, -1 → -1
    // (the shift keeps the sign: a quarter heart stays a quarter heart), -7 → -4.
    for (amount, taken) in [(-16, 8), (-1, 1), (-7, 4)] {
        let mut s = save(0x30, 0x30);
        s.is_double_defense_acquired = true;
        assert!(health_change_by(&mut s, None, amount));
        assert_eq!(0x30 - s.health, taken, "{amount}");
    }
    // A gain isn't halved.
    let mut s = save(0x30, 0x10);
    s.is_double_defense_acquired = true;
    health_change_by(&mut s, None, 0x10);
    assert_eq!(s.health, 0x20);
}

#[test]
fn actor_apply_damage_takes_the_frames_damage_down_to_zero() {
    // Actor_ApplyDamage: health <= damage → 0, else health -= damage; returns health.
    let mut a = Actor::new(Vec3::ZERO, 0);
    a.col_chk_info.health = 2;
    a.col_chk_info.damage = 1;
    assert_eq!(a.apply_damage(), 1);
    a.col_chk_info.damage = 1;
    assert_eq!(a.apply_damage(), 0);
    a.col_chk_info.health = 4;
    a.col_chk_info.damage = 8;
    assert_eq!(a.apply_damage(), 0);
}

#[test]
fn dmg_entry_packs_damage_and_reaction() {
    // DMG_ENTRY(damage, reaction): damage | (reaction << 4).
    assert_eq!(dmg_entry(1, 0xF), 0xF1);
    assert_eq!(dmg_entry(4, 0), 0x04);
    assert_eq!(dmg_entry(0, 1), 0x10);
}

#[test]
fn the_damage_tables_lookup_is_by_the_flags_bit() {
    let mut t = DamageTable { table: [0; 32] };
    for (i, e) in t.table.iter_mut().enumerate() {
        *e = i as u8;
    }
    // The loop shifts the flags down until they're 1: the bit's index.
    assert_eq!(t.lookup(DMG_DEKU_NUT), 0);
    assert_eq!(t.lookup(DMG_SLASH_KOKIRI), 8);
    assert_eq!(t.lookup(DMG_JUMP_MASTER), 27);
    assert_eq!(t.lookup(DMG_UNKNOWN_2), 31);
    // Several flags: the lower ones are shifted out, so the highest one's entry
    // (DMG_SLASH_MASTER's 9, DMG_SPIN_KOKIRI's 22).
    assert_eq!(t.lookup(DMG_SLASH_KOKIRI | DMG_SLASH_MASTER), 9);
    assert_eq!(t.lookup(DMG_DEKU_NUT | DMG_SPIN_KOKIRI), 22);
    // @bug (game): no flag never shifts down to 1, so i reaches 32 and the C reads the byte
    // after the table; that reads 0 here.
    assert_eq!(t.lookup(0), 0);
}

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

fn holder(actors: &mut ActorContext, cat: usize, pos: Vec3, col: Collider) -> ActorHandle {
    let mut actor = Actor::new(pos, 0);
    actor.category = cat;
    actors.insert(Box::new(Holder { actor, col })).unwrap()
}

fn register(ctx: &mut CollisionCheckContext, actors: &mut ActorContext, h: ActorHandle, at: bool) {
    let mut a = actors.take(h).unwrap();
    let base = a.base().clone();
    let holder = a.as_any_mut().downcast_mut::<Holder>().unwrap();
    match &mut holder.col {
        Collider::Cylinder(c) => {
            if at {
                ctx.set_at(Some(h), &base, 0, c);
            } else {
                ctx.set_ac(Some(h), &base, 0, c);
            }
        }
        Collider::JntSph(c) => {
            if at {
                ctx.set_at(Some(h), &base, 0, c);
            } else {
                ctx.set_ac(Some(h), &base, 0, c);
            }
        }
        _ => unreachable!(),
    }
    actors.put_back(h, a);
}

/// An AT cylinder with `dmg_flags` and `damage`, `hit_special_effect`.
fn attacker(pos: [i16; 3], dmg_flags: u32, damage: u8, effect: u8) -> Collider {
    Collider::Cylinder(ColliderCylinder::new(&ColliderCylinderInit {
        base: ColliderInit { col_type: COL_MATERIAL_HIT0, at_flags: AT_ON | AT_TYPE_ENEMY | AT_TYPE_PLAYER, ac_flags: AC_NONE, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags, hit_special_effect: effect, damage },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: 0, defense: 0 },
            at_elem_flags: ATELEM_ON,
            ac_elem_flags: ACELEM_NONE,
            oc_elem_flags: OCELEM_NONE,
        },
        dim: Cylinder16 { radius: 20, height: 40, y_shift: 0, pos },
    }))
}

/// A JntSph target of two spheres 100 apart along x, AC from both types, `ac_flags` extra.
fn target(ac_extra: u8) -> Collider {
    target_with(ac_extra, OC2_TYPE_1)
}

fn target_with(ac_extra: u8, oc_flags2: u8) -> Collider {
    let elem = |x: i16| ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT::default(),
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: 0, defense: 0 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_NONE,
        },
        limb: 0,
        model_sphere: Sphere16 { center: [x, 20, 0], radius: 15 },
        scale: 100,
    };
    let mut c = ColliderJntSph::new(
        &ColliderInit { col_type: COL_MATERIAL_HIT6, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_ENEMY | AC_TYPE_PLAYER | ac_extra, oc_flags1: OC1_NONE, oc_flags2, shape: COLSHAPE_JNTSPH },
        &[elem(0), elem(100)],
    );
    c.update_spheres(0, &glam::Mat4::IDENTITY);
    Collider::JntSph(c)
}

#[test]
fn a_jntsph_takes_each_hit_spheres_damage_from_its_table() {
    // CollisionCheck_ApplyDamageJntSph: CollisionCheck_ApplyDamage for each element; one
    // attacker over the first sphere only. The table gives DMG_SLASH_KOKIRI's entry
    // DMG_ENTRY(1, 0xF): damage 1, damageReaction 0xF.
    static TABLE: DamageTable = {
        let mut t = [0u8; 32];
        t[8] = 0xF1;
        DamageTable { table: t }
    };
    let mut actors = ActorContext::default();
    let mut ctx = CollisionCheckContext::default();
    let t = holder(&mut actors, ACTORCAT_ENEMY, Vec3::ZERO, target(0));
    actors.actor_mut(t).unwrap().col_chk_info.damage_table = Some(&TABLE);
    let a = holder(&mut actors, ACTORCAT_PLAYER, Vec3::ZERO, attacker([0, 0, 0], DMG_SLASH_KOKIRI, 3, HIT_SPECIAL_EFFECT_NONE));
    register(&mut ctx, &mut actors, a, true);
    register(&mut ctx, &mut actors, t, false);
    ctx.check(&mut actors);
    let info = actors.actor(t).unwrap().col_chk_info;
    assert_eq!((info.damage, info.damage_reaction), (1, 0xF));
    // Both spheres under the attacker: CollisionCheck_ATCylVsACJntSph stops at the first hit
    // sphere unless the target has OC2_FIRST_ONLY (`if (!(ocFlags2 & OC2_FIRST_ONLY)) break;`), so
    // 1; with it, each hit element adds its entry (2 in all).
    for (oc2, damage) in [(OC2_TYPE_1, 1), (OC2_TYPE_1 | OC2_FIRST_ONLY, 2)] {
        let mut actors = ActorContext::default();
        let mut ctx = CollisionCheckContext::default();
        let t = holder(&mut actors, ACTORCAT_ENEMY, Vec3::ZERO, target_with(0, oc2));
        actors.actor_mut(t).unwrap().col_chk_info.damage_table = Some(&TABLE);
        let mut wide = attacker([50, 0, 0], DMG_SLASH_KOKIRI, 3, HIT_SPECIAL_EFFECT_NONE);
        if let Collider::Cylinder(c) = &mut wide {
            c.dim.radius = 80;
        }
        let a = holder(&mut actors, ACTORCAT_PLAYER, Vec3::ZERO, wide);
        register(&mut ctx, &mut actors, a, true);
        register(&mut ctx, &mut actors, t, false);
        ctx.check(&mut actors);
        assert_eq!(actors.actor(t).unwrap().col_chk_info.damage, damage);
    }
}

#[test]
fn a_hard_collider_is_hit_but_takes_no_damage() {
    // AC_HARD: CollisionCheck_SetATvsAC bounces both sides (AT_BOUNCED, AC_BOUNCED), and
    // CollisionCheck_ApplyDamage adds nothing (the table's reaction is still recorded).
    static TABLE: DamageTable = DamageTable { table: [0x21; 32] };
    let mut actors = ActorContext::default();
    let mut ctx = CollisionCheckContext::default();
    let t = holder(&mut actors, ACTORCAT_ENEMY, Vec3::ZERO, target(AC_HARD));
    actors.actor_mut(t).unwrap().col_chk_info.damage_table = Some(&TABLE);
    let a = holder(&mut actors, ACTORCAT_PLAYER, Vec3::ZERO, attacker([0, 0, 0], DMG_SLASH_KOKIRI, 3, HIT_SPECIAL_EFFECT_NONE));
    register(&mut ctx, &mut actors, a, true);
    register(&mut ctx, &mut actors, t, false);
    ctx.check(&mut actors);
    let info = actors.actor(t).unwrap().col_chk_info;
    assert_eq!((info.damage, info.damage_reaction), (0, 2));
    let Collider::JntSph(c) = &actors.downcast::<Holder>(t).unwrap().col else { unreachable!() };
    assert_eq!(c.base.ac_flags & (AC_HIT | AC_BOUNCED), AC_HIT | AC_BOUNCED);
}

#[test]
fn the_attackers_special_effect_reaches_the_hit_actor() {
    // CollisionCheck_SetATvsAC: ac->actor->colChkInfo.acHitSpecialEffect = atElem->atDmgInfo.hitSpecialEffect.
    for effect in [HIT_SPECIAL_EFFECT_FIRE, HIT_SPECIAL_EFFECT_ICE, HIT_SPECIAL_EFFECT_ELECTRIC, HIT_SPECIAL_EFFECT_KNOCKBACK] {
        let mut actors = ActorContext::default();
        let mut ctx = CollisionCheckContext::default();
        let t = holder(&mut actors, ACTORCAT_PLAYER, Vec3::ZERO, target(0));
        let a = holder(&mut actors, ACTORCAT_ENEMY, Vec3::ZERO, attacker([0, 0, 0], 0xFFCF_FFFF, 8, effect));
        register(&mut ctx, &mut actors, a, true);
        register(&mut ctx, &mut actors, t, false);
        ctx.check(&mut actors);
        let info = actors.actor(t).unwrap().col_chk_info;
        // No table: the attack's damage less the element's defence (0).
        assert_eq!((info.ac_hit_special_effect, info.damage), (effect, 8));
    }
}

#[test]
fn gfx_set_fog_turns_near_and_far_into_the_fog_factor() {
    // gSPFogPosition(min, max): fm = 128000 / (max - min), fo = (500 - min) * 256 / (max - min),
    // integer divisions, each the low 16 bits.
    let f = oot_game::gbi::gfx_set_fog(255, 0, 0, 0, 0, 4000);
    assert_eq!((f.color, f.multiplier, f.offset), ([255, 0, 0, 0], 32, 32));
    let f = oot_game::gbi::gfx_set_fog(1, 2, 3, 4, 996, 1000);
    // (500 - 996) * 256 / 4 = -31744.
    assert_eq!((f.multiplier, f.offset), (32000, -31744));
    // near >= 1000: gSPFogFactor(0, 0), no fog; near > 996: (0x7FFF, -0x7F00); near < 0: (0, 255).
    assert_eq!((oot_game::gbi::gfx_set_fog(0, 0, 0, 0, 1000, 2000).multiplier, oot_game::gbi::gfx_set_fog(0, 0, 0, 0, 1000, 2000).offset), (0, 0));
    let f = oot_game::gbi::gfx_set_fog(0, 0, 0, 0, 998, 999);
    assert_eq!((f.multiplier, f.offset), (0x7FFF, -0x7F00));
    let f = oot_game::gbi::gfx_set_fog(0, 0, 0, 0, -1, 500);
    assert_eq!((f.multiplier, f.offset), (0, 255));
    // far == near: far + 1 first.
    let f = oot_game::gbi::gfx_set_fog(0, 0, 0, 0, 500, 500);
    assert_eq!((f.multiplier, f.offset), (128000i32 as i16, 0));
}

#[test]
fn the_colour_filter_tints_by_its_fog_over_its_duration() {
    // Actor_SetColorFilter(RED, 255, OPA, 42): params = 0x4000 | 0 | ((255 & 0xF8) << 5) | 42
    // = 0x4000 | 0x1F00 | 0x2A, timer 42.
    let mut a = Actor::new(Vec3::ZERO, 0);
    a.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 42);
    assert_eq!((a.color_filter_params, a.color_filter_timer), (0x5F2A, 42));
    // Actor_Draw: color.r = COLORFILTER_GET_COLORINTENSITY | 7 = (0x1F00 >> 5) | 7 = 0xFF; then
    // func_80026400: cos = Math_CosS((0x4000 / 42) * 42) = Math_CosS(16380), near 0, so
    // gSPFogPosition(0, (s16)(2800 * |cos|) + 1700) = (0, 1700 + a little).
    let (fog, xlu) = oot_game::play::color_filter_fog((a.color_filter_params, a.color_filter_timer)).unwrap();
    assert!(!xlu);
    assert_eq!(fog.color, [0xFF, 0, 0, 255]);
    let far = (2800.0 * eng_math::cos_s(16380).abs()) as i16 as i32 + 1700;
    assert_eq!((fog.multiplier, fog.offset), ((128000 / far) as i16, (500 * 256 / far) as i16));
    // At the timer's 1: Math_CosS(390), near 1: far near 4500.
    let (fog, _) = oot_game::play::color_filter_fog((a.color_filter_params, 1)).unwrap();
    let far = (2800.0 * eng_math::cos_s(390).abs()) as i16 as i32 + 1700;
    assert_eq!(fog.multiplier, (128000 / far) as i16);
    // The stunned blue: COLORFLAG_BLUE, 155: intensity (155 & 0xF8) = 0x98 << 5 >> 5 | 7 = 0x9F.
    a.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 155, COLORFILTER_BUFFLAG_OPA, 62);
    let (fog, _) = oot_game::play::color_filter_fog((a.color_filter_params, a.color_filter_timer)).unwrap();
    assert_eq!(fog.color, [0, 0, 0x9F, 255]);
    // Off at 0.
    assert!(oot_game::play::color_filter_fog((a.color_filter_params, 0)).is_none());
}

#[test]
fn a_bottled_fairy_is_used_up() {
    // Inventory_ConsumeFairy: the first bottle slot holding ITEM_BOTTLE_FAIRY empties.
    let mut s = SaveContext::new(0, false, 0);
    assert!(!inventory_consume_fairy(&mut s));
    s.inventory.items[SLOT_BOTTLE_1 + 2] = ITEM_BOTTLE_FAIRY;
    assert!(inventory_consume_fairy(&mut s));
    assert_eq!(s.inventory.items[SLOT_BOTTLE_1 + 2], ITEM_BOTTLE_EMPTY);
    assert!(!inventory_consume_fairy(&mut s));
    // With a C button holding a fairy, that button's bottle (cButtonSlots) empties and the
    // button shows an empty bottle.
    let mut s = SaveContext::new(0, false, 0);
    s.inventory.items[SLOT_BOTTLE_1] = ITEM_BOTTLE_FAIRY;
    s.inventory.items[SLOT_BOTTLE_1 + 3] = ITEM_BOTTLE_FAIRY;
    s.equips.button_items[2] = ITEM_BOTTLE_FAIRY;
    s.equips.c_button_slots[1] = (SLOT_BOTTLE_1 + 3) as u8;
    assert!(inventory_consume_fairy(&mut s));
    assert_eq!((s.inventory.items[SLOT_BOTTLE_1], s.inventory.items[SLOT_BOTTLE_1 + 3], s.equips.button_items[2]), (ITEM_BOTTLE_FAIRY, ITEM_BOTTLE_EMPTY, ITEM_BOTTLE_EMPTY));
}

#[test]
fn a_deleted_shield_is_off_and_out_of_the_inventory() {
    // Inventory_DeleteEquipment(EQUIP_TYPE_SHIELD) with the Deku Shield worn: the worn nibble
    // cleared, the owned bit (OWNED_EQUIP_FLAG(SHIELD, 1 - 1)) toggled off; returns 1.
    let mut s = SaveContext::new(0, false, 0);
    s.inventory.equipment |= owned_equip_flag(EQUIP_TYPE_SHIELD, 0);
    s.equips.equipment |= 1 << 4;
    assert_eq!(inventory_delete_equipment(&mut s, EQUIP_TYPE_SHIELD), 1);
    assert_eq!(s.cur_equip_value(EQUIP_TYPE_SHIELD), 0);
    assert_eq!(s.inventory.equipment & owned_equip_flag(EQUIP_TYPE_SHIELD, 0), 0);
    // Nothing worn: nothing changes, 0.
    assert_eq!(inventory_delete_equipment(&mut s, EQUIP_TYPE_SHIELD), 0);
}

#[test]
fn a_hits_drop_flag_follows_the_arrows_and_magic() {
    // Actor_SetDropFlagJntSph: each element's hit, last first, OR'd: a fire arrow 0x01, an ice
    // arrow 0x02; a sword, nothing.
    let mut jnt = match target(0) {
        Collider::JntSph(c) => c,
        _ => unreachable!(),
    };
    let mut actors = ActorContext::default();
    let someone = holder(&mut actors, ACTORCAT_PLAYER, Vec3::ZERO, target(0));
    let hit = |flags: u32| HitElem {
        elem: ElemRef { col: ColliderRef { actor: someone, id: 0 }, elem: 0 },
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: flags, hit_special_effect: 0, damage: 3 },
        ac_dmg_info: ColliderElementDamageInfoAC::default(),
        elem_material: 0,
    };
    jnt.elements[0].info.ac_hit_elem = Some(hit(DMG_ARROW_FIRE));
    jnt.elements[1].info.ac_hit_elem = Some(hit(DMG_ARROW_ICE));
    let mut a = Actor::new(Vec3::ZERO, 0);
    a.set_drop_flag_jnt_sph(&jnt, true);
    assert_eq!(a.drop_flag, 0x03);
    jnt.elements[0].info.ac_hit_elem = Some(hit(DMG_SLASH_KOKIRI));
    jnt.elements[1].info.ac_hit_elem = None;
    a.set_drop_flag_jnt_sph(&jnt, true);
    assert_eq!(a.drop_flag, 0);
    // Ice magic with freezeFlag: freezeTimer = the hit's damage, flag 0.
    jnt.elements[0].info.ac_hit_elem = Some(hit(DMG_MAGIC_ICE));
    a.set_drop_flag_jnt_sph(&jnt, true);
    assert_eq!((a.drop_flag, a.freeze_timer), (0, 3));
}
