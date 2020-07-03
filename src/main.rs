extern crate nanomsg;

use nanomsg::{Protocol, Socket};

use std::thread;
use std::time::Duration;

use std::io::{Read, Write};
use clipboard;

use clipboard::ClipboardProvider;
use clipboard::ClipboardContext;

const LOCAL_DEVICE_URL_PUB: &'static str = "tcp://0.0.0.0:9991";
const LOCAL_DEVICE_URL_SUP: &'static str = "tcp://0.0.0.0:9992";
const PUB_PORT: &'static str = "9991";
const SUP_PORT: &'static str = "9992";
const DELAY: u64 = 3;
const TOPIC: &'static str = "clipboard";

fn subscriber(device_address: &str) {
    let mut socket = Socket::new(Protocol::Sub).unwrap();
    let setopt = socket.subscribe(TOPIC.as_ref());
    let url = generate_url(device_address, SUP_PORT);
    let mut endpoint = socket.connect(&url).unwrap();

    match setopt {
        Ok(_) => println!("Subscribed to '{}'.", String::from_utf8_lossy(TOPIC.as_ref())),
        Err(err) => println!("Client failed to subscribe '{}'.", err),
    }

    let mut msg = String::new();
    loop {
        match socket.read_to_string(&mut msg) {
            Ok(_) => {
                let without_topic = &msg[TOPIC.len()..];
                println!("Recv '{}'.", without_topic);
                let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                ctx.set_contents(without_topic.to_string()).expect("unable to set clipboard");
                msg.clear()
            }
            Err(err) => {
                println!("Client failed to receive msg '{}'.", err);
                break;
            }
        }
    }

    endpoint.shutdown().expect("error closing endpoint");
}

fn publisher(device_address: &str) {
    let mut socket = Socket::new(Protocol::Pub).unwrap();
    let url = generate_url(device_address, PUB_PORT);
    let mut endpoint = socket.connect(&url).unwrap();

    println!("Server is ready.");

    fn get_new_clipboard_content(old_content: &String) -> String {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        loop {
            let content = ctx.get_contents().expect("could not get the content of the clipboard");
            if &content == old_content {
                thread::sleep(Duration::new(DELAY, 0));
            } else {
                break content;
            }
        }
    }

    let mut msg = Vec::with_capacity(TOPIC.len() + 16);
    let mut clipboard_content = "".to_string();
    loop {
        clipboard_content = get_new_clipboard_content(&clipboard_content);
        msg.clear();
        msg.extend_from_slice(TOPIC.as_ref());
        msg.extend_from_slice(clipboard_content.as_bytes());
        match socket.write_all(&msg) {
            Ok(..) => println!("Published '{}'.", String::from_utf8_lossy(&msg)),
            Err(err) => {
                println!("Server failed to publish '{}'.", err);
                break;
            }
        }
    }

    endpoint.shutdown().expect("error closing endpoint");
}

fn generate_url(device_address: &str, port: &'static str) -> String {
    let mut url = "tcp://".to_string();
    url.push_str(device_address);
    url.push_str(":");
    url.push_str(port);
    url
}

fn device() {
    let mut front_socket = Socket::new_for_device(Protocol::Pub).unwrap();
    let mut front_endpoint = front_socket.bind(LOCAL_DEVICE_URL_SUP).unwrap();
    let mut back_socket = Socket::new_for_device(Protocol::Sub).unwrap();
    let setopt = back_socket.subscribe(TOPIC.as_ref());
    let mut back_endpoint = back_socket.bind(LOCAL_DEVICE_URL_PUB).unwrap();

    match setopt {
        Ok(_) => println!("Subscribed to '{}'.", String::from_utf8_lossy(TOPIC.as_ref())),
        Err(err) => println!("Device failed to subscribe '{}'.", err),
    }

    println!("Device is ready.");
    Socket::device(&front_socket, &back_socket).expect("error calling Socket::device");
    println!("Device is stopped.");

    front_endpoint.shutdown().expect("error closing front_endpoint");
    back_endpoint.shutdown().expect("error closing back_endpoint");
}

fn usage() {
    println!("Usage: pubsub [client device_ip|device]");
    println!("  Try running several clients and servers");
    println!("  And also try killing and restarting them");
    println!("  Don't forget to start the device !");
}

fn main() {
    let args: Vec<_> = std::env::args().collect();

    if args.len() < 3 {
        return usage();
    }

    match args[1].as_ref() {
        "client" => {
            let address = args[2].clone();
            thread::spawn(move || {subscriber(address.as_ref())});
            publisher(args[2].as_ref());
        },
        "device" => device(),
        _ => usage(),
    }
}
