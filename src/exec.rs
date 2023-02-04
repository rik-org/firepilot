use crate::Result;

pub trait Args {
    fn args(&self) -> Result<Vec<String>>;
}

pub trait Executable: Args {
    fn exec(&self, args: &Vec<String>) -> Result<String>;

    fn concat_args(&self, args: &Vec<String>) -> Result<Vec<String>> {
        let mut combined = self.args()?;
        combined.extend(args.iter().cloned());
        Ok(combined)
    }
}
