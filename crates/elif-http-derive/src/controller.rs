//! Controller macro implementation
//!
//! Provides the #[controller] macro for defining controller base path and metadata.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ImplItem, ItemImpl, ItemStruct, LitStr};

use crate::utils::{
    extract_http_method_info, extract_middleware_from_attrs, extract_param_types_from_attrs,
    extract_path_parameters,
};

/// Controller macro for defining controller base path and metadata
///
/// This macro should be applied to impl blocks to enable route registration.
///
/// Example:
/// ```rust,ignore
/// pub struct UserController;
///
/// #[controller("/users")]
/// impl UserController {
///     #[get("/{id}")]
///     async fn show(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
///         // handler implementation
///     }
/// }
/// ```
pub fn controller_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let path_lit = parse_macro_input!(args as LitStr);
    let base_path = path_lit.value();

    // Try to parse as impl block first (new approach)
    if let Ok(mut input_impl) = syn::parse::<ItemImpl>(input.clone()) {
        let self_ty = &input_impl.self_ty;
        let struct_name = if let syn::Type::Path(type_path) = &**self_ty {
            if let Some(segment) = type_path.path.segments.last() {
                segment.ident.to_string()
            } else {
                return syn::Error::new_spanned(
                    self_ty, 
                    "Cannot extract struct name from type path. Hint: Use a simple struct name like `MyController`."
                )
                .to_compile_error()
                .into();
            }
        } else {
            return syn::Error::new_spanned(
                self_ty, 
                "Expected a simple type for impl block. Hint: Apply #[controller] to `impl MyStruct { ... }` not complex types."
            )
            .to_compile_error()
            .into();
        };

        // Collect route information from methods
        let mut routes = Vec::new();
        let mut method_handlers = Vec::new();

        for item in &input_impl.items {
            if let ImplItem::Fn(method) = item {
                let method_name = &method.sig.ident;

                // Check for HTTP method attributes
                if let Some((http_method, path)) = extract_http_method_info(&method.attrs) {
                    let handler_name = method_name.to_string();
                    let handler_name_lit = LitStr::new(&handler_name, method_name.span());

                    // Convert http_method ident to proper HttpMethod enum variant
                    let http_method_variant = match http_method.to_string().as_str() {
                        "get" => quote! { GET },
                        "post" => quote! { POST },
                        "put" => quote! { PUT },
                        "delete" => quote! { DELETE },
                        "patch" => quote! { PATCH },
                        "head" => quote! { HEAD },
                        "options" => quote! { OPTIONS },
                        _ => unreachable!(
                            "extract_http_method_info should only return valid HTTP methods"
                        ),
                    };

                    // Extract middleware from method attributes
                    let middleware = extract_middleware_from_attrs(&method.attrs);
                    let middleware_vec = quote! { vec![#(#middleware.to_string()),*] };

                    // Extract path parameters from the route path
                    let path_params = extract_path_parameters(&path);

                    // Extract parameter type specifications from #[param] attributes
                    let param_types = extract_param_types_from_attrs(&method.attrs);

                    // Build parameter metadata with proper types
                    let mut param_tokens = Vec::new();
                    for param_name in &path_params {
                        // Get the type from #[param] attributes, default to String
                        let param_type = param_types
                            .get(param_name)
                            .cloned()
                            .unwrap_or_else(|| "String".to_string());
                        let param_type_enum = match param_type.as_str() {
                            "String" => quote! { ParamType::String },
                            "Integer" => quote! { ParamType::Integer },
                            "Uuid" => quote! { ParamType::Uuid },
                            _ => quote! { ParamType::String }, // Default fallback
                        };

                        param_tokens.push(quote! {
                            RouteParam::new(#param_name, #param_type_enum)
                        });
                    }

                    routes.push(quote! {
                        ControllerRoute {
                            method: HttpMethod::#http_method_variant,
                            path: #path.to_string(),
                            handler_name: #handler_name.to_string(),
                            middleware: #middleware_vec,
                            params: vec![#(#param_tokens),*],
                        }
                    });

                    // Generate handler for async dispatch with Arc<Self>
                    method_handlers.push(quote! {
                        #handler_name_lit => self.#method_name(request).await
                    });
                }
            }
        }

        // Add constants to the impl block
        input_impl.items.push(syn::parse_quote! {
            pub const BASE_PATH: &'static str = #base_path;
        });

        input_impl.items.push(syn::parse_quote! {
            pub const CONTROLLER_NAME: &'static str = #struct_name;
        });

        // Generate method handlers for async dispatch
        let method_match_arms = method_handlers.iter();

        // Check if struct has #[inject] to generate IocControllable trait
        let has_inject_trait = input_impl.items.iter().any(|item| {
            if let syn::ImplItem::Fn(method) = item {
                method.sig.ident == "from_ioc_container"
            } else {
                false
            }
        });

        let ioc_controllable_impl = if has_inject_trait {
            quote! {
                impl IocControllable for #self_ty {
                    fn from_ioc_container(
                        container: &elif_core::container::IocContainer,
                        scope: Option<&elif_core::container::ScopeId>,
                    ) -> Result<Self, String> {
                        Self::from_ioc_container(container, scope)
                    }
                }
            }
        } else {
            quote! {}
        };


        // Generate the expanded code with ElifController trait implementation
        // Using async-trait for proper async method support
        let expanded = quote! {
            #input_impl

            #[::async_trait::async_trait]
            impl ElifController for #self_ty {
                fn name(&self) -> &str {
                    #struct_name
                }

                fn base_path(&self) -> &str {
                    #base_path
                }

                fn routes(&self) -> Vec<ControllerRoute> {
                    vec![
                        #(#routes),*
                    ]
                }

                async fn handle_request(
                    self: std::sync::Arc<Self>,
                    method_name: String,
                    request: ElifRequest,
                ) -> HttpResult<ElifResponse> {
                    match method_name.as_str() {
                        #(#method_match_arms,)*
                        _ => {
                            Ok(ElifResponse::not_found()
                                .text(&format!("Handler '{}' not found", method_name)))
                        }
                    }
                }
            }

            #ioc_controllable_impl

            // Auto-register the controller type using inventory pattern
            // This creates a static that gets initialized when the binary loads
            ::elif_http::__controller_auto_register! {
                #struct_name,
                #self_ty
            }
        };

        TokenStream::from(expanded)
    } else if let Ok(input_struct) = syn::parse::<ItemStruct>(input) {
        // Legacy support: If applied to struct, just add constants
        let struct_name = &input_struct.ident;
        let struct_name_str = struct_name.to_string();

        let expanded = quote! {
            #input_struct

            impl #struct_name {
                pub const BASE_PATH: &'static str = #base_path;
                pub const CONTROLLER_NAME: &'static str = #struct_name_str;
            }
        };

        TokenStream::from(expanded)
    } else {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "controller attribute must be applied to an impl block or struct. Hint: Use `#[controller(\"/path\")] impl MyController { ... }` or `#[controller(\"/path\")] struct MyController;`"
        )
        .to_compile_error()
        .into()
    }
}
