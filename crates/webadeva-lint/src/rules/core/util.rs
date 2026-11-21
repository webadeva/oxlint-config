//! Helpers shared by the core rules.

/// JS whitespace as matched by `\s` and stripped by `String#trim`.
pub fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\u{9}' | '\u{A}' | '\u{B}' | '\u{C}' | '\u{D}' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// JS `String#trim`.
pub fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

/// The parent as ESTree sees it: oxc keeps `ParenthesizedExpression` nodes,
/// ESTree drops them.
pub fn estree_parent(
    nodes: &oxc_semantic::AstNodes<'_>,
    id: oxc_semantic::NodeId,
) -> oxc_semantic::NodeId {
    let mut p = nodes.parent_id(id);
    while matches!(nodes.kind(p), oxc_ast::AstKind::ParenthesizedExpression(_)) {
        let next = nodes.parent_id(p);
        if next == p {
            break;
        }
        p = next;
    }
    p
}

/// End offset of the token before `offset`, skipping whitespace and comments.
pub fn prev_token_end(source: &str, comments: &[oxc_ast::Comment], offset: u32) -> u32 {
    let mut i = offset as usize;
    loop {
        while i > 0 {
            let prev = source[..i].chars().next_back().unwrap_or(' ');
            if is_js_space(prev) {
                i -= prev.len_utf8();
            } else {
                break;
            }
        }
        if let Some(c) = comments.iter().find(|c| c.span.end as usize == i) {
            i = c.span.start as usize;
            continue;
        }
        return i as u32;
    }
}

/// Start offset of the first token at or after `offset`, skipping
/// whitespace and comments.
pub fn next_token_start(source: &str, comments: &[oxc_ast::Comment], offset: u32) -> u32 {
    let mut i = offset as usize;
    loop {
        while i < source.len() {
            let c = source[i..].chars().next().unwrap_or('x');
            if is_js_space(c) {
                i += c.len_utf8();
            } else {
                break;
            }
        }
        if let Some(c) = comments.iter().find(|c| c.span.start as usize == i) {
            i = c.span.end as usize;
            continue;
        }
        return i as u32;
    }
}
