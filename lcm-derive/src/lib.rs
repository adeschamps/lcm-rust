extern crate proc_macro;
extern crate syn;

#[macro_use]
extern crate quote;

mod parse;

/// Entry point of the procedural macro.
#[proc_macro_derive(LcmMessage, attributes(lcm))]
pub fn lcm_message(input: proc_macro::TokenStream) -> proc_macro::TokenStream
{
	let input: syn::DeriveInput = syn::parse(input).unwrap();

	// Parse the fields of the struct.
	let fields = if let syn::Data::Struct(syn::DataStruct { fields: syn::Fields::Named(ref fields), ..}) = input.data {
		fields.named.iter().map(|f| parse::Field::from_syn(f)).collect::<Vec<_>>()
	} else { panic!("LCM only supports structs with named fields.") };

	// Calculate the hash of the struct
	let hash = calculate_hash(&fields);
	let hash_included_fields = fields.iter().filter_map(|f| {
		match f.base_type {
			//parse::Ty::User(ref s) => Some(syn::Ident::from(s as &str)),
			parse::Ty::User(ref s) => Some(syn::parse_str::<syn::Expr>(s).expect("Failed to parse field name")),
			_                      => None,
		}
	});

	// Get the name of the struct
	let name = input.ident;

	// Gather the tokens needed for the encode/decode process
	let encode_tokens = fields.iter().map(|f| f.encode_tokens());
	let decode_tokens = fields.iter().map(|f| f.decode_tokens());
	let field_names = fields.iter().map(|f| f.name);
	let size_tokens = fields.iter().map(|f| f.size_tokens());

	// Output the implementation
	let output = quote! {
		impl ::lcm::Message for #name
		{
			const HASH: u64 = {
				const PRE_HASH: u64 = #hash #(+ <#hash_included_fields as ::lcm::Message>::HASH)*;
				(PRE_HASH << 1) + ((PRE_HASH >> 63) & 1)
			};

			fn encode(&self, mut buffer: &mut ::std::io::Write) -> ::std::io::Result<()>
			{
				#(#encode_tokens)*
				Ok(())
			}

			fn decode(mut buffer: &mut ::std::io::Read) -> Result<Self>
			{
				#(#decode_tokens)*
				Ok(#name {
					#(#field_names,)*
				})
			}

			fn size(&self) -> usize
			{
				0
				#(+ #size_tokens)*
			}
		}
	};

	output.into()
}

/// Calculates the hash for the type using its fields.
///
/// This function purposefully does *not* include the message name in the hash.
/// Additionally, it will not include the names of any user defined type in the
/// hash.
///
/// This function was based on the C version of lcmgen but it will not produce
/// identical output as it implements the final shift at generation rather than
/// at runtime.
fn calculate_hash(fields: &Vec<parse::Field>) -> u64
{
	/// Make the hash dependent on the value of the given character.
	///
	/// The order that this function is called in *is* important. This function
	/// was copied from the C version of lcmgen.
	fn hash_update(v: i64, c: i8) -> i64
	{
		((v << 8) ^ (v >> 55)) + c as i64
	}

	/// Make the hash dependent on each character in a string.
	///
	/// This function was copied from the C version of LCM gen.
	fn hash_string_update(v: i64, s: &[u8]) -> i64
	{
		s.iter().fold(hash_update(v, s.len() as i8), |acc, &c| hash_update(acc, c as i8))
	}

	let mut v = 0x12345678i64;

	for f in fields {
		// Hash the field name
		v = hash_string_update(v, f.name.as_ref().as_bytes());

		// Hash the type information *only* if it is a primitive type
		if f.base_type.is_primitive_type() {
			v = hash_string_update(v, f.base_type.as_str().as_bytes());
		}

		// Hash the dimension information
		v = hash_update(v, f.dims.len() as i8);
		for d in f.dims.iter() {
			// Hash the kind of dimension it was and the value of the dimension
			v = hash_update(v, d.mode());
			v = hash_string_update(v, d.as_cow().as_bytes());
		}
	}

	v as u64
}
