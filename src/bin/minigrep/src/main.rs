use twizzler::object::TypedObject;
use std::env;
use std::error::Error;
use std::process;
use minigrep::{search, search_case_insensitive};
use twizzler::marker::BaseType;
use twizzler::object::Object;
use twizzler_abi::object::ObjID;
use twizzler_rt_abi::object::MapFlags;
use heapless::String as HString;
use std::string::String as StdString;

const MAX_QUERY_LEN: usize = 64;

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::build(&args).unwrap_or_else(|err| {
        eprintln!("Argument error: {err}");
        process::exit(1);
    });

    println!("Searching for {:?}", config.query);
    println!("In object {:?}", config.obj_id);

    if let Err(e) = run(config) {
        eprintln!("Application error: {e}");
        process::exit(1);
    }
}

#[derive(Debug)]
struct MessageStoreObj {
    _message: HString<256>,
}

impl BaseType for MessageStoreObj {
    fn fingerprint() -> u64 {
        11234
    }
}
/// Core logic
fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let obj = Object::<MessageStoreObj>::map(config.obj_id, MapFlags::READ)?;

    let contents = obj.base()._message.as_str();

    let results = if config.ignore_case {
        search_case_insensitive(&config.query, contents)
    } else {
        search(&config.query, contents)
    };

    for line in results {
        println!("{line}");
    }

    Ok(())
}

/// CLI configuration
struct Config {
    pub query: String,
    pub obj_id: ObjID,
    pub ignore_case: bool,
}

impl Config {
    fn build(args: &[StdString]) -> Result<Config, &'static str> {
        if args.len() < 3 {
            return Err("usage: minigrep <query> <object-id>");
        }

	let mut query = String::new();
	query.push_str(&args[1]);

        let raw = args[2].trim_start_matches("0x");
	let id_num = u128::from_str_radix(raw,16).map_err(|_| "invalid object id")?;
	
	let obj_id = ObjID::from(id_num);	

        let ignore_case = env::var("IGNORE_CASE").is_ok();

        Ok(Config {
            query,
            obj_id,
            ignore_case,
        })
    }
}

