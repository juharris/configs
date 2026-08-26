use personal_dashboard::artifacts::{check_generated_artifacts, write_generated_artifacts};

fn main() {
    let result = match std::env::args().nth(1).as_deref() {
        None => write_generated_artifacts(),
        Some("--check") => check_generated_artifacts(),
        Some(argument) => {
            eprintln!("unsupported argument: {argument}");
            std::process::exit(2);
        }
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
