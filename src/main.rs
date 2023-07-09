use tokio::net::{UdpSocket};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::{Arc};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author = "followQ", version = "1.0.20230709", about = "a simple vpn.", long_about = None)]
struct Args {
	/// local listen addr
    #[arg(short = 'l', long = "local", default_value = "0.0.0.0:12353")]
    local_listen: String,
    /// peer node addr
    #[arg(short = 'p', long = "peer", default_value = "192.168.100.18:12353")]
    peer_addr: String,
	/// crypto key if obfs was enabled
    #[arg(short = 'k', long = "key", default_value = "abcdefghijklmnopqrstuvwxyz")]
    crypto_key: String,
	/// tun device name
    #[arg(long = "dev", default_value = "gw0")]
	device_name: String,
	/// tun ipv4 cidr
    #[arg(long = "cidr4", default_value = "172.16.0.1/24")]
	cidr4: String,
	/// tun ipv6 cidr
    #[arg(long = "cidr6", default_value = "fced:9999:9999::1/64")]
	cidr6: String,
    /// enable debug mode
    #[arg(short = 'v', long = "debug", default_value_t = false)]
    debug_mode: bool,
	/// enable obfs
    #[arg(long = "obfs", default_value_t = false)]
    obfs: bool,
}

fn xor(s: Vec<u8>, key: &[u8]) -> Vec<u8> {
    let mut b = key.iter().cycle();
    s.into_iter().map(|x| x ^ b.next().unwrap()).collect()
}

fn main() {
	let args = Args::parse();
	let rt = tokio::runtime::Builder::new_multi_thread()
					.worker_threads(4)
					.thread_stack_size(3 * 1024 * 1024)
					.enable_all()
					.build()
					.unwrap();
	rt.block_on(async move {
		let crypto_key = Box::leak(args.crypto_key.into_boxed_str());
		let crypto_key = crypto_key.as_bytes();
		let sock = UdpSocket::bind(args.local_listen).await.unwrap();
		let sock_r = Arc::new(sock);
		let sock_w = sock_r.clone();
		let tun = tokio_tun::Tun::builder()
			.name(&args.device_name)
			.tap(false)
			.packet_info(false)
			.up()
			.try_build().unwrap();

		let output = std::process::Command::new("/usr/sbin/ip")
			.arg("addr")
			.arg("add")
			.arg(&args.cidr4)
			.arg("dev")
			.arg(&args.device_name)
			.output()
			.expect("failed to execute command!");
		if !output.status.success() {
			println!("add cidr4 failed!")
		}

		let output = std::process::Command::new("/usr/sbin/ip")
			.arg("-6")
			.arg("addr")
			.arg("add")
			.arg(&args.cidr6)
			.arg("dev")
			.arg(&args.device_name)
			.output()
			.expect("failed to execute command!");
		if !output.status.success() {
			println!("add cidr6 failed!")
		}

		let (mut dev_r, mut dev_w) = tokio::io::split(tun);

		tokio::spawn(async move {
			let mut buf = [0; 4096];
			loop {
				let n = dev_r.read(&mut buf).await.unwrap();
				if args.debug_mode {
					println!("{:?} bytes read", n);
				}
				let ct;
				if args.obfs {
					ct = xor(buf[..n].to_vec(), &crypto_key);
				} else {
					ct = buf[..n].to_vec();
				}
				let len = sock_w.send_to(&ct, &args.peer_addr).await.unwrap();
				if args.debug_mode {
					println!("{:?} bytes sent to {:?}", len, &args.peer_addr);
				}
			}
		});
		tokio::spawn(async move {
			let mut buf = [0; 4096];
			loop {
				let (len, addr) = sock_r.recv_from(&mut buf).await.unwrap();
				if args.debug_mode {
					println!("{:?} bytes received from {:?}", len, addr);
				}
				let pt;
				if args.obfs {
					pt = xor(buf[..len].to_vec(), &crypto_key);
				} else {
					pt = buf[..len].to_vec();
				}
				let n = dev_w.write(&pt[..]).await.unwrap();
				if args.debug_mode {
					println!("{:?} bytes write", n);
				}
			}
		});
	});
	loop{}
}
