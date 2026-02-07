#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discover_quadlets() {
        let result = discover_quadlets().await;

        match result {
            Ok(quadlets) => {
                println!("Quadlets encontrados: {}", quadlets.len());
                for quadlet in quadlets {
                    println!(
                        "- {}: {:?} ({:?})",
                        quadlet.name, quadlet.kind, quadlet.status
                    );
                }
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}
