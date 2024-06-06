use std::fmt::Display;
use std::path::PathBuf;
use std::process::{self, Termination};

use solana_program::pubkey::Pubkey;

#[derive(Debug, PartialEq)]
pub(crate) enum Report {
    Build(PathBuf),
    Deploy(Pubkey),
    Init(String),
}

impl Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Report::Build(output_contract_path) => {
                f.write_str("Contract binary: ")?;
                f.write_str(&output_contract_path.to_string_lossy())
            }
            Report::Deploy(id) => {
                f.write_str("Following contract was deployed: ")?;
                f.write_str(id.to_string().as_str())?;
                f.write_str("\n")?;
                Ok(())
            }
            Report::Init(program) => {
                // TODO, do we have something useful to return in the output ? This is just a
                // placeholder...
                f.write_str("Initialized contract: ")?;
                f.write_str(program.as_str())?;
                Ok(())
            }
        }
    }
}

impl Termination for Report {
    fn report(self) -> std::process::ExitCode {
        println!("Report: {self}");
        process::ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {

    use std::str::FromStr;

    use indoc::indoc;

    use super::*;

    #[test]
    fn deploy_result_prints_information_correctly() {
        let result = Report::Deploy(
            Pubkey::from_str("HgVNmBRqGuhu9qfQCs6mEykDWfMG4hHfRZ3r9kPbQT1t").unwrap(),
        );

        let expected = indoc! {"
            Following contract was deployed: HgVNmBRqGuhu9qfQCs6mEykDWfMG4hHfRZ3r9kPbQT1t
        "};
        assert_eq!(expected, format!("{result}"));
    }
}
