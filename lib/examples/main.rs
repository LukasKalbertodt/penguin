use std::path::Path;

use penguin::Config;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::new(([127, 0, 0, 1], 3001).into())
        .add_serve_dir("/", Path::new("."));
        // .proxy("http://localhost:8000".parse()?);

    let (_controller, server) = penguin::serve(config)?;

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
