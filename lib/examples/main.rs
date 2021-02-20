use penguin::Config;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::new(([127, 0, 0, 1], 3001).into())
        .proxy("http://localhost:8000".parse()?);

    let (_controller, server) = penguin::serve(config)?;
    server.await?;

    Ok(())
}
