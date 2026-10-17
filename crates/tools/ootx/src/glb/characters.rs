//! The characters `export-glb` writes by name (`characters\<Name>.glb`): for each, the mesh it's
//! drawn with (the default look: eyes open, opaque, nothing extra in hand), the idle animation
//! whose first frame stands it up, the colours its draw code sets on the bake's dynamic segments,
//! and its arms' limbs (0-based, shoulder to hand) for the T-pose. Read from each actor's module
//! and its C.

use oot_game::player_lib::Age;

/// A colour the draw code sets: `gDPSetEnvColor` or `gDPSetPrimColor` on a segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Env,
    Prim,
}
use Color::{Env, Prim};

/// Where a character's mesh comes from.
#[derive(Debug, Clone, Copy)]
pub enum Cast {
    /// An actor's bake (`keys::bake`'s name).
    Bake(&'static str),
    /// A skeleton's whole mesh as the pack has it (`file`, `symbol`), when the actor draws it so.
    Skel(&'static str, &'static str),
    /// Link's default loadout.
    Link(Age),
    /// A skin skeleton (`file`, `symbol`): a horse.
    Skin(&'static str, &'static str),
}

/// A mesh the draw code puts at a limb's matrix after the skeleton (a carpenter's head, Queen
/// Gohma's eye), with the colours it sets for it.
#[derive(Debug)]
pub struct Part {
    pub bake: &'static str,
    /// 0-based.
    pub limb: u8,
    pub colors: &'static [(u8, Color, [u8; 4])],
}

#[derive(Debug)]
pub struct Character {
    /// The file's name.
    pub name: &'static str,
    pub cast: Cast,
    /// The idle animation (`anim/<file>/<symbol>`).
    pub idle: Option<(&'static str, &'static str)>,
    pub colors: &'static [(u8, Color, [u8; 4])],
    pub left: &'static [u8],
    pub right: &'static [u8],
    pub parts: &'static [Part],
}

impl Character {
    const fn with(self, parts: &'static [Part]) -> Character {
        Character { parts, ..self }
    }
}

const fn part(bake: &'static str, limb: u8, colors: &'static [(u8, Color, [u8; 4])]) -> Part {
    Part { bake, limb, colors }
}

const fn c(
    name: &'static str,
    bake: &'static str,
    idle: Option<(&'static str, &'static str)>,
    colors: &'static [(u8, Color, [u8; 4])],
    left: &'static [u8],
    right: &'static [u8],
) -> Character {
    Character { name, cast: Cast::Bake(bake), idle, colors, left, right, parts: &[] }
}

const fn skel(
    name: &'static str,
    file: &'static str,
    symbol: &'static str,
    idle: Option<(&'static str, &'static str)>,
    left: &'static [u8],
    right: &'static [u8],
) -> Character {
    Character { name, cast: Cast::Skel(file, symbol), idle, colors: &[], left, right, parts: &[] }
}

const fn horse(name: &'static str, file: &'static str, symbol: &'static str, idle: &'static str) -> Character {
    Character { name, cast: Cast::Skin(file, symbol), idle: Some((file, idle)), colors: &[], left: &[], right: &[], parts: &[] }
}

const fn link(name: &'static str, age: Age) -> Character {
    // `PLAYER_LIMB_L_SHOULDER` to `_L_HAND`, `_R_SHOULDER` to `_R_HAND`.
    Character {
        name,
        cast: Cast::Link(age),
        idle: Some(("gameplay_keep", "gPlayerAnim_link_normal_wait")),
        colors: &[],
        left: &[13, 14, 15],
        right: &[16, 17, 18],
        parts: &[],
    }
}

/// The alpha `func_80034BA0` and the like set on segment 0x0E at full opacity.
const OPAQUE_0E: (u8, Color, [u8; 4]) = (0x0E, Env, [0, 0, 0, 255]);
/// A Poe's `lightColor` at rest (`func_80AE067C`'s default: up to 255, 255, 210), opaque.
const POE_LIGHT: [u8; 4] = [255, 255, 210, 255];
/// Queen Gohma's and her larvae's eye colour in their default state (`targetEyeEnvColors`).
const GOHMA_RED: [u8; 4] = [255, 17, 0, 255];

#[rustfmt::skip]
pub const CHARACTERS: &[Character] = &[
    link("Link (adult)", Age::Adult),
    link("Link (child)", Age::Child),

    // Kokiri Forest and the Lost Woods.
    c("Kokiri Boy", "En_Ko/km1_opa", Some(("object_os_anime", "gKokiriStandingAnim")), &[(0x08, Env, [0, 130, 70, 255]), (0x09, Env, [110, 170, 20, 255]), OPAQUE_0E], &[8, 9, 10], &[11, 12, 13]),
    c("Kokiri Girl", "En_Ko/kw1_opa_eye0", Some(("object_os_anime", "gKokiriStandUpAnim")), &[(0x08, Env, [70, 190, 60, 255]), (0x09, Env, [100, 30, 0, 255]), OPAQUE_0E], &[8, 9, 10], &[11, 12, 13]),
    c("Fado", "En_Ko/fa_opa_eye0", Some(("object_os_anime", "gKokiriIdleAnim")), &[(0x08, Env, [70, 190, 60, 255]), (0x09, Env, [100, 30, 0, 255]), OPAQUE_0E], &[8, 9, 10], &[11, 12, 13]),
    c("Mido", "En_Md/opa_eye0", Some(("object_md", "gMidoIdleAnim")), &[OPAQUE_0E], &[9, 10, 11], &[12, 13, 14]),
    c("Saria", "En_Sa/opa", Some(("object_sa", "gSariaWaitArmsToSideAnim")), &[OPAQUE_0E], &[9, 10, 11], &[12, 13, 14]),
    c("Kokiri Shopkeeper", "En_Ossan/kokiri_eye0", Some(("object_masterkokiri", "object_masterkokiri_Anim_0004A8")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Navi", "En_Elf/navi", None, &[(0x0E, Env, [0, 0, 255, 255]), (0x08, Prim, [255, 255, 255, 255])], &[], &[]),
    c("Fairy", "En_Elf/fairy", None, &[(0x0E, Env, [255, 50, 100, 255]), (0x08, Prim, [255, 220, 220, 255])], &[], &[]),
    skel("Business Scrub", "object_shopnuts", "gBusinessScrubSkel", Some(("object_shopnuts", "gBusinessScrubPeekAnim")), &[9, 10, 11], &[12, 13, 14]),
    skel("Deku Scrub", "object_hintnuts", "gHintNutsSkel", Some(("object_hintnuts", "gHintNutsStandAnim")), &[], &[]),
    c("Deku Scrub (theatre)", "En_Dnt_Nomal/stage_eye0", Some(("object_dnk", "gDntStageWaitAnim")), &[(0x0B, Env, [255, 255, 255, 255])], &[], &[]),
    c("Skull Kid", "En_Skj/opa", Some(("object_skj", "gSkullKidWaitAnim")), &[OPAQUE_0E], &[13, 14, 15], &[16, 17]),
    c("Owl (Kaepora Gaebora)", "En_Owl/perching_eye0", Some(("object_owl", "gOwlPerchAnim")), &[], &[], &[]),
    c("Wolfos", "En_Wf/gray_eye0", Some(("object_wf", "gWolfosIdleAnim")), &[], &[18, 19, 20], &[12, 13, 14]),
    c("White Wolfos", "En_Wf/white_eye0", Some(("object_wf", "gWolfosIdleAnim")), &[], &[18, 19, 20], &[12, 13, 14]),
    c("Skulltula", "En_St/skel", Some(("object_st", "object_st_Anim_000304")), &[(0x08, Env, [0, 0, 0, 0])], &[], &[]),
    c("Gold Skulltula", "En_Sw/gold", Some(("object_st", "object_st_Anim_000304")), &[], &[], &[]),
    c("Queen Gohma", "Boss_Goma/skel", Some(("object_goma", "gGohmaIdleCrouchedAnim")), &[(0x0B, Env, [0, 255, 170, 255])], &[], &[])
        .with(&[part("Boss_Goma/eye", 4, &[(0x0B, Env, [255, 17, 0, 63])]), part("Boss_Goma/iris", 38, &[(0x0B, Env, [255, 255, 255, 255])])]),
    c("Gohma Larva", "En_Goma/larva", Some(("object_gol", "gObjectGolStandAnim")), &[(0x0B, Env, GOHMA_RED)], &[], &[])
        .with(&[part("En_Goma/larva_body", 2, &[(0x0B, Env, GOHMA_RED)])]),

    // Hyrule Field and Lon Lon Ranch.
    c("Peahat", "En_Peehat/skeleton", Some(("object_peehat", "gPeehatFlyingAnim")), &[], &[], &[])
        .with(&[part("En_Peehat/body", 2, &[]), part("En_Peehat/top", 22, &[])]),
    c("Stalchild", "En_Skb/skeleton", Some(("object_skb", "gStalchildWalkingAnim")), &[(0x0B, Env, [160, 160, 160, 255])], &[15, 16, 17], &[12, 13, 14]),
    c("Butterfly", "En_Butte/skel", Some(("gameplay_field_keep", "gButterflyAnim")), &[], &[], &[]),
    c("Talon", "En_Ta/eye0", Some(("object_ta", "gTalonStandAnim")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Malon (child)", "En_Ma1/eye0", Some(("object_ma1", "gMalonChildSingAnim")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Cow", "En_Cow/body", Some(("object_cow", "gCowBodyChewAnim")), &[], &[], &[]),
    c("Ingo", "En_In/eye0", Some(("object_in", "gIngoStandingAnim")), &[], &[9, 10, 11], &[12, 13, 14]),
    c("Guay", "En_Crow/skel", Some(("object_crow", "gGuayFlyAnim")), &[], &[], &[]),
    // GAME-09 milestone 2.
    c("Goron", "En_Go2/eye1_mouth0", Some(("object_oF1d_map", "gGoronAnim_0021D0")), &[], &[10, 11, 12], &[13, 14, 15]),
    c("Darunia", "En_Du/skel", Some(("object_du", "gDaruniaIdleAnim")), &[], &[9, 10, 11], &[12, 13, 14]),
    c("Medigoron", "En_Gm/eye0", Some(("object_gm", "object_gm_Anim_0002B8")), &[], &[10, 11, 12], &[13, 14, 15]),
    c("Red Tektite", "En_Tite/red", Some(("object_tite", "object_tite_Anim_0012E4")), &[], &[], &[]),
    c("Blue Tektite", "En_Tite/blue", Some(("object_tite", "object_tite_Anim_0012E4")), &[], &[], &[]),
    // GAME-09 milestone 3: Dodongo's Cavern (Master Quest).
    c("Lizalfos", "En_Zf/lizalfos_opa", Some(("object_zf", "gZfIdleAnim")), &[OPAQUE_0E], &[], &[]),
    c("Dinolfos", "En_Zf/dinolfos_opa", Some(("object_zf", "gZfIdleAnim")), &[OPAQUE_0E], &[], &[]),
    c("Poe", "En_Poh/poe_opa", Some(("object_poh", "gPoeFloatAnim")), &[(0x0E, Env, POE_LIGHT)], &[], &[]),
    c("Poe Composer (Sharp)", "En_Poh/sharp_opa", Some(("object_po_composer", "gPoeComposerFloatAnim")), &[(0x08, Env, POE_LIGHT), (0x0A, Env, [75, 20, 25, 255]), (0x0B, Env, [90, 85, 50, 255])], &[], &[]),
    c("Poe Composer (Flat)", "En_Poh/flat_opa", Some(("object_po_composer", "gPoeComposerFloatAnim")), &[(0x08, Env, POE_LIGHT), (0x0A, Env, [80, 110, 90, 255]), (0x0B, Env, [100, 90, 100, 255])], &[], &[]),
    c("Armos", "En_Am/skel", Some(("object_am", "gArmosRicochetAnim")), &[(0x0B, Env, [0, 0, 0, 0])], &[], &[]),
    skel("Baby Dodongo", "object_dodojr", "object_dodojr_Skel_0020E0", Some(("object_dodojr", "object_dodojr_Anim_0009D4")), &[], &[]),
    // GAME-09 milestone 4: King Dodongo (Demo_Du's Darunia and Demo_Go's Gorons are En_Du's and
    // En_Go2's bakes: the rows above).
    c("King Dodongo", "Boss_Dodongo/skel", Some(("object_kingdodongo", "object_kingdodongo_Anim_00F0D8")), &[], &[], &[]),

    // Castle Town and Hyrule Castle.
    c("Hylian (Dog Lady)", "En_Hy/type0_eye0", Some(("object_os_anime", "gObjOsAnim_092C")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Market Woman)", "En_Hy/type1_eye0", Some(("object_os_anime", "gObjOsAnim_4F28")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Bearded Man)", "En_Hy/type2_eye0", Some(("object_os_anime", "gObjOsAnim_0228")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Bald Man with Bag)", "En_Hy/type3_eye0", Some(("object_os_anime", "gObjOsAnim_300C")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Shaved Man, Black Shirt)", "En_Hy/type4_eye0", Some(("object_os_anime", "gObjOsAnim_0170")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Beggar)", "En_Hy/type5_eye0", Some(("object_os_anime", "gObjOsAnim_31B0")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Old Woman)", "En_Hy/type6_eye0", Some(("object_os_anime", "gObjOsAnim_1E7C")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Old Man)", "En_Hy/type7_eye0", Some(("object_os_anime", "gObjOsAnim_265C")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Young Woman, Brown Hair)", "En_Hy/type8_eye0", Some(("object_os_anime", "gObjOsAnim_4E90")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Mustached Man, Red Shirt)", "En_Hy/type9_eye0", Some(("object_os_anime", "gObjOsAnim_3D84")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Mustached Man, Blue Shirt)", "En_Hy/type10_eye0", Some(("object_os_anime", "gObjOsAnim_41F8")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Young Woman, Orange Hair)", "En_Hy/type11_eye0", Some(("object_os_anime", "gObjOsAnim_4408")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Mustached Man, Maroon Shirt)", "En_Hy/type12_eye0", Some(("object_os_anime", "gObjOsAnim_2D0C")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Bowl Cut Man, Purple Shirt)", "En_Hy/type13_eye0", Some(("object_os_anime", "gObjOsAnim_00B4")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Bearded Man, Green Shirt)", "En_Hy/type14_eye0", Some(("object_os_anime", "gObjOsAnim_2DC0")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Bald Old Man, Brown Robe)", "En_Hy/type15_eye0", Some(("object_os_anime", "gObjOsAnim_1F18")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Mustached Man, White Shirt)", "En_Hy/type16_eye0", Some(("object_os_anime", "gObjOsAnim_42AC")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Shaved Man, Green Shirt)", "En_Hy/type17_eye0", Some(("object_os_anime", "gObjOsAnim_0170")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Kakariko Woman)", "En_Hy/type18_eye0", Some(("object_os_anime", "gObjOsAnim_28DC")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Bald Old Man, Purple Robe)", "En_Hy/type19_eye0", Some(("object_os_anime", "gObjOsAnim_1F18")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Hylian (Bowl Cut Man, Green Shirt)", "En_Hy/type20_eye0", Some(("object_os_anime", "gObjOsAnim_00B4")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Haggling Townsfolk", "En_Mu/colors0", Some(("object_mu", "object_mu_Anim_0003F4")), &[], &[], &[]),
    c("Haggling Townsfolk (colours swapped)", "En_Mu/colors1", Some(("object_mu", "object_mu_Anim_0003F4")), &[], &[], &[]),
    c("Dancing Couple", "En_Tg/skel", Some(("object_mu", "gDancingCoupleAnim")), &[], &[], &[]),
    c("Dog (cream)", "En_Dog/color0", Some(("object_dog", "gDogSitAnim")), &[], &[], &[]),
    c("Dog (brown)", "En_Dog/color1", Some(("object_dog", "gDogSitAnim")), &[], &[], &[]),
    c("Hyrule Guard", "En_Heishi/skel", Some(("object_sd", "gEnHeishiIdleAnim")), &[], &[9, 10, 11], &[12, 13, 14]),
    c("Princess Zelda (child)", "En_Zl4/skel", Some(("object_zl4", "gChildZeldaAnim_000654")), &[], &[10, 11, 12], &[13, 14, 15]),
    c("Princess Zelda (child, riding dress)", "En_Viewer/zelda_field_0_0", Some(("object_zl4", "gChildZeldaAnim_000654")), &[], &[10, 11, 12], &[13, 14, 15]),
    c("Impa", "Demo_Im/skel", Some(("object_im", "gImpaIdleAnim")), &[], &[9, 10, 11], &[12, 13, 14]),
    c("Impa (masked)", "En_Viewer/impa", Some(("object_im", "gImpaIdleAnim")), &[], &[9, 10, 11], &[12, 13, 14]),
    c("Ganondorf (young)", "En_Viewer/ganondorf_0_closed", Some(("object_gndd", "gYoungGanondorfArmsCrossedAnim")), &[], &[2, 3, 4], &[5, 6, 7]),

    // Kakariko Village and the graveyard.
    c("Carpenter (Ichiro)", "En_Daiku_Kakariko/skel0", Some(("object_daiku", "object_daiku_Anim_001AB0")), &[], &[8, 9, 10], &[11, 12, 13])
        .with(&[part("En_Daiku_Kakariko/head0", 14, &[])]),
    c("Carpenter (Sabooro)", "En_Daiku_Kakariko/skel1", Some(("object_daiku", "object_daiku_Anim_001AB0")), &[], &[8, 9, 10], &[11, 12, 13])
        .with(&[part("En_Daiku_Kakariko/head1", 14, &[])]),
    c("Carpenter (Jiro)", "En_Daiku_Kakariko/skel2", Some(("object_daiku", "object_daiku_Anim_001AB0")), &[], &[8, 9, 10], &[11, 12, 13])
        .with(&[part("En_Daiku_Kakariko/head2", 14, &[])]),
    c("Carpenter (Shiro)", "En_Daiku_Kakariko/skel3", Some(("object_daiku", "object_daiku_Anim_001AB0")), &[], &[8, 9, 10], &[11, 12, 13])
        .with(&[part("En_Daiku_Kakariko/head3", 14, &[])]),
    c("Mutoh (head carpenter)", "En_Toryo/skel", Some(("object_toryo", "object_toryo_Anim_000E50")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Carpenter's Son", "En_Hs2/skel", Some(("object_hs", "gCarpenterSonSittingAnim")), &[], &[], &[]),
    c("Man on the Roof", "En_Ani/eye0", Some(("object_ani", "gRoofManIdleAnim")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Cucco Lady", "En_Niw_Lady/eye0", Some(("object_os_anime", "gObjOsAnim_07D0")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Cucco-Chasing Girl", "En_Niw_Girl/eye0", Some(("object_gr", "gNiwGirlJumpAnim")), &[], &[6, 7], &[8, 9]),
    c("Cucco", "En_Niw/skel", Some(("object_niw", "gCuccoAnim")), &[], &[], &[]),
    c("Guru-Guru", "En_Fu/face0", Some(("object_fu", "gWindmillManPlayStillAnim")), &[], &[], &[]),
    c("Dampe (gravekeeper)", "En_Tk/eye0", Some(("object_tk", "gDampeRestAnim")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Graveyard Kid", "En_Cs/eye0", Some(("object_cs", "gGraveyardKidIdleAnim")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Redead", "En_Rd/redead_opa", Some(("object_rd", "gGibdoRedeadIdleAnim")), &[OPAQUE_0E], &[15, 16, 17], &[19, 20, 21]),
    c("Gibdo", "En_Rd/gibdo_opa", Some(("object_rd", "gGibdoRedeadIdleAnim")), &[OPAQUE_0E], &[15, 16, 17], &[19, 20, 21]),
    c("Cursed Skulltula Man", "En_Ssh/father_eye0", Some(("object_ssh", "object_ssh_Anim_000304")), &[], &[], &[]),
    c("Cursed Skulltula Child", "En_Ssh/child_eye0", Some(("object_ssh", "object_ssh_Anim_000304")), &[], &[], &[]),
    c("Skulltula House Father (cured)", "En_Sth/member0", Some(("ovl_En_Sth", "sParentDanceAnim")), &[], &[8, 9, 10], &[11, 12, 13]),
    c("Skulltula House Son (10 tokens)", "En_Sth/member1", Some(("ovl_En_Sth", "sChildDanceAnim")), &[], &[8, 9, 10], &[11, 12, 13])
        .with(&[part("En_Sth/hair", 14, &[])]),
    c("Skulltula House Son (20 tokens)", "En_Sth/member2", Some(("ovl_En_Sth", "sChildDanceAnim")), &[], &[8, 9, 10], &[11, 12, 13])
        .with(&[part("En_Sth/hair", 14, &[])]),
    c("Skulltula House Son (30 tokens)", "En_Sth/member3", Some(("ovl_En_Sth", "sChildDanceAnim")), &[], &[8, 9, 10], &[11, 12, 13])
        .with(&[part("En_Sth/hair", 14, &[])]),
    c("Skulltula House Son (40 tokens)", "En_Sth/member4", Some(("ovl_En_Sth", "sChildDanceAnim")), &[], &[8, 9, 10], &[11, 12, 13])
        .with(&[part("En_Sth/hair", 14, &[])]),
    c("Skulltula House Son (50 tokens)", "En_Sth/member5", Some(("ovl_En_Sth", "sChildDanceAnim")), &[], &[8, 9, 10], &[11, 12, 13])
        .with(&[part("En_Sth/hair", 14, &[])]),

    // The horses (skin skeletons).
    horse("Epona", "object_horse", "gEponaSkel", "gEponaIdleAnim"),
    horse("Epona (young)", "object_horse_link_child", "gChildEponaSkel", "gChildEponaIdleAnim"),
    horse("Horse", "object_horse_normal", "gHorseNormalSkel", "gHorseNormalIdleAnim"),
    horse("Ingo's Horse", "object_hni", "gHorseIngoSkel", "gHorseIngoIdleAnim"),
    horse("Ganondorf's Horse", "object_horse_ganon", "gHorseGanonSkel", "gHorseGanonIdleAnim"),
    horse("Zelda's Horse", "object_horse_zelda", "gHorseZeldaSkel", "gHorseZeldaGallopingAnim"),
    horse("Phantom Ganon's Horse", "object_fhg", "gPhantomHorseSkel", "gPhantomHorseIdleAnim"),
];
