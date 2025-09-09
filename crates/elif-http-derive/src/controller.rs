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
                        #handler_name_lit => {
                            self.#method_name(request).await
                        }
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
        let method_match_arms = &method_handlers;

        // Check if this controller needs dependency injection and extract constructor info
        let mut needs_dependency_injection = false;
        let mut constructor_param_types = Vec::new();
        let mut constructor_param_names = Vec::new();
        
        for item in &input_impl.items {
            if let syn::ImplItem::Fn(method) = item {
                if method.sig.ident == "new" && !method.sig.inputs.is_empty() {
                    needs_dependency_injection = true;
                    // Extract parameter types and names from constructor
                    // Note: new() is a static method, so no 'self' parameter to skip
                    for (i, input) in method.sig.inputs.iter().enumerate() {
                        if let syn::FnArg::Typed(pat_type) = input {
                            if let syn::Type::Path(type_path) = &*pat_type.ty {
                                if let Some(segment) = type_path.path.segments.last() {
                                    constructor_param_types.push(segment.ident.clone());
                                    // Generate parameter variable names like param_0, param_1, etc.
                                    let param_name = syn::Ident::new(&format!("param_{}", i), segment.ident.span());
                                    constructor_param_names.push(param_name);
                                }
                            }
                        }
                    }
                    break;
                }
            }
        }

        // Generate appropriate trait implementations based on DI needs
        let ioc_controllable_impl = if needs_dependency_injection {
            quote! {
                // Auto-generated IocControllable implementation for dependency injection
                impl ::elif_http::controller::factory::IocControllable for #self_ty {
                    fn from_ioc_container(
                        container: &::elif_core::container::IocContainer,
                        _scope: Option<&::elif_core::container::ScopeId>,
                    ) -> Result<Self, String> {
                        // Auto-resolve dependencies from container
                        // This provides automatic dependency injection for common patterns
                        Self::from_container_auto(container)
                    }
                }

                impl #self_ty {
                    /// Auto-generated dependency resolution method
                    /// This attempts to auto-resolve dependencies using Default implementations
                    /// Controllers can override this for custom dependency injection logic
                    fn from_container_auto(_container: &::elif_core::container::IocContainer) -> Result<Self, String> {
                        // Attempt to create the controller using Default implementations of dependencies
                        // This works for services that implement Default trait
                        match Self::try_new_with_defaults() {
                            Ok(controller) => Ok(controller),
                            Err(e) => Err(format!(
                                "Controller {} requires dependency injection. {}\n\
                                Please either:\n\
                                1. Ensure all dependencies implement Default trait, or\n\
                                2. Implement a custom `from_container_auto` method, or\n\
                                3. Use router.controller_from_container::<{}>()\n\
                                4. Register dependencies in IoC container and use Injectable trait",
                                stringify!(#self_ty),
                                e,
                                stringify!(#self_ty)
                            ))
                        }
                    }

                    /// Try to create controller with Default implementations of dependencies
                    fn try_new_with_defaults() -> Result<Self, String> {
                        // This is a fallback - try to call new() expecting Default dependencies
                        // If this fails, it means dependencies don't implement Default
                        Self::try_new_auto()
                    }

                    /// Auto-generated attempt to create controller with Default dependencies
                    fn try_new_auto() -> Result<Self, String> {
                        // This will work if all constructor parameters implement Default
                        Self::new_with_default_deps()
                    }

                    /// Template method for creating with default dependencies
                    /// This gets specialized per controller based on constructor signature
                    fn new_with_default_deps() -> Result<Self, String> {
                        // Generate dependency instances using Default trait
                        #(
                            let #constructor_param_names = #constructor_param_types::default();
                        )*
                        Ok(Self::new(#(#constructor_param_names),*))
                    }
                }
            }
        } else {
            // For controllers without dependencies, provide empty implementation block
            quote! {
                // No additional trait implementations needed for parameterless controllers
                // They should implement Default themselves or use #[derive(Default)]
            }
        };



        let registration_code = if needs_dependency_injection {
            quote! {
                // For IoC controllers, provide helpful compile-time guidance
                const _: () = {
                    // This is a marker to indicate this controller needs IoC container registration
                    // To register: router.controller_from_container::<ControllerType>()
                };
            }
        } else {
            quote! {
                // For simple controllers, use traditional auto-registration
                ::elif_http::__controller_auto_register! {
                    #struct_name,
                    #self_ty
                }
            }
        };

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

            // Registration code (conditional based on DI needs)
            #registration_code
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
