use std::borrow::Cow;
use std::mem::size_of;

use proc_macro2::{Span, TokenStream};
use quote::{quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{Expr, Lit, LitStr};

/// Stringify a value at runtime.
fn stringify_at_runtime(src: impl ToTokens) -> TokenStream {
    quote_spanned! {src.span().resolved_at(Span::call_site())=>
        ::std::convert::Into::<::yew::virtual_dom::AttrValue>::into(#src)
    }
}

/// Create `AttrValue` construction calls.
///
/// This is deliberately not implemented for strings to preserve spans.
pub trait Stringify {
    /// Try to turn the value into a string literal.
    fn try_into_lit(&self) -> Option<LitStr>;
    /// Create `AttrValue` however possible.
    fn stringify(&self) -> TokenStream;

    /// Optimize literals to `&'static str`, otherwise keep the value as is.
    fn optimize_literals(&self) -> TokenStream
    where
        Self: ToTokens,
    {
        self.optimize_literals_tagged().to_token_stream()
    }

    /// Like `optimize_literals` but tags static or dynamic strings with [Value]
    fn optimize_literals_tagged(&self) -> Value
    where
        Self: ToTokens,
    {
        if let Some(lit) = self.try_into_lit() {
            Value::Static(lit.to_token_stream())
        } else {
            Value::Dynamic(self.to_token_stream())
        }
    }
}
impl<T: Stringify + ?Sized> Stringify for &T {
    fn try_into_lit(&self) -> Option<LitStr> {
        (*self).try_into_lit()
    }

    fn stringify(&self) -> TokenStream {
        (*self).stringify()
    }
}

/// A stringified value that can be either static (known at compile time) or dynamic (known only at
/// runtime)
pub enum Value {
    Static(TokenStream),
    Dynamic(TokenStream),
}

impl ToTokens for Value {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Value::Static(tt) | Value::Dynamic(tt) => tt.clone(),
        });
    }
}

impl Stringify for LitStr {
    fn try_into_lit(&self) -> Option<LitStr> {
        Some(self.clone())
    }

    fn stringify(&self) -> TokenStream {
        quote_spanned! {self.span()=>
            ::yew::virtual_dom::AttrValue::Static(#self)
        }
    }
}

impl Stringify for Lit {
    fn try_into_lit(&self) -> Option<LitStr> {
        let mut buf = [0; size_of::<char>()];
        let s: Cow<'_, str> = match self {
            Lit::Str(v) => return v.try_into_lit(),
            Lit::Char(v) => (&*v.value().encode_utf8(&mut buf)).into(),
            Lit::Int(v) => v.base10_digits().into(),
            Lit::Float(v) => v.base10_digits().into(),
            Lit::Bool(v) => if v.value() { "true" } else { "false" }.into(),
            Lit::Byte(v) => v.value().to_string().into(),
            Lit::Verbatim(_) | Lit::ByteStr(_) => return None,
            _ => unreachable!("unknown Lit {:?}", self),
        };
        Some(LitStr::new(&s, self.span()))
    }

    fn stringify(&self) -> TokenStream {
        self.try_into_lit()
            .as_ref()
            .map_or_else(|| stringify_at_runtime(self), Stringify::stringify)
    }
}

impl Stringify for Expr {
    fn try_into_lit(&self) -> Option<LitStr> {
        if let Expr::Lit(v) = self {
            v.lit.try_into_lit()
        } else {
            None
        }
    }

    fn stringify(&self) -> TokenStream {
        self.try_into_lit()
            .as_ref()
            .map_or_else(|| stringify_at_runtime(self), Stringify::stringify)
    }
}
