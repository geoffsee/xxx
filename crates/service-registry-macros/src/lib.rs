use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr, Token};
use syn::parse::{Parse, ParseStream};

struct ServiceRegistrationArgs {
    name: LitStr,
    address: LitStr,
    port: syn::LitInt,
}

impl Parse for ServiceRegistrationArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let address: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let port: syn::LitInt = input.parse()?;

        Ok(ServiceRegistrationArgs {
            name,
            address,
            port,
        })
    }
}

/// Macro to bootstrap service registration with etcd
///
/// # Example
/// ```ignore
/// use service_registry::register_service;
///
/// #[tokio::main]
/// async fn main() {
///     let (service, lease_id) = register_service!("my-service", "localhost", 8080).await;
/// }
/// ```
#[proc_macro]
pub fn register_service(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as ServiceRegistrationArgs);

    let name = args.name;
    let address = args.address;
    let port = args.port;

    let expanded = quote! {
        service_registry::bootstrap_service(#name, #address, #port)
    };

    TokenStream::from(expanded)
}