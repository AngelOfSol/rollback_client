mod game;
mod input_history;
mod net_client;
mod netcode;
mod rollback_runner;

use ggez::event;
use ggez::ContextBuilder;

fn main() -> std::io::Result<()> {
    let mut input = String::new();
    println!("Host (Y/n)?");
    std::io::stdin().read_line(&mut input).unwrap();

    let (client, player) = if input.trim() == "y" || input.trim().is_empty() {
        input.clear();
        println!("Host local (Y/n)?");
        std::io::stdin().read_line(&mut input).unwrap();

        let ip = if input.trim() == "y" || input.trim().is_empty() {
            "127.0.0.1:10800".to_owned()
        } else {
            let adapter = ipconfig::get_adapters()
                .unwrap()
                .into_iter()
                .find(|x| x.friendly_name() == "Ethernet");

            adapter
                .and_then(|adapter| {
                    adapter
                        .ip_addresses()
                        .iter()
                        .find(|item| item.is_ipv4())
                        .cloned()
                })
                .map(|ip| ip.to_string() + ":10800")
                .unwrap_or("127.0.0.1:10800".to_owned())
        };

        let mut client = net_client::TestNetClient::host(&ip)?;
        println!("Input player (1/2):");
        input.clear();
        std::io::stdin().read_line(&mut input).unwrap();
        let selected_player = input.trim().parse::<i32>().unwrap() == 1;
        client.write_tcp(&!selected_player)?;
        // TODO its dropping the host player select packet.
        (client, selected_player)
    } else {
        println!("Input target ip (defaults to 127.0.0.1:10800):");
        input.clear();
        std::io::stdin().read_line(&mut input).unwrap();
        if input.trim().is_empty() {
            input = "127.0.0.1:10800".to_owned();
        }
        let mut client = net_client::TestNetClient::connect(&input.trim())?;
        let assigned_player: bool = client.read_tcp()?;
        (client, assigned_player)
    };

    let resource_dir = if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut path = std::path::PathBuf::from(manifest_dir);
        path.push(".");
        path
    } else {
        std::path::PathBuf::from("./")
    };
    // Make a Context.
    let (mut ctx, mut event_loop) = ContextBuilder::new(
        &format!("my_game {}", if player { 1 } else { 2 }),
        "Cool Game Author",
    )
    .add_resource_path(resource_dir)
    .build()
    .expect("aieee, could not create ggez context!");

    // Create an instance of your event handler.
    // Usually, you should provide it with the Context object to
    // use when setting your game up.

    let mut my_game = rollback_runner::RollbackRunner::new(&mut ctx, player, client);

    // Run!
    match event::run(&mut ctx, &mut event_loop, &mut my_game) {
        Ok(_) => println!("Exited cleanly."),
        Err(e) => println!("Error occured: {}", e),
    }
    Ok(())
}
