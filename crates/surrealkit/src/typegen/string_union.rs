use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::typegen::FieldType;

/// Wraps a string in double quotes
/// Used for printing strings in typescript files
macro_rules! string_literal {
	($text: expr) => {
		format!("\"{}\"", $text)
	};
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringUnion {
	pub name: String,
	pub items: HashSet<String>,
	/// Aliases are used for any types encountered while parsing which are structurally identical to this type, but have a different name
	pub aliases: HashSet<String>,
	pub export: bool,
}

impl PartialEq for StringUnion {
	fn eq(&self, other: &Self) -> bool {
		self.items == other.items
	}
}

impl StringUnion {
	pub fn new(name: &str, export: bool) -> Self {
		Self {
			name: name.to_string(),
			export,
			items: HashSet::new(),
			aliases: HashSet::new(),
		}
	}

	pub fn add_alias(&mut self, alias_name: &str) {
		self.aliases.insert(alias_name.to_string());
	}

	pub fn add(&mut self, item: &str) {
		self.items.insert(item.to_string());
	}

	/// Converts the string union to a series of CSV (Comma-separated-values) elements
	pub fn to_csv(&self) -> String {
		self.as_literal_strings().join(",")
	}

	/// Converts the string union to a series of PSV (Pipe-separated-values) elements
	pub fn to_psv(&self) -> String {
		self.as_literal_strings().join(" | ")
	}

	pub fn to_emitted_string(&self, emit_const: bool) -> String {
		if emit_const {
			let const_name = self.const_name();
			let export_str = self.export_as_string();
			format!(
				"\n{export} const {const} = [{}] as const;\n{export}type {} = (typeof {const})[number];\n",
				self.to_csv(),
				self.name,
				const=const_name,
				export=export_str,
			)
		} else {
			format!("\n{}type {} = {};\n", self.export_as_string(), self.name, self.to_psv())
		}
	}

	/// Transforms the hashset of `items` into a vector of literal strings.
	/// Literal strings in this context are strings with double quotes `"` surronding them.
	#[inline(always)]
	fn as_literal_strings(&self) -> Vec<String> {
		self.items.iter().map(|f| string_literal!(f)).collect::<Vec<String>>()
	}

	#[inline(always)]
	fn const_name(&self) -> String {
		format!("{}_VALUES", screaming_snake_case(&self.name))
	}

	#[inline(always)]
	fn export_as_string(&self) -> &'static str {
		if self.export {
			"export "
		} else {
			""
		}
	}
}

#[derive(Debug, Clone)]
pub struct StringUnionMap {
	pub map: HashMap<String, StringUnion>,
	pub export: bool,
}

impl StringUnionMap {
	pub fn new(export: bool) -> Self {
		Self {
			export,
			map: HashMap::new(),
		}
	}

	pub fn add_to_union(&mut self, union_name: &str, value: &str, export: Option<bool>) {
		self.map
			.entry(union_name.to_string())
			.or_insert_with(|| StringUnion::new(union_name, export.unwrap_or(self.export)))
			.add(value);
	}

	pub fn add_union(&mut self, union: &StringUnion) -> String {
		if let Some(existing) = self.map.get_mut(&union.name) {
			if *existing != *union {
				// If a union already exists AND the values are different, panic. This should not happen.
				panic!(
					"Duplicate Union name detected: name '{}' is used for differing unions {} and {}",
					&union.name,
					self.map
						.get(&union.name)
						.unwrap_or(&StringUnion::new(&union.name, true))
						.to_psv(),
					union.to_psv(),
				)
			}
			existing.name.clone()
		} else {
			if let Some(equiv_union) = self.map.iter_mut().find_map(|(_, v)| {
				if *v == *union {
					Some(v)
				} else {
					None
				}
			}) {
				equiv_union.add_alias(&union.name);
				equiv_union.name.clone()
			} else {
				self.map.insert(union.name.clone(), union.clone());
				union.name.clone()
			}
		}
	}

	pub fn get_union(&self, union_name: &str) -> Option<&StringUnion> {
		let get_result = self.map.get(union_name);
		match get_result {
			Some(_) => get_result,
			// Search for aliases of the given union name
			None => self.map.iter().find_map(|(_, union)| {
				if union.aliases.contains(union_name) {
					Some(union)
				} else {
					None
				}
			}),
		}
	}

	// /// Converts a string union map to a series of typescript type. Extracting any unions into their own type.
	// pub fn to_ts_extracted(&self, name: &str, emit_const: bool) -> String {}

	/// Converts a string union map into one typescript type.
	pub fn to_ts(&self, name: &str, emit_const: bool) -> String {
		let export_str = self.export_as_string();
		let const_name = screaming_snake_case(&format!("{}_VALUES", name));
		if emit_const {
			return format!(
				"{}const {} = {{\n{}}} as const;\n\n{}type {} = {{\n[K in keyof typeof {}]: (typeof {})[K][number]  \n}};",
				export_str,
				const_name,
				self.map
					.iter()
					.map(|(k, v)| format!("  {}: [{}]\n", k, v.to_csv()))
					.collect::<Vec<String>>()
					.join(","),
				export_str,
				name,
				const_name,
				const_name
			);
		}

		format!(
			"{}interface {} {{\n{}}}",
			export_str,
			name,
			self.map
				.iter()
				.map(|(k, v)| format!("  {}: {}", k, v.to_psv()))
				.collect::<Vec<String>>()
				.join(",\n")
		)
	}

	#[inline(always)]
	fn export_as_string(&self) -> &'static str {
		if self.export {
			"export "
		} else {
			""
		}
	}
}

/// Convert a table name to a PascalCase interface name (`user` → `User`,
/// `user_profile` → `UserProfile`).
pub fn pascal_case(name: &str) -> String {
	let mut out = String::new();
	for part in name.split(|c: char| !c.is_ascii_alphanumeric()) {
		if part.is_empty() {
			continue;
		}
		let mut chars = part.chars();
		if let Some(first) = chars.next() {
			out.extend(first.to_uppercase());
			out.push_str(chars.as_str());
		}
	}
	if out.is_empty() {
		name.to_string()
	} else {
		out
	}
}

fn is_separator(c: char) -> bool {
	matches!(c, '_' | ' ' | '-')
}

/// Convert a string to a screaming snake_case name (`UserInfo` -> `USER_INFO`)
pub fn screaming_snake_case(name: &str) -> String {
	let mut out = String::new();
	let mut iter = name.chars();
	let mut can_split = false;
	let mut capitalized_streak = false;
	if let Some(c) = iter.next() {
		can_split = c.is_alphanumeric();
		capitalized_streak = c.is_ascii_uppercase();
		out.push(c.to_ascii_uppercase())
	}
	for char in iter {
		if char.is_ascii_uppercase() && can_split && !capitalized_streak {
			out.push('_');
			out.push(char);
			can_split = false;
			capitalized_streak = true;
		} else if can_split && is_separator(char) {
			out.push('_');
			can_split = false;
			capitalized_streak = false;
		} else {
			out.push(char.to_ascii_uppercase());
			can_split = true;
			capitalized_streak = char.is_ascii_uppercase();
		}
	}
	if out.is_empty() {
		name.to_string()
	} else {
		out
	}
}

/// Inspects a FieldType and returns a boolean indicating if the type is a pure string union or not.
pub fn is_string_union(field_type: &FieldType) -> bool {
	match field_type {
		FieldType::Union {
			variants,
		} => variants.iter().all(|f| {
			matches!(
				f,
				FieldType::Literal {
					value: Value::String(_)
				}
			)
		}),
		_ => false,
	}
}

#[cfg(test)]
mod tests {
	use serde_json::json;

	use super::*;

	#[test]
	fn pascal_case_variants() {
		assert_eq!(pascal_case("user"), "User");
		assert_eq!(pascal_case("user_profile"), "UserProfile");
		assert_eq!(pascal_case("blog-post"), "BlogPost");
		assert_eq!(pascal_case("User"), "User");
	}

	#[test]
	fn screaming_snake_case_variants() {
		assert_eq!(screaming_snake_case("User"), "USER");
		assert_eq!(screaming_snake_case("UserProfile"), "USER_PROFILE");
		assert_eq!(screaming_snake_case("BlogPost"), "BLOG_POST");
		assert_eq!(screaming_snake_case("UserProfileInformation"), "USER_PROFILE_INFORMATION");
		assert_eq!(screaming_snake_case("$User"), "$USER");
		assert_eq!(screaming_snake_case("$User_VALUES"), "$USER_VALUES");
	}

	#[test]
	fn is_string_union_variants() {
		assert_eq!(
			is_string_union(&FieldType::Primitive {
				name: crate::typegen::PrimitiveType::Datetime
			}),
			false
		);
		assert_eq!(
			is_string_union(&FieldType::Primitive {
				name: crate::typegen::PrimitiveType::String
			}),
			false
		);
		assert_eq!(
			is_string_union(&FieldType::Union {
				variants: vec![
					FieldType::Literal {
						value: json!("USER")
					},
					FieldType::Literal {
						value: json!("ADMIN")
					},
					FieldType::Literal {
						value: json!("GUEST")
					}
				]
			}),
			true
		);
		assert_eq!(
			is_string_union(&FieldType::Union {
				variants: vec![
					FieldType::Literal {
						value: json!("USER")
					},
					FieldType::Literal {
						value: json!(200)
					},
					FieldType::Literal {
						value: json!("GUEST")
					}
				]
			}),
			false
		);
	}
}
