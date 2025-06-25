use anyhow::{anyhow, Result};
use std::fs;

fn main() -> Result<()> {
    let file_path = "./src/lib.rs";
    let content = fs::read_to_string(file_path)?;

    if let Some(val) = option_env!("CHAIN_ENV") {
        let old_id = "mC11111111111111111111111111111111111111111";
        let new_id = match val {
            "devnet" => "mCWeJ5ajaV9nwR3eMAYRTt8UL2eRKnmV96ykbrWkA1q",
            "stagenet" => "mcHYeFvgcAsQqQDesRjbNQ7viuJgyn726pCWti4YgAi",
            _ => {
                return Err(anyhow!(
                    "Wrong CHAIN_ENV value. It can only be: devnet or stagenet"
                ))
            }
        };
        let updated_content = content.replace(old_id, new_id);
        fs::write(file_path, updated_content)?;
    }
    Ok(())
}
