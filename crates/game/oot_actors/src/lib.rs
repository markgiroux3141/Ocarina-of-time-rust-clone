//! Actors ported from the decomp's overlays (`src/overlays/actors`), one module per overlay:
//! Player (`ovl_player_actor`, `z_player.c`), `En_Holl`, `Bg_Ydan_Hasi`, `Bg_Treemouth` and
//! Kokiri Forest's props and NPCs, plus the sandbox's dummy Z-target. Each implements
//! `oot_game::actor_ctx::ActorImpl` and lives in `PlayState`'s actor context.
//!
//! - `overlays()` gives `Actor_Spawn` the constructors of the actors spawned by id (Player and
//!   `En_Holl`; every other id spawns a placeholder), and `play_entrance` enters a scene from
//!   the pack with them (`Play_Init`).
//! - `new_play` sets up a `PlayState` on any collision with Player standing in it (the tests'
//!   and the sandbox's).
//! - `PlayExt` gives typed access to the actors the tests and the apps look at.

use std::sync::Arc;

use eng_collision::bgcheck::CollisionContext;
use eng_collision::collision::CollisionHeader;
use glam::Vec3;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTOR_BG_YDAN_HASI, ACTOR_SANDBOX_DUMMY_TARGET, ACTORCAT_BG, ACTORCAT_ENEMY, ActorHandle, ActorProfile};
use oot_game::data::GameData;
use oot_game::pack::GamePack;
use oot_game::play::PlayState;
use oot_game::play_scene::GameAssets;
use oot_game::player_lib::PlayerRules;
use oot_game::save::SaveContext;
use oot_game::spawn::Overlays;

pub mod bg_haka;
pub mod bg_spot00_hanebasi;
pub mod boss_dodongo;
pub mod boss_goma;
pub mod bg_spot09_obj;
pub mod bg_spot16_doughnut;
pub mod bg_treemouth;
pub mod bg_ydan_hasi;
pub mod bg_ydan_maruta;
pub mod bg_ydan_sp;
pub mod demo_effect;
pub mod demo_du;
pub mod demo_go;
pub mod bg_dy_yoseizo;
pub mod en_dy_extra;
pub mod demo_im;
pub mod eff_dust;
pub mod en_m_thunder;
pub mod demo_kankyo;
pub mod demo_sa;
pub mod demo_tre_lgt;
pub mod door_shutter;
pub mod door_warp1;
pub mod dummy_target;
pub mod elf_msg;
pub mod elf_msg2;
pub mod en_box;
pub mod en_dekubaba;
pub mod en_dekunuts;
pub mod en_dns;
pub mod en_door;
pub mod en_elf;
pub mod en_firefly;
pub mod en_girla;
pub mod en_hintnuts;
pub mod en_goma;
pub mod en_goroiwa;
pub mod en_holl;
pub mod en_item00;
pub mod en_ishi;
pub mod en_kanban;
pub mod en_karebaba;
pub mod en_ko;
pub mod en_kusa;
pub mod en_md;
pub mod en_nutsball;
pub mod en_owl;
pub mod en_wood02;
pub mod en_zl4;
pub mod obj_mure2;
pub mod en_a_obj;
pub mod door_ana;
pub mod en_encount1;
pub mod en_part;
pub mod en_skb;
pub mod en_peehat;
pub mod en_ossan;
pub mod en_shopnuts;
pub mod en_river_sound;
pub mod en_st;
pub mod en_si;
pub mod en_sw;
pub mod en_tana;
pub mod en_viewer;
pub mod en_wonder_item;
pub mod en_wonder_talk2;
pub mod item_b_heart;
pub mod item_shield;
pub mod obj_bean;
pub mod obj_bombiwa;
pub mod obj_hana;
pub mod obj_kibako2;
pub mod obj_lift;
pub mod obj_makeoshihiki;
pub mod obj_oshihiki;
pub mod obj_switch;
pub mod obj_syokudai;
pub mod obj_timeblock;
pub mod object_kankyo;
pub mod player;
pub mod playthrough;
pub mod script;
pub mod en_arrow;
pub mod en_m_fire1;
pub mod bg_mjin;
pub mod bg_toki_hikari;
pub mod en_wonder_talk;
pub mod obj_mure;
pub mod en_butte;
pub mod en_gs;
pub mod en_niw_girl;
pub mod en_niw;
pub mod en_tg;
pub mod en_mu;
pub mod en_ma1;
pub mod en_dog;
pub mod en_heishi4;
pub mod en_hy;
pub mod en_ex_ruppy;
pub mod en_heishi1;
pub mod en_heishi3;
pub mod en_heishi2;
pub mod bg_spot15_saku;
pub mod bg_gate_shutter;
pub mod en_light;
pub mod obj_tsubo;
pub mod bg_spot15_rrbox;
pub mod en_ta;
pub mod oceff_wipe;
pub mod oceff_wipe2;
pub mod oceff_wipe3;
pub mod oceff_wipe4;
pub mod oceff_spot;
pub mod oceff_storm;
pub mod en_okarina_effect;
pub mod shot_sun;
pub mod en_okarina_tag;

pub mod en_cow;
pub mod en_fu;
pub mod bg_relay_objects;
pub mod en_niw_lady;
pub mod en_attack_niw;
pub mod en_daiku_kakariko;
pub mod en_toryo;
pub mod en_hs2;
pub mod en_ani;
pub mod bg_spot01_fusya;
pub mod bg_spot01_idohashira;
pub mod bg_spot01_idomizu;
pub mod bg_spot01_objects2;
pub mod en_ssh;
pub mod en_sth;
pub mod en_cs;
pub mod obj_makekinsuta;
pub mod bg_spot02_objects;
pub mod en_weather_tag;
pub mod bg_breakwall;
pub mod en_tk;
pub mod en_it;
pub mod en_rd;
pub mod bg_spot05_soko;
pub mod en_wf;
pub mod en_sa;
pub mod en_skj;
pub mod en_skjneedle;
pub mod en_dnt_nomal;
pub mod en_ex_item;
pub mod en_crow;
pub mod en_in;
pub mod en_ma2;
pub mod en_horse_link_child;
pub mod en_horse_normal;
pub mod en_bdfire;
pub mod en_bom;
pub mod en_bombf;
pub mod en_tite;
pub mod en_du;
pub mod bg_spot18_shutter;
pub mod bg_spot18_obj;
pub mod bg_spot16_bombstone;
pub mod bg_bombwall;
pub mod en_hata;
pub mod en_go2;
pub mod bg_spot18_basket;
pub mod bg_spot18_futa;
pub mod en_gm;
pub mod obj_hamishi;
pub mod obj_hsblock;
pub mod en_poh;
pub mod bg_dodoago;
pub mod bg_ddan_jd;
pub mod en_dodojr;
pub mod en_dodongo;
pub mod obj_elevator;
pub mod en_zf;
pub mod en_am;
pub mod en_vm;
pub mod bg_hidan_curtain;
pub mod bg_gnd_darkmeiro;
pub mod en_encount2;
pub mod en_fire_rock;
pub mod bg_ddan_kd;
pub mod en_g_switch;
use bg_ydan_hasi::BgYdanHasi;
use dummy_target::DummyTarget;
use player::Player;

/// The profiles of the actors this crate ports.
pub const PROFILES: &[ActorProfile] =
    &[player::PROFILE, en_holl::PROFILE, bg_ydan_hasi::PROFILE, dummy_target::PROFILE, obj_hana::PROFILE, en_ishi::PROFILE, en_kusa::PROFILE, en_kanban::PROFILE, en_ko::PROFILE, en_door::PROFILE, en_wonder_talk2::PROFILE, en_item00::PROFILE, bg_treemouth::PROFILE, en_box::PROFILE, en_wonder_item::PROFILE, en_goroiwa::PROFILE, en_md::PROFILE, en_ossan::PROFILE, en_girla::PROFILE, en_tana::PROFILE, en_elf::PROFILE, en_river_sound::PROFILE, demo_tre_lgt::PROFILE, bg_spot00_hanebasi::PROFILE, en_viewer::PROFILE, object_kankyo::PROFILE, en_dekubaba::PROFILE, en_karebaba::PROFILE, en_firefly::PROFILE, en_nutsball::PROFILE, item_shield::PROFILE, en_dekunuts::PROFILE, en_hintnuts::PROFILE, en_shopnuts::PROFILE, en_dns::PROFILE, en_st::PROFILE, en_goma::PROFILE, en_sw::PROFILE, en_si::PROFILE, door_shutter::PROFILE, bg_ydan_sp::PROFILE, obj_switch::PROFILE, obj_syokudai::PROFILE, elf_msg::PROFILE, elf_msg2::PROFILE, obj_kibako2::PROFILE, obj_lift::PROFILE, bg_ydan_maruta::PROFILE, obj_oshihiki::PROFILE, obj_makeoshihiki::PROFILE, bg_haka::PROFILE, obj_timeblock::PROFILE, obj_bombiwa::PROFILE, en_arrow::PROFILE, en_m_fire1::PROFILE, boss_goma::PROFILE, item_b_heart::PROFILE, door_warp1::PROFILE, demo_effect::PROFILE, demo_kankyo::PROFILE, bg_spot09_obj::PROFILE, bg_spot16_doughnut::PROFILE, demo_sa::PROFILE, obj_bean::PROFILE, en_owl::PROFILE, en_wood02::PROFILE, obj_mure2::PROFILE, en_a_obj::PROFILE, door_ana::PROFILE, en_encount1::PROFILE, en_part::PROFILE, en_skb::PROFILE, en_peehat::PROFILE, en_hy::PROFILE, en_heishi4::PROFILE, en_dog::PROFILE, en_ma1::PROFILE, en_mu::PROFILE, en_tg::PROFILE, en_niw::PROFILE, en_niw_girl::PROFILE, en_gs::PROFILE, en_butte::PROFILE, obj_mure::PROFILE, en_wonder_talk::PROFILE, bg_toki_hikari::PROFILE, bg_mjin::PROFILE, en_heishi3::PROFILE, en_heishi2::PROFILE, bg_spot15_saku::PROFILE, bg_gate_shutter::PROFILE, en_light::PROFILE, obj_tsubo::PROFILE, bg_spot15_rrbox::PROFILE, en_ta::PROFILE, oceff_wipe::PROFILE, oceff_wipe2::PROFILE, oceff_wipe3::PROFILE, oceff_wipe4::PROFILE, oceff_spot::PROFILE, oceff_storm::PROFILE, en_okarina_effect::PROFILE, shot_sun::PROFILE, en_okarina_tag::PROFILE, en_heishi1::PROFILE, en_ex_ruppy::PROFILE, en_zl4::PROFILE, demo_im::PROFILE, en_cow::PROFILE, en_fu::PROFILE, bg_relay_objects::PROFILE, en_niw_lady::PROFILE, en_attack_niw::PROFILE, en_daiku_kakariko::PROFILE, en_toryo::PROFILE, en_hs2::PROFILE, en_ani::PROFILE, bg_spot01_fusya::PROFILE, bg_spot01_idohashira::PROFILE, bg_spot01_idomizu::PROFILE, bg_spot01_objects2::PROFILE, en_ssh::PROFILE, en_sth::PROFILE, en_cs::PROFILE, obj_makekinsuta::PROFILE, bg_spot02_objects::PROFILE, en_weather_tag::PROFILE, bg_breakwall::PROFILE, en_tk::PROFILE, en_it::PROFILE, en_rd::PROFILE, bg_spot05_soko::PROFILE, en_wf::PROFILE, en_sa::PROFILE, en_skj::PROFILE, en_skjneedle::PROFILE, en_dnt_nomal::PROFILE, en_ex_item::PROFILE, en_crow::PROFILE, en_in::PROFILE, en_ma2::PROFILE, en_horse_link_child::PROFILE, en_horse_normal::PROFILE, en_bom::PROFILE, en_bombf::PROFILE, en_tite::PROFILE, bg_spot18_obj::PROFILE, bg_spot18_shutter::PROFILE, en_du::PROFILE, bg_bombwall::PROFILE, bg_spot16_bombstone::PROFILE, en_hata::PROFILE, en_go2::PROFILE, bg_spot18_basket::PROFILE, bg_spot18_futa::PROFILE, en_gm::PROFILE, obj_hamishi::PROFILE, obj_hsblock::PROFILE, en_poh::PROFILE, bg_ddan_jd::PROFILE, bg_dodoago::PROFILE, en_dodojr::PROFILE, obj_elevator::PROFILE, en_zf::PROFILE, bg_gnd_darkmeiro::PROFILE, bg_hidan_curtain::PROFILE, en_am::PROFILE, boss_dodongo::PROFILE, en_bdfire::PROFILE, demo_du::PROFILE, demo_go::PROFILE, en_encount2::PROFILE, en_fire_rock::PROFILE, bg_dy_yoseizo::PROFILE, en_dy_extra::PROFILE, en_m_thunder::PROFILE, eff_dust::PROFILE, bg_ddan_kd::PROFILE, en_g_switch::PROFILE, en_vm::PROFILE, en_dodongo::PROFILE];

/// The constructors `Actor_Spawn` uses for ids this crate ports.
pub fn overlays() -> Overlays {
    let mut o = Overlays::default();
    o.register(oot_game::actor_ctx::ACTOR_PLAYER, Player::init);
    o.register(en_holl::ACTOR_EN_HOLL, en_holl::EnHoll::init);
    o.register(obj_hana::ACTOR_OBJ_HANA, obj_hana::ObjHana::init);
    o.register(en_ishi::ACTOR_EN_ISHI, en_ishi::EnIshi::init);
    o.register(en_kusa::ACTOR_EN_KUSA, en_kusa::EnKusa::init);
    o.register(en_kanban::ACTOR_EN_KANBAN, en_kanban::EnKanban::init);
    o.register(en_ko::ACTOR_EN_KO, en_ko::EnKo::init);
    o.register(en_door::ACTOR_EN_DOOR, en_door::EnDoor::init);
    o.register(en_wonder_talk2::ACTOR_EN_WONDER_TALK2, en_wonder_talk2::EnWonderTalk2::init);
    o.register(en_item00::ACTOR_EN_ITEM00, en_item00::EnItem00::init);
    o.register(bg_treemouth::ACTOR_BG_TREEMOUTH, bg_treemouth::BgTreemouth::init);
    o.register(en_box::ACTOR_EN_BOX, en_box::EnBox::init);
    o.register(en_wonder_item::ACTOR_EN_WONDER_ITEM, en_wonder_item::EnWonderItem::init);
    o.register(en_goroiwa::ACTOR_EN_GOROIWA, en_goroiwa::EnGoroiwa::init);
    o.register(en_md::ACTOR_EN_MD, en_md::EnMd::init);
    o.register(en_ossan::ACTOR_EN_OSSAN, en_ossan::EnOssan::init);
    o.register(en_girla::ACTOR_EN_GIRLA, en_girla::EnGirlA::init);
    o.register(en_tana::ACTOR_EN_TANA, en_tana::EnTana::init);
    o.register(en_elf::ACTOR_EN_ELF, en_elf::EnElf::init);
    o.register(en_river_sound::ACTOR_EN_RIVER_SOUND, en_river_sound::EnRiverSound::init);
    o.register(demo_tre_lgt::ACTOR_DEMO_TRE_LGT, demo_tre_lgt::DemoTreLgt::init);
    o.register(bg_spot00_hanebasi::ACTOR_BG_SPOT00_HANEBASI, bg_spot00_hanebasi::BgSpot00Hanebasi::init);
    o.register(en_viewer::ACTOR_EN_VIEWER, en_viewer::EnViewer::init);
    o.register(object_kankyo::ACTOR_OBJECT_KANKYO, object_kankyo::ObjectKankyo::init);
    o.register(en_dekubaba::ACTOR_EN_DEKUBABA, en_dekubaba::EnDekubaba::init);
    o.register(en_karebaba::ACTOR_EN_KAREBABA, en_karebaba::EnKarebaba::init);
    o.register(en_firefly::ACTOR_EN_FIREFLY, en_firefly::EnFirefly::init);
    o.register(en_nutsball::ACTOR_EN_NUTSBALL, en_nutsball::EnNutsball::init);
    o.register(item_shield::ACTOR_ITEM_SHIELD, item_shield::ItemShield::init);
    o.register(en_dekunuts::ACTOR_EN_DEKUNUTS, en_dekunuts::EnDekunuts::init);
    o.register(en_hintnuts::ACTOR_EN_HINTNUTS, en_hintnuts::EnHintnuts::init);
    o.register(en_shopnuts::ACTOR_EN_SHOPNUTS, en_shopnuts::EnShopnuts::init);
    o.register(en_dns::ACTOR_EN_DNS, en_dns::EnDns::init);
    o.register(en_st::ACTOR_EN_ST, en_st::EnSt::init);
    // Spawned as ACTOR_EN_GOMA; its profile's id (what the actor gets) is ACTOR_BOSS_GOMA.
    o.register(en_goma::ACTOR_EN_GOMA, en_goma::EnGoma::init);
    o.register(en_sw::ACTOR_EN_SW, en_sw::EnSw::init);
    o.register(en_si::ACTOR_EN_SI, en_si::EnSi::init);
    o.register(door_shutter::ACTOR_DOOR_SHUTTER, door_shutter::DoorShutter::init);
    o.register(bg_ydan_sp::ACTOR_BG_YDAN_SP, bg_ydan_sp::BgYdanSp::init);
    o.register(ACTOR_BG_YDAN_HASI, BgYdanHasi::init);
    o.register(bg_ydan_maruta::ACTOR_BG_YDAN_MARUTA, bg_ydan_maruta::BgYdanMaruta::init);
    o.register(obj_switch::ACTOR_OBJ_SWITCH, obj_switch::ObjSwitch::init);
    o.register(obj_syokudai::ACTOR_OBJ_SYOKUDAI, obj_syokudai::ObjSyokudai::init);
    o.register(elf_msg::ACTOR_ELF_MSG, elf_msg::ElfMsg::init);
    o.register(elf_msg2::ACTOR_ELF_MSG2, elf_msg2::ElfMsg2::init);
    o.register(obj_kibako2::ACTOR_OBJ_KIBAKO2, obj_kibako2::ObjKibako2::init);
    o.register(obj_lift::ACTOR_OBJ_LIFT, obj_lift::ObjLift::init);
    o.register(obj_oshihiki::ACTOR_OBJ_OSHIHIKI, obj_oshihiki::ObjOshihiki::init);
    o.register(obj_makeoshihiki::ACTOR_OBJ_MAKEOSHIHIKI, obj_makeoshihiki::ObjMakeoshihiki::init);
    o.register(bg_haka::ACTOR_BG_HAKA, bg_haka::BgHaka::init);
    o.register(obj_timeblock::ACTOR_OBJ_TIMEBLOCK, obj_timeblock::ObjTimeblock::init);
    o.register(obj_bombiwa::ACTOR_OBJ_BOMBIWA, obj_bombiwa::ObjBombiwa::init);
    o.register(en_arrow::ACTOR_EN_ARROW, en_arrow::EnArrow::init);
    o.register(en_m_fire1::ACTOR_EN_M_FIRE1, en_m_fire1::EnMFire1::init);
    o.register(en_goma::ACTOR_BOSS_GOMA, boss_goma::BossGoma::init);
    o.register(boss_goma::ACTOR_ITEM_B_HEART, item_b_heart::ItemBHeart::init);
    o.register(boss_goma::ACTOR_DOOR_WARP1, door_warp1::DoorWarp1::init);
    o.register(demo_effect::ACTOR_DEMO_EFFECT, demo_effect::DemoEffect::init);
    o.register(demo_kankyo::ACTOR_DEMO_KANKYO, demo_kankyo::DemoKankyo::init);
    o.register(bg_spot09_obj::ACTOR_BG_SPOT09_OBJ, bg_spot09_obj::BgSpot09Obj::init);
    o.register(bg_spot16_doughnut::ACTOR_BG_SPOT16_DOUGHNUT, bg_spot16_doughnut::BgSpot16Doughnut::init);
    o.register(demo_sa::ACTOR_DEMO_SA, demo_sa::DemoSa::init);
    o.register(obj_bean::ACTOR_OBJ_BEAN, obj_bean::ObjBean::init);
    o.register(en_owl::ACTOR_EN_OWL, en_owl::EnOwl::init);
    o.register(en_wood02::ACTOR_EN_WOOD02, en_wood02::EnWood02::init);
    o.register(obj_mure2::ACTOR_OBJ_MURE2, obj_mure2::ObjMure2::init);
    o.register(en_a_obj::ACTOR_EN_A_OBJ, en_a_obj::EnAObj::init);
    o.register(door_ana::ACTOR_DOOR_ANA, door_ana::DoorAna::init);
    o.register(en_encount1::ACTOR_EN_ENCOUNT1, en_encount1::EnEncount1::init);
    o.register(en_part::ACTOR_EN_PART, en_part::EnPart::init);
    o.register(en_skb::ACTOR_EN_SKB, en_skb::EnSkb::init);
    o.register(en_peehat::ACTOR_EN_PEEHAT, en_peehat::EnPeehat::init);
    o.register(en_hy::ACTOR_EN_HY, en_hy::EnHy::init);
    o.register(en_heishi4::ACTOR_EN_HEISHI4, en_heishi4::EnHeishi4::init);
    o.register(en_dog::ACTOR_EN_DOG, en_dog::EnDog::init);
    o.register(en_ma1::ACTOR_EN_MA1, en_ma1::EnMa1::init);
    o.register(en_mu::ACTOR_EN_MU, en_mu::EnMu::init);
    o.register(en_tg::ACTOR_EN_TG, en_tg::EnTg::init);
    o.register(en_niw::ACTOR_EN_NIW, en_niw::EnNiw::init);
    o.register(en_niw_girl::ACTOR_EN_NIW_GIRL, en_niw_girl::EnNiwGirl::init);
    o.register(en_gs::ACTOR_EN_GS, en_gs::EnGs::init);
    o.register(en_butte::ACTOR_EN_BUTTE, en_butte::EnButte::init);
    o.register(obj_mure::ACTOR_OBJ_MURE, obj_mure::ObjMure::init);
    o.register(en_wonder_talk::ACTOR_EN_WONDER_TALK, en_wonder_talk::EnWonderTalk::init);
    o.register(bg_toki_hikari::ACTOR_BG_TOKI_HIKARI, bg_toki_hikari::BgTokiHikari::init);
    o.register(bg_mjin::ACTOR_BG_MJIN, bg_mjin::BgMjin::init);
    o.register(en_heishi3::ACTOR_EN_HEISHI3, en_heishi3::EnHeishi3::init);
    o.register(en_heishi2::ACTOR_EN_HEISHI2, en_heishi2::EnHeishi2::init);
    o.register(bg_spot15_saku::ACTOR_BG_SPOT15_SAKU, bg_spot15_saku::BgSpot15Saku::init);
    o.register(bg_gate_shutter::ACTOR_BG_GATE_SHUTTER, bg_gate_shutter::BgGateShutter::init);
    o.register(en_light::ACTOR_EN_LIGHT, en_light::EnLight::init);
    o.register(obj_tsubo::ACTOR_OBJ_TSUBO, obj_tsubo::ObjTsubo::init);
    o.register(bg_spot15_rrbox::ACTOR_BG_SPOT15_RRBOX, bg_spot15_rrbox::BgSpot15Rrbox::init);
    o.register(en_ta::ACTOR_EN_TA, en_ta::EnTa::init);
    o.register(oceff_wipe::ACTOR_OCEFF_WIPE, oceff_wipe::OceffWipe::init);
    o.register(oceff_wipe2::ACTOR_OCEFF_WIPE2, oceff_wipe2::OceffWipe2::init);
    o.register(oceff_wipe3::ACTOR_OCEFF_WIPE3, oceff_wipe3::OceffWipe3::init);
    o.register(oceff_wipe4::ACTOR_OCEFF_WIPE4, oceff_wipe4::OceffWipe4::init);
    o.register(oceff_spot::ACTOR_OCEFF_SPOT, oceff_spot::OceffSpot::init);
    o.register(oceff_storm::ACTOR_OCEFF_STORM, oceff_storm::OceffStorm::init);
    o.register(en_okarina_effect::ACTOR_EN_OKARINA_EFFECT, en_okarina_effect::EnOkarinaEffect::init);
    o.register(shot_sun::ACTOR_SHOT_SUN, shot_sun::ShotSun::init);
    o.register(en_okarina_tag::ACTOR_EN_OKARINA_TAG, en_okarina_tag::EnOkarinaTag::init);
    o.register(en_heishi1::ACTOR_EN_HEISHI1, en_heishi1::EnHeishi1::init);
    o.register(en_ex_ruppy::ACTOR_EN_EX_RUPPY, en_ex_ruppy::EnExRuppy::init);
    o.register(en_zl4::ACTOR_EN_ZL4, en_zl4::EnZl4::init);
    o.register(demo_im::ACTOR_DEMO_IM, demo_im::DemoIm::init);
    o.register(en_cow::ACTOR_EN_COW, en_cow::EnCow::init);
    o.register(en_fu::ACTOR_EN_FU, en_fu::EnFu::init);
    o.register(bg_relay_objects::ACTOR_BG_RELAY_OBJECTS, bg_relay_objects::BgRelayObjects::init);
    o.register(en_niw_lady::ACTOR_EN_NIW_LADY, en_niw_lady::EnNiwLady::init);
    o.register(en_attack_niw::ACTOR_EN_ATTACK_NIW, en_attack_niw::EnAttackNiw::init);
    o.register(en_daiku_kakariko::ACTOR_EN_DAIKU_KAKARIKO, en_daiku_kakariko::EnDaikuKakariko::init);
    o.register(en_toryo::ACTOR_EN_TORYO, en_toryo::EnToryo::init);
    o.register(en_hs2::ACTOR_EN_HS2, en_hs2::EnHs2::init);
    o.register(en_ani::ACTOR_EN_ANI, en_ani::EnAni::init);
    o.register(bg_spot01_fusya::ACTOR_BG_SPOT01_FUSYA, bg_spot01_fusya::BgSpot01Fusya::init);
    o.register(bg_spot01_idohashira::ACTOR_BG_SPOT01_IDOHASHIRA, bg_spot01_idohashira::BgSpot01Idohashira::init);
    o.register(bg_spot01_idomizu::ACTOR_BG_SPOT01_IDOMIZU, bg_spot01_idomizu::BgSpot01Idomizu::init);
    o.register(bg_spot01_objects2::ACTOR_BG_SPOT01_OBJECTS2, bg_spot01_objects2::BgSpot01Objects2::init);
    o.register(en_ssh::ACTOR_EN_SSH, en_ssh::EnSsh::init);
    o.register(en_sth::ACTOR_EN_STH, en_sth::EnSth::init);
    o.register(en_cs::ACTOR_EN_CS, en_cs::EnCs::init);
    o.register(obj_makekinsuta::ACTOR_OBJ_MAKEKINSUTA, obj_makekinsuta::ObjMakekinsuta::init);
    o.register(bg_spot02_objects::ACTOR_BG_SPOT02_OBJECTS, bg_spot02_objects::BgSpot02Objects::init);
    o.register(en_weather_tag::ACTOR_EN_WEATHER_TAG, en_weather_tag::EnWeatherTag::init);
    o.register(bg_breakwall::ACTOR_BG_BREAKWALL, bg_breakwall::BgBreakwall::init);
    o.register(en_tk::ACTOR_EN_TK, en_tk::EnTk::init);
    o.register(en_it::ACTOR_EN_IT, en_it::EnIt::init);
    o.register(en_rd::ACTOR_EN_RD, en_rd::EnRd::init);
    o.register(bg_spot05_soko::ACTOR_BG_SPOT05_SOKO, bg_spot05_soko::BgSpot05Soko::init);
    o.register(en_wf::ACTOR_EN_WF, en_wf::EnWf::init);
    o.register(en_sa::ACTOR_EN_SA, en_sa::EnSa::init);
    o.register(en_skj::ACTOR_EN_SKJ, en_skj::EnSkj::init);
    o.register(en_skjneedle::ACTOR_EN_SKJNEEDLE, en_skjneedle::EnSkjneedle::init);
    o.register(en_dnt_nomal::ACTOR_EN_DNT_NOMAL, en_dnt_nomal::EnDntNomal::init);
    o.register(en_ex_item::ACTOR_EN_EX_ITEM, en_ex_item::EnExItem::init);
    o.register(en_crow::ACTOR_EN_CROW, en_crow::EnCrow::init);
    o.register(en_in::ACTOR_EN_IN, en_in::EnIn::init);
    o.register(en_ma2::ACTOR_EN_MA2, en_ma2::EnMa2::init);
    o.register(en_horse_link_child::ACTOR_EN_HORSE_LINK_CHILD, en_horse_link_child::EnHorseLinkChild::init);
    o.register(en_horse_normal::ACTOR_EN_HORSE_NORMAL, en_horse_normal::EnHorseNormal::init);
    o.register(en_bom::ACTOR_EN_BOM, en_bom::EnBom::init);
    o.register(en_bombf::ACTOR_EN_BOMBF, en_bombf::EnBombf::init);
    o.register(en_tite::ACTOR_EN_TITE, en_tite::EnTite::init);
    o.register(en_du::ACTOR_EN_DU, en_du::EnDu::init);
    o.register(bg_spot18_shutter::ACTOR_BG_SPOT18_SHUTTER, bg_spot18_shutter::BgSpot18Shutter::init);
    o.register(bg_spot18_obj::ACTOR_BG_SPOT18_OBJ, bg_spot18_obj::BgSpot18Obj::init);
    o.register(bg_spot16_bombstone::ACTOR_BG_SPOT16_BOMBSTONE, bg_spot16_bombstone::BgSpot16Bombstone::init);
    o.register(bg_bombwall::ACTOR_BG_BOMBWALL, bg_bombwall::BgBombwall::init);
    o.register(en_hata::ACTOR_EN_HATA, en_hata::EnHata::init);
    o.register(en_go2::ACTOR_EN_GO2, en_go2::EnGo2::init);
    o.register(bg_spot18_basket::ACTOR_BG_SPOT18_BASKET, bg_spot18_basket::BgSpot18Basket::init);
    o.register(bg_spot18_futa::ACTOR_BG_SPOT18_FUTA, bg_spot18_futa::BgSpot18Futa::init);
    o.register(en_gm::ACTOR_EN_GM, en_gm::EnGm::init);
    o.register(obj_hamishi::ACTOR_OBJ_HAMISHI, obj_hamishi::ObjHamishi::init);
    o.register(obj_hsblock::ACTOR_OBJ_HSBLOCK, obj_hsblock::ObjHsblock::init);
    o.register(en_poh::ACTOR_EN_POH, en_poh::EnPoh::init);
    o.register(bg_dodoago::ACTOR_BG_DODOAGO, bg_dodoago::BgDodoago::init);
    o.register(bg_ddan_jd::ACTOR_BG_DDAN_JD, bg_ddan_jd::BgDdanJd::init);
    o.register(en_dodojr::ACTOR_EN_DODOJR, en_dodojr::EnDodojr::init);
    o.register(obj_elevator::ACTOR_OBJ_ELEVATOR, obj_elevator::ObjElevator::init);
    o.register(en_zf::ACTOR_EN_ZF, en_zf::EnZf::init);
    o.register(en_am::ACTOR_EN_AM, en_am::EnAm::init);
    o.register(bg_hidan_curtain::ACTOR_BG_HIDAN_CURTAIN, bg_hidan_curtain::BgHidanCurtain::init);
    o.register(bg_gnd_darkmeiro::ACTOR_BG_GND_DARKMEIRO, bg_gnd_darkmeiro::BgGndDarkmeiro::init);
    o.register(boss_dodongo::ACTOR_BOSS_DODONGO, boss_dodongo::BossDodongo::init);
    o.register(en_bdfire::ACTOR_EN_BDFIRE, en_bdfire::EnBdfire::init);
    o.register(demo_du::ACTOR_DEMO_DU, demo_du::DemoDu::init);
    o.register(demo_go::ACTOR_DEMO_GO, demo_go::DemoGo::init);
    o.register(en_encount2::ACTOR_EN_ENCOUNT2, en_encount2::EnEncount2::init);
    o.register(en_fire_rock::ACTOR_EN_FIRE_ROCK, en_fire_rock::EnFireRock::init);
    o.register(bg_dy_yoseizo::ACTOR_BG_DY_YOSEIZO, bg_dy_yoseizo::BgDyYoseizo::init);
    o.register(en_dy_extra::ACTOR_EN_DY_EXTRA, en_dy_extra::EnDyExtra::init);
    o.register(en_m_thunder::ACTOR_EN_M_THUNDER, en_m_thunder::EnMThunder::init);
    o.register(eff_dust::ACTOR_EFF_DUST, eff_dust::EffDust::init);
    o.register(bg_ddan_kd::ACTOR_BG_DDAN_KD, bg_ddan_kd::BgDdanKd::init);
    o.register(en_g_switch::ACTOR_EN_G_SWITCH, en_g_switch::EnGSwitch::init);
    o.register(en_vm::ACTOR_EN_VM, en_vm::EnVm::init);
    o.register(en_dodongo::ACTOR_EN_DODONGO, en_dodongo::EnDodongo::init);
    o
}

/// The meshes the ported actors need baked (docs/adr/0012-actor-bakes.md), for the importer.
pub fn bakes() -> Vec<oot_game::pack::MeshBake> {
    let mut v = en_kanban::bakes();
    v.extend(en_ko::bakes());
    v.extend(en_door::bakes());
    v.extend(en_item00::bakes());
    v.extend(bg_treemouth::bakes());
    v.extend(en_box::bakes());
    v.extend(en_md::bakes());
    v.extend(en_ossan::bakes());
    v.extend(en_tana::bakes());
    v.extend(en_elf::bakes());
    v.extend(bg_spot00_hanebasi::bakes());
    v.extend(en_viewer::bakes());
    v.extend(object_kankyo::bakes());
    v.extend(player::bakes());
    v.extend(en_shopnuts::bakes());
    v.extend(en_st::bakes());
    v.extend(en_goma::bakes());
    v.extend(boss_goma::bakes());
    v.extend(boss_dodongo::bakes());
    v.extend(en_bdfire::bakes());
    v.extend(demo_du::bakes());
    v.extend(bg_dy_yoseizo::bakes());
    v.extend(en_dy_extra::bakes());
    v.extend(en_m_thunder::bakes());
    v.extend(eff_dust::bakes());
    v.extend(door_warp1::bakes());
    v.extend(demo_effect::bakes());
    v.extend(demo_kankyo::bakes());
    v.extend(bg_spot16_doughnut::bakes());
    v.extend(demo_sa::bakes());
    v.extend(en_owl::bakes());
    v.extend(en_wood02::bakes());
    v.extend(en_a_obj::bakes());
    v.extend(en_part::bakes());
    v.extend(en_skb::bakes());
    v.extend(en_peehat::bakes());
    v.extend(en_sw::bakes());
    v.extend(door_shutter::bakes());
    v.extend(bg_ydan_sp::bakes());
    v.extend(bg_ydan_hasi::bakes());
    v.extend(obj_switch::bakes());
    v.extend(obj_syokudai::bakes());
    v.extend(obj_oshihiki::bakes());
    v.extend(obj_timeblock::bakes());
    // z_kankyo.c's rain and lightning bolts.
    v.extend(oot_game::weather::bakes());
    v.extend(oot_game::env_draw::bakes());
    // The effects (z_effect_soft_sprite.c's overlays and z_effect.c's).
    v.extend(oot_game::effect::bakes());
    // z_actor.c's shadows (ActorShadow_Draw's lists).
    v.extend(oot_game::shadow::bakes());
    // The circle transition (z_fbdemo_circle.c).
    v.extend(oot_game::transition::bakes());
    // z_actor.c's target reticle.
    v.extend(oot_game::target::bakes());
    // The message box's sprites (docs/adr/0017-interface-sprites.md).
    v.extend(oot_game::message::bakes().iter().map(|b| b.mesh_bake()));
    // The HUD's.
    v.extend(oot_game::interface::bakes().iter().map(|b| b.mesh_bake()));
    // The pause menu's (docs/adr/0047-the-pause-menu.md).
    v.extend(oot_game::kaleido::gfx::mesh_bakes());
    v.extend(en_arrow::bakes());
    v.extend(en_hy::bakes());
    v.extend(en_heishi4::bakes());
    v.extend(en_dog::bakes());
    v.extend(en_ma1::bakes());
    v.extend(en_mu::bakes());
    v.extend(en_tg::bakes());
    v.extend(en_niw::bakes());
    v.extend(en_niw_girl::bakes());
    v.extend(en_gs::bakes());
    v.extend(en_butte::bakes());
    v.extend(bg_toki_hikari::bakes());
    v.extend(bg_mjin::bakes());
    v.extend(en_heishi2::bakes());
    v.extend(en_light::bakes());
    v.extend(en_ta::bakes());
    v.extend(oceff_wipe::bakes());
    v.extend(oceff_wipe2::bakes());
    v.extend(oceff_wipe3::bakes());
    v.extend(oceff_wipe4::bakes());
    v.extend(oceff_spot::bakes());
    v.extend(oceff_storm::bakes());
    v.extend(en_zl4::bakes());
    v.extend(demo_im::bakes());
    v.extend(en_cow::bakes());
    v.extend(en_fu::bakes());
    v.extend(en_niw_lady::bakes());
    v.extend(en_daiku_kakariko::bakes());
    v.extend(en_toryo::bakes());
    v.extend(en_hs2::bakes());
    v.extend(en_ani::bakes());
    v.extend(bg_spot01_idomizu::bakes());
    v.extend(en_ssh::bakes());
    v.extend(en_sth::bakes());
    v.extend(en_cs::bakes());
    v.extend(bg_spot02_objects::bakes());
    v.extend(en_tk::bakes());
    v.extend(en_rd::bakes());
    v.extend(en_wf::bakes());
    v.extend(en_sa::bakes());
    v.extend(en_holl::bakes());
    v.extend(en_skj::bakes());
    v.extend(en_dnt_nomal::bakes());
    v.extend(en_ex_item::bakes());
    v.extend(en_crow::bakes());
    v.extend(en_in::bakes());
    v.extend(en_horse_link_child::bakes());
    v.extend(en_horse_normal::bakes());
    v.extend(en_bom::bakes());
    v.extend(en_bombf::bakes());
    v.extend(en_tite::bakes());
    v.extend(en_du::bakes());
    v.extend(en_hata::bakes());
    v.extend(en_go2::bakes());
    v.extend(en_gm::bakes());
    v.extend(obj_hamishi::bakes());
    v.extend(obj_hsblock::bakes());
    v.extend(en_poh::bakes());
    v.extend(en_zf::bakes());
    v.extend(en_am::bakes());
    v.extend(en_g_switch::bakes());
    v.extend(bg_hidan_curtain::bakes());
    v.extend(bg_gnd_darkmeiro::bakes());
    v.extend(en_vm::bakes());
    v
}

/// `ACTOR_ARMS_HOOK` (`actor_table.h`: 0x0066): the hookshot's tip (not ported).
pub const ACTOR_ARMS_HOOK: i16 = 0x0066;

/// `Actor_GetProjectileActor`: the first hookshot tip or arrow in the item-action list (newest
/// first) other than `me`, within `radius` of `ref_actor`, still flying, whose next ten frames'
/// path (`speed × 10` along its yaw, `velocity.y + gravity × 10` up) crosses `ref_actor`'s
/// cylinder (`colChkInfo`'s radius and height: `CollisionCheck_CylSideVsLineSeg`). @bug (game):
/// the "still flying" check reads every projectile as a hookshot (`ArmsHook`'s `timer`, at 0x210);
/// for an arrow that's the top half of `unk_210.x`, so an arrow at x 0 (to 16 bits) is skipped.
pub fn actor_get_projectile_actor(play: &PlayState, ref_actor: &Actor, me: Option<ActorHandle>, radius: f32) -> Option<ActorHandle> {
    use eng_math::{cos_s, sin_s};
    use oot_game::actor_ctx::ACTORCAT_ITEMACTION;
    play.actors.category(ACTORCAT_ITEMACTION).iter().copied().find(|&h| {
        if Some(h) == me {
            return false;
        }
        let Some(a) = play.actors.get(h) else { return false };
        let p = a.base();
        if p.id != ACTOR_ARMS_HOOK && p.id != en_arrow::ACTOR_EN_ARROW {
            return false;
        }
        // ((ArmsHook*)actor)->timer == 0: no hookshot is ported (its placeholder reads 0).
        let timer_zero = match a.as_any().downcast_ref::<en_arrow::EnArrow>() {
            Some(arrow) => arrow.unk_210.x.to_bits() >> 16 == 0,
            None => true,
        };
        if ref_actor.world_pos.distance(p.world_pos) > radius || timer_zero {
            return false;
        }
        let delta = Vec3::new((p.speed_xz * 10.0) * sin_s(p.world_rot.y), p.velocity.y + (p.gravity * 10.0), (p.speed_xz * 10.0) * cos_s(p.world_rot.y));
        let (mut sp90, mut sp84) = (Vec3::ZERO, Vec3::ZERO);
        let c = &ref_actor.col_chk_info;
        oot_game::collision_check::cyl_side_vs_line_seg(c.cyl_radius as f32, c.cyl_height as f32, 0.0, ref_actor.world_pos, p.world_pos, p.world_pos + delta, &mut sp90, &mut sp84) != 0
    })
}

/// `Player_IsBurningStickInRange(play, pos, xzRange, yRange)`: Link's lit Deku Stick's tip within
/// `xz_range` across of `pos` and 0 to `y_range` above it.
pub(crate) fn player_is_burning_stick_in_range(play: &PlayState, pos: Vec3, xz_range: f32, y_range: f32) -> bool {
    play.player.and_then(|h| play.actors.downcast::<Player>(h)).is_some_and(|p| p.is_burning_stick_in_range(pos, xz_range, y_range))
}

/// `Gfx_DrawDListOpa`: `file`'s display list `symbol` (baked with `Gfx_SetupDL_25Opa` in the
/// pack) at `Actor_Draw`'s model matrix.
pub(crate) fn gfx_draw_dlist_opa(out: &mut oot_game::play::DrawOut, file: &str, symbol: &str, rs: &oot_game::play::RenderState) {
    let m = oot_game::play::actor_draw_matrix(rs);
    out.opa.push(eng_gfx::DrawCmd::new(eng_gfx::MeshKey::named(oot_game::pack::keys::mesh(file, symbol)), m));
}

/// The game's assets for play from `pack`, with this crate's actors.
pub fn game_assets(pack: GamePack) -> anyhow::Result<Arc<GameAssets>> {
    Ok(Arc::new(GameAssets::load(pack, overlays())?))
}

/// `Play_Init` for `save.entrance_index` (see `oot_game::play_scene`).
pub fn play_entrance(assets: Arc<GameAssets>, data: Arc<GameData>, rules: Arc<PlayerRules>, save: SaveContext) -> anyhow::Result<PlayState> {
    PlayState::play_init(assets, data, rules, save)
}

/// A play state on `col` with Player (adult or child) standing at `pos`, facing `yaw`, and
/// both cameras behind it.
pub fn new_play(data: Arc<GameData>, rules: Arc<PlayerRules>, col: CollisionContext, adult: bool, pos: Vec3, yaw: i16) -> PlayState {
    let mut play = PlayState::new(data.clone(), rules, col, (pos, yaw), adult);
    let mut p = Player::new(&data, adult, pos, yaw);
    // Player_InitCommon's: the sword's trail.
    p.melee_weapon_effect_index = play.effect_add(oot_game::effect::EffectInit::Blure(oot_game::effect::blure::EffectBlureInit::Two(player::D_8085470C)));
    play.spawn(Box::new(p));
    play.respawn_player = Some(respawn_player);
    play.reset_cameras();
    play.reset_blending();
    play
}

/// The animations in objects without a skeleton, with the skeleton their actor plays them on,
/// for the importer.
pub fn foreign_anims() -> Vec<oot_game::pack::ForeignAnim> {
    let mut v = en_viewer::foreign_anims();
    v.extend(en_gm::foreign_anims());
    v
}

/// Rebuilds Player at the play state's spawn, keeping its age (the sandbox's void-out).
fn respawn_player(play: &mut PlayState) {
    let (pos, yaw) = play.spawn;
    let data = play.data.clone();
    if let Some(p) = play.player.and_then(|h| play.actors.downcast_mut::<Player>(h)) {
        let (adult, trail) = (p.adult, p.melee_weapon_effect_index);
        *p = Player::new(&data, adult, pos, yaw);
        p.melee_weapon_effect_index = trail;
    }
}

/// Typed access to the ported actors in a play state.
pub trait PlayExt {
    /// `GET_PLAYER(play)`.
    fn player(&self) -> &Player;
    fn player_mut(&mut self) -> &mut Player;
    /// The dummy targets, oldest first.
    fn targets(&self) -> Vec<ActorHandle>;
    fn target(&self, i: usize) -> &Actor;
    fn target_mut(&mut self, i: usize) -> &mut Actor;
    /// The `Bg_Ydan_Hasi` platforms, oldest first.
    fn platforms(&self) -> Vec<ActorHandle>;
    fn platform(&self, i: usize) -> &BgYdanHasi;
    /// Moves Player to `pos` facing `yaw`, standing, with the cameras behind it (a test or
    /// sandbox start somewhere other than the entrance's spawn).
    fn place_player(&mut self, pos: Vec3, yaw: i16);
    /// The equipment page's stand-in (`PlayState::equip_owned_unworn_stand_in`):
    /// [`oot_game::save::SaveContext::equip_owned_unworn`], then `Player_SetEquipmentData` as the menu's
    /// resume runs it. Returns whether anything was equipped.
    fn equip_owned_unworn(&mut self) -> bool;
    /// Adds a dummy Z-target standing at `pos`.
    fn spawn_target(&mut self, pos: Vec3) -> ActorHandle;
    /// Adds a dummy whose touch hurts Link with `effect` (`HIT_SPECIAL_EFFECT_*`).
    fn spawn_hurting_target(&mut self, pos: Vec3, effect: u8) -> ActorHandle;
    /// Adds a `Bg_Ydan_Hasi` floating block (params `0xFF00`, as Master Quest's room 5 places it)
    /// with `header` (`gDTSlidingPlatformCol`) at `home` facing `yaw`, through its init
    /// (`BgYdanHasi_Init`, as `Actor_Spawn` would run it: the sandbox's course has no object
    /// banks), floating on the collision's `waterBoxes[1]`, with its collision built for the next
    /// frame.
    fn spawn_platform(&mut self, header: Arc<CollisionHeader>, home: Vec3, yaw: i16) -> ActorHandle;
}

impl PlayExt for PlayState {
    fn player(&self) -> &Player {
        self.player.and_then(|h| self.actors.downcast::<Player>(h)).expect("Player")
    }
    fn player_mut(&mut self) -> &mut Player {
        let h = self.player.expect("Player");
        self.actors.downcast_mut::<Player>(h).expect("Player")
    }
    fn targets(&self) -> Vec<ActorHandle> {
        let mut v: Vec<ActorHandle> = self.actors.category(ACTORCAT_ENEMY).iter().copied().filter(|&h| self.actors.actor(h).is_some_and(|a| a.id == ACTOR_SANDBOX_DUMMY_TARGET)).collect();
        v.reverse();
        v
    }
    fn target(&self, i: usize) -> &Actor {
        self.actors.actor(self.targets()[i]).unwrap()
    }
    fn target_mut(&mut self, i: usize) -> &mut Actor {
        let h = self.targets()[i];
        self.actors.actor_mut(h).unwrap()
    }
    fn platforms(&self) -> Vec<ActorHandle> {
        let mut v: Vec<ActorHandle> = self.actors.category(ACTORCAT_BG).iter().copied().filter(|&h| self.actors.actor(h).is_some_and(|a| a.id == ACTOR_BG_YDAN_HASI)).collect();
        v.reverse();
        v
    }
    fn platform(&self, i: usize) -> &BgYdanHasi {
        self.actors.downcast::<BgYdanHasi>(self.platforms()[i]).unwrap()
    }
    fn place_player(&mut self, pos: Vec3, yaw: i16) {
        let data = self.data.clone();
        let p = self.player_mut();
        let a = &mut p.actor;
        a.world_pos = pos;
        a.home_pos = pos;
        a.prev_pos = pos;
        a.world_rot.y = yaw;
        a.shape_rot.y = yaw;
        a.teleported = true;
        p.current_yaw = yaw;
        p.fall_start_height = pos.y as i16;
        p.stand_still(&data);
        self.reset_cameras();
        self.reset_blending();
    }
    fn equip_owned_unworn(&mut self) -> bool {
        self.equip_owned_unworn_stand_in()
    }
    fn spawn_hurting_target(&mut self, pos: Vec3, effect: u8) -> ActorHandle {
        let h = self.spawn(Box::new(DummyTarget::hurting(pos, effect))).expect("spawn");
        self.reset_blending();
        h
    }
    fn spawn_target(&mut self, pos: Vec3) -> ActorHandle {
        let h = self.spawn(Box::new(DummyTarget::new(pos))).expect("spawn");
        self.reset_blending();
        h
    }
    fn spawn_platform(&mut self, header: Arc<CollisionHeader>, home: Vec3, yaw: i16) -> ActorHandle {
        // Actor_Spawn's copy of the profile and the placement.
        let mut a = Actor::new(home, yaw);
        bg_ydan_hasi::PROFILE.apply(&mut a);
        a.params = 0xFF00u16 as i16;
        let p = BgYdanHasi::init_with(a, self, Some(header));
        let h = self.spawn(Box::new(p)).expect("spawn");
        self.col.dyna.update_context();
        self.reset_blending();
        h
    }
}
