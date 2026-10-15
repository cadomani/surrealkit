use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

/// Wraps a string in double quotes
/// Used for printing strings in typescript files
macro_rules! string_literal {
	($text: expr) => {
		format!("\"{}\"", $text)
	};
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StringUnion {
	pub name: String,
	pub items: HashSet<String>,
	pub aliases: HashSet<String>,
	pub export: bool,
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

	pub fn add(&mut self, item: &str) {
		self.items.insert(item.to_string());
	}

	pub fn to_type(&self, using_const_array: bool) -> String {
		format!(
			"{}type {} = {};",
			self.export_as_string(),
			self.name,
			if using_const_array {
				format!("(typeof {})[number]", self.const_name())
			} else {
				self.as_literal_strings().join(" | ")
			}
		)
	}

	pub fn to_const_array(&self) -> String {
		format!(
			"{}const {} = [{}] as const;",
			self.export_as_string(),
			self.const_name(),
			self.as_literal_strings().join(",")
		)
	}

	/// Transforms the hashset of `items` into a vector of literal strings.
	/// Literal strings in this context are strings with double quotes `"` surronding them.
	#[inline(always)]
	fn as_literal_strings(&self) -> Vec<String> {
		self.items.iter().map(|f| string_literal!(f)).collect::<Vec<String>>()
	}

	#[inline(always)]
	fn const_name(&self) -> String {
		format!("{}_VALUES", self.name.to_ascii_uppercase())
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
	map: HashMap<String, StringUnion>,
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

	pub fn get_union(&self, union_name: &str) -> Option<&StringUnion> {
		self.map.get(union_name)
		// TODO: Search for aliases of the given union name
	}

	// /// Converts a string union map to a series of typescript type. Extracting any unions into their own type.
	// pub fn to_ts_extracted(&self, name: &str, emit_const: bool) -> String {}

	/// Converts a string union map into one typescript type.
	pub fn to_ts(&self, name: &str, emit_const: bool, type_name_override: Option<String>) -> String {
		let export_str = self.export_as_string();
		let const_name = format!("{}_VALUES", name.to_ascii_uppercase());
		let type_name = type_name_override.unwrap_or(pascal_case(name));
		format!(
			"{}{}",
			// If we're emitting a constant, do that first
			if emit_const {
				format!(
					"{}const {} = {{\n{}}} as const;\n",
					export_str,
					const_name,
					self.map
						.iter()
						.map(|(k, v)| format!("  {}: [{}]\n", k, v.as_literal_strings().join(",")))
						.collect::<Vec<String>>()
						.join(",")
				)
			} else {
				"".to_string()
			},
			if emit_const {
				format!(
					"\n{}type {} = {{\n[K in keyof typeof {}]: (typeof {})[K][number]  \n}};",
					export_str, type_name, const_name, const_name
				)
			} else {
				format!(
					"{}interface {} {{\n{}}}",
					export_str,
					type_name,
					self.map
						.iter()
						.map(|(k, v)| format!("  {}: {}", k, v.as_literal_strings().join(" | ")))
						.collect::<Vec<String>>()
						.join(",\n")
				)
			}
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
