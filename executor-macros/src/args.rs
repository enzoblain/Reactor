use syn::parse::{Parse, ParseStream};
use syn::{Error, Ident, LitInt, Result, Token};

pub struct MainArgs {
    pub worker_threads: usize,
}

impl Parse for MainArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut worker_threads = 1;

        if input.is_empty() {
            return Ok(Self { worker_threads });
        }

        let ident: Ident = input.parse()?;

        if ident != "worker_threads" {
            return Err(Error::new_spanned(ident, "expected `worker_threads`"));
        }

        input.parse::<Token![=]>()?;

        let value: LitInt = input.parse()?;
        worker_threads = value.base10_parse::<usize>()?;

        if worker_threads == 0 {
            return Err(Error::new_spanned(
                value,
                "worker_threads must be greater than 0",
            ));
        }

        Ok(Self { worker_threads })
    }
}
