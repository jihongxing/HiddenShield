#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    hiddenshield_feedback_backend::run().await
}
