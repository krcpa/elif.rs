//! Route grouping macros
//!
//! Provides #[group] macro for grouping routes with shared attributes.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::Parse, parse::ParseStream, parse_macro_input, Expr, Ident, ImplItem, ItemImpl, LitStr,
    Token,
};

use crate::utils::extract_http_method_info;

#[derive(Debug, Default)]
pub struct GroupConfig {
    pub prefix: String,
    pub middleware: Vec<String>,
}

/// Parsing struct for group arguments like: "/api/v1", middleware = [cors, auth]
pub struct GroupArgs {
    prefix: LitStr,
    _comma: Option<Token![,]>,
    middleware_assignment: Option<(Ident, Token![=], Expr)>,
}

impl Parse for GroupArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let prefix = input.parse()?;

        let _comma = if input.peek(Token![,]) {
            Some(input.parse()?)
        } else {
            None
        };

        let middleware_assignment = if !input.is_empty() {
            let ident: Ident = input.parse()?;
            if ident != "middleware" {
                return Err(syn::Error::new_spanned(
                    ident,
                    "Expected 'middleware' keyword. Hint: Use syntax like #[group(\"/api\", middleware = [auth, cors])]"
                ));
            }
            let eq: Token![=] = input.parse()?;
            let expr: Expr = input.parse()?;
            Some((ident, eq, expr))
        } else {
            None
        };

        Ok(GroupArgs {
            prefix,
            _comma,
            middleware_assignment,
        })
    }
}

/// Parse group attribute arguments using robust syn parsing
pub fn parse_group_args_robust(args: TokenStream) -> syn::Result<GroupConfig> {
    let parsed_args = syn::parse::<GroupArgs>(args)?;

    let mut config = GroupConfig {
        prefix: parsed_args.prefix.value(),
        middleware: Vec::new(),
    };

    // Extract middleware from expression if present
    if let Some((_ident, _eq, expr)) = parsed_args.middleware_assignment {
        // Try to parse middleware list from various expression forms
        match &expr {
            Expr::Array(array) => {
                for elem in &array.elems {
                    if let Expr::Path(path) = elem {
                        if let Some(ident) = path.path.get_ident() {
                            config.middleware.push(ident.to_string());
                        }
                    }
                }
            }
            Expr::Path(path) => {
                // Single middleware item
                if let Some(ident) = path.path.get_ident() {
                    config.middleware.push(ident.to_string());
                }
            }
            _ => {
                // For now, ignore other expression types
                // In production, you might want to handle more complex expressions
            }
        }
    }

    Ok(config)
}

/// Parse group attribute arguments (legacy - kept for compatibility)
#[allow(dead_code)]
pub fn parse_group_args(args: TokenStream) -> GroupConfig {
    // Use the robust parser and fall back to default on error
    parse_group_args_robust(args).unwrap_or_default()
}

/// Route group macro for grouping routes with shared attributes
///
/// Groups routes under a common prefix with shared middleware.
///
/// Example:
/// ```rust,ignore
/// #[group("/api/v1", middleware = [cors, auth])]
/// impl ApiV1Routes {
///     #[get("/profile")]
///     fn profile() -> HttpResult<ElifResponse> { ... }
/// }
/// ```
pub fn group_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let input_impl = parse_macro_input!(input as ItemImpl);

    // Parse group arguments (path and optional middleware)
    let group_config = match parse_group_args_robust(args) {
        Ok(config) => config,
        Err(err) => return err.to_compile_error().into(),
    };

    let impl_name = if let syn::Type::Path(type_path) = &*input_impl.self_ty {
        if let Some(segment) = type_path.path.segments.last() {
            &segment.ident
        } else {
            return syn::Error::new_spanned(
                &input_impl.self_ty,
                "Cannot get identifier from an empty type path. Hint: Use a proper struct name like `impl MyGroup`.",
            )
            .to_compile_error()
            .into();
        }
    } else {
        return syn::Error::new_spanned(
            &input_impl.self_ty, 
            "Expected a simple type for the impl block. Hint: Apply #[group] to `impl MyStruct { ... }` not complex types."
        )
        .to_compile_error()
        .into();
    };

    let prefix = group_config.prefix;
    let _middleware_items = group_config.middleware.iter().map(|mw| {
        quote! { group = group.middleware(#mw); }
    });

    let mut route_registrations = Vec::new();

    // Process methods in the group
    for item in &input_impl.items {
        if let ImplItem::Fn(method) = item {
            let method_name = &method.sig.ident;

            if let Some((http_method, path)) = extract_http_method_info(&method.attrs) {
                let full_path = if path.is_empty() { "" } else { &path };
                route_registrations.push(quote! {
                    group = group.#http_method(#full_path, Self::#method_name);
                });
            }
        }
    }

    let route_count = route_registrations.len();

    let expanded = quote! {
        #input_impl

        impl #impl_name {
            /// Generated route group setup function
            pub fn build_group() -> String {
                // In production, this would return a proper RouteGroup instance
                // For now, return a string for testing purposes
                format!("RouteGroup at {} with {} routes", #prefix, #route_count)
            }
        }
    };

    TokenStream::from(expanded)
}
