use twizzler_net::{NetClient, ControlCmd, ControlResponse};

pub fn handle_show(client: &mut NetClient) {
	let cmd = ControlCmd::GetInterfaceState;

	let request_id = client.submit_control_cmd(cmd)
		.expect("Failed to submit command to net-srv");

	let response = client.wait_for_completion(request_id)
		.expect("Timeout or error waiting on net-srv");

	match response {
        	ControlResponse::InterfaceState(state) => {

			let name = String::from_utf8_lossy(&state.iface_name);
                        let clean_name = name.trim_matches(char::from(0));
                        
                        println!("{}: {}", state.iface_index, clean_name);

                        // Only loop up to `address_count`!
                        for i in 0..state.address_count {
                                let addr = &state.addresses[i];
                                println!("    inet {}/{} scope global {}", addr.ip, addr.prefix, clean_name);
                        }
                },
		ControlResponse::Ok => {
                        // Handle the newly added variant gracefully
                        eprintln!("Error: Received an unexpected acknowledgment status during a show query.");
                },        	
		ControlResponse::Error => {
            		eprintln!("RTNETLINK answers: Op failed");
        	},
    	}
}

pub fn handle_add(client: &mut twizzler_net::NetClient, args: &[String]) {
	if args.len() < 3 || args[1] != "dev" {
		eprintln!("Used: ip addr add <ip/prefix> dev <interface>");
		return;
	}
	let ip_str = &args[0]; // "192.168.1.50/24"
    
	let parts: Vec<&str> = ip_str.split('/').collect();
    	if parts.len() != 2 {
        	eprintln!("Error: Invalid IP format. Use <ip>/<prefix>");
        	return;
    	}

    	// Parse the actual data
    	let ip: std::net::Ipv4Addr = match parts[0].parse() {
        	Ok(ip) => ip,
        	Err(_) => { eprintln!("Error: Invalid IPv4 address"); return; }
    	};
    
    	let prefix: u8 = match parts[1].parse() {
        	Ok(p) if p <= 32 => p,
        	_ => { eprintln!("Error: Invalid prefix length"); return; }
    	};

    	// Pack it into our memory-safe C-struct
    	let new_info = twizzler_net::Ipv4Info { ip, prefix };

    	// Send it to the kernel!
    	let req_id = client.submit_control_cmd(twizzler_net::ControlCmd::AddIp(new_info))
        	.expect("Failed to send add command");

    	// Wait for the kernel to acknowledge it
    	match client.wait_for_completion(req_id) {
        	Ok(twizzler_net::ControlResponse::Ok) => {
        	},
        	Ok(twizzler_net::ControlResponse::Error) => {
            	eprintln!("RTNETLINK answers: Operation failed");
        	},
        	_ => {
            	eprintln!("Error: Unexpected response from kernel");
        }
    }
}

pub fn handle_del(client: &mut twizzler_net::NetClient, args: &[String]) {
    if args.len() < 3 || args[1] != "dev" {
        eprintln!("Usage: ip addr del <ip>/<prefix> dev <interface>");
        return;
    }

    let parts: Vec<&str> = args[0].split('/').collect();
    if parts.len() != 2 {
        eprintln!("Error: Invalid IP format. Use <ip>/<prefix>");
        return;
    }

    let ip: std::net::Ipv4Addr = match parts[0].parse() {
        Ok(ip) => ip,
        Err(_) => { eprintln!("Error: Invalid IPv4 address"); return; }
    };
    let prefix: u8 = match parts[1].parse() {
        Ok(p) if p <= 32 => p,
        _ => { eprintln!("Error: Invalid prefix length"); return; }
    };

    let target_info = twizzler_net::Ipv4Info { ip, prefix };

    // Send the DEL command instead of ADD
    let req_id = client.submit_control_cmd(twizzler_net::ControlCmd::DelIp(target_info))
        .expect("Failed to send del command");

    match client.wait_for_completion(req_id) {
        Ok(twizzler_net::ControlResponse::Ok) => {}, // Silently succeed
        Ok(twizzler_net::ControlResponse::Error) => eprintln!("RTNETLINK answers: Cannot assign requested address"),
        _ => eprintln!("Error: Unexpected response from kernel"),
    }
}
