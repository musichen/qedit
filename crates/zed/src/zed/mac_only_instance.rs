use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream},
    thread,
    time::Duration,
};

use sysinfo::System;

use release_channel::ReleaseChannel;

const LOCALHOST: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(10);
const RECEIVE_TIMEOUT: Duration = Duration::from_millis(35);
const SEND_TIMEOUT: Duration = Duration::from_millis(20);
const USER_BLOCK: u16 = 100;

fn address() -> SocketAddr {
    // Offset the base port by release channel and user ID so different Qedit
    // variants and OS users do not contend for the same localhost port.
    let release_channel = *release_channel::RELEASE_CHANNEL;
    let port = match release_channel {
        ReleaseChannel::Dev => release_channel.single_instance_port_base(),
        ReleaseChannel::Preview => release_channel.single_instance_port_base() + USER_BLOCK,
        ReleaseChannel::Stable => release_channel.single_instance_port_base() + (2 * USER_BLOCK),
        ReleaseChannel::Nightly => release_channel.single_instance_port_base() + (3 * USER_BLOCK),
    };
    let mut user_port = port;
    let mut sys = System::new_all();
    sys.refresh_all();
    if let Ok(current_pid) = sysinfo::get_current_pid()
        && let Some(uid) = sys
            .process(current_pid)
            .and_then(|process| process.user_id())
    {
        let uid_u32 = get_uid_as_u32(uid);
        // Ensure that the user ID is not too large to avoid overflow when
        // calculating the port number. This seems unlikely but it doesn't
        // hurt to be safe.
        let max_port = 65535;
        let max_uid: u32 = max_port - port as u32;
        let wrapped_uid: u16 = (uid_u32 % max_uid) as u16;
        user_port += wrapped_uid;
    }

    SocketAddr::V4(SocketAddrV4::new(LOCALHOST, user_port))
}

#[cfg(unix)]
fn get_uid_as_u32(uid: &sysinfo::Uid) -> u32 {
    *uid.clone()
}

#[cfg(windows)]
fn get_uid_as_u32(uid: &sysinfo::Uid) -> u32 {
    // Extract the RID which is an integer
    uid.to_string()
        .rsplit('-')
        .next()
        .and_then(|rid| rid.parse::<u32>().ok())
        .unwrap_or(0)
}

fn single_instance_handshake() -> &'static str {
    release_channel::RELEASE_CHANNEL.single_instance_handshake()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsOnlyInstance {
    Yes,
    No,
}

pub fn ensure_only_instance() -> IsOnlyInstance {
    if check_got_handshake() {
        return IsOnlyInstance::No;
    }

    let listener = match TcpListener::bind(address()) {
        Ok(listener) => listener,

        Err(err) => {
            log::warn!("Error binding to single instance port: {err}");
            if check_got_handshake() {
                return IsOnlyInstance::No;
            }

            // Avoid failing to start when some other application by chance already has
            // a claim on the port. This is sub-par as any other instance that gets launched
            // will be unable to communicate with this instance and will duplicate
            log::warn!("Backup handshake request failed, continuing without handshake");
            return IsOnlyInstance::Yes;
        }
    };

    thread::Builder::new()
        .name("EnsureSingleton".to_string())
        .spawn(move || {
            for stream in listener.incoming() {
                let mut stream = match stream {
                    Ok(stream) => stream,
                    Err(_) => return,
                };

                _ = stream.set_nodelay(true);
                _ = stream.set_read_timeout(Some(SEND_TIMEOUT));
                _ = stream.write_all(single_instance_handshake().as_bytes());
            }
        })
        .unwrap();

    IsOnlyInstance::Yes
}

fn check_got_handshake() -> bool {
    match TcpStream::connect_timeout(&address(), CONNECT_TIMEOUT) {
        Ok(mut stream) => {
            let mut buf = vec![0u8; single_instance_handshake().len()];

            stream.set_read_timeout(Some(RECEIVE_TIMEOUT)).unwrap();
            if let Err(err) = stream.read_exact(&mut buf) {
                log::warn!("Connected to single instance port but failed to read: {err}");
                return false;
            }

            if buf == single_instance_handshake().as_bytes() {
                log::info!("Got instance handshake");
                return true;
            }

            log::warn!("Got wrong instance handshake value");
            false
        }

        Err(_) => false,
    }
}
