use std::io::Write;
use argon2::PasswordHasher;

#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    /// generates a new password hash
    #[clap(alias="generate-password-hash", alias="generate-hash", alias="password")]
    Generate
}
impl Commands {
    pub fn run(self) -> Result<(), ()> {
        match self {
            Commands::Generate => {
                print!("Password: ");
                if let Err(e) = std::io::stdout().flush() {
                    error!("error flushing stdout: {e}");
                    return Err(());
                }
                let mut password_buf = String::new();
                if let Err(e) = std::io::stdin().read_line(&mut password_buf) {
                    error!("error reading stdin: {e}");
                    return Err(());
                }
                let password = password_buf.trim();
                if password.is_empty() {
                    error!("must enter a password!");
                    return Err(());
                }
                let hasher = argon2::Argon2::default();
                let hash = match hasher.hash_password(password.as_bytes()) {
                    Ok(v) => v.to_string(),
                    Err(e) => {
                        error!("error hashing password: {e}");
                        return Err(());
                    }
                };
                println!("{hash}");
            }
        }
        Ok(())
    }
}