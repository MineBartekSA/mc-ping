use std::{
    process::ExitCode,
    time::Duration,
    error::Error,
    borrow::Cow,
};
use async_std::{
    net::{
        TcpStream,
        Shutdown,
    },
    io::{
        WriteExt,
        ReadExt,
    },
    task,
};
use resolv::{
    record::SRV,
    RecordType,
    Resolver,
    Class,
};
use once_cell::sync::OnceCell;

mod notify;
mod models;
use models::{
    InternalError,
    Status,
};

static HOSTNAME: OnceCell<String> = OnceCell::new();

#[async_std::main]
async fn main() -> Result<ExitCode, Box<dyn Error>> {
    let mut args = std::env::args();
    if args.len() < 2 {
        println!("Usage: {} <hostname> [port]", args.next().unwrap());
        return Ok(ExitCode::SUCCESS)
    }
    args.next();
    HOSTNAME.set(args.next().unwrap())?;
    let mut host = Cow::from(HOSTNAME.get().unwrap());
    let mut port = 25565_u16;
    if let Some(raw) = args.next() {
        match raw.parse::<u16>() {
            Ok(p) => port = p,
            Err(err) => {
                println!("Error: '{}' is not a valid port number: {}", raw, err);
                return Ok(ExitCode::FAILURE)
            }
        }
    }

    pretty_env_logger::init();

    if let Err(err) = notify::init().await {
        log::error!("failed to initialize: {}", err);
        return Ok(ExitCode::FAILURE)
    }
    
    let dname = format!("_minecraft._tcp.{}", host);
    if let Ok(mut res) = Resolver::new().unwrap().query(dname.as_bytes(), Class::IN, RecordType::SRV) {
        if let Some(record) = res.answers::<SRV>().next() {
            log::info!("SRV record found: {}:{} -> {}:{}", host, port, record.data.name, record.data.port);
            *host.to_mut() = record.data.name;
            port = record.data.port;
        }
    }

    let handshake = handshake(&host, &port);
    let mut last = 0;
    let mut fail: u8 = 0;
    let mut forge_data_fail: u8 = 0;
    loop {
        match ping(&handshake, &host, &port).await {
            Ok(mut status) => {
                forge_data_fail = 0;
                fail = 0;
                if last != status.players.online {
                    last = status.players.online;
                    log::info!("Status for {}:{}: {} {}/{}", host, port, status.description.text, status.players.online, status.players.max);
                    status.host = host.clone();
                    status.port = port;
                    notify::notify(status);
                }
            },
            Err(mut err) => {
                if err.to_string().starts_with("control character (\\u0000-\\u001F)") {
                    forge_data_fail += 1;
                    if forge_data_fail < 10 {
                        continue;
                    }
                    err = Box::new(InternalError::new("forge data control character parse failure"));
                    forge_data_fail = 0;
                }
                fail += 1;
                if fail == 10 {
                    panic!("Failed to request status 10 times! Error: {}", err);
                }
                log::error!("Failed to request status: {}", err);
            }
        }
        task::sleep(Duration::from_secs(1)).await;
    }
}

async fn ping(handshake: &[u8], host: &str, port: &u16) -> Result<Status, Box<dyn Error>> {
    log::debug!("connecting to: {}:{}", host, port);
    let mut stream = TcpStream::connect(format!("{}:{}", host, port)).await?;

    log::debug!("writing handshake {:?}", handshake);
    stream.write_all(handshake).await?;

    let status = request_status(&mut stream).await?;

    stream.write(&Vec::<u8>::new()).await?;
    stream.shutdown(Shutdown::Both)?;
    Ok(status)
}

const REQUEST: [u8; 2] = [1, 0];

async fn request_status(stream: &mut TcpStream) -> Result<Status, Box<dyn Error>> {
    log::debug!("writing request");
    stream.write_all(&REQUEST).await?;
    log::debug!("reading length");
    let length = from_var_int(stream).await?;
    log::debug!("length {}", length);
    if length > 0 {
        let prefix = from_var_int(stream).await?;
        log::debug!("string prefix: {}", prefix);

        let string_length = from_var_int(stream).await?;
        log::debug!("string length: {}", string_length);

        let mut buf = vec![0u8; string_length as usize];
        stream.read(&mut buf).await?;
        log::debug!("read status ({} bytes)\n{}", string_length, String::from_utf8_lossy(&buf));
        Ok(serde_json::from_slice::<Status>(&buf)?)
    } else {
        Err(InternalError::new("invalid status length").into())
    }
}

fn handshake(host: &str, port: &u16) -> Vec<u8> {
    let host = host.to_owned() + "\0FML3\0";
    let mut data = to_var_int(-1); // Protocol Number
    data.extend(to_var_int(host.len() as i32)); // Host length
    data.extend(host.bytes()); // Host
    data.push((port & 0x00FF) as u8); // Port lower
    data.push((port >> 8) as u8); // Port upper
    data.push(1); // Next state
    let mut handshake = to_var_int(data.len() as i32 + 1); // Packet length
    handshake.push(0); // Packet ID
    handshake.extend(data);
    handshake
}

const SEGMENT: u32 = 0x7F;
const CONTINUE: u32 = 0x80;

fn to_var_int(input: i32) -> Vec<u8> {
    let mut input = input as u32;
    let mut data = Vec::<u8>::new();
    loop {
        if input & !SEGMENT == 0 {
            data.push(input as u8);
            break;
        }
        data.push(((input & SEGMENT) | CONTINUE) as u8);
        input >>= 7;
    }
    data
}

async fn from_var_int(input: &mut TcpStream) -> Result<i32, Box<dyn Error>> {
    let mut result = 0;
    let mut i = 0;
    loop {
        let mut buf = vec![0u8; 1];
        if input.read(&mut buf).await? == 1 {
            let byte = buf[0];
            result |= (((byte as u32) & SEGMENT) as i32) << i;
            if byte & (CONTINUE as u8) == 0 {
                break
            }
        }
        i += 7;
        if i >= 32 {
            return Err(InternalError::new("varint too long").into());
        }
    }
    Ok(result as i32)
}
