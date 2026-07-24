fn main() {
    if let Err(error) =
        hidden_shield_lib::desktop_offline_release_gate::run(std::env::args().skip(1).collect())
    {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
