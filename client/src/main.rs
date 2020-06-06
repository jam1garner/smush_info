use smush_discord_shared::{Info, Stage};
use std::net::{TcpStream, IpAddr};
use std::io::{BufRead, BufReader};
use rustcord::{Rustcord, EventHandlers, User, RichPresenceBuilder, RichPresence};
use std::time::{Duration, SystemTime};

pub struct Handlers;

impl EventHandlers for Handlers {
    fn ready(user: User) {
        println!("User {}#{} logged in...", user.username, user.discriminator);
    }
}

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

fn stage_to_image_key(stage: Stage) -> String {
    let name = format!("{:?}", stage);
    let name = name.trim_start_matches('_');
    if name.starts_with("Battle_") {
        &name["Battle_".len()..]
    } else if name.starts_with("End_") {
        &name["End_".len()..]
    } else {
        name
    }.to_owned().to_lowercase()
}

fn info_to_presence(info: &Info) -> RichPresence {
    if info.is_match() {
        RichPresenceBuilder::new()
            .state("In Match")
            .details(
                &format!(
                    "{} {} - {} {}",
                    info.players[0].character(),
                    info.players[0].stocks(),
                    info.players[1].stocks(),
                    info.players[1].character(),
                )
            )
            .large_image_key(&stage_to_image_key(info.stage()))
            .large_image_text(&info.stage().into_normal().to_string())
            .end_time(
                SystemTime::now()
                    + Duration::from_secs_f64(
                        (info.remaining_frames() as f64) / 60.0
                    )
            )
            .build()
    } else {
        RichPresenceBuilder::new()
            .state("In Menus")
            .large_image_key("smash_ball")
            .build()
    }
}

fn main() {
    let packets = BufReader::new(TcpStream::connect((get_home_ip(), 4242u16)).unwrap()).split(b'\n');

    let discord = Rustcord::init::<Handlers>("718317785016565790", true, None).unwrap();


    for packet in packets {
        let info = get_info(&packet.unwrap());
        let presence = info_to_presence(&info);

        let res = discord.update_presence(presence);
        if let Err(error) = res {
            dbg!(error);
        }

        discord.run_callbacks();
    }
}
