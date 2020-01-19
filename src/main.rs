mod game;
mod rollback;
mod rollback_runner;

use ggez::event;
use ggez::ContextBuilder;

use std::net::UdpSocket;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
enum Packet {
    Hello,
    Goodbye,
}

fn main() -> std::io::Result<()> {
    let mut input = String::new();
    println!("Host (y/n)?");
    std::io::stdin().read_line(&mut input).unwrap();

    if input.trim() == "y" || input.trim().len() == 0 {
        let socket = UdpSocket::bind("127.0.0.1:10800")?;
        println!("waiting for connection");
        // Receives a single datagram message on the socket. If `buf` is too small to hold
        // the message, it will be cut off.
        let mut buf = [0; 10];
        let (amt, src) = socket.recv_from(&mut buf)?;
        socket.connect(src)?;

        // Redeclare `buf` as slice of the received data and send reverse data back to origin.
        let buf = &mut buf[..amt];
        println!("recieved {:?}", bincode::deserialize::<Packet>(&buf));

        buf.reverse();
        socket.send(&bincode::serialize(&Packet::Goodbye).unwrap())?;
    } else {
        let socket = UdpSocket::bind("127.0.0.1:10801")?;
        socket.connect("127.0.0.1:10800")?;
        let data = Packet::Hello;
        let mut buf = [5; 10];
        buf[0] = 1;

        println!("sending packet");
        socket.send(&bincode::serialize(&data).unwrap())?;
        let received_size = socket.recv(&mut buf)?;

        println!(
            "recieved {:?}",
            bincode::deserialize::<Packet>(&buf[..received_size])
        );
    }

    println!("Input player (1/2):");
    std::io::stdin().read_line(&mut input).unwrap();
    let n: i32 = input.trim().parse().unwrap();

    let resource_dir = if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut path = std::path::PathBuf::from(manifest_dir);
        path.push(".");
        path
    } else {
        std::path::PathBuf::from("./")
    };
    // Make a Context.
    let (mut ctx, mut event_loop) = ContextBuilder::new("my_game", "Cool Game Author")
        .add_resource_path(resource_dir)
        .build()
        .expect("aieee, could not create ggez context!");

    // Create an instance of your event handler.
    // Usually, you should provide it with the Context object to
    // use when setting your game up.

    let mut my_game = rollback_runner::RollbackRunner::new(
        &mut ctx,
        match n {
            1 => true,
            2 => false,
            _ => panic!("expected valid player"),
        },
        panic!(),
    );

    // Run!
    match event::run(&mut ctx, &mut event_loop, &mut my_game) {
        Ok(_) => println!("Exited cleanly."),
        Err(e) => println!("Error occured: {}", e),
    }
    Ok(())
}
