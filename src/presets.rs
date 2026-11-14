use crate::dsp::Params;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub desc: String,
    pub category: String,
    pub color: [u8; 3],
    pub shape: usize,
    pub params: Params,
}
pub fn factory() -> Vec<Preset> {
    let defs: [(&str, &str, &str, [u8; 3]); 24] = [
        (
            "Original",
            "Clean and balanced.",
            "Character",
            [199, 200, 186],
        ),
        (
            "Velvet",
            "Low, warm and smooth.",
            "Character",
            [225, 175, 113],
        ),
        ("Titan", "Deep and powerful.", "Character", [186, 149, 117]),
        (
            "Bright",
            "Higher, lighter, brighter.",
            "Character",
            [220, 199, 132],
        ),
        (
            "Helium",
            "Small voice, big energy.",
            "Character",
            [197, 204, 133],
        ),
        (
            "Broadcast",
            "Full broadcast tone.",
            "Character",
            [146, 192, 177],
        ),
        (
            "Android",
            "A voice from the future.",
            "Synthetic",
            [137, 194, 187],
        ),
        (
            "Robot",
            "Metallic ring modulation.",
            "Synthetic",
            [154, 180, 203],
        ),
        (
            "Overdrive",
            "Grit and saturation.",
            "Synthetic",
            [215, 143, 119],
        ),
        (
            "8-bit",
            "Vintage game console.",
            "Synthetic",
            [181, 201, 142],
        ),
        (
            "Alien",
            "An otherworldly tremble.",
            "Synthetic",
            [181, 166, 206],
        ),
        (
            "Cyberpunk",
            "Distorted neon character.",
            "Synthetic",
            [207, 150, 172],
        ),
        (
            "Phantom",
            "A haunting, trailing voice.",
            "Atmosphere",
            [170, 175, 209],
        ),
        (
            "Cathedral",
            "Stone cathedral ambience.",
            "Atmosphere",
            [198, 183, 151],
        ),
        ("Orbit", "Echoes from orbit.", "Atmosphere", [141, 172, 201]),
        (
            "Underwater",
            "Muted and drifting.",
            "Atmosphere",
            [127, 185, 185],
        ),
        (
            "Demon",
            "Deep voice, dark tail.",
            "Atmosphere",
            [204, 125, 112],
        ),
        (
            "Dream",
            "Airy tone, soft echoes.",
            "Atmosphere",
            [201, 176, 195],
        ),
        (
            "Telephone",
            "Vintage telephone band.",
            "Retro",
            [182, 197, 162],
        ),
        (
            "Radio",
            "Overdriven portable radio.",
            "Retro",
            [205, 184, 128],
        ),
        (
            "Megaphone",
            "Sharp megaphone tone.",
            "Retro",
            [215, 157, 109],
        ),
        (
            "Cassette",
            "Warm cassette flutter.",
            "Retro",
            [192, 169, 140],
        ),
        (
            "Arcade",
            "Digital arcade texture.",
            "Retro",
            [173, 190, 126],
        ),
        (
            "Noir",
            "A low voice from classic cinema.",
            "Retro",
            [171, 176, 181],
        ),
    ];
    defs.into_iter()
        .enumerate()
        .map(|(i, (name, desc, category, color))| {
            let mut p = Params::default();
            match i {
                1 => {
                    p.pitch = -3.;
                    p.bass = 4.;
                    p.air = -2.;
                    p.ratio = 3.;
                }
                2 => {
                    p.pitch = -7.;
                    p.bass = 6.;
                    p.body = 2.;
                    p.drive = 0.07;
                }
                3 => {
                    p.pitch = 3.;
                    p.air = 4.;
                    p.bass = -2.;
                }
                4 => {
                    p.pitch = 9.;
                    p.bass = -6.;
                    p.air = 3.;
                }
                5 => {
                    p.bass = 3.;
                    p.body = 1.5;
                    p.air = 3.;
                    p.threshold = -24.;
                    p.ratio = 4.;
                    p.deess = 0.45;
                }
                6 => {
                    p.pitch = -2.;
                    p.robot = 0.55;
                    p.ring_hz = 95.;
                    p.drive = 0.12;
                }
                7 => {
                    p.robot = 0.95;
                    p.ring_hz = 65.;
                    p.body = 3.;
                    p.bass = -2.;
                }
                8 => {
                    p.drive = 0.65;
                    p.body = 4.;
                    p.lowpass = 6500.;
                }
                9 => {
                    p.bits = 5.;
                    p.lowpass = 6500.;
                    p.pitch = 2.;
                }
                10 => {
                    p.pitch = 5.;
                    p.robot = 0.35;
                    p.ring_hz = 175.;
                    p.vibrato = 0.3;
                    p.reverb = 0.15;
                }
                11 => {
                    p.pitch = -4.;
                    p.drive = 0.35;
                    p.robot = 0.45;
                    p.ring_hz = 130.;
                    p.delay = 0.16;
                }
                12 => {
                    p.pitch = -2.;
                    p.reverb = 0.65;
                    p.room = 0.85;
                    p.delay = 0.23;
                    p.lowpass = 6500.;
                }
                13 => {
                    p.reverb = 0.75;
                    p.room = 0.95;
                    p.delay = 0.12;
                    p.delay_ms = 320.;
                }
                14 => {
                    p.pitch = 2.;
                    p.delay = 0.6;
                    p.delay_ms = 420.;
                    p.feedback = 0.65;
                    p.reverb = 0.2;
                }
                15 => {
                    p.pitch = -2.;
                    p.lowpass = 1400.;
                    p.vibrato = 0.65;
                    p.vibrato_hz = 2.;
                    p.reverb = 0.3;
                }
                16 => {
                    p.pitch = -10.;
                    p.drive = 0.18;
                    p.bass = 5.;
                    p.robot = 0.2;
                    p.ring_hz = 35.;
                    p.reverb = 0.4;
                }
                17 => {
                    p.pitch = 4.;
                    p.air = 4.;
                    p.reverb = 0.5;
                    p.delay = 0.25;
                    p.vibrato = 0.12;
                }
                18 => {
                    p.bass = -12.;
                    p.body = 7.;
                    p.air = -9.;
                    p.lowpass = 3200.;
                    p.drive = 0.08;
                }
                19 => {
                    p.bass = -10.;
                    p.body = 6.;
                    p.lowpass = 4200.;
                    p.drive = 0.3;
                    p.bits = 9.;
                }
                20 => {
                    p.bass = -9.;
                    p.body = 9.;
                    p.lowpass = 5500.;
                    p.drive = 0.5;
                }
                21 => {
                    p.air = -6.;
                    p.bass = 2.;
                    p.drive = 0.1;
                    p.vibrato = 0.22;
                    p.vibrato_hz = 0.7;
                    p.lowpass = 7500.;
                }
                22 => {
                    p.pitch = 7.;
                    p.bits = 5.;
                    p.robot = 0.3;
                    p.ring_hz = 220.;
                    p.delay = 0.13;
                }
                23 => {
                    p.pitch = -4.;
                    p.air = -5.;
                    p.bass = 3.;
                    p.lowpass = 6500.;
                    p.drive = 0.08;
                }
                _ => {}
            }
            Preset {
                name: name.into(),
                desc: desc.into(),
                category: category.into(),
                color,
                shape: i,
                params: p,
            }
        })
        .collect()
}
