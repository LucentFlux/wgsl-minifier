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
        name += &NEXT_LETTERS[id % NEXT_LETTERS.len()].to_string();
        id /= FIRST_LETTERS.len();
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

/// Iterates through all objects in a module and re-generates any names or identifiers to smaller ones.
///
/// This method has to re-create the types arena, as changing the names may mean the types are no longer unique.
///
/// Does not remove names on entry points, or constants with overrides.
pub fn remove_identifiers(module: &mut naga::Module) {
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
        if constant.r#override == naga::Override::None {
            constant.name = Some(name_from_count(&mut name_counter));
        }
        constant.ty = type_handle_mapping[&constant.ty];
    }
    for (_, global) in module.global_variables.iter_mut() {
        global.name = Some(name_from_count(&mut name_counter));
        global.ty = type_handle_mapping[&global.ty];
    }
    for (_, function) in module.functions.iter_mut() {
        remove_fn_identifiers(&mut name_counter, function, &type_handle_mapping)
    }
    for entry in module.entry_points.iter_mut() {
        remove_fn_identifiers(&mut name_counter, &mut entry.function, &type_handle_mapping)
    }
}

/// Returns true only if the given character cannot be in an identifier. Returning false gives no information.
fn non_identifier_char(c: char) -> bool {
    match c {
        '(' | ')' | '{' | '}' | '[' | ']' | '<' | '>' | ',' | '+' | '*' | '/' | '!' | '\\'
        | '"' | '\'' | '|' | '=' | '^' | '&' | ';' | ':' | '?' | '%' | '@' | '#' | '~' | '.'
        | '£' | '$' | '`' => true,
        _ => false,
    }
}

/// Removes all the whitespace it can in some wgsl sourcecode without joining any keywords or identifiers together.
pub fn minify_wgsl_source_whitespace(src: &str) -> String {
    let mut new_src = String::new();
    let mut last_char = ' ';

    let mut chars = src.chars().peekable();

    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            let next_char = chars.peek().unwrap_or(&' ');
            if !last_char.is_whitespace()
                && !non_identifier_char(*next_char)
                && !non_identifier_char(last_char)
            {
                new_src.push(' ');
                last_char = ' ';
            }
        } else {
            new_src.push(c);
            last_char = c;
        }
    }

    return new_src;
}
