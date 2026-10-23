//! Named string-literal union types for the TypeScript emitter, enabled via
//! `[typegen.ts_opts] emit`.

use serde_json::Value;

use super::types::FieldType;

/// The members of a union made only of string literals, in declared order and
/// without duplicates. `None` for any other type.
pub(super) fn string_union_values(ty: &FieldType) -> Option<Vec<&str>> {
	let FieldType::Union {
		variants,
	} = ty
	else {
		return None;
	};
	let mut values = Vec::new();
	for variant in variants {
		let FieldType::Literal {
			value: Value::String(s),
		} = variant
		else {
			return None;
		};
		if !values.contains(&s.as_str()) {
			values.push(s.as_str());
		}
	}
	Some(values)
}

/// Render `export type {name} = "a" | "b";`. With `consts`, the values are
/// also exported as an `as const` array and the type is derived from it.
pub(super) fn render_union(name: &str, values: &[&str], consts: bool) -> String {
	if consts {
		let values_const = const_name(name);
		return format!(
			"export const {values_const} = [{}] as const;\nexport type {name} = (typeof {values_const})[number];\n",
			string_literals(values).join(", "),
		);
	}
	let ty = if values.is_empty() {
		"never".to_string()
	} else {
		string_literals(values).join(" | ")
	};
	format!("export type {name} = {ty};\n")
}

pub(super) fn string_literals(values: &[&str]) -> Vec<String> {
	values.iter().map(|v| format!("{v:?}")).collect()
}

/// The `as const` array behind a union type (`UserRole` → `USER_ROLE_VALUES`).
pub(super) fn const_name(type_name: &str) -> String {
	format!("{}_VALUES", screaming_snake_case(type_name))
}

fn is_separator(c: char) -> bool {
	matches!(c, '_' | ' ' | '-')
}

/// Convert a string to a screaming snake_case name (`UserInfo` -> `USER_INFO`)
fn screaming_snake_case(name: &str) -> String {
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
	out
}

#[cfg(test)]
mod tests {
	use serde_json::json;

	use super::*;
	use crate::typegen::PrimitiveType;

	fn lit(value: Value) -> FieldType {
		FieldType::Literal {
			value,
		}
	}

	fn union(variants: Vec<FieldType>) -> FieldType {
		FieldType::Union {
			variants,
		}
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
	fn string_union_values_variants() {
		let datetime = FieldType::Primitive {
			name: PrimitiveType::Datetime,
		};
		assert_eq!(string_union_values(&datetime), None);
		assert_eq!(
			string_union_values(&union(vec![
				lit(json!("USER")),
				lit(json!("ADMIN")),
				lit(json!("USER")),
				lit(json!("GUEST")),
			])),
			Some(vec!["USER", "ADMIN", "GUEST"]),
		);
		assert_eq!(string_union_values(&union(vec![lit(json!("USER")), lit(json!(200))])), None);
	}

	#[test]
	fn render_union_escapes_values() {
		assert_eq!(
			render_union("Quote", &["say \"hi\"", "plain"], false),
			"export type Quote = \"say \\\"hi\\\"\" | \"plain\";\n",
		);
	}

	#[test]
	fn render_union_with_consts() {
		assert_eq!(
			render_union("UserRole", &["admin", "guest"], true),
			"export const USER_ROLE_VALUES = [\"admin\", \"guest\"] as const;\n\
			 export type UserRole = (typeof USER_ROLE_VALUES)[number];\n",
		);
	}
}
