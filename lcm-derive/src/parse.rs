use quote;
use syn;

#[derive(Debug)]
pub struct Field
{
	pub name: String,
	pub base_type: Ty,
	pub dims: Vec<Dim>,
}
impl Field
{
	pub fn from_syn(input: &syn::Field) -> Self
	{
		// The name is easy but figuring out the base type and the dimensions
		// is more involved.
		let base_type = Ty::get_base_type(&input.ty);
		let dims = Dim::get_dims(&input.ty, &input.attrs);

		Field { name: input.ident.expect("Unnamed field").to_string(), base_type, dims }
	}

	pub fn encode_tokens(&self) -> quote::Tokens
	{
		let name = syn::Ident::from(&self.name as &str);

		// The easiest case are the non-arrays.
		if self.dims.is_empty() {
			quote! { ::lcm::Message::encode(&self.#name, &mut buffer)?; }
		} else {
			let mut tokens = quote! { ::lcm::Message::encode(&item, &mut buffer)?; };
			for dim in self.dims.iter().rev() {
				tokens = match *dim {
					Dim::Fixed(_) => quote! {for item in item.iter() { #tokens }},
					Dim::Variable(ref s) => {
						let size_name = syn::Ident::from(s as &str);
						quote! {
							if self.#size_name as usize != item.len() {
								return Err(::std::io::Error::new(::std::io::ErrorKind::Other, "Size is larger than vector"));
							}
							for item in item.iter() { #tokens }
						}
					},
				};
			}

			quote! {let item = &self.#name; #tokens}
		}
	}

	pub fn decode_tokens(&self) -> quote::Tokens
	{
		let name = syn::Ident::from(&self.name as &str);

		if self.dims.is_empty() {
			quote! {let #name = ::lcm::Message::decode(&mut buffer)?; }
		} else {
			let mut tokens = quote! { ::lcm::Message::decode(&mut buffer) };
			let mut need_q_mark = true;
			for d in self.dims.iter().rev() {
				tokens = match *d {
					Dim::Fixed(s) => {
						let inner = (0..s).map(|_| tokens.clone());
						let old_q_mark = need_q_mark;
						need_q_mark = false;

						if old_q_mark {
							quote! { [ #(#inner?,)* ] }
						} else {
							quote! { [ #(#inner,)* ] }
						}
					},
					Dim::Variable(ref s) => {
						let dim_name = syn::Ident::from(s as &str);
						need_q_mark = true;
						quote! { (0..#dim_name).map(|_| #tokens).collect::<Result<_>>() }
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

	pub fn size_tokens(&self) -> quote::Tokens
	{
		// If this isn't a string or a user type, we can make this a constant.
		match self.base_type {
			Ty::String | Ty::User(_) => self.size_tokens_nonconst(),
			_                        => self.size_tokens_const(),
		}
	}

	fn size_tokens_const(&self) -> quote::Tokens
	{
		let dim_multipliers = self.dims.iter().map(|d| {
			match *d {
				Dim::Fixed(s) => quote! { #s },
				Dim::Variable(ref s) => {
					let dim_name = syn::Ident::from(s as &str);
					quote! { self.#dim_name as usize }
				}
			}
		});

		let type_size = self.base_type.size();

		quote!{ (#type_size #(* #dim_multipliers)*) }
	}

	fn size_tokens_nonconst(&self) -> quote::Tokens
	{
		let name = syn::Ident::from(&self.name as &str);

		if self.dims.is_empty() {
			quote! { ::lcm::Message::size(&self.#name)}
		} else {
			let mut tokens = quote! { ::lcm::Message::size(&item) };
			for _ in self.dims.iter().skip(1).rev() {
				tokens = quote!{ item.iter().map(|item| #tokens).sum::<usize>() }
			}

			quote!{ self.#name.iter().map(|item| #tokens).sum::<usize>()}
		}
	}
}

fn get_vec_inner_type(t: &syn::Type) -> &syn::Type
{
	let segs = match *t {
		syn::Type::Path(syn::TypePath { path: syn::Path { ref segments, .. }, ..}) => segments,
		_ => panic!("Bug: `get_vec_inner_type` called on non-Vec type (1)"),
	};

	match *segs.iter().last().unwrap() {
		syn::PathSegment {
			arguments: syn::PathArguments::AngleBracketed(
				syn::AngleBracketedGenericArguments { ref args, .. }
			),
			..
		} => {
			if let syn::GenericArgument::Type(ref ty) = args[0] {
				ty
			} else { panic!("Bug: `get_vec_inner_type` called on non-Vec type (2)"); }
		},
		_ => panic!("Bug: `get_vec_inner_type` called on non-Vec type (3)")
	}
}

#[derive(Clone, Debug)]
pub enum Ty
{
	Int8,
	Int16,
	Int32,
	Int64,
	Float,
	Double,
	String,
	Boolean,
	User(String)
}
impl Ty
{
	pub fn is_primitive_type(&self) -> bool
	{
		match *self {
			Ty::User(_) => false,
			_           => true
		}
	}

	pub fn as_str(&self) -> &str
	{
		match *self {
			Ty::Int8        => "int8_t",
			Ty::Int16       => "int16_t",
			Ty::Int32       => "int32_t",
			Ty::Int64       => "int64_t",
			Ty::Float       => "float",
			Ty::Double      => "double",
			Ty::String      => "string",
			Ty::Boolean     => "boolean",
			Ty::User(ref s) => s
		}
	}

	fn size(&self) -> usize
	{
		match *self {
			Ty::Int8    => ::std::mem::size_of::<i8>(),
			Ty::Int16   => ::std::mem::size_of::<i16>(),
			Ty::Int32   => ::std::mem::size_of::<i32>(),
			Ty::Int64   => ::std::mem::size_of::<i64>(),
			Ty::Float   => ::std::mem::size_of::<f32>(),
			Ty::Double  => ::std::mem::size_of::<f64>(),
			Ty::Boolean => ::std::mem::size_of::<i8>(),
			_           => panic!("Tried to get fixed size of non-primitive"),
		}
	}

	fn get_base_type(t: &syn::Type) -> Self
	{
		// There are two base types allowed here. The `Path` type contains all
		// of the primitives and `Vec`. The `Array` type is fixed-size arrays.
		match *t {
			syn::Type::Path(syn::TypePath { path: syn::Path { ref segments, .. }, .. }) => {
				// This is either a `Vec` or a primitive
				match segments.iter().last().unwrap().ident.as_ref() {
					"i8"     => Ty::Int8,
					"i16"    => Ty::Int16,
					"i32"    => Ty::Int32,
					"i64"    => Ty::Int64,
					"f32"    => Ty::Float,
					"f64"    => Ty::Double,
					"bool"   => Ty::Boolean,
					"String" => Ty::String,
					"Vec"    => Ty::get_base_type(get_vec_inner_type(t)),
					n @ _    => Ty::User(n.to_string()),
				}
			},
			syn::Type::Array(syn::TypeArray { ref elem, ..}) => {
				// This is an array. Go a level deeper to figure out the base type
				Ty::get_base_type(elem.as_ref())
			},
			_ => panic!("Type must either be an LCM primitive or an array"),
		}
	}
}

#[derive(Debug)]
pub enum Dim
{
	Fixed(usize),
	Variable(String),
}
impl Dim
{
	pub fn dim_type(&self) -> i8
	{
		match *self {
			Dim::Fixed(_)    => 0,
			Dim::Variable(_) => 1,
		}
	}

	pub fn as_string(&self) -> String
	{
		match *self {
			Dim::Fixed(s)        => format!("{}", s),
			Dim::Variable(ref s) => s.clone(),
		}
	}

	fn get_dims(t: &syn::Type, attrs: &Vec<syn::Attribute>) -> Vec<Self>
	{
		let mut res = Vec::new();
		let mut vec_dims = Dim::get_vec_dims(attrs);
		Dim::get_dims_internal(t, &mut vec_dims, &mut res);

		assert!(vec_dims.is_empty(), "Too many vector dimensions specified");

		res
	}

	fn get_dims_internal(t: &syn::Type, vec_dims: &mut Vec<Self>, res: &mut Vec<Self>)
	{
		match *t {
			syn::Type::Path(syn::TypePath { path: syn::Path { ref segments, .. }, .. }) => {
				match segments.iter().last().unwrap().ident.as_ref() {
					"Vec" => {
						res.push(vec_dims.pop().expect("Missing size for variable length array"));
						Dim::get_dims_internal(get_vec_inner_type(t), vec_dims, res);
					},
					_     => { /* lcmgen (C version) does not store this info */},
				}
			},
			syn::Type::Array(syn::TypeArray { ref elem, len: syn::Expr::Lit(syn::ExprLit{ lit: syn::Lit::Int(ref lit), ..}), ..}) => {
				res.push(Dim::Fixed(lit.value() as usize));
				Dim::get_dims_internal(elem.as_ref(), vec_dims, res);
			},
			_ => panic!("Type must either be an LCM primitive or an array"),
		}
	}

	fn get_vec_dims(attrs: &Vec<syn::Attribute>) -> Vec<Self>
	{
		let mut sizes = Vec::new();

		for a in attrs {
			match a.interpret_meta() {
				Some(syn::Meta::List(ref meta_list)) if meta_list.ident.as_ref() == "lcm" => {
					// This is an `lcm(...)` attribute
					for n in meta_list.nested.iter() {
						match *n {
							syn::NestedMeta::Meta(
								syn::Meta::NameValue(
									syn::MetaNameValue {
										ref ident,
										lit: syn::Lit::Str(ref var_name),
										..
									}
								)
							) if ident.as_ref() == "length" => {
								// This is a length attribute
								sizes.push(Dim::Variable(var_name.value()));
							},
							_ => {},
						}
					}
				},
				_ => {},
			}
		}

		sizes
	}
}
