use regex::{Captures, Regex};
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

/// Removes all the characters it can in some wgsl sourcecode without joining any keywords or identifiers together.
pub fn minify_wgsl_source_whitespace(src: &str) -> String {
    let mut src = Cow::<'_, str>::Borrowed(src);

    // Remove whitespace
    let mut new_src = String::new();
    let mut last_char = ' ';
    let mut chars = src.chars().peekable();
    while let Some(current_char) = chars.next() {
        let next_char = *chars.peek().unwrap_or(&' ');

        if current_char.is_whitespace() {
            // Only keep whitespace if it separates identifiers
            if unicode_ident::is_xid_continue(last_char)
                && unicode_ident::is_xid_continue(next_char)
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

    // Get rid of `let _e1=d;return _e1;`
    let re = Regex::new(r"let (_\w*)=([^;]*);return (_\w*)").unwrap();
    let new_src = re.replace_all(&src, |caps: &Captures| {
        if caps[1] == caps[3] {
            format!("return {}", &caps[2])
        } else {
            caps[0].to_owned()
        }
    });

    return new_src.to_string();
}
