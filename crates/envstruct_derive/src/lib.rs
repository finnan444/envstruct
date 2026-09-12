mod default_attr;
mod normalize_type_path;

use darling::{ast, FromDeriveInput, FromField, FromVariant};
use default_attr::*;
use normalize_type_path::*;
use proc_macro::TokenStream;
use quote::*;
use syn::spanned::Spanned;

/// Derives the `EnvStruct` trait for a struct or enum.
#[proc_macro_derive(EnvStruct, attributes(env))]
pub fn derive(input: TokenStream) -> TokenStream {
    let derive_input: syn::DeriveInput = syn::parse(input).expect("Failed to parse derive input");
    let receiver = EnvStructInputReceiver::from_derive_input(&derive_input)
        .expect("Failed to parse input for darling receiver");
    quote!(#receiver).into()
}

/// Receiver for the `EnvStruct` derive input.
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(env), supports(any))]
struct EnvStructInputReceiver {
    ident: syn::Ident,
    generics: syn::Generics,
    data: ast::Data<EnvStructVariantReceiver, EnvStructFieldReceiver>,
    title: Option<String>,
    /// Variable selecting the variant of an enum whose variants carry configuration,
    /// relative to the prefix of the enum, as in `#[env(tag = "mode")]`.
    tag: Option<String>,
}

/// Receiver for enum variants of the `EnvStruct`.
#[derive(Debug, FromVariant)]
#[darling(attributes(env))]
struct EnvStructVariantReceiver {
    ident: syn::Ident,
    fields: ast::Fields<EnvStructFieldReceiver>,
    name: Option<String>,
    #[darling(default)]
    flatten: bool,
}

impl EnvStructVariantReceiver {
    /// The payload of a newtype variant, or `None` for a unit variant.
    fn payload(&self) -> Option<&EnvStructFieldReceiver> {
        self.fields.fields.first()
    }

    /// Value of the tag variable that selects this variant.
    fn tag_value(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| snake_case(&self.ident.to_string()))
    }

    /// Prefix the payload of this variant parses from.
    fn payload_prefix_expr(&self) -> proc_macro2::TokenStream {
        if self.flatten {
            quote!(prefix)
        } else {
            let segment = self.tag_value();
            quote!(::envstruct::concat_env_name(prefix, #segment))
        }
    }

    /// Rejects variants the derive cannot express as one tag value with one payload.
    fn validate(&self) -> Option<String> {
        match self.fields.style {
            ast::Style::Struct => {
                return Some(format!(
                    "env enum variant `{}` must be a unit or a newtype variant",
                    self.ident
                ))
            }
            ast::Style::Tuple if self.fields.fields.len() != 1 => {
                return Some(format!(
                    "env enum variant `{}` must hold exactly one payload",
                    self.ident
                ))
            }
            _ => {}
        }

        let payload = self.payload()?;
        if payload.name.is_some()
            || payload.default.is_some()
            || payload.default_note.is_some()
            || payload.title.is_some()
            || payload.used_if.is_some()
            || payload.flatten
            || payload.inline
            || payload.skip
            || payload.secret
        {
            return Some(format!(
                "env enum variant `{}` supports only `with` on its payload; other attributes \
                 belong on the variant or on the payload type",
                self.ident
            ));
        }
        None
    }
}

/// Receiver for the fields of the `EnvStruct`.
#[derive(Debug, FromField)]
#[darling(attributes(env))]
struct EnvStructFieldReceiver {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    name: Option<String>,
    default: Option<DefaultAttr>,
    with: Option<syn::Expr>,
    title: Option<String>,
    used_if: Option<String>,
    #[darling(default)]
    flatten: bool,
    #[darling(default)]
    inline: bool,
    #[darling(default)]
    skip: bool,
    #[darling(default)]
    secret: bool,
    default_note: Option<String>,
}

impl EnvStructFieldReceiver {
    /// Generates a token stream for the field name or index.
    pub fn name_exr(&self, index: usize) -> proc_macro2::TokenStream {
        self.ident
            .as_ref()
            .map(quote::ToTokens::to_token_stream)
            .unwrap_or_else(|| {
                let index = syn::Index::from(index);
                quote!(#index)
            })
    }

    /// Generates a token stream for the field type.
    pub fn type_expr(&self) -> proc_macro2::TokenStream {
        self.with
            .as_ref()
            .map(|ty| quote_spanned! { ty.span() => #ty })
            .unwrap_or({
                let ty = normalize_type_path(&self.ty);
                quote_spanned! { ty.span() => #ty }
            })
    }

    /// Generates a token stream for the default value of the field.
    pub fn default_expr(&self) -> proc_macro2::TokenStream {
        self.default
            .as_ref()
            .map(|default| match default {
                DefaultAttr::String(str) => {
                    quote!(Some(#str))
                }
                DefaultAttr::Type(typ) => {
                    quote!(Some(&#typ.to_string()))
                }
                DefaultAttr::Default => {
                    let ty = normalize_type_path(&self.ty);
                    quote!(Some(&#ty::default().to_string()))
                }
            })
            .unwrap_or_else(|| quote!(None))
    }

    /// Generates a token stream for the environment variable name.
    pub fn var_name_expr(&self) -> proc_macro2::TokenStream {
        let var_name = self.name.clone().unwrap_or_else(|| {
            self.ident
                .as_ref()
                .map(|v| quote!(#v).to_string())
                .unwrap_or_default()
        });

        if self.flatten {
            quote!(&prefix)
        } else {
            quote!(::envstruct::concat_env_name(&prefix, #var_name))
        }
    }

    fn field_name_str(&self) -> String {
        self.ident
            .as_ref()
            .map(|ident| ident.to_string())
            .unwrap_or_default()
    }
}

fn parse_used_if(spec: &str) -> Result<(String, String), String> {
    let Some((field, value)) = spec.split_once('=') else {
        return Err(format!("used_if must be `field=value`, got `{spec}`"));
    };
    if field.is_empty() || value.is_empty() {
        return Err(format!("used_if must be `field=value`, got `{spec}`"));
    }
    Ok((field.to_string(), value.to_string()))
}

fn used_if_expr(
    field: &EnvStructFieldReceiver,
    fields: &ast::Fields<EnvStructFieldReceiver>,
) -> proc_macro2::TokenStream {
    let Some(spec) = &field.used_if else {
        return quote!(None);
    };
    let (sibling_name, value) = match parse_used_if(spec) {
        Ok(parsed) => parsed,
        Err(err) => {
            let ty = &field.ty;
            return quote_spanned! { ty.span() => compile_error!(#err) };
        }
    };
    let Some(sibling) = fields
        .iter()
        .find(|item| item.field_name_str() == sibling_name)
    else {
        let ty = &field.ty;
        let err = format!("used_if references unknown field `{sibling_name}`");
        return quote_spanned! { ty.span() => compile_error!(#err) };
    };
    let sibling_var_name = sibling.var_name_expr();
    let sibling_default = sibling.default_expr();
    quote! {
        Some(::envstruct::UsageUsedIf {
            env_name: #sibling_var_name,
            value: #value.to_string(),
            switch_default: #sibling_default.map(|value| value.to_string()),
            enforced: false,
        })
    }
}

/// `RemoteBackend` becomes `remote_backend`, `HTTPProxy` becomes `http_proxy`, and an
/// already lowercase `gcs` stays as it is.
fn snake_case(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::with_capacity(name.len());
    for (index, ch) in chars.iter().enumerate() {
        if ch.is_uppercase() {
            let ends_word = index > 0 && !chars[index - 1].is_uppercase();
            let starts_word = chars
                .get(index + 1)
                .is_some_and(|next| next.is_lowercase() && index > 0);
            if ends_word || starts_word {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(*ch);
        }
    }
    out
}

fn derive_error(span: proc_macro2::Span, message: &str) -> proc_macro2::TokenStream {
    quote_spanned! { span => compile_error!(#message); }
}

impl EnvStructInputReceiver {
    /// An enum is a single value parsed by its own `FromStr`, unless `tag` declares that its
    /// variants select a configuration of their own.
    fn enum_tokens(&self, variants: &[EnvStructVariantReceiver]) -> proc_macro2::TokenStream {
        for variant in variants {
            if let Some(message) = variant.validate() {
                return derive_error(variant.ident.span(), &message);
            }
        }
        match &self.tag {
            Some(tag) => self.tagged_enum_tokens(tag, variants),
            None => self.unit_enum_tokens(variants),
        }
    }

    /// Implements `EnvParsePrimitive` for an enum of unit variants, listing the variants as
    /// the accepted values of the variable.
    fn unit_enum_tokens(&self, variants: &[EnvStructVariantReceiver]) -> proc_macro2::TokenStream {
        let EnvStructInputReceiver {
            ident, generics, ..
        } = self;
        let (imp, ty, where_clause) = generics.split_for_impl();

        if let Some(variant) = variants.iter().find(|variant| variant.payload().is_some()) {
            let message = format!(
                "env enum `{ident}` has variants with a payload and needs the variable that \
                 selects them, as in `#[env(tag = \"mode\")]`"
            );
            return derive_error(variant.ident.span(), &message);
        }
        if let Some(variant) = variants
            .iter()
            .find(|variant| variant.name.is_some() || variant.flatten)
        {
            let message = format!(
                "env attributes on variant `{}` need `#[env(tag = \"...\")]` on the enum",
                variant.ident
            );
            return derive_error(variant.ident.span(), &message);
        }

        let variant_names: Vec<String> = variants
            .iter()
            .map(|variant| variant.ident.to_string())
            .collect();

        quote_spanned! {ty.span() =>
            impl #imp ::envstruct::EnvParsePrimitive for #ident #ty #where_clause {
                fn parse(val: &str) -> std::result::Result<Self, ::envstruct::BoxError> {
                    Ok(val.parse::<#ident>()?)
                }

                fn usage_type() -> ::envstruct::UsageType {
                    ::envstruct::UsageType::Enum
                }

                fn usage_values() -> Option<Vec<String>> {
                    Some(vec![#( #variant_names.to_string(), )*])
                }
            }
        }
    }

    /// Implements `EnvParseNested` for an enum whose variants carry configuration: the tag
    /// variable selects the variant, and only the selected payload is parsed.
    fn tagged_enum_tokens(
        &self,
        tag: &str,
        variants: &[EnvStructVariantReceiver],
    ) -> proc_macro2::TokenStream {
        let EnvStructInputReceiver {
            ident,
            generics,
            title,
            ..
        } = self;
        let (imp, ty, where_clause) = generics.split_for_impl();

        let values: Vec<String> = variants
            .iter()
            .map(EnvStructVariantReceiver::tag_value)
            .collect();
        for (index, value) in values.iter().enumerate() {
            if values[..index].contains(value) {
                let message = format!("env enum value `{value}` selects more than one variant");
                return derive_error(variants[index].ident.span(), &message);
            }
        }

        let parse_arms = variants.iter().map(|variant| {
            let variant_ident = &variant.ident;
            let value = variant.tag_value();
            match variant.payload() {
                None => quote!(#value => Ok(Self::#variant_ident),),
                Some(payload) => {
                    let payload_type = payload.type_expr();
                    let payload_prefix = variant.payload_prefix_expr();
                    quote_spanned! { payload.ty.span() =>
                        #value => Ok(Self::#variant_ident(
                            #payload_type::parse_from_env_var(#payload_prefix, None)?.into(),
                        )),
                    }
                }
            }
        });

        let usage_variants = variants.iter().map(|variant| {
            let value = variant.tag_value();
            match variant.payload() {
                None => quote!(::envstruct::TaggedVariant::unit(#value)),
                Some(payload) => {
                    let payload_type = payload.type_expr();
                    let payload_prefix = variant.payload_prefix_expr();
                    quote_spanned! { payload.ty.span() =>
                        ::envstruct::TaggedVariant::payload(
                            #value,
                            #payload_type::get_usage_tree(#payload_prefix, None)?,
                        )
                    }
                }
            }
        });

        let enum_title = match title {
            Some(title) => quote!(Some(#title.to_string())),
            None => quote!(None),
        };

        quote! {
            #[allow(clippy::useless_conversion)]
            impl #imp ::envstruct::EnvParseNested for #ident #ty #where_clause {
                fn parse_from_env_var(prefix: impl AsRef<str>, default: Option<&str>) -> std::result::Result<Self, ::envstruct::EnvStructError> {
                    let prefix = prefix.as_ref();
                    let tag = ::envstruct::EnumTag::read(
                        ::envstruct::concat_env_name(prefix, #tag),
                        default,
                    )?;
                    match tag.value.as_str() {
                        #( #parse_arms )*
                        _ => Err(tag.unknown_value_error(&[#( #values, )*])),
                    }
                }

                fn get_usage_tree(prefix: impl AsRef<str>, default: Option<&str>) -> std::result::Result<::envstruct::UsageTree, ::envstruct::EnvStructError> {
                    let prefix = prefix.as_ref();
                    Ok(::envstruct::tagged_enum_usage(
                        ::envstruct::concat_env_name(prefix, #tag),
                        default,
                        #enum_title,
                        vec![#( #usage_variants, )*],
                    ))
                }
            }
        }
    }

    /// Implements `EnvParseNested` for a struct: every field parses from its own variable.
    fn struct_tokens(
        &self,
        fields: &ast::Fields<EnvStructFieldReceiver>,
    ) -> proc_macro2::TokenStream {
        let EnvStructInputReceiver {
            ident,
            generics,
            title,
            ..
        } = self;
        let (imp, ty, where_clause) = generics.split_for_impl();

        if self.tag.is_some() {
            let message = format!("env `tag` applies to an enum, not to struct `{ident}`");
            return derive_error(ident.span(), &message);
        }

        if let Some(field) = fields
            .iter()
            .find(|field| field.default.is_some() && field.default_note.is_some())
        {
            let span = field
                .ident
                .as_ref()
                .map(|ident| ident.span())
                .unwrap_or_else(|| field.ty.span());
            return derive_error(
                span,
                "env `default` and `default_note` cannot be set together",
            );
        }

        let field_exprs: Vec<_> = fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let field_name = field.name_exr(index);
                let field_type = field.type_expr();
                let var_default = field.default_expr();
                let var_name_expr = field.var_name_expr();

                if field.skip {
                    quote_spanned! {field.ty.span() =>
                        #field_name: Default::default()
                    }
                } else {
                    quote_spanned! {field.ty.span() =>
                        #field_name: #field_type::parse_from_env_var(#var_name_expr, #var_default)?.into()
                    }
                }
            })
            .collect();

        let inspect_exprs: Vec<_> = fields
            .iter()
            .filter(|field| !field.skip)
            .map(|field| {
                let field_type = field.type_expr();
                let var_default = field.default_expr();
                let var_name_expr = field.var_name_expr();
                let field_name_str = field.field_name_str();
                let flatten = field.flatten;
                let inline = field.inline;
                let secret = field.secret;
                let title_expr = match &field.title {
                    Some(title) => quote!(Some(#title.to_string())),
                    None => quote!(None),
                };
                let default_note_expr = match &field.default_note {
                    Some(note) => quote!(Some(#note.to_string())),
                    None => quote!(None),
                };
                let used_if = used_if_expr(field, fields);
                quote_spanned! {field.ty.span() =>
                    ::envstruct::attach_field_usage(
                        #field_type::get_usage_tree(#var_name_expr, #var_default)?,
                        ::envstruct::FieldUsageMeta {
                            field_name: #field_name_str,
                            title: #title_expr,
                            flatten: #flatten,
                            inline: #inline,
                            used_if: #used_if,
                            secret: #secret,
                            default_note: #default_note_expr,
                        },
                    )
                }
            })
            .collect();

        let struct_title = match title {
            Some(title) => quote!(Some(#title.to_string())),
            None => quote!(None),
        };

        quote! {
            #[allow(clippy::useless_conversion)]
            impl #imp ::envstruct::EnvParseNested for #ident #ty #where_clause {
                fn parse_from_env_var(prefix: impl AsRef<str>, default: Option<&str>) -> std::result::Result<Self, ::envstruct::EnvStructError> {
                    let _ = default;
                    Ok(Self {
                        #( #field_exprs, )*
                    })
                }

                fn get_usage_tree(prefix: impl AsRef<str>, default: Option<&str>) -> std::result::Result<::envstruct::UsageTree, ::envstruct::EnvStructError> {
                    let _ = default;
                    let nested: Vec<Vec<::envstruct::UsageItem>> = vec![#( #inspect_exprs, )*];
                    Ok(::envstruct::UsageTree {
                        title: #struct_title,
                        kind: ::envstruct::UsageTreeKind::Struct,
                        items: nested.into_iter().flatten().collect(),
                    })
                }
            }
        }
    }
}

impl ToTokens for EnvStructInputReceiver {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let impl_block = match &self.data {
            ast::Data::Enum(variants) => self.enum_tokens(variants),
            ast::Data::Struct(fields) => self.struct_tokens(fields),
        };

        tokens.extend(impl_block);
    }
}

#[test]
fn test_snake_case() {
    assert_eq!(snake_case("Local"), "local");
    assert_eq!(snake_case("RemoteBackend"), "remote_backend");
    assert_eq!(snake_case("HTTPProxy"), "http_proxy");
    assert_eq!(snake_case("gcs"), "gcs");
    assert_eq!(snake_case("Mode2"), "mode2");
}
