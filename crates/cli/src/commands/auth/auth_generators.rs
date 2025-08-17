use elif_core::ElifError;

pub async fn generate_key(length: usize) -> Result<(), ElifError> {
    use rand::Rng;
    
    println!("🔑 Generating secure key...");
    
    if length < 32 {
        return Err(ElifError::Validation("Key length must be at least 32 bytes".to_string()));
    }
    
    let mut rng = rand::thread_rng();
    let key: Vec<u8> = (0..length).map(|_| rng.gen()).collect();
    let key_hex = hex::encode(&key);
    
    println!("Generated {}-byte key:", length);
    println!("{}", key_hex);
    println!();
    println!("⚠️  Keep this key secure and never commit it to version control!");
    println!("💡 Add this to your .env file as JWT_SECRET or SESSION_SECRET");
    
    Ok(())
}