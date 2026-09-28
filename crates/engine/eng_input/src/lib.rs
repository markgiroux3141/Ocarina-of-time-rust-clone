//! Input: the N64 pad state the game reads (`PadMgr` / `padutils`: buttons, press/release
//! edges, the relative stick), and the devices mapped onto it (gilrs pads with per-pad
//! profiles, and a keyboard fallback).

pub mod device;
pub mod pad;
