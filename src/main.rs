// Uncomment this block to pass the first stage
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread::sleep,
    time::Duration,
};

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("accepted new connection");
                loop {
                    let mut buf = [0u8; 64];
                    match stream.read(&mut buf) {
                        Ok(_n) => {
                            println!("recieve {:?}", String::from_utf8(buf.to_vec()).unwrap());
                            stream.write_all(b"+PONG\r\n").unwrap();
                            stream.flush().unwrap();
                        }
                        Err(e) => {
                            println!("error: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
