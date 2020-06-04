use std::sync::atomic::{AtomicU32, AtomicBool};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Info {
    pub stage: AtomicU32,
    pub players: [Player; 8]
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Player {
    pub character: AtomicU32,
    pub stocks: AtomicU32,
    pub is_cpu: AtomicBool
}

#[derive(Clone, Copy, Debug)]
pub enum Character {
    None = 0,
    Bayonetta,	
    Brave,	
    Buddy,	
    Captain,	
    Chrom,	
    Cloud,	
    Daisy,	
    Dedede,	
    Diddy,	
    Dolly,	
    Donkey,	
    Duckhunt,	
    Falco,	
    Fox,	
    Fushigisou,	
    Gamewatch,	
    Ganon,	
    Gaogaen,	
    Gekkouga,	
    Ike,	
    Inkling,	
    Jack,	
    Kamui,	
    Ken,	
    Kirby,	
    Koopa,	
    Koopag,	
    Koopajr,	
    Krool,	
    Link,	
    Littlemac,	
    Lizardon,	
    Lucario,	
    Lucas,	
    Lucina,	
    Luigi,	
    Mario,	
    Mariod,	
    Marth,	
    Master,	
    Metaknight,	
    Mewtwo,	
    Miienemyf,	
    Miienemyg,	
    Miienemys,	
    Miifighter,	
    Miigunner,	
    Miiswordsman,	
    Murabito,	
    Nana,	
    Ness,	
    Packun,	
    Pacman,	
    Palutena,	
    Peach,	
    Pfushigisou,	
    Pichu,	
    Pikachu,	
    Pikmin,	
    Pit,	
    Pitb,	
    Plizardon,	
    Popo,	
    Purin,	
    Pzenigame,	
    Reflet,	
    Richter,	
    Ridley,	
    Robot,	
    Rockman,	
    Rosetta,	
    Roy,	
    Ryu,	
    Samus,	
    Samusd,	
    Sheik,	
    Shizue,	
    Shulk,	
    Simon,	
    Snake,	
    Sonic,	
    Szerosuit,	
    Toonlink,	
    Wario,	
    Wiifit,	
    Wolf,	
    Yoshi,	
    Younglink,	
    Zelda,	
    Zenigame,
}

// see `Character` for how this should be used
pub enum Stage {
    None = 0,
}

impl Player {
    pub const fn new() -> Self {
        Self {
            character: AtomicU32::new(Character::None as u32),
            stocks: AtomicU32::new(0),
            is_cpu: AtomicBool::new(false)
        }
    }
}
