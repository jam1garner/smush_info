#![feature(proc_macro_hygiene)]

use std::convert::TryInto;
use skyline::libc::*;
use std::net::TcpStream;
use std::io::prelude::*;
use std::time::Duration;
use std::mem::{size_of, size_of_val};
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};

use smash::app::{self};
use smash::app::lua_bind::*;
use smash::lib::lua_const::*;
use smash::lua2cpp::{L2CFighterCommon, L2CFighterCommon_status_end_Dead, L2CFighterCommon_status_pre_Entry};
use smash::lib::L2CValue;

use serde::{Serialize, Deserialize};

extern "C" {
    fn test_control();
}

fn sleep_a_bit() {
    std::thread::sleep(Duration::from_secs(3));
}

fn parse_ip(ip: &str) -> Option<in_addr> {
    let mut x = ip.split(".");
    let x: [u32; 4] = [
        x.next()?.parse().ok()?,
        x.next()?.parse().ok()?,
        x.next()?.parse().ok()?,
        x.next()?.parse().ok()?
    ];

    Some(in_addr {
        s_addr: ((x[0] << (8 * 3)) + (x[1] << (8 * 2)) + (x[2] << (8 * 1)) + x[3]).to_be()
    })
}

unsafe fn recv_bytes(socket: i32, n: usize) -> Result<Vec<u8>, i64> {
    let mut buffer = vec![0u8; n];
    let x = recv(socket, buffer.as_mut_ptr() as *mut c_void, buffer.len() as _, 0);
    if x < 0 {
        Err(*errno_loc())
    } else {
        Ok(buffer)
    }
}

unsafe fn recv_u32(socket: i32) -> Result<u32, i64> {
    let buffer = recv_bytes(socket, 4)?;
    Ok(u32::from_be_bytes((&buffer[..]).try_into().unwrap()))
}

fn send_bytes(socket: i32, bytes: &[u8]) -> Result<(), i64> {
    unsafe {
        let ret = send(socket, bytes.as_ptr() as *const _, bytes.len(), 0);
        if ret < 0 {
            Err(*errno_loc())
        } else {
            Ok(())
        }
    }
}

// all fields are atomics to allow inner mutation of statics
// use like this...
// ```rust
// GAME_INFO.players[0].stocks.store(3u32, Ordering::SeqCst);
// ```
#[derive(Serialize, Debug)]
struct Info {
    stage: AtomicU32,
    players: [Player; 8]
}

#[derive(Serialize, Debug)]
struct Player {
    character: AtomicU32,
    stocks: AtomicU32,
    is_cpu: AtomicBool
}

// cast to u32 then store in AtomicU32
// will likely need to have a big match for matching each character FIGHTER_KIND to a Character in
// the enum. Will be copy/pasted to PC client for ensuring the stuff matches up.
enum Character {
    None = 0,
}

// see `Character` for how this should be used
enum Stage {
    None = 0,
}

impl Player {
    const fn new() -> Self {
        Self {
            character: AtomicU32::new(Character::None as u32),
            stocks: AtomicU32::new(0),
            is_cpu: AtomicBool::new(false)
        }
    }
}

static GAME_INFO: Info = Info {
    stage: AtomicU32::new(Stage::None as u32),
    players: [
        Player::new(),
        Player::new(),
        Player::new(),
        Player::new(),
        Player::new(),
        Player::new(),
        Player::new(),
        Player::new()
    ]
};

fn start_server() -> Result<(), i64> {
    unsafe {
        let mut serverAddr: sockaddr_in = sockaddr_in {
            sin_family: AF_INET as _,
            sin_port: 4242u16.to_be(),
            sin_addr: in_addr {
                s_addr: INADDR_ANY as _,
            },
            sin_zero: 0,
        };

        let mut g_tcpSocket = socket(AF_INET, SOCK_STREAM, 0);

        macro_rules! dbg_err {
            ($expr:expr) => {
                let rval = $expr;
                if rval < 0 {
                    let errno = *errno_loc();
                    dbg!(errno);
                    close(g_tcpSocket);
                    return Err(errno);
                }
            };
        }

        if (g_tcpSocket as u32 & 0x80000000) != 0 {
            let errno = *errno_loc();
            dbg!(errno);
            return Err(errno);
        }

        let flags: u32 = 1;

        dbg_err!(setsockopt(
            g_tcpSocket,
            SOL_SOCKET,
            SO_KEEPALIVE,
            &flags as *const _ as *const c_void,
            size_of_val(&flags) as u32,
        ));

        dbg_err!(bind(
            g_tcpSocket,
            &serverAddr as *const sockaddr_in as *const sockaddr,
            size_of_val(&serverAddr) as u32,
        ));

        dbg_err!(listen(g_tcpSocket, 1));

        let mut addrLen: u32 = 0;

        g_tcpSocket = accept(
            g_tcpSocket,
            &serverAddr as *const sockaddr_in as *mut sockaddr,
            &mut addrLen,
        );

        loop {
            let data = serde_json::to_vec(&GAME_INFO).unwrap();
            send_bytes(g_tcpSocket, &data).unwrap();
            std::thread::sleep(Duration::from_millis(500));
        }
        /*let magic = recv_bytes(g_tcpSocket, 4).unwrap();
        if &magic == b"HRLD" {
            let num_bytes = recv_u32(g_tcpSocket).unwrap();
        } else if &magic == b"ECHO" {
            println!("\n\n----\nECHO\n\n");
        } else {
            println!("Invalid magic")
        }*/
        
        dbg_err!(close(g_tcpSocket));
    }

    Ok(())
}

extern "C" {   
    #[link_name = "\u{1}_ZN3app7utility8get_kindEPKNS_26BattleObjectModuleAccessorE"]
    pub fn get_kind(module_accessor: &mut app::BattleObjectModuleAccessor) -> i32;

    #[link_name = "\u{1}_ZN3app14sv_information8stage_idEv"]
    pub fn stage_id() -> i32;
}

pub static mut FIGHTER_MANAGER_ADDR: usize = 0;

pub unsafe fn set_player_information(module_accessor: &mut app::BattleObjectModuleAccessor) {
    let entry_id = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as i32;
    let player_num = entry_id as usize;
    let mgr = *(FIGHTER_MANAGER_ADDR as *mut *mut app::FighterManager);
    let fighter_information = FighterManager::get_fighter_information(
        mgr, 
        app::FighterEntryID(entry_id)
    ) as *mut app::FighterInformation;

    let character = get_kind(module_accessor) as u32;
    let stock_count = FighterInformation::stock_count(fighter_information) as u32;
    let dead_count = FighterInformation::dead_count(fighter_information, 0) as u32;
    let is_cpu = FighterInformation::is_operation_cpu(fighter_information);

    GAME_INFO.players[player_num].character.store(character, Ordering::SeqCst);
    GAME_INFO.players[player_num].stocks.store(stock_count - dead_count, Ordering::SeqCst);
    GAME_INFO.players[player_num].is_cpu.store(is_cpu, Ordering::SeqCst);
}

#[skyline::hook(replace = L2CFighterCommon_status_pre_Entry)]
pub unsafe fn handle_pre_entry(fighter: &mut L2CFighterCommon) -> L2CValue {
    let module_accessor = app::sv_system::battle_object_module_accessor(fighter.lua_state_agent);
    set_player_information(module_accessor);

    GAME_INFO.stage.store(stage_id() as u32, Ordering::SeqCst);

    original!()(fighter)
}

#[skyline::hook(replace = L2CFighterCommon_status_end_Dead)]
pub unsafe fn handle_end_dead(fighter: &mut L2CFighterCommon) -> L2CValue {
    let module_accessor = app::sv_system::battle_object_module_accessor(fighter.lua_state_agent);
    set_player_information(module_accessor);

    original!()(fighter)
}


fn nro_main(nro: &skyline::nro::NroInfo<'_>) {
    match nro.name {
        "common" => {
            skyline::install_hooks!(
                handle_pre_entry,
                handle_end_dead
            );
        }
        _ => (),
    }
}

#[skyline::main(name = "skyline_rs_template")]
pub fn main() {
    skyline::nro::add_hook(nro_main).unwrap();
    unsafe {
        skyline::nn::ro::LookupSymbol(
            &mut FIGHTER_MANAGER_ADDR,
            "_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E\u{0}".as_bytes().as_ptr(),
        );
    }

    std::thread::spawn(||{
        loop {
            if let Err(98) = start_server() {
                break
            }
        }
    });
    //let mut sock = TcpStream::connect("192.168.86.46:5001").unwrap();
    //sock.write(b"test test test").unwrap();
}
