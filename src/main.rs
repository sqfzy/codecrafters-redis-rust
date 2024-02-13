use anyhow::{anyhow, bail, Result};
use bytes::Buf;
use std::{
    io::{BufRead, Cursor, Read, Write},
    iter::Sum,
    net::{TcpListener, TcpStream},
    ptr::read_unaligned,
    usize,
};

fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                std::thread::spawn(move || {
                    if let Err(e) = handle(stream) {
                        println!("error: {}", e);
                    }
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

// *2\r\n$4\r\nECHO\r\n$3\r\nhey\r\n
fn handle(mut stream: TcpStream) -> Result<()> {
    loop {
        let mut buf = [0u8; 64];

        let n = stream.read(&mut buf).unwrap();
        if n == 0 {
            stream.write_all(b"$12\r\nclient error\r\n")?;
            println!("Recieve nothing from client");
            return Ok(());
        }

        println!("recieve {:?}", String::from_utf8(buf[0..n].to_vec())?);

        let mut cursor = Cursor::new(&buf[0..n]);
        skip_line(&mut cursor)?;
        skip_line(&mut cursor)?;

        let cmd_name = get_line(&mut cursor)?;
        match &cmd_name.to_ascii_lowercase()[..] {
            b"echo" => {
                let res = &cursor.get_ref()[cursor.position() as usize..];
                stream.write_all(res)?;
                stream.flush()?;
            }
            b"ping" => {
                stream.write_all(b"+PONG\r\n")?;
                stream.flush()?;
            }
            _ => unreachable!(),
        }
    }

    // Ok(())
}

// fn skip(src: &mut Cursor<&[u8]>, n: usize) -> Result<()> {
//     if src.remaining() < n {
//         bail!("Invail command format, cann't skip");
//     }
//
//     src.advance(n);
//     Ok(())
// }

fn skip_line(src: &mut Cursor<&[u8]>) -> Result<()> {
    // Scan the bytes directly
    let start = src.position() as usize;
    // Scan to the second to last byte
    let end = src.get_ref().len() - 1;

    for i in start..end {
        if src.get_ref()[i] == b'\r' && src.get_ref()[i + 1] == b'\n' {
            // We found a line, update the position to be *after* the \n
            src.set_position((i + 2) as u64);

            // Return the line
            return Ok(());
        }
    }

    bail!("Invail command format, cann't skip line");
}

/// # Example
///
/// ```rust
/// let mut cursor = Cursor::new(b"*2\r\n$4\r\nECHO\r\n$3\r\nhey\r\n");
/// let result = get_line(&mut cursor);
/// assert_eq!(result, Ok(b"*2"));
/// ```
fn get_line<'a>(src: &mut Cursor<&'a [u8]>) -> Result<&'a [u8]> {
    // Scan the bytes directly
    let start = src.position() as usize;
    // Scan to the second to last byte
    let end = src.get_ref().len() - 1;

    for i in start..end {
        if src.get_ref()[i] == b'\r' && src.get_ref()[i + 1] == b'\n' {
            // We found a line, update the position to be *after* the \n
            src.set_position((i + 2) as u64);

            // Return the line
            return Ok(&src.get_ref()[start..i]);
        }
    }

    bail!("Invail command format, cann't get line");
}

/// # Example
///
/// ```rust
/// let mut cursor = Cursor::new(b"4\r\n");
/// let len = get_len(&mut cursor);
/// assert_eq!(len, Ok(4));
/// ```
fn get_len(src: &mut Cursor<&[u8]>) -> Result<u64> {
    let line = get_line(src)?;

    String::from_utf8(line[1..].to_vec())?
        .parse::<u64>()
        .map_err(|_| anyhow!("Invail command format, cann't get len"))
}

// fn parse(mut src: Cursor<&[u8]>) -> Result<()> {
//     // match src.get_u8() {
//     //     b'$' => {
//     //         let len = get_len(&mut src)?;
//     //         let n = len as usize + 2;
//     //
//     //         if src.remaining() >= n {
//     //             return Ok(b"")
//     //         }
//     //     }
//     //     _ => {}
//     // }
//     if src.get_u8() != b'*' {
//         bail!("Invail command format");
//     }
//
//     let len = get_len(&mut src)?;
//     for _ in 0..len {
//         if src.get_u8() != b'$' {
//             bail!("Invail command format");
//         }
//     }
//     //
//     // for _ in 0..len {
//     //     let cmd_name_len = get_len(b'$', &mut src)?;
//     //     let cmd_name = get_line(&mut src)?;
//     //     match cmd_name {
//     //         b"ECHO" => {}
//     //         b"PING" => {}
//     //     }
//     // }
//
//     // *2\r\n$4\r\nECHO\r\n$3\r\nhey\r\n
//
//     // for  in  {
//     //
//     // }
//
//     Ok(())
// }
