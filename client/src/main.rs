use smush_discord_shared::Info;
use std::net::{TcpStream, IpAddr};
use std::io::{BufRead, BufReader};

const IP_ADDR_FILE: &str = "ip_addr.txt";

fn get_home_ip_str() -> Option<String> {
    let switch_home_dir = dirs::home_dir()?.join(".switch");
    if switch_home_dir.exists() {
        let ip_addr_file = switch_home_dir.join(IP_ADDR_FILE);
        if ip_addr_file.exists() {
            std::fs::read_to_string(ip_addr_file).ok()
        } else {
            None
        }
    } else {
        None
    }
}

fn get_home_ip() -> IpAddr {
    let ip = get_home_ip_str().unwrap();
    dbg!(ip).trim().parse().unwrap()
}

fn get_info(bytes: &[u8]) -> Info {
    serde_json::from_slice(bytes).unwrap()
}

fn main() {
    let packets = BufReader::new(TcpStream::connect((get_home_ip(), 4242u16)).unwrap()).split(b'\n');

    for packet in packets {
        println!("{:#?}", get_info(&packet.unwrap()));
    }
}
