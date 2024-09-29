use proc_macro2::Literal;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use regex::Regex;
use rust_format::Formatter;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

fn main() {
    let header_path = find_kernel_header_path().expect("Could not find input-event-codes.h");

    let header_content =
        fs::read_to_string(&header_path).expect("Failed to read input-event-codes.h file");

    let define_pattern = r"#define\s+(\w+)\s+(0x[0-9a-fA-F]+|\d+)";
    let regex = Regex::new(define_pattern).unwrap();

    let defines = {
        let mut defines = Vec::new();
        for cap in regex.captures_iter(&header_content) {
            let name = snake_case_to_camel_case(&cap[1]);
            let name_ident = format_ident!("{}", name);
            let value = Literal::from_str(&cap[2]).expect("Failed to convert input event values");
            defines.push((name, name_ident, value));
        }
        defines
    };

    let mut output = TokenStream::new();

    {
        let mut variants = TokenStream::new();
        for (name, name_ident, value) in &defines {
            if !name.starts_with("Ev") {
                continue;
            }
            variants.extend(quote! {
                #name_ident = #value,
            });
        }
        output.extend(quote!(
            #[derive(Debug, Copy, Clone, PartialEq, Eq, num_enum::IntoPrimitive, num_enum::FromPrimitive)]
            #[cfg_attr(feature = "defmt", derive(defmt::Format))]
            #[repr(u16)]
            pub enum EventType {
                #variants
                #[num_enum(default)]
                EvUnknown = 0xffff,
            }
        ));
    }

    {
        let mut variants = TokenStream::new();
        for (name, name_ident, value) in &defines {
            if !name.starts_with("Syn") {
                continue;
            }
            variants.extend(quote! {
                #name_ident = #value,
            });
        }
        output.extend(quote!(
            #[derive(Debug, Copy, Clone, PartialEq, Eq, num_enum::IntoPrimitive, num_enum::FromPrimitive)]
            #[cfg_attr(feature = "defmt", derive(defmt::Format))]
            #[repr(u16)]
            pub enum Syn {
                #variants
                #[num_enum(default)]
                SynUnknown = 0xffff,
            }
        ));
    }

    {
        let mut variants = TokenStream::new();
        let mut buttons: Vec<TokenStream> = vec![];
        for (name, name_ident, value) in &defines {
            if !name.starts_with("Key") && !name.starts_with("Btn") {
                continue;
            }
            if name == "BtnMisc"
                || name == "BtnMouse"
                || name == "BtnJoystick"
                || name == "BtnGamepad"
                || name == "BtnDigi"
                || name == "BtnWheel"
                || name == "BtnTriggerHappy"
            {
                continue;
            }
            if name == "KeyUnknown" {
                variants.extend(quote! {

                    #[num_enum(default)]
                });
            }
            variants.extend(quote! {
                #name_ident = #value,
            });
            if name.starts_with("Btn") {
                buttons.push(quote!(Self::#name_ident));
            }
        }
        output.extend(quote!(
            #[derive(Debug, Copy, Clone, PartialEq, Eq, num_enum::IntoPrimitive, num_enum::FromPrimitive)]
            #[cfg_attr(feature = "defmt", derive(defmt::Format))]
            #[repr(u16)]
            pub enum Key {
                #variants
            }
            impl Key {
                #[allow(non_upper_case_globals)]
                pub const BtnMisc: Self = Self::Btn0;
                #[allow(non_upper_case_globals)]
                pub const BtnMouse: Self = Self::BtnLeft;
                #[allow(non_upper_case_globals)]
                pub const BtnJoystick: Self = Self::BtnTrigger;
                #[allow(non_upper_case_globals)]
                pub const BtnGamepad: Self = Self::BtnSouth;
                #[allow(non_upper_case_globals)]
                pub const BtnDigi: Self = Self::BtnToolPen;
                #[allow(non_upper_case_globals)]
                pub const BtnWheel: Self = Self::BtnGearDown;
                #[allow(non_upper_case_globals)]
                pub const BtnTriggerHappy: Self = Self::BtnTriggerHappy1;

                pub fn is_btn(&self) -> bool {
                    matches!(self, #(#buttons)|*)
                }
            }
        ));
    }

    {
        let mut variants = TokenStream::new();
        for (name, name_ident, value) in &defines {
            if !name.starts_with("Rel") {
                continue;
            }
            variants.extend(quote! {
                #name_ident = #value,
            });
        }
        output.extend(quote!(
            #[derive(Debug, Copy, Clone, PartialEq, Eq, num_enum::IntoPrimitive, num_enum::FromPrimitive)]
            #[cfg_attr(feature = "defmt", derive(defmt::Format))]
            #[repr(u16)]
            pub enum RelAxis {
                #variants
                #[num_enum(default)]
                RelUnknown = 0xffff,
            }
        ));
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let dest_path = out_dir.join("input_event_codes.rs");

    let rust_formatter = rust_format::PrettyPlease::default();

    fs::write(dest_path, rust_formatter.format_tokens(output).unwrap())
        .expect("Failed to write Rust code file");
}

fn snake_case_to_camel_case(name: &str) -> String {
    name.to_lowercase()
        .split('_')
        .map(|s| {
            let mut chars = s.chars();
            match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

fn find_kernel_header_path() -> Option<PathBuf> {
    if let Ok(path) = env::var("INPUT_EVENT_CODES_PATH") {
        return Some(path.into());
    }

    let common_path = PathBuf::from("/usr/include/linux/input-event-codes.h");
    if common_path.exists() {
        return Some(common_path);
    }

    let kernel_version = std::process::Command::new("uname")
        .arg("-r")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())?;
    let src_path = PathBuf::from(format!(
        "/usr/src/linux-headers-{}/include/linux/input-event-codes.h",
        kernel_version.trim()
    ));
    if src_path.exists() {
        return Some(src_path);
    }

    None
}
