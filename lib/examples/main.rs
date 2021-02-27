use std::path::Path;

use penguin::Server;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (server, _controller) = Server::bind(([127, 0, 0, 1], 3001).into())
        .add_mount("/", Path::new("."))?
        .build()?;

    // // Dummy code to regularly reload all sessions.
    // tokio::spawn(async move {
    //     let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));
    //     loop {
    //         interval.tick().await;
    //         controller.reload();
    //     }
    // });

    server.await?;

    Ok(())
}
