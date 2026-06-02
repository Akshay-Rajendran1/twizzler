use clap::{Parser, Subcommand};
use twizzler_net::NetClient;
mod addr;

#[derive(Parser)]
#[command(name = "ip", about = "Twizzler network utility")]
struct Cli {
	#[command(subcommand)]
	object: NetworkObject,
}

#[derive(Subcommand)]
enum NetworkObject {
	Addr {
	#[command(subcommand)]
	action: AddrAction,
	},
	Link {
	#[command(subcommand)]
	action: LinkAction,
	},
}

#[derive(Subcommand)]
pub enum AddrAction {
	Show,
	Add {
		args: Vec<String>,
	},
	Del {
		args: Vec<String>,
	},
}

#[derive(Subcommand)]
pub enum LinkAction {
	Show,
}

fn main() {
	let cli = Cli::parse();

	let mut client = NetClient::connect()
	.expect("Failed to connect to net-srv plane");

	match &cli.object {
		NetworkObject::Addr {action} => match action {
			AddrAction::Show => addr::handle_show(&mut client),
			AddrAction::Add { args } => addr::handle_add(&mut client, args),
			AddrAction::Del { args } => addr::handle_del(&mut client, args),
		},
		NetworkObject::Link {action} => match action {
			LinkAction::Show => println!("not implemented yet"),
		}
	}
}
