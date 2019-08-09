extern crate clap;
use clap::{App, Arg};

use failure::Error;

use std::fs;
use std::io::Write;
use std::io::Read;
use std::net::{TcpStream, SocketAddr, IpAddr, Ipv4Addr};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

fn main() -> Result<(), Error> {
    let matches = App::new("DesignerHelper-rs")
        .version("1.0")
        .author("Jai <814683@qq.com>")
        .about("QT Designer Helper")
        .arg(
            Arg::with_name("file")
                .short("f")
                .long("file")
                .help("Input .ui File")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    std::process::exit(match run(matches) {
        Err(error) => {
            println!("[ERROR] An error has occured. Error chain:");
            println!("{}", error);

            for cause in error.iter_causes() {
                println!("{}", cause);
            }

            1
        }
        Ok(_) => 0,
    });
}

fn run(matches: clap::ArgMatches) -> Result<(), Error> {
    let ui = matches.value_of("file").unwrap();

    if !Path::new(ui).exists() {
        panic!("invalid .ui file: {}", ui);
    }

    let port = get_designer_port();

    if port <= 0 || !send_to_designer(port, ui) {
        launch_desiner_server(ui)?;
    }
    Ok(())
}

fn get_designer_port() -> u16 {
    let pid = Path::new("./.pid");
    let mut port = 0;

    if pid.exists() {
        let c = fs::read_to_string(pid).unwrap();
        port = c.parse().unwrap();
    }

    return port;
}

fn send_to_designer(pid: u16, ui: &str) -> bool {
    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), pid);
    match TcpStream::connect_timeout(&socket, Duration::from_millis(1)) {
        Ok(mut stream) => {
            let msg = format!("{}\n", ui);
            stream.write(msg.as_bytes()).unwrap();
            return true;
        }
        Err(_) => {
            return false;
        }
    }
}

fn launch_desiner_server(ui: &str) -> Result<(), Error> {
    use find_folder::Search;

    let git_path = Search::Parents(20)
        .of(Path::new(ui).parent().unwrap().to_path_buf())
        .for_folder(".git")
        .unwrap();

    let mut search_env = false;

    if git_path.is_dir() {
        let root = Search::Kids(1).of(git_path.parent().unwrap().to_path_buf());
        if !root.for_folder("IDE").unwrap().is_dir() || !root.for_folder(".premake").unwrap().is_dir()
        {
            search_env = true;
        }
    }

    let qtdesigner: String;
    if !search_env {
        qtdesigner = format!(
            "{}\\Client\\3rd\\qt\\bin\\designer.exe",
            git_path.parent().unwrap().display()
        );
    } else {
        let env = std::env::var("QTDIR")?;
        qtdesigner = format!("{}\\bin\\designer.exe", env);
    }

    println!("desinger.exe path: {}", qtdesigner);

    if !Path::new(&qtdesigner).exists() {
        panic!("not found designer: {}", qtdesigner);
    }

    let mut child = Command::new(qtdesigner)
        .arg("--server")
        .arg(ui)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start process");

    let buf = &mut [0; 256];
    let mut port = String::new();
    while let Ok(bytes) = child.stdout.as_mut().unwrap().read(buf) {
        port = String::from_utf8_lossy(&buf[..bytes]).parse()?;
        port = port.trim_end().to_string();
        if !port.is_empty() {
            break;
        }
    }

    write_designer_port(&port);

    Ok(())
}

fn write_designer_port(port: &str) {
    let pid = Path::new("./.pid");
    fs::write(pid, port).expect("failed to write pid");
}
