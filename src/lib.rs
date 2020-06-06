#![feature(proc_macro_hygiene)]

use skyline::libc::*;
use std::time::Duration;
use std::mem::size_of_val;
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};

use smash::app;
use smash::app::lua_bind::*;
use smash::lib::lua_const::*;
use smash::lua2cpp::{L2CFighterCommon, L2CFighterCommon_status_pre_Rebirth, L2CFighterCommon_status_pre_Entry, L2CFighterCommon_sub_damage_uniq_process_init};
use smash::lib::L2CValue;

use smush_discord_shared::{Info, Player, Stage};

mod conversions;
use conversions::{kind_to_char, stage_id_to_stage};

extern "C" {
    #[link_name = "\u{1}_ZN3app7utility8get_kindEPKNS_26BattleObjectModuleAccessorE"]
    pub fn get_kind(module_accessor: &mut app::BattleObjectModuleAccessor) -> i32;

    #[link_name = "\u{1}_ZN3app14sv_information8stage_idEv"]
    pub fn stage_id() -> i32;

    #[link_name = "\u{1}_ZN3app14sv_information27get_remaining_time_as_frameEv"]
    pub fn get_remaining_time_as_frame() -> u32;
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


static GAME_INFO: Info = Info {
    remaining_frames: AtomicU32::new(u32::MAX),
    is_match: AtomicBool::new(false),
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

#[allow(unreachable_code)]
fn start_server() -> Result<(), i64> {
    unsafe {
        let server_addr: sockaddr_in = sockaddr_in {
            sin_family: AF_INET as _,
            sin_port: 4242u16.to_be(),
            sin_addr: in_addr {
                s_addr: INADDR_ANY as _,
            },
            sin_zero: 0,
        };

        let mut tcp_socket = socket(AF_INET, SOCK_STREAM, 0);

        macro_rules! dbg_err {
            ($expr:expr) => {
                let rval = $expr;
                if rval < 0 {
                    let errno = *errno_loc();
                    dbg!(errno);
                    close(tcp_socket);
                    return Err(errno);
                }
            };
        }

        if (tcp_socket as u32 & 0x80000000) != 0 {
            let errno = *errno_loc();
            dbg!(errno);
            return Err(errno);
        }

        let flags: u32 = 1;

        dbg_err!(setsockopt(
            tcp_socket,
            SOL_SOCKET,
            SO_KEEPALIVE,
            &flags as *const _ as *const c_void,
            size_of_val(&flags) as u32,
        ));

        dbg_err!(bind(
            tcp_socket,
            &server_addr as *const sockaddr_in as *const sockaddr,
            size_of_val(&server_addr) as u32,
        ));

        dbg_err!(listen(tcp_socket, 1));

        let mut addr_len: u32 = 0;

        tcp_socket = accept(
            tcp_socket,
            &server_addr as *const sockaddr_in as *mut sockaddr,
            &mut addr_len,
        );

        loop {
            let mgr = *(FIGHTER_MANAGER_ADDR as *mut *mut app::FighterManager);
            let is_match = FighterManager::entry_count(mgr) > 0 &&
                !FighterManager::is_result_mode(mgr);

            if is_match {
                GAME_INFO.remaining_frames.store(get_remaining_time_as_frame(), Ordering::SeqCst);
                GAME_INFO.is_match.store(true, Ordering::SeqCst);
            } else {
                GAME_INFO.remaining_frames.store(-1.0 as u32, Ordering::SeqCst);
                GAME_INFO.is_match.store(false, Ordering::SeqCst);
            }

            let mut data = serde_json::to_vec(&GAME_INFO).unwrap();
            data.push(b'\n');
            send_bytes(tcp_socket, &data).unwrap();
            std::thread::sleep(Duration::from_millis(500));
        }
        /*let magic = recv_bytes(tcp_socket, 4).unwrap();
        if &magic == b"HRLD" {
            let num_bytes = recv_u32(tcp_socket).unwrap();
        } else if &magic == b"ECHO" {
            println!("\n\n----\nECHO\n\n");
        } else {
            println!("Invalid magic")
        }*/
        
        dbg_err!(close(tcp_socket));
    }

    Ok(())
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

    let character = kind_to_char(get_kind(module_accessor)) as u32;
    let damage = DamageModule::damage(module_accessor, 0);
    let stock_count = FighterInformation::stock_count(fighter_information) as u32;
    let is_cpu = FighterInformation::is_operation_cpu(fighter_information);
    let skin = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_COLOR) + 1; //returns costume slot 0-indexed... add 1 here to match costume slot number from in-game

    GAME_INFO.players[player_num].character.store(character, Ordering::SeqCst);
    GAME_INFO.players[player_num].damage.store(damage, Ordering::SeqCst);
    GAME_INFO.players[player_num].stocks.store(stock_count, Ordering::SeqCst);
    GAME_INFO.players[player_num].is_cpu.store(is_cpu, Ordering::SeqCst);
    GAME_INFO.players[player_num].skin.store(skin, Ordering::SeqCst);
}

#[skyline::hook(replace = L2CFighterCommon_status_pre_Entry)]
pub unsafe fn handle_pre_entry(fighter: &mut L2CFighterCommon) -> L2CValue {
    let module_accessor = app::sv_system::battle_object_module_accessor(fighter.lua_state_agent);
    set_player_information(module_accessor);

    GAME_INFO.stage.store(stage_id_to_stage(stage_id()) as u32, Ordering::SeqCst);

    original!()(fighter)
}

#[skyline::hook(replace = L2CFighterCommon_status_pre_Rebirth)]
pub unsafe fn handle_pre_rebirth(fighter: &mut L2CFighterCommon) -> L2CValue {
    let module_accessor = app::sv_system::battle_object_module_accessor(fighter.lua_state_agent);
    set_player_information(module_accessor);

    original!()(fighter)
}

#[skyline::hook(replace = L2CFighterCommon_sub_damage_uniq_process_init)]
pub unsafe fn handle_sub_damage_uniq_process_init(fighter: &mut L2CFighterCommon) -> L2CValue {
    let module_accessor = app::sv_system::battle_object_module_accessor(fighter.lua_state_agent);
    set_player_information(module_accessor);

    original!()(fighter)
}

fn nro_main(nro: &skyline::nro::NroInfo<'_>) {
    match nro.name {
        "common" => {
            skyline::install_hooks!(
                handle_pre_entry,
                handle_pre_rebirth,
                handle_sub_damage_uniq_process_init
            );
        }
        _ => (),
    }
}

#[skyline::main(name = "discord_server")]
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
