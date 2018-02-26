use std::borrow::Cow;
use quote;
use syn;

/// Represents a field in the Rust struct.
#[derive(Debug)]
pub struct Field {
    /// The name of the field.
    pub name: syn::Ident,

    /// The base type of the field.
    ///
    /// E.g., a `Vec<i8>` has the base type of `Ty::Int8`.
    pub base_type: Ty,

    /// The dimensions of the field.
    ///
    /// This vector is empty if the field is not some form of array. If the
    /// field is an array, then this vector contains one entry for each array
    /// dimension. This behavior is copied from the C version of lcmgen.
    pub dims: Vec<Dim>,
}
impl Field {
    /// Parses the `syn::Field` to create a new `Field`.
    pub fn from_syn(input: &syn::Field) -> Self {
        // The name is easy but figuring out the base type and the dimensions
        // is more involved.
        let base_type = Ty::get_base_type(&input.ty);
        let dims = Dim::get_dims(&input.ty, &input.attrs);

        Field {
            name: input.ident.expect("Unnamed field"),
            base_type,
            dims,
        }
    }

    /// Returns the tokens needed to encode this field.
    ///
    /// This will handle the field dimensions, if any.
    pub fn encode_tokens(&self) -> quote::Tokens {
        let name = self.name;

        // The easiest case are the non-arrays.
        if self.dims.is_empty() {
            quote! { ::lcm::Marshall::encode(&self.#name, &mut buffer)?; }
        } else {
            let mut tokens = quote! { ::lcm::Marshall::encode(item, &mut buffer)?; };
            for dim in self.dims.iter().rev() {
                tokens = match *dim {
                    Dim::Fixed(_) => quote! {for item in item.iter() { #tokens }},
                    Dim::Variable(ref s) => {
                        let size_name = syn::Ident::from(s as &str);
                        quote! {
                            if self.#size_name as usize != item.len() {
                                return Err(::lcm::error::EncodeError::SizeMismatch {
                                    size_var: stringify!(#size_name).into(),
                                    expected: self.#size_name as i64,
                                    found: item.len()
                                });
                            }
                            for item in item.iter() { #tokens }
                        }
                    }
                };
            }

            quote! {let item = &self.#name; #tokens}
        }
    }

    /// Returns the tokens needed to decode this field.
    ///
    /// This will handle the field dimensions, if any.
    pub fn decode_tokens(&self) -> quote::Tokens {
        let name = self.name;

        if self.dims.is_empty() {
            quote! {let #name = ::lcm::Marshall::decode(&mut buffer)?; }
        } else {
            let mut tokens = quote! { ::lcm::Marshall::decode(&mut buffer) };
            let mut need_q_mark = true;
            for d in self.dims.iter().rev() {
                tokens = match *d {
                    Dim::Fixed(s) => {
                        let inner = (0..s).map(|_| tokens.clone());
                        let old_q_mark = need_q_mark;
                        need_q_mark = false;

                        if old_q_mark {
                            quote! { Ok([ #(#inner?,)* ]) }
                        } else {
                            quote! { [ #(#inner,)* ] }
                        }
                    }
                    Dim::Variable(ref s) => {
                        let dim_name = syn::Ident::from(s as &str);
                        need_q_mark = true;
                        quote! {
                            (0..#dim_name)
                                .map(|_| #tokens)
                                .collect::<Result<_, ::lcm::error::DecodeError>>()
                        }
                    }
                };
            }

            if need_q_mark {
                quote! { let #name = #tokens?; }
            } else {
                quote! { let #name = #tokens; }
            }
        }
    }

    /// Return the tokens used to get the size of this field.
    ///
    /// If this field is *not* a user defined base type and *not* a string,
    /// then the returning tokens will not involve a function call to determine
    /// the size of the field. If the field additionally does *not* include any
    /// variable sized array, this function returns a set of tokens that can be
    /// resolved to a constant at compile time.
    pub fn size_tokens(&self) -> quote::Tokens {
        // If this isn't a string or a user type, we can make this a constant.
        match self.base_type {
            Ty::String | Ty::User(_) => self.size_tokens_nonconst(),
            _ => self.size_tokens_const(),
        }
    }

    /// Return the tokens for a type that does *not* require a function call.
    ///
    /// Calling this on an incorrect type will produce tokens that will not
    /// compile.
    fn size_tokens_const(&self) -> quote::Tokens {
        let dim_multipliers = self.dims.iter().map(|d| match *d {
            Dim::Fixed(s) => quote! { #s },
            Dim::Variable(ref s) => {
                let dim_name = syn::Ident::from(s as &str);
                quote! { self.#dim_name as usize }
            }
        });

        let type_size = self.base_type.size();

        quote!{ (#type_size #(* #dim_multipliers)*) }
    }

    /// Return the tokens for a type that does require a function call.
    ///
    /// Calling this on an incorrect type will produce tokens that *do* compile
    /// but will be less efficient than otherwise possible.
    fn size_tokens_nonconst(&self) -> quote::Tokens {
        let name = self.name;

        if self.dims.is_empty() {
            quote! { ::lcm::Marshall::size(&self.#name)}
        } else {
            let mut tokens = quote! { ::lcm::Marshall::size(&item) };
            for _ in self.dims.iter().skip(1).rev() {
                tokens = quote!{ item.iter().map(|item| #tokens).sum::<usize>() }
            }

            quote!{ self.#name.iter().map(|item| #tokens).sum::<usize>()}
        }
    }
}

/// Get the inner type of a `Vec`.
///
/// I.e., if this function is given `Vec<E>` then it will return `E`. If the
/// supplied type is not a generic, then this function will panic. If the
/// supplied type has more than one generic argument, only the first will be
/// returned (because this was meant for `Vec` but just happens to work for any
/// generic).
fn get_vec_inner_type(t: &syn::Type) -> &syn::Type {
    let segs = match *t {
        syn::Type::Path(syn::TypePath {
            path: syn::Path { ref segments, .. },
            ..
        }) => segments,
        _ => panic!("Bug: `get_vec_inner_type` called on non-Vec type (1)"),
    };

    match *segs.iter().last().unwrap() {
        syn::PathSegment {
            arguments:
                syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                    ref args, ..
                }),
            ..
        } => {
            if let syn::GenericArgument::Type(ref ty) = args[0] {
                ty
            } else {
                panic!("Bug: `get_vec_inner_type` called on non-Vec type (2)");
            }
        }
        _ => panic!("Bug: `get_vec_inner_type` called on non-Vec type (3)"),
    }
}

/// Convert a `syn::Type` to the string form of the type.
fn type_to_string(t: &syn::Type) -> String {
    // This whole function is a bit inefficient
    let path = match *t {
        syn::Type::Path(syn::TypePath { ref path, .. }) => path,
        _ => panic!("Bug: `type_to_string` called on unknown type"),
    };

    let mut res = if path.leading_colon.is_some() {
        String::from("::")
    } else {
        String::new()
    };

    for pair in path.segments.pairs() {
        let (seg, punctuated) = match pair {
            syn::punctuated::Pair::Punctuated(t, _) => (t, true),
            syn::punctuated::Pair::End(t) => (t, false),
        };

        // Add the type name
        res.push_str(seg.ident.as_ref());

        // Handle generics
        match *seg {
            syn::PathSegment {
                arguments:
                    syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                        ref args,
                        ..
                    }),
                ..
            } => {
                res.push('<');

                for i in 0..args.len() - 1 {
                    if let syn::GenericArgument::Type(ref ty) = args[i] {
                        res.push_str(&type_to_string(ty));
                        res.push(',');
                    } else {
                        panic!("Generic argument had no type");
                    }
                }

                if let syn::GenericArgument::Type(ref ty) = args[args.len() - 1] {
                    res.push_str(&type_to_string(ty));
                } else {
                    panic!("Generic argument had no type");
                }
                res.push('>');
            }
            _ => {}
        }

        // Add the punctuation if necessary
        if punctuated {
            res.push_str("::");
        }
    }

    res
}

/// Represents the data type of the field.
///
/// This type can either be one of LCM's primitives or a "user defined" type.
/// Note that this means that any unsigned integers will be considered
/// user-defined, but they should fail appropriately at compile time.
#[derive(Clone, Debug)]
pub enum Ty {
    /// `int8_t`
    Int8,

    /// `int16_t`
    Int16,

    /// `int32_t`
    Int32,

    /// `int64_t`
    Int64,

    /// `float`
    Float,

    /// `double`
    Double,

    /// `string`
    String,

    /// `boolean`
    Boolean,

    /// Anything that is not an LCM primitive.
    User(String),
}
impl Ty {
    /// Returns `true` if this is an LCM primitive.
    pub fn is_primitive_type(&self) -> bool {
        match *self {
            Ty::User(_) => false,
            _ => true,
        }
    }

    /// Returns the string for this type.
    pub fn as_str(&self) -> &str {
        match *self {
            Ty::Int8 => "int8_t",
            Ty::Int16 => "int16_t",
            Ty::Int32 => "int32_t",
            Ty::Int64 => "int64_t",
            Ty::Float => "float",
            Ty::Double => "double",
            Ty::String => "string",
            Ty::Boolean => "boolean",
            Ty::User(ref s) => s,
        }
    }

    /// Returns the size of this type.
    ///
    /// If the type does not have a size known at generation time (i.e., user
    /// defined types and strings), this function will panic.
    fn size(&self) -> usize {
        match *self {
            Ty::Int8 => ::std::mem::size_of::<i8>(),
            Ty::Int16 => ::std::mem::size_of::<i16>(),
            Ty::Int32 => ::std::mem::size_of::<i32>(),
            Ty::Int64 => ::std::mem::size_of::<i64>(),
            Ty::Float => ::std::mem::size_of::<f32>(),
            Ty::Double => ::std::mem::size_of::<f64>(),
            Ty::Boolean => ::std::mem::size_of::<i8>(),
            _ => panic!("Bug: tried to get fixed size of non-primitive"),
        }
    }

    /// Returns the `Type` that represents the base data type of the `syn::Type`.
    fn get_base_type(t: &syn::Type) -> Self {
        // There are two base types allowed here. The `Path` type contains all
        // of the primitives and `Vec`. The `Array` type is fixed-size arrays.
        match *t {
            syn::Type::Path(syn::TypePath {
                path: syn::Path { ref segments, .. },
                ..
            }) => {
                // This is either a `Vec` or a primitive
                match segments.iter().last().unwrap().ident.as_ref() {
                    "i8" => Ty::Int8,
                    "i16" => Ty::Int16,
                    "i32" => Ty::Int32,
                    "i64" => Ty::Int64,
                    "f32" => Ty::Float,
                    "f64" => Ty::Double,
                    "bool" => Ty::Boolean,
                    "String" => Ty::String,
                    "Vec" => Ty::get_base_type(get_vec_inner_type(t)),
                    _ => Ty::User(type_to_string(t)),
                }
            }
            syn::Type::Array(syn::TypeArray { ref elem, .. }) => {
                // This is an array. Go a level deeper to figure out the base type
                Ty::get_base_type(elem.as_ref())
            }
            _ => panic!("Type must either be an LCM primitive or an array"),
        }
    }
}

/// Represents a dimension for a field consisting of one or more arrays.
#[derive(Debug)]
pub enum Dim {
    /// A dimension whose size is known at compile time.
    Fixed(usize),

    /// A dimension whose size is defined by another field in the message.
    Variable(String),
}
impl Dim {
    /// Returns the mode of this dimension.
    ///
    /// This really does nothing more than pretend to be a C-like enum in order
    /// to achieve the same behavior as the C version of lcmgen.
    pub fn mode(&self) -> i8 {
        match *self {
            Dim::Fixed(_) => 0,
            Dim::Variable(_) => 1,
        }
    }

    /// Returns the `String` representation of this dimension.
    pub fn as_cow(&self) -> Cow<str> {
        match *self {
            Dim::Fixed(s) => Cow::from(format!("{}", s)),
            Dim::Variable(ref s) => Cow::from(s as &str),
        }
    }

    /// Parses a type an its attributes to determine the dimensions.
    fn get_dims(t: &syn::Type, attrs: &Vec<syn::Attribute>) -> Vec<Self> {
        let mut res = Vec::new();
        let mut vec_dims = Dim::get_vec_dims(attrs);
        Dim::get_dims_internal(t, &mut vec_dims, &mut res);

        assert!(vec_dims.is_empty(), "Too many vector dimensions specified");

        res
    }

    /// Recursive utility function for `Dim::get_dims`.
    ///
    /// Should not be called from anywhere except `Dim::get_dims`.
    fn get_dims_internal(t: &syn::Type, vec_dims: &mut Vec<Self>, res: &mut Vec<Self>) {
        match *t {
            syn::Type::Path(syn::TypePath {
                path: syn::Path { ref segments, .. },
                ..
            }) => {
                match segments.iter().last().unwrap().ident.as_ref() {
                    "Vec" => {
                        res.push(
                            vec_dims
                                .pop()
                                .expect("Missing size for variable length array"),
                        );
                        Dim::get_dims_internal(get_vec_inner_type(t), vec_dims, res);
                    }
                    _ => { /* lcmgen (C version) does not store this info */ }
                }
            }
            syn::Type::Array(syn::TypeArray {
                ref elem,
                len:
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Int(ref lit),
                        ..
                    }),
                ..
            }) => {
                res.push(Dim::Fixed(lit.value() as usize));
                Dim::get_dims_internal(elem.as_ref(), vec_dims, res);
            }
            _ => panic!("Type must either be an LCM primitive or an array"),
        }
    }

    /// Returns all of the variable length dimensions specified in the
    /// attributes list.
    ///
    /// Should not be called from anywhere except `Dim::get_dims`.
    fn get_vec_dims(attrs: &Vec<syn::Attribute>) -> Vec<Self> {
        let mut sizes = Vec::new();

        for a in attrs {
            match a.interpret_meta() {
                Some(syn::Meta::List(ref meta_list)) if meta_list.ident.as_ref() == "lcm" => {
                    // This is an `lcm(...)` attribute
                    for n in meta_list.nested.iter() {
                        match *n {
                            syn::NestedMeta::Meta(syn::Meta::NameValue(syn::MetaNameValue {
                                ref ident,
                                lit: syn::Lit::Str(ref var_name),
                                ..
                            })) if ident.as_ref() == "length" =>
                            {
                                // This is a length attribute
                                sizes.push(Dim::Variable(var_name.value()));
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        sizes
    }
}
