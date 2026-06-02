use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    thread::JoinHandle,
};

use smoltcp::{
    phy::{Device, RxToken},
    time::Instant,
    wire::{EthernetFrame, PrettyPrinter},
};
use twizzler_abi::syscall::{sys_thread_sync, ThreadSync};
use twizzler_net::NetServer;
use virtio_net::TxBuffer;

use crate::NETINFO;
static SYSTEM_IPS: Mutex<Vec<twizzler_net::Ipv4Info>> = Mutex::new(Vec::new());

pub struct Client {
    pub ep: Mutex<NetServer>,
    jh: OnceLock<JoinHandle<()>>,
    pub active: AtomicBool,
    pub ports: Mutex<HashMap<u16, usize>>,
}

impl Client {
    pub fn new(ep: NetServer) -> Arc<Self> {
        let client = Arc::new(Client {
            ep: Mutex::new(ep),
            jh: OnceLock::new(),
            active: AtomicBool::new(true),
            ports: Mutex::new(HashMap::new()),
        });
        let _client = client.clone();
        let jh = std::thread::spawn(move || client_thread(_client));
        client.jh.set(jh).unwrap();
        client
    }

    fn active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}

fn client_thread(client: Arc<Client>) {
    let device = NETINFO.get().unwrap().device.clone();
    let tx_po = client.ep.lock().unwrap().client_tx_packet_object().clone();
    
    // 1. Seed the global registry on the very first connection
    {
        let mut ips = SYSTEM_IPS.lock().unwrap();
        if ips.is_empty() {
            ips.push(twizzler_net::Ipv4Info { 
                ip: std::net::Ipv4Addr::new(10, 0, 2, 15), 
                prefix: 8 
            });
        }
    } // The Mutex lock drops here so we don't freeze the system!

    while client.active() {
        let mut ep = client.ep.lock().unwrap();

        // --- INTERCEPT BLOCK ---
        if let Some((req_id, cmd)) = ep.pending_cmd.take() {
            match cmd {
		twizzler_net::ControlCmd::DelIp(target_ip) => {
                    let mut ips = SYSTEM_IPS.lock().unwrap();
                    let original_len = ips.len();
                    
                    // We tell it to KEEP any IP that does NOT match the target!
                    ips.retain(|registered_ip| {
                        registered_ip.ip != target_ip.ip || registered_ip.prefix != target_ip.prefix
                    });

                    if ips.len() < original_len {
                        ep.reply_control_cmd(req_id, twizzler_net::ControlResponse::Ok);
                    } else {
                        ep.reply_control_cmd(req_id, twizzler_net::ControlResponse::Error);
                    }
                }
                twizzler_net::ControlCmd::GetInterfaceState => {
                    let mut addresses = [twizzler_net::Ipv4Info {
                        ip: std::net::Ipv4Addr::new(0, 0, 0, 0),
                        prefix: 0,
                    }; 8];
                    
                    let mut count = 0;
                    
                    // 2. Lock the global state to READ
                    let ips = SYSTEM_IPS.lock().unwrap();
                    for (i, registered_ip) in ips.iter().enumerate() {
                        if i < 8 {
                            addresses[i] = *registered_ip;
                            count += 1;
                        }
                    }
                    drop(ips); // Release lock immediately when done reading

                    let mut iface_name = [0u8; 16];
                    let name_bytes = b"en0";
                    iface_name[..name_bytes.len()].copy_from_slice(name_bytes);

                    let state = twizzler_net::InterfaceState {
                        iface_index: 1,
                        iface_name,
                        addresses,
                        address_count: count,
                    };

                    ep.reply_control_cmd(req_id, twizzler_net::ControlResponse::InterfaceState(state));
                }

                twizzler_net::ControlCmd::AddIp(new_ip) => {
                    // 3. Lock the global state to WRITE
                    let mut ips = SYSTEM_IPS.lock().unwrap();
                    if ips.len() < 8 {
                        ips.push(new_ip); // Now it mutates the GLOBAL registry!
                        ep.reply_control_cmd(req_id, twizzler_net::ControlResponse::Ok);
                    } else {
                        ep.reply_control_cmd(req_id, twizzler_net::ControlResponse::Error);
                    }
                }
            }
        }
        // --- END OF INTERCEPT BLOCK ---

        // Your existing packet loop continues here...
        while let Some((rx, _tx)) = ep.receive(Instant::now()) {
            let packet = rx.packet;
            rx.consume(|buf| {
                if false {
                    let f = EthernetFrame::new_unchecked(&mut *buf);
                    let pp = PrettyPrinter::<EthernetFrame<&mut [u8]>>::print(&f);
                    eprintln!("client thread got {}", pp);
                }
                let tx = TxBuffer::from_packet(tx_po.clone(), buf.len(), packet, false);
                device.transmit(tx);

                //if let Some(dtx) = device.transmit(Instant::now()) {
                //    dtx.consume(buf.len(), |dbuf| dbuf.copy_from_slice(buf));
                //  }
            })
        }

        let rx_waiter = ep.rx_waiter();
        if ep.has_pending_msg_from_client() {
            continue;
        }
        drop(ep);

        let _ = sys_thread_sync(&mut [ThreadSync::new_sleep(rx_waiter)], None);
    }
}
