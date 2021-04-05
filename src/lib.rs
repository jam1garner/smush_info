#![feature(proc_macro_hygiene, asm)]

use skyline::hooks::{getRegionAddress, Region};
use skyline::from_c_str;
use skyline::libc::*;
use std::time::Duration;
use std::mem::size_of_val;
use std::sync::atomic::Ordering;

use smash::app;
use smash::app::lua_bind::*;
use smash::lib::lua_const::*;
use smash::lua2cpp::{L2CFighterCommon, L2CFighterCommon_status_pre_Rebirth, L2CFighterCommon_status_pre_Entry, L2CFighterCommon_sub_damage_uniq_process_init};
use smash::lib::L2CValue;

use smush_discord_shared::Info;

mod conversions;
use conversions::{kind_to_char, stage_id_to_stage};

static mut OFFSET1 : usize = 0x1b52a0;
static mut OFFSET2 : usize = 0x225dc2c;
static mut OFFSET3 : usize = 0xd7140;

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


static GAME_INFO: Info = Info::new();

#[allow(unreachable_code)]
fn start_server() -> Result<(), i64> {
    unsafe {
        let server_addr: sockaddr_in = sockaddr_in {
            sin_family: AF_INET as _,
            sin_port: 4242u16.to_be(),
            sin_len: 4,
            sin_addr: in_addr {
                s_addr: INADDR_ANY as _,
            },
            sin_zero: [0; 8],
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

        let mut w_tcp_socket = accept(
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
            match send_bytes(w_tcp_socket, &data) {
                Ok(_) => (),
                Err(32) => {
                    w_tcp_socket = accept(
                        tcp_socket,
                        &server_addr as *const sockaddr_in as *mut sockaddr,
                        &mut addr_len,
                    );
                }
                Err(e) => {
                    println!("send_bytes errno = {}", e);
                }
            }
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

pub fn offset_to_addr(offset: usize) -> *const () {
    unsafe {
        (getRegionAddress(Region::Text) as *const u8).offset(offset as isize) as _
    }
}

#[inline(always)]
fn get_fp() -> *const u64 {
    let r;
    unsafe { asm!("mov $0, x29" : "=r"(r) ::: "volatile") }
    r
}

#[skyline::hook(offset = OFFSET1)] //1
fn some_strlen_thing(x: usize) -> usize {
    unsafe {
        let y = (x + 0x18) as *const *const c_char;
        if !y.is_null() {
            let text = getRegionAddress(Region::Text) as u64;
            let lr_offset = *get_fp().offset(1) - text;
            let arena_id = from_c_str(*y);
            if lr_offset == OFFSET2 as u64 { //2
                let arena_id = from_c_str(*y);
                if arena_id.len() == 5 {
                    GAME_INFO.arena_id.store_str(Some(&arena_id), Ordering::SeqCst);
                }
            }
        }
    }
    original!()(x)
}

static OFFSET1_SEARCH_CODE: &[u8] = &[ //add 38
    0x81, 0x0e, 0x40, 0xf9, //.text:00000071001B5268                 LDR             X1, [X20,#0x18] ; src
    0xe0, 0x03, 0x16, 0xaa  //.text:00000071001B526C                 MOV             X0, X22 ; dest
                            //.text:00000071001B5270                 MOV             X2, X21 ; n
                            //.text:00000071001B5274                 BL              memcpy_0
                            //.text:00000071001B5278                 LDR             X8, [X19,#0x18]
                            //.text:00000071001B527C                 STRB            WZR, [X8,X21]
                            //.text:00000071001B5280                 LDP             X29, X30, [SP,#0x30+var_s0]
                            //.text:00000071001B5284                 LDP             X20, X19, [SP,#0x30+var_10]
                            //.text:00000071001B5288                 LDP             X22, X21, [SP,#0x30+var_20]
                            //.text:00000071001B528C                 LDP             X24, X23, [SP+0x30+var_30],#0x40
                            //.text:00000071001B5290                 RET
                            //below is req function
                            //.text:00000071001B52A0                 LDR             X0, [X0,#0x18] ; s
                            //.text:00000071001B52A4                 B               strlen_0
];

static OFFSET2_SEARCH_CODE: &[u8] = &[ //add -C, just below is the address needed
                            //.text:000000710225DC2C                 MOV             X0, X20 ; this
                            //.text:000000710225DC30                 BL              _ZNSt3__115recursive_mutex6unlockEv_0 ; std::__1::recursive_mutex::unlock(void)
                            //.text:000000710225DC34                 LDR             X0, [X21]
    0x60, 0x02, 0x00, 0xb4, //.text:000000710225DC38                 CBZ             X0, loc_710225DC84
    0x14, 0x58, 0x40, 0xa9  //.text:000000710225DC3C                 LDP             X20, X22, [X0]
                            //.text:000000710225DC40                 LDR             X8, [X0,#0x10]!
                            //.text:000000710225DC44                 LDR             X8, [X8]
                            //.text:000000710225DC48                 BLR             X8
                            //.text:000000710225DC4C                 LDR             X8, [X27,#0x30]
                            //.text:000000710225DC50                 LDR             X8, [X8,#8]
                            //.text:000000710225DC54                 STR             X8, [X27,#0x20]
                            //.text:000000710225DC58                 CBNZ            X8, loc_710225DC64
                            //.text:000000710225DC5C                 LDR             X8, [X27,#0x28]
                            //.text:000000710225DC60                 STR             X8, [X27,#0x20]
];

#[skyline::hook(offset = OFFSET3)] //3, remained same somehow
fn close_arena(param_1: usize) {
    GAME_INFO.arena_id.store_str(None, Ordering::SeqCst);
    original!()(param_1);
}

static OFFSET3_SEARCH_CODE: &[u8] = &[ //exact
    0xff, 0x83, 0x01, 0xd1, //.text:00000071000D7140                 SUB             SP, SP, #0x60
    0xf6, 0x57, 0x03, 0xa9, //.text:00000071000D7144                 STP             X22, X21, [SP,#0x50+var_20]
    0xf4, 0x4f, 0x04, 0xa9, //.text:00000071000D7148                 STP             X20, X19, [SP,#0x50+var_10]
    0xfd, 0x7b, 0x05, 0xa9, //.text:00000071000D714C                 STP             X29, X30, [SP,#0x50+var_s0]
    0xfd, 0x43, 0x01, 0x91, //.text:00000071000D7150                 ADD             X29, SP, #0x50
    0x15, 0x44, 0x40, 0xf9  //.text:00000071000D7154                 LDR             X21, [X0,#0x88]
                            //.text:00000071000D7158                 MOV             X19, X0
                            //.text:00000071000D715C                 CBZ             X21, loc_71000D71B4
                            //.text:00000071000D7160                 LDR             X8, [X21,#0x10]
                            //.text:00000071000D7164                 CBZ             X8, loc_71000D71B4
                            //.text:00000071000D7168                 LDP             X8, X20, [X21]
                            //.text:00000071000D716C                 LDR             X9, [X8,#8]
                            //.text:00000071000D7170                 LDR             X10, [X20]
                            //.text:00000071000D7174                 STR             X9, [X10,#8]
                            //.text:00000071000D7178                 LDR             X8, [X8,#8]
                            //.text:00000071000D717C                 CMP             X20, X21
                            //.text:00000071000D7180                 STR             X10, [X8]
                            //.text:00000071000D7184                 STR             XZR, [X21,#0x10]
                            //.text:00000071000D7188                 B.EQ            loc_71000D71B4
];

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
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
    let skin = (WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_COLOR) + 1) as u32; //returns costume slot 0-indexed... add 1 here to match costume slot number from in-game

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
        let text_ptr = getRegionAddress(Region::Text) as *const u8;
        let text_size = (getRegionAddress(Region::Rodata) as usize) - (text_ptr as usize);
        let text = std::slice::from_raw_parts(text_ptr, text_size);
        if let Some(offset) = find_subsequence(text, OFFSET1_SEARCH_CODE) {
            OFFSET1 = offset + 0x38;
        }
        if let Some(offset) = find_subsequence(text, OFFSET2_SEARCH_CODE) {
            OFFSET2 = offset - 0xc;
        }
        if let Some(offset) = find_subsequence(text, OFFSET3_SEARCH_CODE) {
            OFFSET3 = offset;
        }
    }
    skyline::install_hooks!(
        some_strlen_thing,
        close_arena
    );

    std::thread::spawn(||{
        loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
            if let Err(98) = start_server() {
                break
            }
        }
    });
}
