mod game;
mod net_client;
mod rollback_runner;

use ggez::event;
use ggez::ContextBuilder;

use serde::{Deserialize, Serialize};

fn main() -> std::io::Result<()> {
    let mut input = String::new();
    println!("Host (y/n)?");
    std::io::stdin().read_line(&mut input).unwrap();

    let (client, player) = if input.trim() == "y" || input.trim().len() == 0 {
        let mut client = net_client::TestNetClient::host("127.0.0.1:10800")?;
        println!("Input player (1/2):");
        input.clear();
        std::io::stdin().read_line(&mut input).unwrap();
        let selected_player = input.trim().parse::<i32>().unwrap() == 1;
        client.write_tcp(&!selected_player)?;
        // TODO its dropping the host player select packet.
        (client, selected_player)
    } else {
        let mut client = net_client::TestNetClient::connect("127.0.0.1:10800")?;
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
