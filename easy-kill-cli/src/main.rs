#![feature(getpid)]

extern crate clap;
extern crate console;
extern crate regex;
extern crate libc;
#[macro_use]
extern crate lazy_static;

mod checkbox;

use std::ffi::CStr;
use std::process::{self, Command};

use regex::{Regex, Captures};

use checkbox::Checkbox;

// $> ps aux:
// USER PID %CPU %MEM VSZ RSS TT STAT STARTED TIME COMMAND
const PS_PATTERN: &'static str = concat!(
    r"(?P<user>\S+)\s+",
    r"(?P<pid>\S+)\s+",
    r"(?P<cpu>\S+)\s+",
    r"(?P<mem>\S+)\s+",
    r"(?P<vsz>\S+)\s+",
    r"(?P<rss>\S+)\s+",
    r"(?P<tt>\S+)\s+",
    r"(?P<stat>\S+)\s+",
    r"(?P<started>\S+)\s+",
    r"(?P<time>\S+)\s+",
    r"(?P<command>.+)",
);
const PID_RANGE_PATTERN: &'static str = r"^(?P<start>\d+)-(?P<end>\d+)$";


lazy_static! {
    static ref PS_REGEX: Regex = Regex::new(PS_PATTERN).unwrap();
    static ref PID_RANGE_REGEX: Regex = Regex::new(PID_RANGE_PATTERN).unwrap();
}

// struct ProcessInfo {
//     user: String,
//     pid: u32,
//     port: u16,
//     command: String,
// }

fn parse_pid_range(s: &str) -> Result<(u32, u32), ()> {
    let get_match = |caps: &Captures, name: &str| -> Option<u32> {
        caps.name(name).map(|m| m.as_str().parse::<u32>().unwrap())
    };
    PID_RANGE_REGEX.captures(s)
        .and_then(|caps| {
            match (get_match(&caps, "start"), get_match(&caps, "end")) {
                (Some(start), Some(end)) if start <= end => Some((start, end)),
                _ => None
            }
        })
        .ok_or(())
}

fn main() {
    let matches = clap::App::new("Easy kill processes")
        .arg(clap::Arg::with_name("pattern")
             .takes_value(true)
             .required(false)
             .index(1))
        .arg(clap::Arg::with_name("selected")
             .long("selected")
             .short("s")
             .takes_value(false))
        .arg(clap::Arg::with_name("pid-range")
             .long("pid-range")
             .short("r")
             .validator(|s| {
                 match parse_pid_range(s.as_str()) {
                     Ok(_) => Ok(()),
                     Err(_) => Err(String::from("Invalid pid range"))
                 }
             })
             .takes_value(true))
        .get_matches();
    let pattern = matches.value_of("pattern");
    let selected = matches.is_present("selected");
    let (pid_start, pid_end) = matches.value_of("pid-range")
        .map_or((0, std::u32::MAX), |s| parse_pid_range(s).unwrap());

    if let Some(pattern) = pattern {
        let mut ps_child = Command::new("ps")
            .arg("aux")
            .stdout(process::Stdio::piped())
            .spawn()
            .expect("ERROR: run `ps aux` failed");
        let ps_pid = ps_child.id();
        let output = ps_child.wait_with_output().unwrap();
        let output_string = String::from_utf8_lossy(&output.stdout);
        let pattern = Regex::new(pattern).unwrap();
        let stats = output_string
            .lines()
            .skip(1)
            .filter_map(|line| {
                if pattern.is_match(line) {
                    let caps = PS_REGEX.captures(line).unwrap();
                    let pid = caps
                        .name("pid")
                        .map_or(0, |m| m.as_str().parse::<u32>().unwrap());
                    if pid != ps_pid
                        && pid != process::id()
                        && pid >= pid_start
                        && pid <= pid_end
                    {
                        let command = caps
                            .name("command")
                            .map_or("", |m| m.as_str());
                        return Some((pid, command));
                    }
                }
                None
            })
            .collect::<Vec<(u32, &str)>>();

        if stats.is_empty() {
            println!("WARNING: no process found!");
        } else {
            let items = stats
                .iter()
                .map(|&(pid, command)| format!("[{}]: {}", pid, command))
                .collect::<Vec<String>>();
            let selections = Checkbox::new()
                .default(selected)
                .items(items
                       .iter()
                       .map(|s| s.as_ref())
                       .collect::<Vec<&str>>()
                       .as_slice())
                .interact()
                .unwrap();

            if selections.is_empty() {
                println!("You did not select anything :(");
            } else {
                println!("You selected these processes:");
                for selection in selections {
                    match unsafe { libc::kill(stats[selection].0 as i32, 15) } {
                        0 => println!(" success {}", items[selection]),
                        code @ _ => {
                            let err_msg = (unsafe {
                                CStr::from_ptr(libc::strerror(-code))
                            }).to_str().unwrap();
                            println!(" failed({}: {}) {}", code, err_msg, items[selection]);
                        },
                    }
                }
            }
        }
    } else {
        println!("no pattern given!");
    }
}
