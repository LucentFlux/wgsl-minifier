#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

use std::borrow::Cow;
use std::collections::HashMap;

const FIRST_LETTERS: [char; 52] = [
    'A', 'a', 'B', 'b', 'C', 'c', 'D', 'd', 'E', 'e', 'F', 'f', 'G', 'g', 'H', 'h', 'I', 'i', 'J',
    'j', 'K', 'k', 'L', 'l', 'M', 'm', 'N', 'n', 'O', 'o', 'P', 'p', 'Q', 'q', 'R', 'r', 'S', 's',
    'T', 't', 'U', 'u', 'V', 'v', 'W', 'w', 'X', 'x', 'Y', 'y', 'Z', 'z',
];
const NEXT_LETTERS: [char; 63] = [
    'A', 'a', 'B', 'b', 'C', 'c', 'D', 'd', 'E', 'e', 'F', 'f', 'G', 'g', 'H', 'h', 'I', 'i', 'J',
    'j', 'K', 'k', 'L', 'l', 'M', 'm', 'N', 'n', 'O', 'o', 'P', 'p', 'Q', 'q', 'R', 'r', 'S', 's',
    'T', 't', 'U', 'u', 'V', 'v', 'W', 'w', 'X', 'x', 'Y', 'y', 'Z', 'z', '1', '2', '3', '4', '5',
    '6', '7', '8', '9', '0', '_',
];

fn name_from_count(count: &mut usize) -> String {
    let mut id = *count;
    *count += 1;

    let mut name = String::from(FIRST_LETTERS[id % FIRST_LETTERS.len()]);
    id /= FIRST_LETTERS.len();

    while id != 0 {
        name.push(NEXT_LETTERS[id % NEXT_LETTERS.len()]);
        id /= NEXT_LETTERS.len();
    }

    name
}

fn remove_type_identifiers(
    name_counter: &mut usize,
    ty: &naga::Type,
    map: &HashMap<naga::Handle<naga::Type>, naga::Handle<naga::Type>>,
) -> naga::Type {
    naga::Type {
        name: Some(name_from_count(name_counter)),
        inner: match ty.inner.clone() {
            naga::TypeInner::Pointer { base, space } => naga::TypeInner::Pointer {
                base: map[&base],
                space,
            },
            naga::TypeInner::Array { base, size, stride } => naga::TypeInner::Array {
                base: map[&base],
                size,
                stride,
            },
            naga::TypeInner::Struct { members, span } => naga::TypeInner::Struct {
                members: members
                    .into_iter()
                    .map(|member| naga::StructMember {
                        name: Some(name_from_count(name_counter)),
                        ty: map[&member.ty],
                        binding: member.binding,
                        offset: member.offset,
                    })
                    .collect(),
                span,
            },
            naga::TypeInner::BindingArray { base, size } => naga::TypeInner::BindingArray {
                base: map[&base],
                size,
            },
            non_referencing => non_referencing,
        },
    }
}

fn remove_fn_identifiers(
    name_counter: &mut usize,
    function: &mut naga::Function,
    type_handle_mapping: &HashMap<naga::Handle<naga::Type>, naga::Handle<naga::Type>>,
) {
    function.name = Some(name_from_count(name_counter));
    if let Some(result) = function.result.as_mut() {
        result.ty = type_handle_mapping[&result.ty];
    }

    for (_, local) in function.local_variables.iter_mut() {
        local.name = Some(name_from_count(name_counter));
        local.ty = type_handle_mapping[&local.ty];
    }
    for argument in function.arguments.iter_mut() {
        argument.name = Some(name_from_count(name_counter));
        argument.ty = type_handle_mapping[&argument.ty];
    }
    function.named_expressions.clear();
}

fn remove_identifiers(module: &mut naga::Module) {
    let mut name_counter = 0;

    let mut new_types = naga::UniqueArena::new();
    let mut type_handle_mapping = HashMap::new();
    for (old_handle, old_type) in module.types.iter() {
        let new_type = remove_type_identifiers(&mut name_counter, old_type, &type_handle_mapping);
        let new_handle = new_types.insert(new_type, module.types.get_span(old_handle));
        type_handle_mapping.insert(old_handle, new_handle);
    }
    module.types = new_types;

    for (_, constant) in module.constants.iter_mut() {
        if !matches!(
            module.global_expressions.try_get(constant.init).unwrap(),
            naga::Expression::Override(_)
        ) {
            constant.name = None;
        }
        constant.ty = type_handle_mapping[&constant.ty];
    }
    for (_, global) in module.global_variables.iter_mut() {
        global.name = Some(name_from_count(&mut name_counter));
        global.ty = type_handle_mapping[&global.ty];
    }
    for (_, function) in module.functions.iter_mut() {
        remove_fn_identifiers(&mut name_counter, function, &type_handle_mapping);
    }
    for entry in module.entry_points.iter_mut() {
        remove_fn_identifiers(&mut name_counter, &mut entry.function, &type_handle_mapping);
    }
}

/// Performs minification on a naga module, changing any names or identifiers to smaller ones.
///
/// This method has to re-create the types arena, as changing the names may mean the types are no longer unique.
///
/// Does not remove names on entry points, or on constants with overrides.
pub fn minify_module(module: &mut naga::Module) {
    // Compact
    naga::compact::compact(module, naga::compact::KeepUnused::Yes);
    // Remove any remaining identifiers
    remove_identifiers(module);
}

fn is_numeric(c: char) -> bool {
    c.is_ascii_digit()
}

/// Removes all the characters it can in some wgsl sourcecode without joining any keywords or identifiers together.
#[must_use]
pub fn minify_wgsl_source(src: &str) -> String {
    let mut src = Cow::<'_, str>::Borrowed(src);

    // Remove whitespace
    let mut new_src = String::new();
    let mut last_char = ' ';
    let mut chars = src.chars().peekable();
    while let Some(current_char) = chars.next() {
        let next_char = *chars.peek().unwrap_or(&' ');

        if current_char.is_whitespace() {
            // Only keep whitespace if it separates identifiers,
            // or separates a hyphen from a literal (since older versions of the spec require whitespace)
            if (unicode_ident::is_xid_continue(last_char)
                && unicode_ident::is_xid_continue(next_char))
                || (last_char == '-' && (is_numeric(next_char) || next_char == '.'))
            {
                new_src.push(' ');
                last_char = ' ';
            }
            continue;
        }

        new_src.push(current_char);
        last_char = current_char;
    }
    src = Cow::Owned(new_src);

    // Anything of the form `,}` or `,)` or `,]` can have the comma removed
    new_src = String::new();
    chars = src.chars().peekable();
    while let Some(current_char) = chars.next() {
        let next_char = *chars.peek().unwrap_or(&' ');

        if current_char == ',' && matches!(next_char, '}' | ')' | ']') {
            continue;
        }

        new_src.push(current_char);
    }
    src = Cow::Owned(new_src);

    // Get rid of double parentheses
    let mut parentheses = HashMap::new(); // Map from parenthesis starts to ends
    let mut unclosed_stack = Vec::new();
    for (i, c) in src.chars().enumerate() {
        if c == '(' {
            unclosed_stack.push(i);
        } else if c == ')' {
            let start = unclosed_stack.pop().expect("wgsl parentheses are balanced");
            parentheses.insert(start, i);
        }
    }
    assert!(unclosed_stack.is_empty());
    new_src = String::new();
    let mut to_drop_stack = Vec::new();
    for (i, c) in src.chars().enumerate() {
        if let Some(outer_end) = parentheses.get(&i)
            && let Some(inner_end) = parentheses.get(&(i + 1))
            && *outer_end == *inner_end + 1
        {
            to_drop_stack.push(*outer_end);
            continue;
        }
        if let Some(next_to_skip) = to_drop_stack.last()
            && *next_to_skip == i
        {
            to_drop_stack.pop();
            continue;
        }
        new_src.push(c);
    }
    src = Cow::Owned(new_src);

    src.to_string()
}
