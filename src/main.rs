use std::{io::{Read, Write}, net::TcpStream};

use ssh2::*;
use anyhow::Result;

const HOST: &str = "behemoth.labs.overthewire.org";
const PORT: usize = 2221;

fn main() -> Result<()> {

    let _pass_1 = behemoth0()?;

    Ok(())
}

fn ssh_session(username: &str, password: &str) -> Result<Session> {
    println!("connecting to server with username '{username}' and password '{password}'");

    let tcp = TcpStream::connect(format!("{HOST}:{PORT}"))?;
    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;

    session.userauth_password(username, password)?;
    println!("connected successfully");
    Ok(session)
}

fn read_all(channel: &mut Channel) -> Result<String> {
    let mut result = Vec::new();
    let mut buf = [0u8; 1024];

    loop {
        let amount_read = channel.read(&mut buf)?;
        result.extend_from_slice(&buf[0..amount_read]);
        if amount_read < buf.len() {
            break;
        }
    }

    let decoded = std::str::from_utf8(&result)?;
    Ok(decoded.to_string())
}

fn behemoth0() -> Result<String> {
    // for behemoth 0, the password to the binary can be found by looking for strcmp in an ltrace
    // upon submitting the real password, it will open a shell

    let session = ssh_session("behemoth0", "behemoth0")?;
    let mut channel = session.channel_session()?;
    channel.request_pty("xterm", None, Some((80, 24, 0, 0)))?;
    channel.shell()?;

    let mut result = String::new();
    while !result.contains("behemoth0@gibson:~$ ") {
        result = read_all(&mut channel)?;
    }

    let test_pass = "test";
    let test_cmd = format!("echo {test_pass} | ltrace /behemoth/behemoth0 2>&1");
    println!("running '{test_cmd}'");

    channel.write(format!("{test_cmd}\n").as_bytes())?;
    channel.flush()?;

    let result = read_all(&mut channel)?;
    println!("{result}");
    let result = read_all(&mut channel)?;
    println!("{result}");

    // let real_pass = result.split("\"").nth(3).unwrap(); // strcmp("my_pass", "real_pass")
    // println!("real pass is '{real_pass}'");

    // let real_cmd = format!("echo {real_pass} | /behemoth/behemoth0");
    // println!("running '{real_cmd}'");
    // write!(&mut channel, "{real_cmd}\n")?;
    // let result = read_all(&mut channel)?;
    // println!("{result}");

    // println!("retrieving /etc/behemoth_pass/behemoth1");
    // write!(&mut channel, "cat /etc/behemoth_pass/behemoth1\n")?;
    // let result = read_all(&mut channel)?;
    // println!("retrieved behemoth1 pass '{result}'");

    channel.send_eof()?;
    channel.wait_close()?;
    println!("{}", channel.exit_status()?);

    Ok(result)
}
