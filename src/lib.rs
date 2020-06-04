#![feature(proc_macro_hygiene)]

use std::convert::TryInto;
use skyline::libc::*;
use std::net::TcpStream;
use std::io::prelude::*;
use std::time::Duration;
use std::mem::{size_of, size_of_val};
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};

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

#[skyline::main(name = "skyline_rs_template")]
pub fn main() {
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
